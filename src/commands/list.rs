//! All commands related to a list

use std::collections::LinkedList;

use crate::resp::{Type, TypeConsumer};

use super::{unwrap_or_err, CommandCreationError};

/// The push command
#[derive(Debug, PartialEq)]
pub struct Push {
    /// The name of list
    pub list_name: String,
    /// The values to push into the list
    pub values: LinkedList<String>,
}

impl Push {
    /// Creates a Push type from [TypeConsumer]
    pub fn from(type_consumer: &mut TypeConsumer) -> Result<Self, CommandCreationError> {
        let list_name = unwrap_or_err(type_consumer.next_string()?, "list name")?;
        let mut values = LinkedList::new();
        while let Some(item) = type_consumer.next_string()? {
            values.push_back(item)
        }
        Ok(Push { list_name, values })
    }
}

impl From<Push> for Type {
    fn from(p: Push) -> Self {
        let mut ll = LinkedList::new();
        ll.push_back(Type::BulkString(b"PUSH".to_vec()));
        ll.push_back(Type::BulkString(p.list_name.into_bytes()));
        // Add all the values
        p.values.into_iter().for_each(|v| {
            ll.push_back(Type::BulkString(v.into_bytes()));
        });
        Type::Array(ll)
    }
}

/// The Pop command
#[derive(Debug, PartialEq)]
pub struct Pop {
    /// The name of list
    list_name: String,
}
