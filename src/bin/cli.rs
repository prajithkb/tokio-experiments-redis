//! This is a cli used to send commands to redis. Under the hood it uses the client

use log::info;
use tokio::io::AsyncReadExt;
use tokio_mini_redis::Result;
use tokio_mini_redis::{client::RedisClient, resp::Type};

use std::{
    collections::LinkedList,
    error::Error,
    fmt::Display,
    io::{stdout, Write},
};
use structopt::StructOpt;

#[derive(StructOpt, Debug)]
#[structopt(name = "mini-redis-cli", version = env!("CARGO_PKG_VERSION"), author = env!("CARGO_PKG_AUTHORS"), about = "Issue Redis commands")]
struct Cli {
    #[structopt(name = "hostname", long = "--host", default_value = "127.0.0.1")]
    host: String,

    #[structopt(name = "port", long = "--port", default_value = "6000")]
    port: String,
}

/// Entry point for CLI tool.
///
/// The `[tokio::main]` annotation signals that the Tokio runtime should be
/// started when the function is called. The body of the function is executed
/// within the newly spawned runtime.
#[tokio::main]
async fn main() -> Result<()> {
    env_logger::init();
    // Parse command line arguments
    let cli = Cli::from_args();

    // Get the remote address to connect to
    let addr = format!("{}:{}", cli.host, cli.port);

    // Establish a connection
    let mut client = RedisClient::connect(&addr).await?;
    info!("Connected to {}", addr);
    let mut stdin = tokio::io::stdin();
    let mut buffer = [0; 512];
    loop {
        prompt();
        // Allows for multi tasking between multiple branches
        tokio::select! {
            n = stdin.read(&mut buffer) => {
                let num_bytes = n.unwrap();
                let c = std::str::from_utf8(&buffer[0..num_bytes-1])?;
                let v = send_command(c.into(), &mut client).await;
                println!("=================================");
                match v {
                    Ok(t) => println!("Command=> {}\nResponse=> {}", c, print_type(t)),
                    Err(e) => {
                        let e :Box<CliError> = e.downcast::<CliError>().unwrap();
                        if let CliError::Quit = *e {
                            break;
                        } else {
                            println!("Command execution failed: {}", e);
                        }
                    }
                }
                println!("=================================");
            }
        }
    }
    Ok(())
}

fn print_type(t: Type) -> String {
    match t {
        Type::Error(message) => format!("Error: {}", message),
        _ => t.to_string(),
    }
}

fn prompt() {
    let mut stdout = stdout();
    stdout.write_all(b">").expect("Failed");
    stdout.flush().unwrap();
}

async fn send_command(command: String, client: &mut RedisClient) -> Result<Type> {
    let mut tokens = command.split(' ');
    if let Some(command) = tokens.next() {
        return match command.to_uppercase().as_ref() {
            "GET" => {
                let key = tokens
                    .next()
                    .ok_or_else(|| CliError::ClientError("key cannot be empty".into()))?;
                let t = client
                    .get(key)
                    .await
                    .map_err(|e| CliError::ServerError(e.to_string()))?;
                Ok(t)
            }
            "SET" => {
                let key = tokens.next().ok_or_else(|| {
                    CliError::ClientError("key cannot be empty, SET key value".into())
                })?;
                let value = tokens.next().ok_or_else(|| {
                    CliError::ClientError("value cannot be empty, SET key value".into())
                })?;
                let t = client
                    .set(key.into(), value.into())
                    .await
                    .map_err(|e| CliError::ServerError(e.to_string()))?;
                Ok(t)
            }
            "PUSH" => {
                let list_name = tokens.next().ok_or_else(|| {
                    CliError::ClientError(
                        "list_name cannot be empty, PUSH list_name value1 value2..".into(),
                    )
                })?;
                let values: LinkedList<String> = tokens.into_iter().map(|v| v.into()).collect();
                let t = client
                    .push(list_name.into(), values)
                    .await
                    .map_err(|e| CliError::ServerError(e.to_string()))?;
                Ok(t)
            }
            "QUIT" => Err(CliError::Quit.into()),
            "HELP" => Ok(Type::SimpleString(
                r#"
                HELP - This message
                GET - GET <key>
                SET - SET <key> <value>
                PUSH - PUSH <list name> <value1> <value2> ...
                "#
                .into(),
            )),
            _ => Err(
                CliError::ClientError(format!("Invalid command: <{}>, try HELP", command)).into(),
            ),
        };
    }
    Err("Invalid Command".into())
}

#[derive(Debug)]
enum CliError {
    ServerError(String),
    ClientError(String),
    Quit,
}

impl Error for CliError {}

impl Display for CliError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_fmt(format_args!("{:?}", self))
    }
}
