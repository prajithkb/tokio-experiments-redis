//! The connection module. 
//! This module encapsulates a connection and provde convenient read write accesors
use std::io::Cursor;

use log::{debug, info, trace};
use tcp::{OwnedReadHalf, OwnedWriteHalf};
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::{tcp, TcpStream},
};

use crate::Result;
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
    pub async fn recv(&mut self) -> Result<Type> {
        debug!("ReadHalf recv");
        // Read 512 bytes at a time
        let mut buf = [0; 512];
        let n = self.inner.read(&mut buf).await?;
        if n > 0 {
            info!("Read {} bytes", n);
            trace!("Read {} bytes, {:?}", n, std::str::from_utf8(&buf));
            let mut cur = Cursor::new(&buf[..]);
            let t = self.parse.parse_next(&mut cur)?;
            Ok(t)
        } else {
            info!("Read returned {} bytes", n);
            Err("Connection closed".into())
        }
    }
}
/// The write half of [Connection]
pub struct WriteHalf {
    inner: OwnedWriteHalf,
}
impl WriteHalf {
    /// Sends the given [Type]
    pub async fn send(&mut self, t: Type) -> Result<usize> {
        let bytes = t.into_bytes();
        let u = self.inner.write(&bytes).await?;
        info!("Wrote {} bytes", u);
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
