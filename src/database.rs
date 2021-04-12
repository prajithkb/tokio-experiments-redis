use std::{
    collections::{HashMap, LinkedList},
    fmt::Debug,
    sync::{Arc, Mutex, MutexGuard},
};

use log::{debug, info};
use tokio::sync::mpsc::Sender;

use crate::{
    commands::{
        get::Get,
        list::Push,
        set::Set,
        watch::{Watch, WatchResult},
    },
    resp::Type,
};

/// The type of changes
#[repr(u8)]
#[derive(Debug, PartialEq, Clone)]
pub enum Operation {
    /// Notification for an addition
    Addition = 1,
    /// Notification for an update
    Update = 2,
    /// Notification for a removal
    Removal = 3,
    /// Notification for any operation
    All = 4,
}

impl From<u8> for Operation {
    fn from(id: u8) -> Self {
        match id {
            1 => Operation::Addition,
            2 => Operation::Update,
            3 => Operation::Removal,
            _ => Operation::All,
        }
    }
}
/// The subscription for a change
#[derive(Debug)]
pub struct OperationSubscription {
    operation: Operation,
    subscriber: Sender<Type>,
}

impl OperationSubscription {
    fn new(operation: Operation, subscriber: Sender<Type>) -> Self {
        OperationSubscription {
            operation,
            subscriber,
        }
    }
}

/// RedisString is how the data is stored in the data base
#[derive(Debug, PartialEq, Hash, Eq, Clone)]
pub(crate) struct RedisString {
    bytes: Arc<Vec<u8>>,
}

impl From<String> for RedisString {
    fn from(s: String) -> Self {
        Self {
            bytes: Arc::new(s.into_bytes()),
        }
    }
}

impl From<RedisString> for String {
    fn from(s: RedisString) -> Self {
        String::from_utf8(s.bytes.as_ref().clone()).expect("Not a string")
    }
}

impl From<&str> for RedisString {
    fn from(s: &str) -> Self {
        Self {
            bytes: Arc::new(s.into()),
        }
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
            Value::String(s) => Type::SimpleString(
                String::from_utf8(s.bytes.as_ref().clone()).expect("Not a valid string"),
            ),
            Value::List(l) => Type::Array(
                l.into_iter()
                    .filter_map(|s| match s {
                        // We look for strings
                        Value::String(s) => Some(s),
                        // No nested lists
                        _ => None,
                    })
                    .map(|s| String::from_utf8(s.bytes.as_ref().clone()))
                    .filter_map(|s| s.ok())
                    .map(Type::SimpleString)
                    .collect(),
            ),
        }
    }
}

pub(crate) trait Subscriber: Send {
    fn notify(&mut self, operation: Operation, before: Option<Value>, after: Value);
}

/// The Redis Data base
#[derive(Default)]
pub(crate) struct Database {
    inner: Arc<Mutex<HashMap<RedisString, Value>>>,
    subscriptions: Arc<Mutex<HashMap<RedisString, LinkedList<OperationSubscription>>>>,
}

impl Database {
    pub(crate) fn new() -> Self {
        Database {
            inner: Arc::new(Mutex::new(HashMap::new())),
            subscriptions: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    fn lock_and_access_inner(&mut self) -> MutexGuard<HashMap<RedisString, Value>> {
        self.inner.lock().expect("Lock failed")
    }

    fn lock_and_access_subscriptions(
        &mut self,
    ) -> MutexGuard<HashMap<RedisString, LinkedList<OperationSubscription>>> {
        self.subscriptions.lock().expect("Lock failed")
    }

    /** The different database operations **/
    pub(crate) fn get(&mut self, get: Get) -> Type {
        let key: RedisString = get.key.into();
        let db = self.lock_and_access_inner();
        match db.get(&key).cloned() {
            Some(v) => v.into(),
            None => Type::Null,
        }
    }

    pub(crate) fn set(&mut self, set: Set) -> Type {
        let key: RedisString = set.key.into();
        let value: RedisString = set.value.into();
        self.insert(key, Value::String(value));
        Type::SimpleString("Ok".into())
    }

    pub(crate) fn push(&mut self, p: Push) -> Type {
        let r_key: RedisString = p.list_name.clone().into();
        let mut db = self.lock_and_access_inner();
        match db.get_mut(&r_key) {
            // If there is a value and it is a list already we are good
            // If it is not a list, return an error
            Some(v) => match v {
                // Not a a list return error
                // A list add these elements to it
                Value::List(list) => {
                    let len = p.values.len();
                    p.values
                        .into_iter()
                        .for_each(|i| list.push_back(Value::String(i.into())));
                    log_and_return(
                        format!("Found list `{}`, and pushed {} elments", p.list_name, len),
                        Type::Integer(list.len() as i64),
                    )
                }
                _ => log_and_return(
                    format!("key `{}` is not a list", p.list_name),
                    Type::Error(format!(
                        "key `{}` exists and it is not a list",
                        &p.list_name
                    )),
                ),
            },
            // There is no value, we will create one
            None => {
                let len = p.values.len();
                let name = p.list_name.clone();
                db.insert(r_key, p.values.into());
                log_and_return(
                    format!("Created a new list {} and pushed {} elements", name, len),
                    Type::Integer(len as i64),
                )
            }
        }
    }

    pub(crate) fn watch(&mut self, watch: Watch, subscriber_sink: Sender<Type>) -> Type {
        self.subscribe_for_changes(
            watch.key.into(),
            OperationSubscription::new(watch.operation, subscriber_sink),
        );
        Type::SimpleString("Ok".into())
    }

    fn insert(&mut self, k: RedisString, v: Value) -> Option<Value> {
        let mut db = self.lock_and_access_inner();
        let before = db.insert(k.clone(), v.clone());
        drop(db);
        self.invoke_subscribers(k, before.clone(), v);
        before
    }

    fn subscribe_for_changes(
        &mut self,
        key: RedisString,
        operation_subscription: OperationSubscription,
    ) {
        let mut db = self.lock_and_access_subscriptions();
        let subscriptions = db.entry(key).or_insert_with(LinkedList::new);
        subscriptions.push_back(operation_subscription)
    }

    fn invoke_subscribers(&mut self, key: RedisString, before: Option<Value>, after: Value) {
        info!(
            "Invoking subscriber before:{:?}, after: {:?}",
            before, after
        );
        let mut subscriptions = self.lock_and_access_subscriptions();
        if let Some(subscriptions) = subscriptions.get_mut(&key) {
            let operation = match before {
                Some(_) => Operation::Update,
                None => Operation::Addition,
            };
            subscriptions
                .iter_mut()
                // .filter(|s| s.operation == operation || s.operation == Operation::All)
                .for_each(|s| {
                    let key = key.clone();
                    let sender = s.subscriber.clone();
                    let operation = operation.clone();
                    let before = before.clone();
                    let after = after.clone();
                    tokio::spawn(async move {
                        let watch_result = WatchResult {
                            key: key.into(),
                            operation,
                            before: before.map(|v| v.into()),
                            after: after.into(),
                        };
                        sender.send(watch_result.into()).await.expect("Error");
                    });
                });
        }
    }
}

fn log_and_return(message: String, result: Type) -> Type {
    debug!("{}", message);
    result
}

impl Clone for Database {
    fn clone(&self) -> Self {
        Self {
            inner: self.inner.clone(),
            subscriptions: self.subscriptions.clone(),
        }
    }
}
