//! This module defines the [RESP protocol](https://redis.io/topics/protocol)

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

use crate::Result;
use bytes::{Bytes, BytesMut};
use log::{debug};

/// The RESP data type
#[derive()]
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
    Integer(i32),
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
    BulkString(Bytes),
    /// Clients send commands to the Redis server using RESP Arrays.
    /// Similarly certain Redis commands returning collections of elements to the client use RESP Arrays are reply type.
    /// `"*2\r\n$3\r\nfoo\r\n$3\r\nbar\r\n"` is an array of two RESP Bulk Strings "foo" and "bar".
    ///RESP Arrays are sent using the following format:
    /// A * character as the first byte, followed by the number of elements in the array as a decimal number, followed by CRLF.
    /// An additional RESP type for every element of the Array.
    /// It can contain mixed types
    Array(Vec<Type>),
}
pub(crate) struct Parser<I> {
    /// the underlying raw bytes
    parts: Parts<I>,
}

impl<I: Iterator<Item=Bytes>> Iterator for Parser<I>  {
    type Item = Result<Type>;
    fn next(&mut self) -> Option<Self::Item> {
        match self.parts.next() {   
            Some(bytes) => {
                // Take the first byte
                if let Some((&marker, rest)) = bytes.split_first() {
                    match marker {
                        b'+' => return self.create_string(rest),
                        b'-' => return self.create_string(rest),
                        b':' => return self.create_string(rest),
                        b'$' => return self.create_string(rest),
                        b'*' => return self.create_string(rest),
                        _ => return Some(Err(format!("Invalid marker {} for part {:?}", marker, rest).into()))
                    }
                }
                None   
            },
            _ => None
        }
    }
}

impl <I> Parser<I> {
    fn new(parts: Parts<I>) -> Self{
        Self {
            parts
        }
    }
    fn create_string(&self, chars: &[u8]) -> Option<Result<Type>> {
        let s = std::str::from_utf8(chars).map(|s| s.to_string());
        Some(s.map(Type::SimpleString).map_err(|u| u.into()))            
    }
}

#[derive(Debug)]
struct Parts<I> {
    buffer: I,
}

impl<I> Parts<I> {
    fn new(buffer: I) -> Self {
        Self { buffer }
    }

}

impl<I: Iterator<Item=Bytes>> Iterator for Parts<I> {
    type Item = Bytes;
    fn next(&mut self) -> Option<Self::Item> {
        if !self.buffer.is_empty() {
            return Some(self.buffer.remove(0))
        }
        None
    }
}
impl Iterator for Parts<BytesMut> {
    type Item = Bytes;
    fn next(&mut self) -> Option<Self::Item> {
        // if it contains less than 2, it is invalid
        // All strings should end with CR LF
        if self.buffer.len() < 2 {
            return None;
        }
        for i in 1..self.buffer.len() {
            // take two bytes at a time
            let (&first, &second) = (&self.buffer[i - 1], &self.buffer[i]);
            // Find CRLF
            if first == b'\r' && second == b'\n' {
                // Found them, take the bytes until (that includes) CRLF
                let mut bytes = self.buffer.split_to(i + 1);
                // This size includes the two extra CR LF characters
                let size = bytes.len();
                // Get rid of the last two bytes (CR & lF)
                let _last_bytes = bytes.split_off(size - 2);
                return Some(bytes.freeze());
            }
        }
        None
    }
}

#[cfg(test)]
mod tests {

    
mod parts {
        use crate::resp::Parts;
        use bytes::{Bytes, BytesMut};

        #[test]
        fn iterator_works() {
            let message = "*2\r\n$3\r\nfoo\r\n$3\r\nbar\r\n";
            let bytes = BytesMut::from(message);
            let parts = Parts::new(bytes);
            let parts: Vec<Bytes> = parts.collect();
            let expected: Vec<Bytes> = message
                .split_terminator("\r\n")
                .map(Bytes::from)
                .collect();
            assert_eq!(parts, expected);
        }

        #[test]
        fn empty_string_works() {
            let message = "\r\n$3\r\n\r\n\r\n\r\n";
            let bytes = BytesMut::from(message);
            let parts = Parts::new(bytes);
            let parts: Vec<Bytes> = parts.collect();
            let expected: Vec<Bytes> = message
                .split_terminator("\r\n")
                .map(Bytes::from)
                .collect();
            assert_eq!(parts, expected);
        }
        #[test]
        fn invalid_string_works() {
            let message = "\r\r\n$3\r\\n\r\n\r\n\r\n";
            let bytes = BytesMut::from(message);
            let parts = Parts::new(bytes);
            let parts: Vec<Bytes> = parts.collect();
            let expected: Vec<Bytes> = message
                .split_terminator("\r\n")
                .map(Bytes::from)
                .collect();
            assert_eq!(parts, expected);
        }
    }
    mod parser {
        use crate::resp::Parts;
        use crate::resp::Parser;
        use std::iter::*;
        use bytes::{Bytes, BytesMut};

        #[test]
        fn parser_works_for_simple_string() {
            let parts:Vec<Bytes> = vec![
                Bytes::from("+SET MY LIFE"),
                Bytes::from("+GET MY LIFE"),
                Bytes::from("+SET MY LIFE"),
            ];
            let parts = Parts::new(parts);
            let parser = Parser::new(parts);
            // let actual: Vec<String> = parser
            // let expected = vec!["+SET MY LIFE", "+SET MY LIFE", "+SET MY LIFE"];
            // assert_eq!()
        }
    }
}
