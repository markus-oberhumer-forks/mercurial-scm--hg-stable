// Copyright 2018 Yuya Nishihara <yuya@tcha.org>
//
// This software may be used and distributed according to the terms of the
// GNU General Public License version 2 or any later version.

//! Low-level utility for signal and process handling.

use libc::{c_int, size_t, ssize_t};
use std::io;
use std::os::unix::io::RawFd;

#[link(name = "procutil", kind = "static")]
extern "C" {
    // sendfds.c
    fn sendfds(sockfd: c_int, fds: *const c_int, fdlen: size_t) -> ssize_t;
}

/// Sends file descriptors via the given socket.
pub fn send_raw_fds(sock_fd: RawFd, fds: &[RawFd]) -> io::Result<()> {
    let r = unsafe { sendfds(sock_fd, fds.as_ptr(), fds.len() as size_t) };
    if r < 0 {
        return Err(io::Error::last_os_error());
    }
    Ok(())
}
