//! Helper for creating a pseudo-terminal
//!
//! see [Pty](struct.Pty.html) for an example on how to use it

use std::os::unix::io::{AsFd, AsRawFd, BorrowedFd, FromRawFd, OwnedFd, RawFd};

use nix::fcntl::OFlag;
use nix::pty::{grantpt, posix_openpt, ptsname_r, unlockpt, PtyMaster};
use nix::sys::stat::Mode;
use nix::unistd::{dup2, setsid};
use nix::{ioctl_write_int_bad, ioctl_write_ptr_bad, Result};

ioctl_write_int_bad!(set_controlling_tty, libc::TIOCSCTTY);
ioctl_write_ptr_bad!(set_size, libc::TIOCSWINSZ, nix::pty::Winsize);

/// Represents a pty.
///
/// Implements Read and Write (from std::io) so one can simply use it
/// to read and write the terminal of a child process
///
/// Example:
/// ```
/// # use proxmox_sys::linux::pty::*;
/// # use std::process::Command;
/// # use nix::Result;
/// fn fork() -> Result<u64> {
///     // Code that forks and returs the pid/0
/// # Ok(1)
/// }
///
/// fn exec(cmd: &str) -> Result<()> {
///     // Code that execs the cmd
/// #    Ok(())
/// }
///
/// fn main() -> Result<()> {
///     let (mut pty, secondary) = Pty::new()?;
///
///     let child = fork()?;
///     if child == 0 {
///         make_controlling_terminal(&secondary)?;
///         exec("/some/binary")?;
///     }
///
///     // read/write or set size of the terminal
///     pty.set_size(80, 20);
///
///     Ok(())
///  }
/// ```
pub struct Pty {
    primary: PtyMaster,
}

/// Used to make a new process group of the current process,
/// and make the given terminal its controlling terminal
pub fn make_controlling_terminal(terminal: &str) -> Result<()> {
    setsid()?; // make new process group
    let mode = Mode::S_IRUSR
        | Mode::S_IWUSR
        | Mode::S_IRGRP
        | Mode::S_IWGRP
        | Mode::S_IROTH
        | Mode::S_IWOTH; // 0666
    let secondary_fd = nix::fcntl::open(terminal, OFlag::O_RDWR | OFlag::O_NOCTTY, mode)
        .map(|fd| unsafe { OwnedFd::from_raw_fd(fd) })?;
    let s_raw_fd = secondary_fd.as_raw_fd();
    unsafe { set_controlling_tty(s_raw_fd, 0) }?;
    dup2(s_raw_fd, 0)?;
    dup2(s_raw_fd, 1)?;
    dup2(s_raw_fd, 2)?;

    if s_raw_fd <= 2 {
        std::mem::forget(secondary_fd); // don't call drop handler
    }

    Ok(())
}

impl Pty {
    /// Creates a new pty by opening /dev/ptmx and returns
    /// a new pty and the path to the secondary terminal on success.
    pub fn new() -> Result<(Self, String)> {
        let primary =
            posix_openpt(OFlag::O_RDWR | OFlag::O_NOCTTY | OFlag::O_NONBLOCK | OFlag::O_CLOEXEC)?;
        grantpt(&primary)?;
        unlockpt(&primary)?;
        let secondary = ptsname_r(&primary)?; // linux specific
        Ok((Self { primary }, secondary))
    }

    /// Uses the ioctl 'TIOCSWINSZ' on the terminal fd to set the terminals
    /// columns and rows
    pub fn set_size(&mut self, col: u16, row: u16) -> Result<()> {
        let size = nix::pty::Winsize {
            ws_row: row,
            ws_col: col,
            ws_xpixel: 0,
            ws_ypixel: 0,
        };

        unsafe { set_size(self.primary.as_raw_fd(), &size) }?;

        Ok(())
    }
}

impl std::io::Read for Pty {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        Ok(nix::unistd::read(self.primary.as_raw_fd(), buf)?)
    }
}

impl std::io::Write for Pty {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        Ok(nix::unistd::write(self.primary.as_raw_fd(), buf)?)
    }

    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}

impl AsRawFd for Pty {
    fn as_raw_fd(&self) -> RawFd {
        self.primary.as_raw_fd()
    }
}

impl AsFd for Pty {
    fn as_fd(&self) -> BorrowedFd<'_> {
        unsafe { BorrowedFd::borrow_raw(self.as_raw_fd()) }
    }
}
