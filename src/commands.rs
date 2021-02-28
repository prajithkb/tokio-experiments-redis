//! The commands module. This module define the suports commands in this implementation of redis


/// Supported commands 
pub enum Command {

    /// Get the value of key. If the key does not exist the special value nil is returned
    ///
    /// Return value: 
    /// * Bulk string reply: the value of key, or nil when key does not exist.
    Get {
        /// The key used to lookup
        key: String
    },
    /// Set key to hold the string value. If key already holds a value, it is overwritten, regardless of its type. 
    ///
    /// Return value: 
    /// * Simple string reply: OK if SET was executed correctly. 
    /// * Bulk string reply: when GET option is set, the old value stored at key, or nil when key did not exist.
    Set {
        /// The key used to lookup
        key: String,
        /// The value
        value: String
    }
}

impl Command {
    
}

