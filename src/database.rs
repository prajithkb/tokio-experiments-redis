use std::{collections::{HashMap, LinkedList}, sync::{Arc, Mutex, MutexGuard}};

use crate::{commands::{Command, get::Get, set::Set}, resp::Type};

/// RedisString is how the data is stored in the data base
#[derive(Debug, PartialEq, Hash, Eq, Clone)]
pub(crate) struct RedisString {
    bytes: Vec<u8>,
}

impl From<String> for RedisString {
    fn from(s: String) -> Self {
        Self {
            bytes: s.into_bytes(),
        }
    }
}

impl From<&str> for RedisString {
    fn from(s: &str) -> Self {
        Self { bytes: s.into() }
    }
}

/// The types of Redis data structures
#[derive(Debug, PartialEq, Hash, Eq, Clone)]
pub(crate) enum Value {
    String(RedisString),
    Null,
    #[allow(dead_code)]
    List(LinkedList<RedisString>),
}

impl From<Value> for Type {
    fn from(v: Value) -> Self {
        match v {
            Value::String(s) => {
                Type::SimpleString(String::from_utf8(s.bytes).expect("Not a valid string"))
            }
            Value::List(l) => Type::Array(
                l.into_iter()
                    .map(|s| {
                        Type::SimpleString(String::from_utf8(s.bytes).expect("Not a valid string"))
                    })
                    .collect(),
            ),
            Value::Null => Type::Null,
        }
    }
}
/// The Redis Data base
#[derive(Debug, Default)]
pub(crate) struct Database {
    inner: Arc<Mutex<HashMap<RedisString, Value>>>,
}

impl Database {
    pub(crate) fn new() -> Self {
        Database {
            inner: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    fn lock_and_access(&mut self) -> MutexGuard<HashMap<RedisString, Value>> {
        self.inner.lock().expect("Lock failed")
    }

    pub(crate) fn act(&mut self, command: Command) -> Value {
        match command {
            Command::Get(g) => get(g, self),
            Command::Set(s) => set(s, self)
        }
    }
}

/** The different database operations **/
fn get(get: Get, db: &mut Database) -> Value {
    let db = db.lock_and_access();
    let key: RedisString = get.key.into();
    match db.get(&key).cloned() {
        Some(v) => v,
        None => Value::Null,
    }
}

fn set(set: Set, db: &mut Database) -> Value {
    let mut db = db.lock_and_access();
    let key: RedisString = set.key.into();
    let value: RedisString = set.value.into();
    db.insert(key, Value::String(value));
    Value::String("Ok".into())
}

impl Clone for Database {
    fn clone(&self) -> Self {
        Self {
            inner: self.inner.clone(),
        }
    }
}

