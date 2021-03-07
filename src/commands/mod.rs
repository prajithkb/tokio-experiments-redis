//! This module lists all the commands

use self::{get::Get, set::Set};
/// The get command related data
pub mod get;
/// The set command related data
pub mod set;

/// All the supported commands
#[derive(Debug)]
pub enum Command {
    /// Used to implement [Get](https://redis.io/commands/get) command from Redis
    Get(Get),
    /// Used to implement [Get](https://redis.io/commands/set) command from Redis
    Set(Set),
}