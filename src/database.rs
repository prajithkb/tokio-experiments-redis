use std::{
    collections::{HashMap, LinkedList},
    sync::{Arc, Mutex, MutexGuard},
};

use crate::{
    commands::{get::Get, list::Push, set::Set, Command},
    resp::Type,
};

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
    List(LinkedList<Value>),
}

impl From<String> for Value {
    fn from(s: String) -> Self {
        Value::String(s.into())
    }
}

impl From<LinkedList<String>> for Value {
    fn from(l: LinkedList<String>) -> Self {
        let mut ll: LinkedList<Value> = LinkedList::new();
        l.into_iter().for_each(|v| ll.push_back(v.into()));
        Value::List(ll)
    }
}

impl From<Value> for Type {
    fn from(v: Value) -> Self {
        match v {
            Value::String(s) => {
                Type::SimpleString(String::from_utf8(s.bytes).expect("Not a valid string"))
            }
            Value::List(l) => Type::Array(
                l.into_iter()
                    .filter_map(|s| match s {
                        // We look for strings
                        Value::String(s) => Some(s),
                        // No nested lists
                        _ => None,
                    })
                    .map(|s| String::from_utf8(s.bytes))
                    .filter_map(|s| s.ok())
                    .map(Type::SimpleString)
                    .collect(),
            ),
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

    pub(crate) fn act(&mut self, command: Command) -> Type {
        match command {
            Command::Get(g) => get(g, self),
            Command::Set(s) => set(s, self),
            Command::Push(p) => push(p, self),
            // Command::Pop(k, u) => pop(k, u, self),
        }
    }
}

/** The different database operations **/
fn get(get: Get, db: &mut Database) -> Type {
    let db = db.lock_and_access();
    let key: RedisString = get.key.into();
    match db.get(&key).cloned() {
        Some(v) => v.into(),
        None => Type::Null,
    }
}

fn set(set: Set, db: &mut Database) -> Type {
    let mut db = db.lock_and_access();
    let key: RedisString = set.key.into();
    let value: RedisString = set.value.into();
    db.insert(key, Value::String(value));
    Type::SimpleString("Ok".into())
}

fn push(p: Push, db: &mut Database) -> Type {
    let mut db = db.lock_and_access();
    let r_key: RedisString = p.list_name.clone().into();
    match db.get_mut(&r_key) {
        // If there is a value and it is a list already we are good
        // If it is not a list, return an error
        Some(v) => match v {
            // Not a a list return error
            // A list add these elements to it
            Value::List(list) => {
                p.values
                    .into_iter()
                    .for_each(|i| list.push_back(Value::String(i.into())));
                Type::Integer(list.len() as i64)
            }
            _ => Type::Error(format!("{} is not a list", &p.list_name)),
        },
        // There is no value, we will create one
        None => {
            let len = p.values.len();
            db.insert(r_key, p.values.into());
            Type::Integer(len as i64)
        }
    }
}

// fn pop(key: String, count: usize, db: &mut Database) -> Type {
//     let db = db.lock_and_access();
//     let r_key = key.into();
//     match db.get(&r_key).cloned() {
//         Some(v) => match v {
//             _ => Type::Error(format!("{} is not a list", key)),
//             Value::List(list) => {
//                 let mut popped_elements: LinkedList<Type> = LinkedList::new();
//                 for i in 0..count {
//                     if let Some(v) = list.pop_back() {
//                         let t: Type = v.into();
//                         popped_elements.push_back(t);
//                     } else {
//                         break;
//                     }
//                 }
//                 Type::Array(popped_elements)
//             }
//         },
//         None => Type::Null,
//     }
// }

impl Clone for Database {
    fn clone(&self) -> Self {
        Self {
            inner: self.inner.clone(),
        }
    }
}
