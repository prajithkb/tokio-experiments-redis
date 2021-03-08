//! Get command. See [Get command](https://redis.io/commands/get) for official documentation

use std::collections::LinkedList;

use crate::resp::{Type, TypeConsumer};

use super::CommandError;
/// Holds key required for the [Get command](super::Command::Get)
#[derive(Debug, PartialEq)]
pub struct Get {
    key: String,
}

impl Get {
    /// Returns an instance of [super::get::Get] 
    pub fn from(type_consumer: &mut TypeConsumer) -> Result<Self, CommandError> {
        let key = type_consumer.next_string().map_err(|t| CommandError::InvalidFrame(t, "key"))?;
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
    use crate::resp::{Type, TypeConsumer, TypeConsumerError, ConversionFailed};
    use super::CommandError;

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
        );
        let mut tc = TypeConsumer::new(Type::Null);
        let get = Get::from(&mut tc);
        assert_eq!(
            get,
            Err(CommandError::InvalidFrame(TypeConsumerError::ConversionFailed(ConversionFailed {
                from: "Null".into(),
                to: "String"
            }), "key"))
        );
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
