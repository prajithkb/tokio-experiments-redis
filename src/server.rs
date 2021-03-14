//! The server module. This module implements a basic Tokio based server

use std::{
    collections::{HashMap, LinkedList},
    sync::{Arc, Mutex},
};

use crate::{
    commands::Command,
    connection,
    resp::{Type, TypeConsumer},
    Result,
};
use connection::Connection;
use log::{error, info};
use tokio::{
    net::{TcpListener, TcpStream},
    sync::mpsc,
};
/// A simple Redis Server that uses Tokio
#[derive(Debug, Default)]
pub struct RedisServer {}

impl RedisServer {
    ///
    pub fn new() -> Self {
        RedisServer {}
    }

    /// Starts listening on a given address
    pub async fn listen(&self, addr: &str) -> Result<()> {
        info!("Starting");
        let db = DataBase::new();
        let listener = TcpListener::bind(addr).await?;
        info!("Listening at {}", addr);
        loop {
            let (socket, addr) = listener.accept().await?;
            info!("Received connection from {:?}", addr);
            let db = db.clone();
            // A new task is spawned for each inbound socket. The socket is
            // moved to the new task and processed there.
            tokio::spawn(async move {
                process(socket, db).await;
            });
        }
    }
}
#[derive(Debug, PartialEq, Hash, Eq, Clone)]
pub(crate) struct RedisString {
    bytes: Vec<u8>,
}

impl From<String> for RedisString {
    fn from(s: String) -> Self {
        Self {
            bytes: s.into_bytes(),
        }
    }
}

impl From<&str> for RedisString {
    fn from(s: &str) -> Self {
        Self { bytes: s.into() }
    }
}

#[derive(Debug, PartialEq, Hash, Eq, Clone)]
pub(crate) enum Value {
    String(RedisString),
    Null,
    #[allow(dead_code)]
    List(LinkedList<RedisString>),
}

impl From<Value> for Type {
    fn from(v: Value) -> Self {
        match v {
            Value::String(s) => {
                Type::SimpleString(String::from_utf8(s.bytes).expect("Not a valid string"))
            }
            Value::List(l) => Type::Array(
                l.into_iter()
                    .map(|s| {
                        Type::SimpleString(String::from_utf8(s.bytes).expect("Not a valid string"))
                    })
                    .collect(),
            ),
            Value::Null => Type::Null,
        }
    }
}
#[derive(Debug, Default)]
pub(crate) struct DataBase {
    inner: Arc<Mutex<HashMap<RedisString, Value>>>,
}

impl DataBase {
    fn new() -> Self {
        DataBase {
            inner: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    fn act(&mut self, command: Command) -> Value {
        match command {
            Command::Get(g) => {
                let inner = self.inner.lock().unwrap();
                let key: RedisString = g.key.into();
                match inner.get(&key).cloned() {
                    Some(v) => v,
                    None => Value::Null,
                }
            }
            Command::Set(s) => {
                let mut inner = self.inner.lock().unwrap();
                let key: RedisString = s.key.into();
                let value: RedisString = s.value.into();
                inner.insert(key, Value::String(value));
                Value::String("Ok".into())
            }
        }
    }
}

impl Clone for DataBase {
    fn clone(&self) -> Self {
        Self {
            inner: self.inner.clone(),
        }
    }
}

async fn process(socket: TcpStream, mut db: DataBase) {
    // create a connection (read and write halves)
    // This allows for independent io
    let (mut read, mut write) = Connection::new(socket).read_write_split();
    // response channel
    let (response_sender, mut response_receiver) = mpsc::channel::<Type>(32);
    // Tokio reads
    tokio::spawn(async move {
        loop {
            match read.recv().await {
                Ok(t) => {
                    info!("Received {}", t);
                    let mut type_consumer = TypeConsumer::new(t);
                    let command = Command::new(&mut type_consumer);
                    let r = match command {
                        Ok(command) => {
                            info!("Received {:?}", command);
                            let r = db.act(command);
                            info!("Recieved {:?} from DB", r);
                            response_sender.send(r.into()).await
                        }
                        Err(e) => response_sender.send(error(e)).await,
                    };
                    if r.is_err() {
                        error!("Error {:?}", r);
                        break;
                    }
                }
                Err(e) => {
                    error!("Error {}", e);
                    break;
                }
            }
        }
        info!("Stopping read");
    });

    // Tokio writes
    tokio::spawn(async move {
        while let Some(t) = response_receiver.recv().await {
            info!("Sending {} to client", t);
            if let Err(e) = write.send(t).await {
                error!("Error {:?}", e);
                break;
            }
        }
        info!("Stopping write");
    });
}

fn error<T>(e: T) -> Type
where
    T: std::error::Error,
{
    Type::Error(format!("Error: {}", e))
}
