use std::fmt;

use redis_resp::BulkString;

use crate::Error;

/// Set key to hold the string value. If key already holds a value, it is overwritten, regardless of its type. Any previous time to live associated with the key is discarded on successful SET operation.
///
/// https://redis.io/docs/latest/commands/set
#[derive(Clone, Debug, PartialEq)]
pub struct Set {
    pub key: BulkString,
    pub value: BulkString,
}

impl Set {
    pub fn from_args(mut args: impl Iterator<Item = BulkString>) -> Result<Self, Error> {
        Ok(Self {
            key: args.next().ok_or(Error::MissingArgument)?,
            value: args.next().ok_or(Error::MissingArgument)?,
        })
    }
}

impl From<Set> for Vec<BulkString> {
    fn from(cmd: Set) -> Self {
        vec![BulkString::from("SET"), cmd.key, cmd.value]
    }
}

impl fmt::Display for Set {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "SET {} {}", self.key, self.value)
    }
}

/// Get the value of key. If the key does not exist the special value nil is returned. An error is returned if the value stored at key is not a string, because GET only handles string values.
///
/// https://redis.io/docs/latest/commands/get
#[derive(Clone, Debug, PartialEq)]
pub struct Get {
    pub key: BulkString,
}

impl Get {
    pub fn from_args(mut args: impl Iterator<Item = BulkString>) -> Result<Self, Error> {
        Ok(Self {
            key: args.next().ok_or(Error::MissingArgument)?,
        })
    }
}

impl From<Get> for Vec<BulkString> {
    fn from(cmd: Get) -> Self {
        vec![BulkString::from("GET"), cmd.key]
    }
}

impl fmt::Display for Get {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "GET {}", self.key)
    }
}

/// If key already exists and is a string, this command appends the value at the end of the string. If key does not exist it is created and set as an empty string, so APPEND will be similar to SET in this special case.
///
/// https://redis.io/docs/latest/commands/append
#[derive(Clone, Debug, PartialEq)]
pub struct Append {
    pub key: BulkString,
    pub value: BulkString,
}

impl Append {
    pub fn from_args(mut args: impl Iterator<Item = BulkString>) -> Result<Self, Error> {
        Ok(Self {
            key: args.next().ok_or(Error::MissingArgument)?,
            value: args.next().ok_or(Error::MissingArgument)?,
        })
    }
}

impl From<Append> for Vec<BulkString> {
    fn from(cmd: Append) -> Self {
        vec![BulkString::from("APPEND"), cmd.key, cmd.value]
    }
}

impl fmt::Display for Append {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "APPEND {} {}", self.key, self.value)
    }
}

/// Decrements the number stored at key by one. If the key does not exist, it is set to 0 before performing the operation. An error is returned if the key contains a value of the wrong type or contains a string that can not be represented as integer. This operation is limited to 64 bit signed integers.
///
/// https://redis.io/docs/latest/commands/decr
#[derive(Clone, Debug, PartialEq)]
pub struct Decr {
    pub key: BulkString,
}

impl Decr {
    pub fn from_args(mut args: impl Iterator<Item = BulkString>) -> Result<Self, Error> {
        Ok(Self {
            key: args.next().ok_or(Error::MissingArgument)?,
        })
    }
}

impl From<Decr> for Vec<BulkString> {
    fn from(cmd: Decr) -> Self {
        vec![BulkString::from("DECR"), cmd.key]
    }
}

impl fmt::Display for Decr {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "DECR {}", self.key)
    }
}

/// Increments the number stored at key by one. If the key does not exist, it is set to 0 before performing the operation. An error is returned if the key contains a value of the wrong type or contains a string that can not be represented as integer. This operation is limited to 64 bit signed integers.
///
/// Note: this is a string operation because Redis does not have a dedicated integer type. The string stored at the key is interpreted as a base-10 64 bit signed integer to execute the operation.
///
/// Redis stores integers in their integer representation, so for string values that actually hold an integer, there is no overhead for storing the string representation of the integer.
///
/// https://redis.io/docs/latest/commands/incr
#[derive(Clone, Debug, PartialEq)]
pub struct Incr {
    pub key: BulkString,
}

impl Incr {
    pub fn from_args(mut args: impl Iterator<Item = BulkString>) -> Result<Self, Error> {
        Ok(Self {
            key: args.next().ok_or(Error::MissingArgument)?,
        })
    }
}

impl From<Incr> for Vec<BulkString> {
    fn from(cmd: Incr) -> Self {
        vec![BulkString::from("INCR"), cmd.key]
    }
}

impl fmt::Display for Incr {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "INCR {}", self.key)
    }
}
