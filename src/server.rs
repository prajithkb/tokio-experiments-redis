//! The server module. This module implements a basic Tokio based server

use crate::{
    commands::Command,
    connection,
    database::Database,
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
        let db = Database::new();
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
async fn process(socket: TcpStream, mut db: Database) {
    // create a connection (read and write halves)
    // This allows for independent io
    let (mut read, mut write) = Connection::new(socket).read_write_split();
    // response channel
    let (response_sender, mut response_receiver) = mpsc::channel::<Type>(32);
    // Tokio reads
    tokio::spawn(async move {
        loop {
            match read.recv().await {
                Ok(t) => match t {
                    Some(t) => {
                        info!("Received {}", t);
                        let mut type_consumer = TypeConsumer::new(t);
                        let command = Command::new(&mut type_consumer);
                        let r = match command {
                            Ok(command) => {
                                info!("Received {:?}", command);
                                let r = db.act(command);
                                info!("Recieved {:?} from DB", r);
                                response_sender.send(r).await
                            }
                            // Error, response sender closed
                            Err(e) => response_sender.send(error(e)).await,
                        };
                        if let Err(e) = r {
                            error!("Error {}", e);
                            break;
                        }
                    }
                    // Connection closed
                    None => break,
                },
                // Connection read failure
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
