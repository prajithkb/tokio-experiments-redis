//! The commands module, lists all the supported commands
use self::{get::Get, set::Set};
use crate::resp::{Type, TypeConsumer, TypeConsumerError};
use std::{error::Error, fmt::Display};
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
            Command::Set(s) => s.into(),
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
    UnSupportedCommand,
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
            _ => Err(CommandCreationError::UnSupportedCommand),
        }
    }
}

#[cfg(test)]
mod test {
    use std::collections::LinkedList;
    use crate::resp::Type;
    use crate::Result;
    use crate::resp::TypeConsumer;
    use crate::commands::Command;
    use crate::commands::*;
        

    #[test]
    fn command_creation_get_works() -> Result<()>{
        let get_command: LinkedList<Type> = vec![
            Type::SimpleString("GET".into()),
            Type::SimpleString("Hello".into()),
        ]
        .into_iter()
        .collect();
        let mut t = TypeConsumer::new(Type::Array(get_command));
        assert_eq!(Command::new(&mut t)?, Command::Get(Get {
            key: "Hello".into()
        }));
        Ok(())
    }

    #[test]
    fn command_creation_set_works() -> Result<()>{
        let set_command: LinkedList<Type> = vec![
            Type::SimpleString("SET".into()),
            Type::SimpleString("Hello".into()),
            Type::SimpleString("World".into()),
        ]
        .into_iter()
        .collect();
        let mut t = TypeConsumer::new(Type::Array(set_command));
        assert_eq!(Command::new(&mut t)?, Command::Set(Set {
            key: "Hello".into(),
            value: "World".into(),
        }));
        Ok(())
    }

    #[test]
    fn command_creation_unsupported_cmd_works() {
        let set_command: LinkedList<Type> = vec![
            Type::SimpleString("RANDOM".into()),
            Type::SimpleString("World".into()),
        ]
        .into_iter()
        .collect();
        let mut t = TypeConsumer::new(Type::Array(set_command));
        assert_eq!(Command::new(&mut t), Err(CommandCreationError::UnSupportedCommand));
    }
}
