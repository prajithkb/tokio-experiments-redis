//! The connection module.
//! This module encapsulates a connection and provides convenient (owned) read write accessors
use crate::Result;
use log::{debug, trace};
use std::io::Cursor;
use tcp::{OwnedReadHalf, OwnedWriteHalf};
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::{tcp, TcpStream},
};

use crate::{parse::Parse, resp::Type};

/// Defines a connection (Client-Server)
/// Provides utility methods to write and read [Type]
#[derive(Debug)]
pub struct Connection {
    socket: TcpStream,
}
/// The read half of [Connection]
pub struct ReadHalf {
    inner: OwnedReadHalf,
    parse: Parse,
}

impl ReadHalf {
    /// Receives [Type]
    /// Attempts to wait for a value, returning an error if there is an error.
    pub async fn recv(&mut self) -> Result<Option<Type>> {
        // Read 512 bytes at a time
        let mut buf = [0; 512];
        let n = self.inner.read(&mut buf).await?;
        if n > 0 {
            debug!("Read {} bytes", n);
            trace!("Read {} bytes, {:?}", n, std::str::from_utf8(&buf));
            let mut cur = Cursor::new(&buf[..]);
            let t = self.parse.parse_next(&mut cur)?;
            Ok(Some(t))
        } else {
            Ok(None)
        }
    }
}
/// The write half of [Connection]
pub struct WriteHalf {
    inner: OwnedWriteHalf,
}
impl WriteHalf {
    /// Sends the given [Type]
    /// Attempts to write this type, returning the number of bytes written (or error).
    pub async fn send(&mut self, t: Type) -> Result<usize> {
        let bytes = t.into_bytes();
        let u = self.inner.write(&bytes).await?;
        debug!("Wrote {} bytes", u);
        trace!("Wrote {} bytes, {:?}", u, std::str::from_utf8(&bytes));
        Ok(u)
    }
}

impl Connection {
    /// Creates a new [Connection]
    pub fn new(socket: TcpStream) -> Self {
        Connection { socket }
    }
    /// Factory method to create a read and write halves
    pub fn read_write_split(self) -> (ReadHalf, WriteHalf) {
        let (r, w) = self.socket.into_split();
        (
            ReadHalf {
                inner: r,
                parse: Parse::new(),
            },
            WriteHalf { inner: w },
        )
    }
}
