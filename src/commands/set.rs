//! Set command. See [Set command](https://redis.io/commands/set) for official documentation

use super::{extract_or_err, CommandCreationError};
use crate::resp::{Type, TypeConsumer};
use std::collections::LinkedList;
/// Holds key and value required for the [Set command](super::Command::Set)
#[derive(Debug, PartialEq)]
pub struct Set {
    pub(crate) key: String,
    pub(crate) value: String,
}

impl Set {
    /// Returns an instance of [super::get::Get]
    pub fn from(type_consumer: &mut TypeConsumer) -> Result<Self, CommandCreationError> {
        let key = extract_or_err(type_consumer.next_string(), "key")?;
        let value = extract_or_err(type_consumer.next_string(), "value")?;
        Ok(Set { key, value })
    }
}

impl From<Set> for Type {
    fn from(get: Set) -> Self {
        let mut ll = LinkedList::new();
        ll.push_back(Type::BulkString(b"SET".to_vec()));
        ll.push_back(Type::BulkString(get.key.into_bytes()));
        ll.push_back(Type::BulkString(get.value.into_bytes()));
        Type::Array(ll)
    }
}

#[cfg(test)]
mod test {
    use super::CommandCreationError;
    use crate::resp::{Type, TypeConsumer};

    use super::Set;

    #[test]
    fn from_works() {
        let mut tc = TypeConsumer::new(Type::Array(
            vec![
                Type::BulkString(b"Hello".to_vec()),
                Type::BulkString(b"World".to_vec()),
            ]
            .into_iter()
            .collect(),
        ));
        let set = Set::from(&mut tc).unwrap();
        assert_eq!(
            set,
            Set {
                key: "Hello".into(),
                value: "World".into()
            }
        );
        let mut tc = TypeConsumer::new(Type::Array(vec![].into_iter().collect()));
        let set = Set::from(&mut tc);
        assert_eq!(set, Err(CommandCreationError::MissingField("key".into())));
        let mut tc = TypeConsumer::new(Type::Array(
            vec![Type::BulkString(b"Hello".to_vec())]
                .into_iter()
                .collect(),
        ));
        let set = Set::from(&mut tc);
        assert_eq!(set, Err(CommandCreationError::MissingField("key".into())));
    }

    #[test]
    fn into_works() {
        let set = Set {
            key: "Hello".into(),
            value: "World".into(),
        };
        let t: Type = set.into();
        let expected = Type::Array(
            vec![
                Type::BulkString(b"SET".to_vec()),
                Type::BulkString(b"Hello".to_vec()),
                Type::BulkString(b"World".to_vec()),
            ]
            .into_iter()
            .collect(),
        );
        assert_eq!(t, expected);
    }
}
