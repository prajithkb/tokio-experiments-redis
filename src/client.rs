//! This is the client module.  
//! This provides a simple [RedisClient] which supports the [super::commands::Command]

use std::collections::LinkedList;

use log::debug;
use tokio::{net::TcpStream, sync::mpsc::Sender};

use crate::{
    commands::CommandCreationError,
    commands::{get::Get, list::Push, set::Set, watch::Watch, watch::WatchResult, Command},
    connection::{Connection, ReadHalf, WriteHalf},
    resp::Type,
};
use crate::{database::Operation, Result};

/// A RedisClient
pub struct RedisClient {
    read_half: ReadHalf,
    write_half: WriteHalf,
}

impl RedisClient {
    /// Creates a client that is connected to a given address
    pub async fn connect(addr: &str) -> Result<Self> {
        let socket = TcpStream::connect(addr).await?;
        let (read_half, write_half) = Connection::new(socket).read_write_split();
        Ok(RedisClient {
            read_half,
            write_half,
        })
    }

    /// Get command
    pub async fn get(&mut self, key: &str) -> Result<Type> {
        let get = Command::Get(Get { key: key.into() });
        debug!("{:?}", get);
        self.send(get.into()).await
    }
    /// Set command
    pub async fn set(&mut self, key: String, value: String) -> Result<Type> {
        let set = Command::Set(Set { key, value });
        debug!("{:?}", set);
        self.send(set.into()).await
    }

    /// push command
    pub async fn push(&mut self, list_name: String, values: LinkedList<String>) -> Result<Type> {
        let push = Command::Push(Push { list_name, values });
        debug!("{:?}", push);
        self.send(push.into()).await
    }

    /// push command
    pub async fn watch(
        &mut self,
        key: String,
        operation: Operation,
        watcher: Sender<WatchResult>,
    ) -> Result<()> {
        let watch = Command::Watch(Watch { key, operation });
        debug!("{:?}", watch);
        self.write_half.send(watch.into()).await?;
        while let Some(v) = self.read_half.recv().await? {
            let watch_result = std::result::Result::<WatchResult, CommandCreationError>::from(v)?;
            debug!("Read: {:?}", watch_result);
            watcher.send(watch_result).await?;
        }
        Ok(())
    }

    async fn send(&mut self, t: Type) -> Result<Type> {
        self.write_half.send(t).await?;
        self.read_half
            .recv()
            .await?
            .ok_or_else(|| "Connection closed".into())
    }
}
