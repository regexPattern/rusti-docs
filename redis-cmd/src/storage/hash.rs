use std::fmt;

use redis_resp::BulkString;

use crate::Error;

/// Sets the specified fields to their respective values in the hash stored at key.
///
/// This command overwrites the values of specified fields that exist in the hash. If key doesn't exist, a new key holding a hash is created.
///
/// https://redis.io/docs/latest/commands/hset
#[derive(Clone, Debug, PartialEq)]
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

impl From<HSet> for Vec<BulkString> {
    fn from(cmd: HSet) -> Self {
        let mut args = vec![BulkString::from("HSET"), cmd.key];
        args.extend(cmd.field_value_pairs);
        args
    }
}

impl fmt::Display for HSet {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "HSET {}", self.key)?;

        for pair in self.field_value_pairs.chunks_exact(2) {
            write!(f, " {} {}", pair[0], pair[1])?;
        }

        Ok(())
    }
}

/// Returns the value associated with field in the hash stored at key.
///
/// https://redis.io/docs/latest/commands/hget
#[derive(Clone, Debug, PartialEq)]
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

impl From<HGet> for Vec<BulkString> {
    fn from(cmd: HGet) -> Self {
        vec![BulkString::from("HGET"), cmd.key, cmd.field]
    }
}

impl fmt::Display for HGet {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "HGET {} {}", self.key, self.field)
    }
}

/// Removes the specified fields from the hash stored at key. Specified fields that do not exist within this hash are ignored. Deletes the hash if no fields remain. If key does not exist, it is treated as an empty hash and this command returns 0.
///
/// https://redis.io/docs/latest/commands/hdel
#[derive(Clone, Debug, PartialEq)]
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

impl From<HDel> for Vec<BulkString> {
    fn from(cmd: HDel) -> Self {
        let mut args = vec![BulkString::from("HDEL"), cmd.key];
        args.extend(cmd.fields);
        args
    }
}

impl fmt::Display for HDel {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "HDEL {}", self.key)?;

        for field in &self.fields {
            write!(f, " {field}")?;
        }

        Ok(())
    }
}

/// Returns all fields and values of the hash stored at key. In the returned value, every field name is followed by its value, so the length of the reply is twice the size of the hash.
///
/// https://redis.io/docs/latest/commands/hgetall
#[derive(Clone, Debug, PartialEq)]
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

impl From<HGetAll> for Vec<BulkString> {
    fn from(cmd: HGetAll) -> Self {
        vec![BulkString::from("HGETALL"), cmd.key]
    }
}

impl fmt::Display for HGetAll {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "HGETALL {}", self.key)
    }
}

/// Returns all field names in the hash stored at key.
///
/// https://redis.io/docs/latest/commands/hkeys
#[derive(Clone, Debug, PartialEq)]
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

impl From<HKeys> for Vec<BulkString> {
    fn from(cmd: HKeys) -> Self {
        vec![BulkString::from("HKEYS"), cmd.key]
    }
}

impl fmt::Display for HKeys {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "HKEYS {}", self.key)
    }
}

/// Returns all values in the hash stored at key.
///
/// https://redis.io/docs/latest/commands/hvals
#[derive(Clone, Debug, PartialEq)]
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

impl From<HVals> for Vec<BulkString> {
    fn from(cmd: HVals) -> Self {
        vec![BulkString::from("HVALS"), cmd.key]
    }
}

impl fmt::Display for HVals {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "HVALS {}", self.key)
    }
}

/// Returns if field is an existing field in the hash stored at key.
///
/// https://redis.io/docs/latest/commands/hexists
#[derive(Clone, Debug, PartialEq)]
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

impl From<HExists> for Vec<BulkString> {
    fn from(cmd: HExists) -> Self {
        vec![BulkString::from("HEXISTS"), cmd.key, cmd.field]
    }
}

impl fmt::Display for HExists {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "HEXISTS {} {}", self.key, self.field)
    }
}
