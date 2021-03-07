//! This module provides the parsing ability and constructs a [Type]

use crate::resp::Type;
use atoi::atoi;
use bytes::Buf;
use std::io::Cursor;
use std::{
    collections::VecDeque,
    convert::TryInto,
    error::Error,
    fmt::Display,
    io::{Seek, SeekFrom},
    mem::discriminant,
    string::FromUtf8Error,
};

/// The different kinds of errors while parsing
#[derive(Debug)]
pub enum ParseError {
    /// This error indicates that the Parsing is incomplete (i.e. did not find CRLF). If this happens
    /// invoke the [Parse::parse_next] method again with next additional bytes
    Incomplete,
    /// This error indicates and invalid marker byte (allowed ones are '+', ':'..). All strings must be encoded using the RESP format
    InvalidEncoding(u8),
    /// Not an integer
    NotAnInteger,
    /// Implies the end the stream, try again when there are more bytes.
    EndOfBytes,
    /// Implies that the byte length prefix is incorrect (e.g. negative numbers)
    InvalidByteLength(i64),
    /// Any other error
    Other(crate::Error),
}

impl PartialEq<ParseError> for ParseError {
    fn eq(&self, other: &ParseError) -> bool {
        discriminant(self) == discriminant(other)
    }
}
impl Error for ParseError {}

impl Display for ParseError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ParseError::Incomplete => f.write_str("Incomplete message, does not end with CRLF"),
            ParseError::InvalidEncoding(s) => f.write_fmt(format_args!("{:?}", &s)),
            ParseError::EndOfBytes => f.write_str("End of stream"),
            ParseError::Other(any) => f.write_fmt(format_args!("{:?}", any)),
            ParseError::NotAnInteger => f.write_str("Not a number"),
            ParseError::InvalidByteLength(any) => f.write_fmt(format_args!("{:?}", any)),
        }
    }
}
impl From<FromUtf8Error> for ParseError {
    fn from(e: FromUtf8Error) -> Self {
        ParseError::Other(e.into())
    }
}

/// A utility struct that is used to create [Type] instances from a byte array ([u8])
#[derive(Debug, Default)]
pub struct Parse {
    /// An iterator for type
    parts: VecDeque<Type>,
}

impl Parse {
    /// Creates a new instance of [Parse]
    pub fn new() -> Self {
        Self {
            parts: VecDeque::new(),
        }
    }

    /// Everytime this is called either a [Type] is returned or an error is returned.
    /// See [ParseError] to see the different errors and how it should be handled.
    pub fn parse_next(&mut self, bytes: &mut Cursor<&[u8]>) -> Result<Type, ParseError> {
        if bytes.remaining() < 1 {
            return Err(ParseError::EndOfBytes);
        }
        let marker = bytes.get_u8();
        match marker {
            b'+' => {
                let line = get_line(bytes)?;
                parse_string(line.to_vec())
            }
            b'-' => {
                let line = get_line(bytes)?;
                parse_error(line.to_vec())
            }
            b':' => {
                let line = get_line(bytes)?;
                parse_integer(line.to_vec())
            }
            b'$' => {
                let number = get_line(bytes)?;
                // gets the number of bytes to read
                let number_of_bytes: i64 = atoi::atoi(number).ok_or(ParseError::NotAnInteger)?;
                if number_of_bytes < 0 {
                    if number_of_bytes == -1 {
                        return Ok(Type::Null);
                    } else {
                        return Err(ParseError::InvalidByteLength(number_of_bytes));
                    }
                }
                let line = get_bytes(bytes, number_of_bytes.try_into().unwrap())?;
                parse_bulk_string(line.to_vec())
            }
            b'*' => {
                let line = get_line(bytes)?;
                let number_of_elements: usize = atoi::atoi(line).ok_or(ParseError::NotAnInteger)?;
                let mut types_array: Vec<Type> = Vec::with_capacity(number_of_elements);
                for _ in 0..number_of_elements {
                    match self.parse_next(bytes) {
                        Ok(t) => types_array.push(t),
                        Err(e) => {
                            match e {
                                // in case of array, either of these mean incomplete
                                ParseError::EndOfBytes | ParseError::Incomplete => {
                                    return Err(ParseError::Incomplete)
                                }
                                _ => return Err(e),
                            }
                        }
                    };
                }
                parse_array(types_array)
            }
            _ => Err(ParseError::InvalidEncoding(marker)),
        }
    }
}

fn get_bytes<'a>(
    bytes: &'a mut Cursor<&[u8]>,
    number_of_bytes: usize,
) -> Result<&'a [u8], ParseError> {
    if bytes.remaining() < number_of_bytes {
        Err(ParseError::Incomplete)
    } else {
        // this is fine
        let position = bytes.position() as usize;
        let &r = bytes.get_ref();
        // get the number of bytes
        let result = &r[position..(position + number_of_bytes)];
        if bytes.remaining() >= 2 {
            // Seek till the end of  CRLF
            let seek_to = (number_of_bytes + 2).try_into().unwrap();
            bytes
                .seek(SeekFrom::Current(seek_to))
                .expect("Should not seek beyond limits");
        }
        Ok(result)
    }
}

/// Gets a line (ending in CRLF) from the cursor and returns it.
fn get_line<'a>(bytes: &'a mut Cursor<&[u8]>) -> Result<&'a [u8], ParseError> {
    let start = bytes.position() as usize;
    let end = bytes.get_ref().len() - 1;
    for i in start..end {
        if bytes.get_ref()[i] == b'\r' && bytes.get_ref()[i + 1] == b'\n' {
            // We found a line, update the position to be *after* the \n
            bytes.set_position((i + 2) as u64);

            // Return the line
            return Ok(&bytes.get_ref()[start..i]);
        }
    }
    Err(ParseError::Incomplete)
}

fn as_string(bytes: Vec<u8>) -> Result<String, ParseError> {
    Ok(String::from_utf8(bytes)?)
}
fn parse_string(bytes: Vec<u8>) -> Result<Type, ParseError> {
    Ok(Type::SimpleString(as_string(bytes)?))
}
fn parse_error(bytes: Vec<u8>) -> Result<Type, ParseError> {
    Ok(Type::Error(as_string(bytes)?))
}
fn parse_integer(bytes: Vec<u8>) -> Result<Type, ParseError> {
    let integer = atoi(&bytes[..]).ok_or(ParseError::NotAnInteger)?;
    Ok(Type::Integer(integer))
}
fn parse_bulk_string(bytes: Vec<u8>) -> Result<Type, ParseError> {
    Ok(Type::BulkString(bytes))
}
fn parse_array(types: Vec<Type>) -> Result<Type, ParseError> {
    Ok(Type::Array(types))
}
#[cfg(test)]
mod test {

    mod get {
        use crate::parse::get_bytes;
        use crate::parse::get_line;
        use crate::parse::ParseError;
        use std::io::Cursor;
        #[test]
        fn get_line_works() {
            let mut test = Cursor::new(&b"hellow\n"[..]);
            let actual = get_line(&mut test).unwrap_err();
            assert_eq!(actual, ParseError::Incomplete);
            let mut test = Cursor::new(&b"hellow\r"[..]);
            let actual = get_line(&mut test).unwrap_err();
            assert_eq!(actual, ParseError::Incomplete);
            let mut test = Cursor::new(&b"hellow\rs"[..]);
            let actual = get_line(&mut test).unwrap_err();
            assert_eq!(actual, ParseError::Incomplete);
            let mut test = Cursor::new(&b"hellow\ns"[..]);
            let actual = get_line(&mut test).unwrap_err();
            assert_eq!(actual, ParseError::Incomplete);
            let mut test = Cursor::new(&b"hellow\r\n"[..]);
            let actual = get_line(&mut test);
            assert_eq!(actual, Ok(&b"hellow"[..]));
            let mut test = Cursor::new(&b"\r\n"[..]);
            let actual = get_line(&mut test);
            assert_eq!(actual, Ok(&b""[..]));
        }

        #[test]
        fn get_bytes_works() {
            let mut test = Cursor::new(&b"hellow\r\n"[..]);
            let actual = get_bytes(&mut test, 4);
            assert_eq!(actual, Ok(&b"hell"[..]));
            let actual = get_bytes(&mut test, 4);
            assert_eq!(actual, Err(ParseError::Incomplete));
        }
    }

    mod parse {
        use crate::parse::Parse;
        use crate::parse::ParseError;
        use crate::parse::*;
        use crate::resp::Type;
        use std::io::Cursor;

        #[test]
        fn parse_string_works() {
            // success
            let t = parse_string(b"OK"[..].to_vec());
            assert_eq!(t, Ok(Type::SimpleString("OK".into())));
        }

        #[test]
        fn parse_error_works() {
            // success
            let t = parse_error(b"Error"[..].to_vec());
            assert_eq!(t, Ok(Type::Error("Error".into())));
        }

        #[test]
        fn parse_integer_works() {
            // success
            let t = parse_integer(b"100"[..].to_vec());
            assert_eq!(t, Ok(Type::Integer(100u64)));
        }

        #[test]
        fn parse_bulk_string_works() {
            // success
            let t = parse_bulk_string(b"bulk string"[..].to_vec());
            assert_eq!(t, Ok(Type::BulkString(b"bulk string"[..].to_vec())));
        }

        #[test]
        fn parse_array_works() {
            let a = vec![Type::SimpleString("a".into()), Type::Integer(3)];
            // success
            let t = parse_array(a.clone());
            assert_eq!(t, Ok(Type::Array(a)));
        }

        #[test]
        fn parse_next_string_works() {
            // Success
            let mut test = Cursor::new(&b"+OK\r\n"[..]);
            let mut parse = Parse::new();
            let t = parse.parse_next(&mut test);
            assert_eq!(t, Ok(Type::SimpleString("OK".into())));
            // Error
            let mut test = Cursor::new(&b"+OK"[..]);
            let mut parse = Parse::new();
            let t = parse.parse_next(&mut test);
            assert_eq!(t, Err(ParseError::Incomplete));
        }
        #[test]
        fn parse_next_error_works() {
            // Success
            let mut test = Cursor::new(&b"-Error\r\n"[..]);
            let mut parse = Parse::new();
            let t = parse.parse_next(&mut test);
            assert_eq!(t, Ok(Type::Error("Error".into())));
            // Error
            let mut test = Cursor::new(&b"-Error"[..]);
            let mut parse = Parse::new();
            let t = parse.parse_next(&mut test);
            assert_eq!(t, Err(ParseError::Incomplete));
        }

        #[test]
        fn parse_next_integer_works() {
            // Success
            let mut test = Cursor::new(&b":12345\r\n"[..]);
            let mut parse = Parse::new();
            let t = parse.parse_next(&mut test);
            assert_eq!(t, Ok(Type::Integer(12345u64)));
            // Success (partial integer)
            let mut test = Cursor::new(&b":123s45\r\n"[..]);
            let mut parse = Parse::new();
            let t = parse.parse_next(&mut test);
            assert_eq!(t, Ok(Type::Integer(123u64)));
            // Error (incomplete)
            let mut test = Cursor::new(&b":12345"[..]);
            let mut parse = Parse::new();
            let t = parse.parse_next(&mut test);
            assert_eq!(t, Err(ParseError::Incomplete));
            // Error (not integer)
            let mut test = Cursor::new(&b":asda\r\n"[..]);
            let mut parse = Parse::new();
            let t = parse.parse_next(&mut test);
            assert_eq!(t, Err(ParseError::NotAnInteger));
        }

        #[test]
        fn parse_next_null_works() {
            // Success
            let mut test = Cursor::new(&b"$-1\r\n"[..]);
            let mut parse = Parse::new();
            let t = parse.parse_next(&mut test);
            assert_eq!(t, Ok(Type::Null));
        }


        #[test]
        fn parse_next_bulk_works() {
            // Success
            let mut test = Cursor::new(&b"$10\r\n1234567890\r\n"[..]);
            let mut parse = Parse::new();
            let t = parse.parse_next(&mut test);
            assert_eq!(t, Ok(Type::BulkString(b"1234567890"[..].to_vec())));
            // Error
            let mut test = Cursor::new(&b"$10\r\n12345"[..]);
            let mut parse = Parse::new();
            let t = parse.parse_next(&mut test);
            assert_eq!(t, Err(ParseError::Incomplete));
            // Error
            let mut test = Cursor::new(&b"$-10\r\n1234567890\r\n"[..]);
            let mut parse = Parse::new();
            let t = parse.parse_next(&mut test);
            assert_eq!(t, Err(ParseError::InvalidByteLength(-10)));
        }

        #[test]
        fn parse_next_array_works() {
            // success
            let mut test = Cursor::new(&b"*2\r\n$4\r\nLLEN\r\n$6\r\nmylist\r\n"[..]);
            let mut parse = Parse::new();
            let t = parse.parse_next(&mut test);
            let types: Vec<Type> = vec![
                Type::BulkString(b"LLEN"[..].into()),
                Type::BulkString(b"mylist"[..].into()),
            ];
            assert_eq!(t, Ok(Type::Array(types)));
            // error
            let mut test = Cursor::new(&b"*4\r\n$4\r\nLLEN\r\n$6\r\nmylist\r\n"[..]);
            let mut parse = Parse::new();
            let t = parse.parse_next(&mut test);
            assert_eq!(t, Err(ParseError::Incomplete));
        }
        #[test]
        fn parse_invalid_encoding() {
            // Success
            let mut test = Cursor::new(&b"#-1\r\n"[..]);
            let mut parse = Parse::new();
            let t = parse.parse_next(&mut test);
            assert_eq!(t, Err(ParseError::InvalidEncoding(b'#')));
        }
    }
}
