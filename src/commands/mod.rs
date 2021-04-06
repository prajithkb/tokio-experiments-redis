//! The commands module, lists all the supported commands
use self::{get::Get, list::Push, set::Set};
use crate::resp::{Type, TypeConsumer, TypeConsumerError};
use std::{error::Error, fmt::Display};
/// The get command related data
pub mod get;
/// The list commands module
pub mod list;
/// The set command related data
pub mod set;

/// All the supported commands
#[derive(Debug, PartialEq)]
pub enum Command {
    /// Used to implement [Get](https://redis.io/commands/get) command from Redis
    Get(Get),
    /// Used to implement [Get](https://redis.io/commands/set) command from Redis
    Set(Set),
    /// Pushes the given strings into a list
    /// Accepts a tuple of key (name of the list), list of elements
    /// Used to implement [Push](https://redis.io/commands/rpush)
    Push(Push),
}

impl From<Command> for Type {
    fn from(c: Command) -> Self {
        match c {
            Command::Get(g) => g.into(),
            Command::Set(s) => s.into(),
            Command::Push(p) => p.into(),
            // Command::Pop(key, count) => Type::Integer(u.into())
        }
    }
}

/// Used to indicate the different errors during the creation of a [Command]
#[derive(Debug, PartialEq)]
pub enum CommandCreationError {
    /// an invalid Frame for the command
    /// Shows the error and the field for which the error occurred
    InvalidFrame(TypeConsumerError, &'static str),
    /// Thrown when a field is missing for a command
    MissingField(String),
    /// A command that is not supported
    UnSupportedCommand,
}

/// Extracts the field or returns an error
pub(crate) fn unwrap_or_err<T>(v: Option<T>, field: &str) -> Result<T, CommandCreationError> {
    match v {
        Some(v) => Ok(v),
        None => Err(CommandCreationError::MissingField(field.into())),
    }
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
        let command = unwrap_or_err(type_consumer.next_string()?, "Command")?;
        match command.as_ref() {
            "GET" => Ok(Command::Get(Get::from(type_consumer)?)),
            "SET" => Ok(Command::Set(Set::from(type_consumer)?)),
            "PUSH" => Ok(Command::Push(Push::from(type_consumer)?)),
            _ => Err(CommandCreationError::UnSupportedCommand),
        }
    }
}

#[cfg(test)]
mod test {
    use crate::commands::Command;
    use crate::commands::*;
    use crate::resp::Type;
    use crate::resp::TypeConsumer;
    use crate::Result;
    use std::collections::LinkedList;

    #[test]
    fn command_creation_get_works() -> Result<()> {
        let get_command: LinkedList<Type> = vec![
            Type::SimpleString("GET".into()),
            Type::SimpleString("Hello".into()),
        ]
        .into_iter()
        .collect();
        let mut t = TypeConsumer::new(Type::Array(get_command));
        assert_eq!(
            Command::new(&mut t)?,
            Command::Get(Get {
                key: "Hello".into()
            })
        );
        Ok(())
    }

    #[test]
    fn command_creation_set_works() -> Result<()> {
        let set_command: LinkedList<Type> = vec![
            Type::SimpleString("SET".into()),
            Type::SimpleString("Hello".into()),
            Type::SimpleString("World".into()),
        ]
        .into_iter()
        .collect();
        let mut t = TypeConsumer::new(Type::Array(set_command));
        assert_eq!(
            Command::new(&mut t)?,
            Command::Set(Set {
                key: "Hello".into(),
                value: "World".into(),
            })
        );
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
        assert_eq!(
            Command::new(&mut t),
            Err(CommandCreationError::UnSupportedCommand)
        );
    }
}
