use std::ffi::OsString;
use std::os::fd::RawFd;

use anyhow::{bail, Result};

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
pub enum PortOrFd {
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
}

#[derive(Debug)]
pub struct Options {
    /// The actual command to run proxied in a pseudo terminal.
    pub terminal_command: Vec<OsString>,
    /// The port or FD that termproxy will listen on for an incoming conection
    pub listen_port: PortOrFd,
    /// The port of the local privileged daemon that authentication is relayed to. Defaults to `85`
    pub api_daemon_port: u16,
    /// The ACL object path the 'acl_permission' is checked on
    pub acl_path: String,
    /// The ACL permission that the ticket, read from the stream, is required to have on 'acl_path'
    pub acl_permission: Option<String>,
}

impl Options {
    pub fn from_env() -> Result<Self> {
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

        let options = Self {
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

    pub fn use_listen_port_as_fd(&self) -> bool {
        match self.listen_port {
            PortOrFd::Fd(_) => true,
            _ => false,
        }
    }
}
