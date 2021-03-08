//! This module lists all the commands

use crate::resp::{TypeConsumerError};

use self::{get::Get, set::Set};
/// The get command related data
pub mod get;
/// The set command related data
pub mod set;

/// All the supported commands
#[derive(Debug, PartialEq)]
pub enum Command {
    /// Used to implement [Get](https://redis.io/commands/get) command from Redis
    Get(Get),
    /// Used to implement [Get](https://redis.io/commands/set) command from Redis
    Set(Set),
}
/// Used to indicate the different errors during the creation of a [Command]
#[derive(Debug, PartialEq)]
pub enum CommandError {
    /// an invalid Frame for the command
    /// Shows the error and the field for which the error occurred
    InvalidFrame(TypeConsumerError, &'static str)
}