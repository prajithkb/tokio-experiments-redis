//! This module defines the [RESP protocol](https://redis.io/topics/protocol)

//!###  RESP protocol
//! The way RESP is used in Redis as a request-response protocol is the following:
//! * Clients send commands to a Redis server as a RESP Array of Bulk Strings.
//! * The server replies with one of the RESP types according to the command implementation.
//! #### Data types
//! In RESP, the type of some data depends on the first byte:
//! * For Simple Strings the first byte of the reply is "+"
//! * For Errors the first byte of the reply is "-"
//! * For Integers the first byte of the reply is ":"
//! * For Bulk Strings the first byte of the reply is "$"
//! * For Arrays the first byte of the reply is "*"
//! 
//! See [Type] for the different data types

/// The RESP data type
#[derive()]
pub enum Type {
    /// Simple Strings are encoded in the following way: a plus character, 
    /// followed by a string that cannot contain a CR or LF character (no newlines are allowed), 
    /// terminated by CRLF (that is "\r\n")
    ///
    /// Example: `"+OK\r\n"`
    SimpleString,
    /// RESP has a specific data type for errors. Actually errors are exactly like RESP Simple Strings, 
    /// but the first character is a minus '-' character instead of a plus. 
    /// The real difference between Simple Strings and Errors in RESP is that errors are treated by clients as exceptions, 
    /// and the string that composes the Error type is the error message itself.
    ///
    /// Example: `"-Error message\r\n"`
    Error,
    /// This type is just a CRLF terminated string representing an integer, prefixed by a ":" byte.
    ///
    ///Example: `":1000\r\n"`
    Integer,
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
    BulkString,
    /// Clients send commands to the Redis server using RESP Arrays. 
    /// Similarly certain Redis commands returning collections of elements to the client use RESP Arrays are reply type. 
    /// `"*2\r\n$3\r\nfoo\r\n$3\r\nbar\r\n"` is an array of two RESP Bulk Strings "foo" and "bar".
    ///RESP Arrays are sent using the following format:
    /// A * character as the first byte, followed by the number of elements in the array as a decimal number, followed by CRLF.
    ///An additional RESP type for every element of the Array.
    /// It can contain mixed types
    Array
}
