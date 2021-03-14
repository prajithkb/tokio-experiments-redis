//! This is the client module. 


use log::debug;
use tokio::net::TcpStream;

use crate::{commands::{Command, get::Get, set::Set}, connection::{Connection, ReadHalf, WriteHalf}, resp::Type};
use crate::Result;

/// A RedisClient
pub struct RedisClient {
    read_half: ReadHalf,
    write_half: WriteHalf,
}

impl RedisClient {
  
    /// Creates a client that is connected to a given address
    pub async fn connect(addr: &str) -> Result<Self>{
        let socket = TcpStream::connect(addr).await?;
        let(read_half, write_half) = Connection::new(socket).read_write_split();
        Ok(RedisClient {
            read_half, 
            write_half
        })
    }

    /// Get command
    pub async fn get(&mut self, key: &str) -> Result<Type> {
        let get = Command::Get(Get {
            key: key.into()
        });
        debug!("{:?}", get);
        self.write_half.send(get.into()).await?;
        let v = self.read_half.recv().await?; 
        Ok(v)
    }
    /// Set command
    pub async fn set(&mut self, key: String, value: String) -> Result<Type> {
        let set = Command::Set(Set {
            key,
            value
        });
        debug!("{:?}", set);
        self.write_half.send(set.into()).await?;
        let v = self.read_half.recv().await?; 
        Ok(v)
    }
}