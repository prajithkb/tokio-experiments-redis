//! Set command 
/// Holds the data for [Get command](super::Command::Get)
#[derive(Debug)]
pub struct Set {
    key: String,
    value: String
}