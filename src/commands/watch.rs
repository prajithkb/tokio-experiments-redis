//! The watch module.
//! This module is responsible for watching on changes to a key

use std::{collections::LinkedList, fmt::Display};

use crate::{
    database::Operation,
    resp::{Type, TypeConsumer},
};

use super::{extract_or_err, CommandCreationError};

/// Defines the watch
#[derive(Debug, PartialEq)]
pub struct Watch {
    /// the key to watch
    pub key: String,
    /// the type of operation to watch
    pub operation: Operation,
}

/// Represents the result of [Watch]
#[derive(Debug)]
pub struct WatchResult {
    /// Key
    pub key: String,
    /// Operation
    pub operation: Operation,
    /// value before
    pub before: Option<Type>,
    /// Value after
    pub after: Type,
}

impl From<WatchResult> for Type {
    fn from(w: WatchResult) -> Self {
        let mut message: LinkedList<Type> = LinkedList::new();
        message.push_back(Type::SimpleString(w.key));
        message.push_back(Type::Integer(w.operation as i64));
        message.push_back(w.before.unwrap_or(Type::Null));
        message.push_back(w.after);
        Type::Array(message)
    }
}

impl From<Type> for Result<WatchResult, CommandCreationError> {
    fn from(w: Type) -> Self {
        let mut type_consumer = TypeConsumer::new(w);
        let key = extract_or_err(type_consumer.next_string(), "key")?;
        let operation = extract_or_err(type_consumer.next_integer(), "operation")?;
        let operation: Operation = (operation as u8).into();
        let before = type_consumer
            .next_type()
            .map_err(|t| CommandCreationError::InvalidFrame(t, "before"))?;
        let after = extract_or_err(type_consumer.next_type(), "after")?;
        Ok(WatchResult {
            key,
            operation,
            before,
            after,
        })
    }
}

impl Watch {
    /// Returns an instance of [super::watch::Watch]
    pub fn from(type_consumer: &mut TypeConsumer) -> Result<Self, CommandCreationError> {
        let key = extract_or_err(type_consumer.next_string(), "key")?;
        let operation = extract_or_err(type_consumer.next_integer(), "operation")?;
        let operation: Operation = (operation as u8).into();
        Ok(Watch { key, operation })
    }
}

impl From<&str> for Operation {
    fn from(s: &str) -> Self {
        match s {
            "Update" => Operation::Update,
            "Addition" => Operation::Addition,
            "Removal" => Operation::Removal,
            "All" => Operation::All,
            _ => Operation::All,
        }
    }
}

impl Display for Operation {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}", self)
    }
}

impl From<Watch> for Type {
    fn from(watch: Watch) -> Self {
        let mut ll = LinkedList::new();
        ll.push_back(Type::BulkString(b"WATCH".to_vec()));
        ll.push_back(Type::BulkString(watch.key.into_bytes()));
        ll.push_back(Type::Integer(watch.operation as i64));
        Type::Array(ll)
    }
}
