use std::cmp::min;
use std::collections::HashMap;
use std::ffi::OsString;
use std::io::{ErrorKind, Write};
use std::os::fd::RawFd;
use std::os::unix::io::{AsRawFd, FromRawFd};
use std::os::unix::process::CommandExt;
use std::process::Command;
use std::time::{Duration, Instant};

use anyhow::{bail, format_err, Result};
use mio::net::{TcpListener, TcpStream};
use mio::unix::SourceFd;
use mio::{Events, Interest, Poll, Token};

use proxmox_io::ByteBuffer;
use proxmox_lang::error::io_err_other;
use proxmox_sys::linux::pty::{make_controlling_terminal, PTY};

const MSG_TYPE_DATA: u8 = 0;
const MSG_TYPE_RESIZE: u8 = 1;
//const MSG_TYPE_PING: u8 = 2;

fn remove_number(buf: &mut ByteBuffer) -> Option<usize> {
    loop {
        if let Some(pos) = &buf.iter().position(|&x| x == b':') {
            let data = buf.remove_data(*pos);
            buf.consume(1); // the ':'
            let len = match std::str::from_utf8(&data) {
                Ok(len_str) => match len_str.parse() {
                    Ok(len) => len,
                    Err(err) => {
                        eprintln!("error parsing number: '{err}'");
                        break;
                    }
                },
                Err(err) => {
                    eprintln!("error decoding number: '{err}'");
                    break;
                }
            };
            return Some(len);
        } else if buf.len() > 20 {
            buf.consume(20);
        } else {
            break;
        }
    }
    None
}

fn process_queue(buf: &mut ByteBuffer, pty: &mut PTY) -> Option<usize> {
    if buf.is_empty() {
        return None;
    }

    loop {
        if buf.len() < 2 {
            break;
        }

        let msgtype = buf[0] - b'0';

        if msgtype == MSG_TYPE_DATA {
            buf.consume(2);
            if let Some(len) = remove_number(buf) {
                return Some(len);
            }
        } else if msgtype == MSG_TYPE_RESIZE {
            buf.consume(2);
            if let Some(cols) = remove_number(buf) {
                if let Some(rows) = remove_number(buf) {
                    pty.set_size(cols as u16, rows as u16).ok()?;
                }
            }
        // ignore incomplete messages
        } else {
            buf.consume(1);
            // ignore invalid or ping (msgtype 2)
        }
    }

    None
}

type TicketResult = Result<(Box<[u8]>, Box<[u8]>)>;

/// Reads from the stream and returns the first line and the rest
fn read_ticket_line(
    stream: &mut TcpStream,
    buf: &mut ByteBuffer,
    timeout: Duration,
) -> TicketResult {
    let mut poll = Poll::new()?;
    poll.registry()
        .register(stream, Token(0), Interest::READABLE)?;
    let mut events = Events::with_capacity(1);

    let now = Instant::now();
    let mut elapsed = Duration::new(0, 0);

    loop {
        poll.poll(&mut events, Some(timeout - elapsed))?;
        if !events.is_empty() {
            match buf.read_from(stream) {
                Ok(n) => {
                    if n == 0 {
                        bail!("connection closed before authentication");
                    }
                }
                Err(err) if err.kind() == ErrorKind::WouldBlock => {}
                Err(err) => return Err(err.into()),
            }

            if buf[..].contains(&b'\n') {
                break;
            }

            if buf.is_full() {
                bail!("authentication data is incomplete: {:?}", &buf[..]);
            }
        }

        elapsed = now.elapsed();
        if elapsed > timeout {
            bail!("timed out");
        }
    }

    let newline_idx = &buf[..].iter().position(|&x| x == b'\n').unwrap();

    let line = buf.remove_data(*newline_idx);
    buf.consume(1); // discard newline

    match line.iter().position(|&b| b == b':') {
        Some(pos) => {
            let (username, ticket) = line.split_at(pos);
            Ok((username.into(), ticket[1..].into()))
        }
        None => bail!("authentication data is invalid"),
    }
}

fn authenticate(username: &[u8], ticket: &[u8], options: &Options, listen_port: u16) -> Result<()> {
    let mut post_fields: Vec<(&str, &str)> = Vec::with_capacity(5);
    post_fields.push(("username", std::str::from_utf8(username)?));
    post_fields.push(("password", std::str::from_utf8(ticket)?));
    post_fields.push(("path", &options.acl_path));
    if let Some(perm) = options.acl_permission.as_ref() {
        post_fields.push(("privs", perm));
    }

    // if the listen-port was passed indirectly via an FD, it's encoded also in the ticket so that
    // the access system can enforce that the users actually can access that port.
    let port_str;
    if options.listen_port.is_fd() {
        port_str = listen_port.to_string();
        post_fields.push(("port", &port_str));
    }

    let url = format!(
        "http://localhost:{}/api2/json/access/ticket",
        options.api_daemon_port
    );

    match ureq::post(&url).send_form(&post_fields[..]) {
        Ok(res) if res.status() == 200 => Ok(()),
        Ok(res) | Err(ureq::Error::Status(_, res)) => {
            let code = res.status();
            bail!("invalid authentication - {code} {}", res.status_text())
        }
        Err(err) => bail!("authentication request failed - {err}"),
    }
}

fn listen_and_accept(
    hostname: &str,
    listen_port: &PortOrFd,
    timeout: Duration,
) -> Result<(TcpStream, u16)> {
    let listener = match listen_port {
        PortOrFd::Fd(fd) => unsafe { std::net::TcpListener::from_raw_fd(*fd) },
        PortOrFd::Port(port) => std::net::TcpListener::bind((hostname, *port as u16))?,
    };
    let port = listener.local_addr()?.port();
    let mut listener = TcpListener::from_std(listener);
    let mut poll = Poll::new()?;

    poll.registry()
        .register(&mut listener, Token(0), Interest::READABLE)?;

    let mut events = Events::with_capacity(1);

    let now = Instant::now();
    let mut elapsed = Duration::new(0, 0);

    loop {
        poll.poll(&mut events, Some(timeout - elapsed))?;
        if !events.is_empty() {
            let (stream, client) = listener.accept()?;
            println!("client connection: {client:?}");
            return Ok((stream, port));
        }

        elapsed = now.elapsed();
        if elapsed > timeout {
            bail!("timed out");
        }
    }
}

fn run_pty<'a>(mut full_cmd: impl Iterator<Item = &'a OsString>) -> Result<PTY> {
    let cmd_exe = full_cmd.next().unwrap();
    let params = full_cmd; // rest

    let (mut pty, secondary_name) = PTY::new().map_err(io_err_other)?;

    let mut filtered_env: HashMap<OsString, OsString> = std::env::vars_os()
        .filter(|&(ref k, _)| {
            k == "PATH"
                || k == "USER"
                || k == "HOME"
                || k == "LANG"
                || k == "LANGUAGE"
                || k.to_string_lossy().starts_with("LC_")
        })
        .collect();
    filtered_env.insert("TERM".into(), "xterm-256color".into());

    let mut command = Command::new(cmd_exe);

    command.args(params).env_clear().envs(&filtered_env);

    unsafe {
        command.pre_exec(move || {
            make_controlling_terminal(&secondary_name).map_err(io_err_other)?;
            Ok(())
        });
    }

    command.spawn()?;

    pty.set_size(80, 20)?;
    Ok(pty)
}

const TCP: Token = Token(0);
const PTY: Token = Token(1);

const CMD_HELP: &str = "\
Usage: proxmox-termproxy [OPTIONS] --path <path> <listen-port> -- <terminal-cmd>...

Arguments:
  <listen-port>           Port or file descriptor to listen for TCP connections
  <terminal-cmd>...       The command to run connected via a proxied PTY

Options:
      --authport <authport>       Port to relay auth-request, default 85
      --port-as-fd                Use <listen-port> as file descriptor.
      --path <path>               ACL object path to test <perm> on.
      --perm <perm>               Permission to test.
      -h, --help                  Print help
";

#[derive(Debug)]
enum PortOrFd {
    Port(u16),
    Fd(RawFd),
}

impl PortOrFd {
    fn from_cli(value: u64, use_as_fd: bool) -> Result<PortOrFd> {
        if use_as_fd {
            if value > RawFd::MAX as u64 {
                bail!("FD value too big");
            }
            Ok(Self::Fd(value as RawFd))
        } else {
            if value > u16::MAX as u64 {
                bail!("invalid port number");
            }
            Ok(Self::Port(value as u16))
        }
    }

    fn is_fd(&self) -> bool {
        match self {
            Self::Fd(_) => true,
            _ => false,
        }
    }
}

#[derive(Debug)]
struct Options {
    /// The actual command to run proxied in a pseudo terminal.
    terminal_command: Vec<OsString>,
    /// The port or FD that termproxy will listen on for an incoming conection
    listen_port: PortOrFd,
    /// The port of the local privileged daemon that authentication is relayed to. Defaults to `85`
    api_daemon_port: u16,
    /// The ACL object path the 'acl_permission' is checked on
    acl_path: String,
    /// The ACL permission that the ticket, read from the stream, is required to have on 'acl_path'
    acl_permission: Option<String>,
}

fn parse_args() -> Result<Options> {
    let mut args: Vec<_> = std::env::args_os().collect();
    args.remove(0); // remove the executable path.

    // handle finding command after `--` first so that we only parse our options later
    let terminal_command = if let Some(dash_dash) = args.iter().position(|arg| arg == "--") {
        let later_args = args.drain(dash_dash + 1..).collect();
        args.pop(); // .. then remove the `--`
        Some(later_args)
    } else {
        None
    };

    // Now pass the remaining arguments through to `pico_args`.
    let mut args = pico_args::Arguments::from_vec(args);

    if args.contains(["-h", "--help"]) {
        print!("{CMD_HELP}");
        std::process::exit(0);
    } else if terminal_command.is_none() {
        bail!("missing terminal command or -- option-end marker, see '-h' for usage");
    }

    let options = Options {
        terminal_command: terminal_command.unwrap(), // checked above
        listen_port: PortOrFd::from_cli(args.free_from_str()?, args.contains("--port-as-fd"))?,
        api_daemon_port: args.opt_value_from_str("--authport")?.unwrap_or(85),
        acl_path: args.value_from_str("--path")?,
        acl_permission: args.opt_value_from_str("--perm")?,
    };

    if !args.finish().is_empty() {
        bail!("unexpected extra arguments, use '-h' for usage");
    }

    Ok(options)
}

fn do_main() -> Result<()> {
    let options = parse_args()?;

    let (mut tcp_handle, listen_port) =
        listen_and_accept("localhost", &options.listen_port, Duration::new(10, 0))
            .map_err(|err| format_err!("failed waiting for client: {err}"))?;

    let mut pty_buf = ByteBuffer::new();
    let mut tcp_buf = ByteBuffer::new();

    let (username, ticket) = read_ticket_line(&mut tcp_handle, &mut pty_buf, Duration::new(10, 0))
        .map_err(|err| format_err!("failed reading ticket: {err}"))?;

    authenticate(&username, &ticket, &options, listen_port)?;

    tcp_handle.write_all(b"OK").expect("error writing response");

    let mut poll = Poll::new()?;
    let mut events = Events::with_capacity(128);

    let mut pty = run_pty(options.terminal_command.iter())?;

    poll.registry().register(
        &mut tcp_handle,
        TCP,
        Interest::READABLE | Interest::WRITABLE,
    )?;
    poll.registry().register(
        &mut SourceFd(&pty.as_raw_fd()),
        PTY,
        Interest::READABLE | Interest::WRITABLE,
    )?;

    let mut tcp_writable = true;
    let mut pty_writable = true;
    let mut tcp_readable = true;
    let mut pty_readable = true;
    let mut remaining = 0;
    let mut finished = false;

    while !finished {
        if tcp_readable && !pty_buf.is_full() || pty_readable && !tcp_buf.is_full() {
            poll.poll(&mut events, Some(Duration::new(0, 0)))?;
        } else {
            poll.poll(&mut events, None)?;
        }

        for event in &events {
            let writable = event.is_writable();
            let readable = event.is_readable();
            if event.is_read_closed() {
                finished = true;
            }
            match event.token() {
                TCP => {
                    if readable {
                        tcp_readable = true;
                    }
                    if writable {
                        tcp_writable = true;
                    }
                }
                PTY => {
                    if readable {
                        pty_readable = true;
                    }
                    if writable {
                        pty_writable = true;
                    }
                }
                _ => unreachable!(),
            }
        }

        while tcp_readable && !pty_buf.is_full() {
            let bytes = match pty_buf.read_from(&mut tcp_handle) {
                Ok(bytes) => bytes,
                Err(err) if err.kind() == ErrorKind::WouldBlock => {
                    tcp_readable = false;
                    break;
                }
                Err(err) => {
                    if !finished {
                        return Err(format_err!("error reading from tcp: {err}"));
                    }
                    break;
                }
            };
            if bytes == 0 {
                finished = true;
                break;
            }
        }

        while pty_readable && !tcp_buf.is_full() {
            let bytes = match tcp_buf.read_from(&mut pty) {
                Ok(bytes) => bytes,
                Err(err) if err.kind() == ErrorKind::WouldBlock => {
                    pty_readable = false;
                    break;
                }
                Err(err) => {
                    if !finished {
                        return Err(format_err!("error reading from pty: {err}"));
                    }
                    break;
                }
            };
            if bytes == 0 {
                finished = true;
                break;
            }
        }

        while !tcp_buf.is_empty() && tcp_writable {
            let bytes = match tcp_handle.write(&tcp_buf[..]) {
                Ok(bytes) => bytes,
                Err(err) if err.kind() == ErrorKind::WouldBlock => {
                    tcp_writable = false;
                    break;
                }
                Err(err) => {
                    if !finished {
                        return Err(format_err!("error writing to tcp : {err}"));
                    }
                    break;
                }
            };
            tcp_buf.consume(bytes);
        }

        while !pty_buf.is_empty() && pty_writable {
            if remaining == 0 {
                remaining = match process_queue(&mut pty_buf, &mut pty) {
                    Some(val) => val,
                    None => break,
                };
            }
            let len = min(remaining, pty_buf.len());
            let bytes = match pty.write(&pty_buf[..len]) {
                Ok(bytes) => bytes,
                Err(err) if err.kind() == ErrorKind::WouldBlock => {
                    pty_writable = false;
                    break;
                }
                Err(err) => {
                    if !finished {
                        return Err(format_err!("error writing to pty : {err}"));
                    }
                    break;
                }
            };
            remaining -= bytes;
            pty_buf.consume(bytes);
        }
    }

    Ok(())
}

fn main() {
    std::process::exit(match do_main() {
        Ok(_) => 0,
        Err(err) => {
            eprintln!("{err}");
            1
        }
    });
}
