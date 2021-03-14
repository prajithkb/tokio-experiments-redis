//! This module lists all the commands

use std::{error::Error, fmt::Display};

use crate::{resp::{Type, TypeConsumer, TypeConsumerError}};

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

impl From<Command> for Type {
    fn from(c: Command) -> Self {
        match c {
            Command::Get(g) => g.into(),
            Command::Set(s) => s.into()
        }
    }
}
/// Used to indicate the different errors during the creation of a [Command]
#[derive(Debug, PartialEq)]
pub enum CommandCreationError {
    /// an invalid Frame for the command
    /// Shows the error and the field for which the error occurred
    InvalidFrame(TypeConsumerError, &'static str),
    /// A command that is not supported
    UnSupportedCommand
}

impl Error for CommandCreationError {}

impl Display for CommandCreationError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_fmt(format_args!("{:?}", self))
    }
}

impl From<TypeConsumerError> for CommandCreationError {
    fn from(t: TypeConsumerError) -> Self {
        CommandCreationError::InvalidFrame(t, "Not a String")
    }
}

impl Command {

    /// Creates a new instance of a [Command]
    pub fn new(type_consumer: &mut TypeConsumer) -> Result<Command, CommandCreationError> {
        let command = type_consumer.next_string()?;
        match command.as_ref() {
            "GET" => Ok(Command::Get(Get::from(type_consumer)?)),
            "SET" => Ok(Command::Set(Set::from(type_consumer)?)),
            _ => Err(CommandCreationError::UnSupportedCommand)
        }
    }

}