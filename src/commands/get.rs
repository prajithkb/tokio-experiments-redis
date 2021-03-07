//! Get command. See [Get command](https://redis.io/commands/get) for official documentation

use std::collections::LinkedList;

use crate::resp::{Type, TypeConsumer};
use crate::Result;
/// Holds key required for the [Get command](super::Command::Get)
#[derive(Debug, PartialEq)]
pub struct Get {
    key: String,
}

impl Get {
    /// Returns an instance of [super::get::Get] 
    pub fn from(type_consumer: &mut TypeConsumer) -> Result<Self> {
        let key = type_consumer.next_string()?;
        Ok(Get { key })
    }
}

impl From<Get> for Type {
    fn from(get: Get) -> Self {
        let mut ll = LinkedList::new();
        ll.push_back(Type::BulkString(b"GET".to_vec()));
        ll.push_back(Type::BulkString(get.key.into_bytes()));
        Type::Array(ll)
    }
}

#[cfg(test)]
mod test {
    use crate::resp::{Type, TypeConsumer};

    use super::Get;

    #[test]
    fn from_works() {
        let mut tc = TypeConsumer::new(Type::BulkString(b"Hello".to_vec()));
        let get = Get::from(&mut tc).unwrap();
        assert_eq!(
            get,
            Get {
                key: "Hello".into()
            }
        )
    }

    #[test]
    fn into_works() {
        let get = Get {
            key: "Hello".into(),
        };
        let t: Type = get.into();
        let expected = Type::Array(
            vec![
                Type::BulkString(b"GET".to_vec()),
                Type::BulkString(b"Hello".to_vec()),
            ]
            .into_iter()
            .collect(),
        );
        assert_eq!(t, expected);
    }
}
