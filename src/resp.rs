//! This module defines the different frames from [RESP protocol](https://redis.io/topics/protocol)

//!###  RESP protocol
//! The way RESP is used in Redis as a request-response protocol is the following:
//! * Clients send commands to a Redis server as a RESP Array of Bulk Strings.
//! * The server replies with one of the RESP types according to the command implementation.
//!
//! #### Data types
//! In RESP, the type of some data depends on the first byte:
//! * For Simple Strings the first byte of the reply is "+"
//! * For Errors the first byte of the reply is "-"
//! * For Integers the first byte of the reply is ":"
//! * For Bulk Strings the first byte of the reply is "$"
//! * For Arrays the first byte of the reply is "*"
//!
//! See [Type] for the different data types

use std::{collections::LinkedList, error::Error, fmt::Display};

/// The RESP data type
#[derive(Debug, PartialEq, Clone)]
pub enum Type {
    /// Simple Strings are encoded in the following way: a plus character,
    /// followed by a string that cannot contain a CR or LF character (no newlines are allowed),
    /// terminated by CRLF (that is "\r\n")
    ///
    /// Example: `"+OK\r\n"`
    SimpleString(String),
    /// RESP has a specific data type for errors. Actually errors are exactly like RESP Simple Strings,
    /// but the first character is a minus '-' character instead of a plus.
    /// The real difference between Simple Strings and Errors in RESP is that errors are treated by clients as exceptions,
    /// and the string that composes the Error type is the error message itself.
    ///
    /// Example: `"-Error message\r\n"`
    Error(String),
    /// This type is just a CRLF terminated string representing an integer, prefixed by a ":" byte.
    ///
    ///Example: `":1000\r\n"`
    Integer(i64),
    /// A special value
    Null,
    /// Bulk Strings are used in order to represent a single binary safe string up to 512 MB in length.
    /// Bulk Strings are encoded in the following way:
    /// * A "$" byte followed by the number of bytes composing the string (a prefixed length), terminated by CRLF.
    /// * The actual string data.
    /// * A final CRLF.
    ///
    /// Examples:
    /// * `"$6\r\nfoobar\r\n"`
    /// * `"$-1\r\n"` is a NULL string
    /// * `"$0\r\n\r\n"` is an empty string
    BulkString(Vec<u8>),
    /// Clients send commands to the Redis server using RESP Arrays.
    /// Similarly certain Redis commands returning collections of elements to the client use RESP Arrays are reply type.
    /// `"*2\r\n$3\r\nfoo\r\n$3\r\nbar\r\n"` is an array of two RESP Bulk Strings "foo" and "bar".
    ///RESP Arrays are sent using the following format:
    /// A * character as the first byte, followed by the number of elements in the array as a decimal number, followed by CRLF.
    /// An additional RESP type for every element of the Array.
    /// It can contain mixed types
    Array(LinkedList<Type>),
}

impl Display for Type {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_fmt(format_args!("{:?}", self))
    }
}
/// Holds the data for `from`  and `to` for [TypeConsumerError::ConversionFailed]
#[derive(Debug, PartialEq)]
pub struct ConversionFailed{ pub(crate) from: String, pub(crate) to: &'static str }

/// The possible errors emitted by [TypeConsumer
#[derive(Debug, PartialEq)]
pub enum TypeConsumerError {
    /// Indicates that conversion has failed (e.g. string to integer)
    ConversionFailed(ConversionFailed),
    /// Indicates that the consumer is empty (this happens if you can `next` on a consumer that has finished)
    Empty,
}

impl Error for TypeConsumerError {}

impl Display for TypeConsumerError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_fmt(format_args!("{:?}", self))
    }
}

/// Creates a consumer that provides a `Iterator` like API for getting the next Type
//
/// Each [Type] is stored as a token. It provides convenient methods to extract `String`, `Integer` or `Bytes`
#[derive(Debug, PartialEq)]
pub struct TypeConsumer {
    inner: Option<Type>,
}

impl TypeConsumer {
    /// Creates a new instance of [TypeConsumer]
    pub fn new(t: Type) -> Self {
        TypeConsumer { inner: Some(t) }
    }

    /// Returns the next token as a [String] if possible or an error otherwise
    pub fn next_string(&mut self) -> Result<String, TypeConsumerError> {
        self.next_token::<String>(next_string)
    }
    /// Returns the next token as a [i64] if possible or an error otherwise
    pub fn next_integer(&mut self) -> Result<i64, TypeConsumerError> {
        self.next_token::<i64>(next_integer)
    }
    /// Returns the next token as a Bytes ([Vec]) if possible or an error otherwise
    pub fn next_bytes(&mut self) -> Result<Vec<u8>, TypeConsumerError> {
        self.next_token::<Vec<u8>>(next_bytes)
    }

    fn next_token<T>(
        &mut self,
        extractor: fn(Type) -> Result<T, TypeConsumerError>,
    ) -> Result<T, TypeConsumerError> {
        match &mut self.inner {
            Some(t) => match t {
                Type::Array(values) => next_token_from_values::<T>(values, extractor),
                _ => extractor(self.inner.take().expect("Cannot be None")),
            },
            _ => Err(TypeConsumerError::Empty),
        }
    }
}

/** Utility methods **/

fn next_token_from_values<T>(
    values: &mut LinkedList<Type>,
    extractor: fn(Type) -> Result<T, TypeConsumerError>,
) -> Result<T, TypeConsumerError> {
    match values.pop_front() {
        Some(t) => extractor(t),
        _ => Err(TypeConsumerError::Empty),
    }
}

fn next_bytes(value: Type) -> Result<Vec<u8>, TypeConsumerError> {
    match value {
        Type::SimpleString(s) => Ok(s.into()),
        Type::BulkString(s) => Ok(s),
        _ => Err(cannot_convert_err(value.to_string(), "Bytes")),
    }
}
fn next_integer(value: Type) -> Result<i64, TypeConsumerError> {
    let v = value.to_string();
    match value {
        Type::SimpleString(s) => {
            atoi::atoi(s.as_bytes()).ok_or_else(|| cannot_convert_err(v, "Integer"))
        }
        Type::BulkString(s) => atoi::atoi(&s).ok_or_else(|| cannot_convert_err(v, "Integer")),
        Type::Integer(s) => Ok(s),
        _ => Err(cannot_convert_err(v, "Integer")),
    }
}
fn next_string(value: Type) -> Result<String, TypeConsumerError> {
    let v = value.to_string();
    match value {
        Type::SimpleString(s) => Ok(s),
        Type::Integer(i) => Ok(i.to_string()),
        Type::BulkString(s) => {
            Ok(String::from_utf8(s).map_err(|_| cannot_convert_err(v, "String"))?)
        }
        _ => Err(cannot_convert_err(v, "String")),
    }
}

fn cannot_convert_err(from: String, to: &'static str) -> TypeConsumerError {
    TypeConsumerError::ConversionFailed(ConversionFailed {
        from,
        to,
    })
}

#[cfg(test)]
mod test {
    use std::collections::LinkedList;

    use super::{Type, TypeConsumer, TypeConsumerError};
    use super::ConversionFailed;
    #[test]
    fn next_string_works() {
        // String
        let t = Type::SimpleString("Hello".into());
        let mut type_consumer = TypeConsumer::new(t);
        assert_eq!(type_consumer.next_string(), Ok("Hello".to_string()));
        assert_eq!(type_consumer.next_string(), Err(TypeConsumerError::Empty));

        // Bulk string
        let t = Type::BulkString(b"Hello".to_vec());
        let mut type_consumer = TypeConsumer::new(t);
        assert_eq!(type_consumer.next_string(), Ok("Hello".to_string()));
        assert_eq!(type_consumer.next_string(), Err(TypeConsumerError::Empty));

        // Integer
        let t = Type::Integer(34);
        let mut type_consumer = TypeConsumer::new(t);
        assert_eq!(
            type_consumer.next_bytes(),
            Err(TypeConsumerError::ConversionFailed(ConversionFailed {
                from: "Integer(34)".into(),
                to: "Bytes"
            }))
        );

        // Array
        let t = Type::Array(LinkedList::new());
        let mut type_consumer = TypeConsumer::new(t);
        assert_eq!(type_consumer.next_bytes(), Err(TypeConsumerError::Empty));

        let t = Type::Array(
            vec![Type::SimpleString("Hello".into())]
                .into_iter()
                .collect(),
        );
        let mut type_consumer = TypeConsumer::new(t);
        assert_eq!(type_consumer.next_string(), Ok("Hello".to_string()));
        assert_eq!(type_consumer.next_string(), Err(TypeConsumerError::Empty));
        // Null
        let t = Type::Null;
        let mut type_consumer = TypeConsumer::new(t);
        assert_eq!(
            type_consumer.next_string(),
            Err(TypeConsumerError::ConversionFailed(ConversionFailed {
                from: "Null".into(),
                to: "String"
            }))
        );
    }

    #[test]
    fn next_integer_works() {
        // String
        let t = Type::SimpleString("Hello".into());
        let mut type_consumer = TypeConsumer::new(t);
        assert_eq!(
            type_consumer.next_integer(),
            Err(TypeConsumerError::ConversionFailed(ConversionFailed {
                from: "SimpleString(\"Hello\")".into(),
                to: "Integer"
            }))
        );

        // Bulk string
        let t = Type::BulkString(b"Hello".to_vec());
        let mut type_consumer = TypeConsumer::new(t);
        assert_eq!(
            type_consumer.next_integer(),
            Err(TypeConsumerError::ConversionFailed(ConversionFailed {
                from: "BulkString([72, 101, 108, 108, 111])".into(),
                to: "Integer"
            }))
        );

        // Integer
        let t = Type::Integer(34);
        let mut type_consumer = TypeConsumer::new(t);
        assert_eq!(type_consumer.next_integer(), Ok(34));
        assert_eq!(type_consumer.next_integer(), Err(TypeConsumerError::Empty));

        // Array
        let t = Type::Array(LinkedList::new());
        let mut type_consumer = TypeConsumer::new(t);
        assert_eq!(type_consumer.next_integer(), Err(TypeConsumerError::Empty));

        let t = Type::Array(vec![Type::Integer(34)].into_iter().collect());
        let mut type_consumer = TypeConsumer::new(t);
        assert_eq!(type_consumer.next_integer(), Ok(34));
        assert_eq!(type_consumer.next_integer(), Err(TypeConsumerError::Empty));
        // Null
        let t = Type::Null;
        let mut type_consumer = TypeConsumer::new(t);
        assert_eq!(
            type_consumer.next_string(),
            Err(TypeConsumerError::ConversionFailed(ConversionFailed {
                from: "Null".into(),
                to: "String"
            }))
        );
    }

    #[test]
    fn next_bytes_works() {
        // String
        let t = Type::SimpleString("Hello".into());
        let mut type_consumer = TypeConsumer::new(t);
        assert_eq!(type_consumer.next_bytes(), Ok("Hello".as_bytes().to_vec()));
        assert_eq!(type_consumer.next_integer(), Err(TypeConsumerError::Empty));
        // Bulk string
        let t = Type::BulkString(b"Hello".to_vec());
        let mut type_consumer = TypeConsumer::new(t);
        assert_eq!(type_consumer.next_bytes(), Ok("Hello".as_bytes().to_vec()));
        assert_eq!(type_consumer.next_bytes(), Err(TypeConsumerError::Empty));

        // Integer
        let t = Type::Integer(34);
        let mut type_consumer = TypeConsumer::new(t);
        assert_eq!(
            type_consumer.next_bytes(),
            Err(TypeConsumerError::ConversionFailed(ConversionFailed {
                from: "Integer(34)".into(),
                to: "Bytes"
            }))
        );

        // Array
        let t = Type::Array(LinkedList::new());
        let mut type_consumer = TypeConsumer::new(t);
        assert_eq!(type_consumer.next_bytes(), Err(TypeConsumerError::Empty));

        let t = Type::Array(vec![Type::BulkString(b"Hello".to_vec())].into_iter().collect());
        let mut type_consumer = TypeConsumer::new(t);
        assert_eq!(type_consumer.next_bytes(), Ok("Hello".as_bytes().to_vec()));
        assert_eq!(type_consumer.next_bytes(), Err(TypeConsumerError::Empty));
        // Null
        let t = Type::Null;
        let mut type_consumer = TypeConsumer::new(t);
        assert_eq!(
            type_consumer.next_string(),
            Err(TypeConsumerError::ConversionFailed(ConversionFailed {
                from: "Null".into(),
                to: "String"
            }))
        );
    }
}
