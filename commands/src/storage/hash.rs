use resp::BulkString;

use crate::Error;

/// Sets the specified fields to their respective values in the hash stored at key.
///
/// This command overwrites the values of specified fields that exist in the hash. If key doesn't exist, a new key holding a hash is created.
///
/// https://redis.io/docs/latest/commands/hset
#[derive(Debug, PartialEq)]
pub struct HSet {
    pub key: BulkString,
    pub field_value_pairs: Vec<BulkString>,
}

impl HSet {
    pub fn from_args(mut args: impl Iterator<Item = BulkString>) -> Result<Self, Error> {
        let key = args.next().ok_or(Error::MissingArgument)?;
        let field_value_pairs: Vec<_> = args.collect();

        if field_value_pairs.is_empty() || field_value_pairs.len() % 2 != 0 {
            return Err(Error::MissingArgument);
        }

        Ok(Self {
            key,
            field_value_pairs,
        })
    }
}

/// Returns the value associated with field in the hash stored at key.
///
/// https://redis.io/docs/latest/commands/hget
#[derive(Debug, PartialEq)]
pub struct HGet {
    pub key: BulkString,
    pub field: BulkString,
}

impl HGet {
    pub fn from_args(mut args: impl Iterator<Item = BulkString>) -> Result<Self, Error> {
        Ok(Self {
            key: args.next().ok_or(Error::MissingArgument)?,
            field: args.next().ok_or(Error::MissingArgument)?,
        })
    }
}

/// Removes the specified fields from the hash stored at key. Specified fields that do not exist within this hash are ignored. Deletes the hash if no fields remain. If key does not exist, it is treated as an empty hash and this command returns 0.
///
/// https://redis.io/docs/latest/commands/hdel
#[derive(Debug, PartialEq)]
pub struct HDel {
    pub key: BulkString,
    pub fields: Vec<BulkString>,
}

impl HDel {
    pub fn from_args(mut args: impl Iterator<Item = BulkString>) -> Result<Self, Error> {
        let key = args.next().ok_or(Error::MissingArgument)?;
        let fields: Vec<_> = args.collect();

        if fields.is_empty() {
            return Err(Error::MissingArgument);
        }

        Ok(Self { key, fields })
    }
}

/// Returns all fields and values of the hash stored at key. In the returned value, every field name is followed by its value, so the length of the reply is twice the size of the hash.
///
/// https://redis.io/docs/latest/commands/hgetall
#[derive(Debug, PartialEq)]
pub struct HGetAll {
    pub key: BulkString,
}

impl HGetAll {
    pub fn from_args(mut args: impl Iterator<Item = BulkString>) -> Result<Self, Error> {
        Ok(Self {
            key: args.next().ok_or(Error::MissingArgument)?,
        })
    }
}

/// Returns all field names in the hash stored at key.
///
/// https://redis.io/docs/latest/commands/hkeys
#[derive(Debug, PartialEq)]
pub struct HKeys {
    pub key: BulkString,
}

impl HKeys {
    pub fn from_args(mut args: impl Iterator<Item = BulkString>) -> Result<Self, Error> {
        Ok(Self {
            key: args.next().ok_or(Error::MissingArgument)?,
        })
    }
}

/// Returns all values in the hash stored at key.
///
/// https://redis.io/docs/latest/commands/hvals
#[derive(Debug, PartialEq)]
pub struct HVals {
    pub key: BulkString,
}

impl HVals {
    pub fn from_args(mut args: impl Iterator<Item = BulkString>) -> Result<Self, Error> {
        Ok(Self {
            key: args.next().ok_or(Error::MissingArgument)?,
        })
    }
}

/// Returns if field is an existing field in the hash stored at key.
///
/// https://redis.io/docs/latest/commands/hexists
#[derive(Debug, PartialEq)]
pub struct HExists {
    pub key: BulkString,
    pub field: BulkString,
}

impl HExists {
    pub fn from_args(mut args: impl Iterator<Item = BulkString>) -> Result<Self, Error> {
        Ok(Self {
            key: args.next().ok_or(Error::MissingArgument)?,
            field: args.next().ok_or(Error::MissingArgument)?,
        })
    }
}
