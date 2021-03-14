//! This is cli that runs the server. Under the hood it runs the server

use tokio_mini_redis::server::RedisServer;

#[tokio::main]
async fn main(){
   // initialize the logger
   env_logger::init();
   let server = RedisServer::new();
   server.listen("127.0.0.1:6000").await.expect("Error");
}
