use std::io;
use std::os::fd::{AsRawFd, FromRawFd, OwnedFd};
use std::os::unix::ffi::OsStrExt;
use std::path::Path;

use anyhow::{bail, Result};

const MAX_MESSAGE: usize = 10 * 1024 * 1024;

pub struct Connection {
    fd: OwnedFd,
}

fn address(path: &Path) -> Result<libc::sockaddr_un> {
    let bytes = path.as_os_str().as_bytes();
    let mut address: libc::sockaddr_un = unsafe { std::mem::zeroed() };
    if bytes.len() >= address.sun_path.len() || bytes.contains(&0) {
        bail!("Invalid AppLoad socket path");
    }
    address.sun_family = libc::AF_UNIX as _;
    for (out, input) in address.sun_path.iter_mut().zip(bytes) {
        *out = *input as _;
    }
    Ok(address)
}

impl Connection {
    pub fn connect(path: &Path) -> Result<Self> {
        let address = address(path)?;
        let raw =
            unsafe { libc::socket(libc::AF_UNIX, libc::SOCK_SEQPACKET | libc::SOCK_CLOEXEC, 0) };
        if raw < 0 {
            return Err(io::Error::last_os_error().into());
        }
        let fd = unsafe { OwnedFd::from_raw_fd(raw) };
        let result = unsafe {
            libc::connect(
                fd.as_raw_fd(),
                &address as *const _ as *const libc::sockaddr,
                std::mem::size_of_val(&address) as _,
            )
        };
        if result < 0 {
            return Err(io::Error::last_os_error().into());
        }
        Ok(Self { fd })
    }

    fn receive_packet(&self, buffer: &mut [u8]) -> Result<usize> {
        loop {
            let size = unsafe {
                libc::recv(
                    self.fd.as_raw_fd(),
                    buffer.as_mut_ptr() as *mut _,
                    buffer.len(),
                    libc::MSG_TRUNC,
                )
            };
            if size >= 0 {
                return Ok(size as usize);
            }
            let error = io::Error::last_os_error();
            if error.kind() != io::ErrorKind::Interrupted {
                return Err(error.into());
            }
        }
    }

    fn send_packet(&self, buffer: &[u8]) -> Result<()> {
        loop {
            let size = unsafe {
                libc::send(
                    self.fd.as_raw_fd(),
                    buffer.as_ptr() as *const _,
                    buffer.len(),
                    libc::MSG_NOSIGNAL,
                )
            };
            if size == buffer.len() as isize {
                return Ok(());
            }
            if size >= 0 {
                bail!("Incomplete AppLoad packet");
            }
            let error = io::Error::last_os_error();
            if error.kind() != io::ErrorKind::Interrupted {
                return Err(error.into());
            }
        }
    }

    pub fn receive(&self) -> Result<Option<(i32, Vec<u8>)>> {
        let mut header = [0; 8];
        let received = self.receive_packet(&mut header)?;
        if received == 0 {
            return Ok(None);
        }
        if received != header.len() {
            bail!("Invalid AppLoad header");
        }
        let kind = i32::from_le_bytes(header[..4].try_into().unwrap());
        let length = i32::from_le_bytes(header[4..].try_into().unwrap());
        if length < 0 || length as usize > MAX_MESSAGE {
            bail!("Invalid AppLoad message length");
        }
        let mut body = vec![0; length as usize];
        if length > 0 && self.receive_packet(&mut body)? != body.len() {
            bail!("Incomplete AppLoad message");
        }
        Ok(Some((kind, body)))
    }

    pub fn send(&self, kind: i32, body: &[u8]) -> Result<()> {
        if body.len() > MAX_MESSAGE {
            bail!("AppLoad message is too large");
        }
        let mut header = [0; 8];
        header[..4].copy_from_slice(&kind.to_le_bytes());
        header[4..].copy_from_slice(&(body.len() as i32).to_le_bytes());
        self.send_packet(&header)?;
        if !body.is_empty() {
            self.send_packet(body)?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn pair() -> (Connection, Connection) {
        let mut fds = [0; 2];
        assert_eq!(
            unsafe { libc::socketpair(libc::AF_UNIX, libc::SOCK_SEQPACKET, 0, fds.as_mut_ptr()) },
            0
        );
        unsafe {
            (
                Connection {
                    fd: OwnedFd::from_raw_fd(fds[0]),
                },
                Connection {
                    fd: OwnedFd::from_raw_fd(fds[1]),
                },
            )
        }
    }
    #[test]
    fn matches_appload_header_and_body_packet_boundaries() {
        let (client, server) = pair();
        client.send(101, b"native state").unwrap();
        let mut header = [0; 32];
        assert_eq!(server.receive_packet(&mut header).unwrap(), 8);
        assert_eq!(&header[..8], &[101, 0, 0, 0, 12, 0, 0, 0]);
        assert_eq!(server.receive_packet(&mut header).unwrap(), 12);
        assert_eq!(&header[..12], b"native state");
        server.send(-2, b"1").unwrap();
        assert_eq!(client.receive().unwrap(), Some((-2, b"1".to_vec())));
        drop(server);
        assert!(client.receive().unwrap().is_none());
    }
    #[test]
    fn rejects_invalid_and_truncated_packets() {
        let (client, server) = pair();
        server.send_packet(&[1, 2, 3]).unwrap();
        assert!(client.receive().is_err());
        server
            .send_packet(&[1, 0, 0, 0, 255, 255, 255, 255])
            .unwrap();
        assert!(client.receive().is_err());
        server.send_packet(&[1, 0, 0, 0, 4, 0, 0, 0]).unwrap();
        server.send_packet(b"hi").unwrap();
        assert!(client.receive().is_err());
    }
}
