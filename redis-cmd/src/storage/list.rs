use crate::Error;
use redis_resp::BulkString;

/// Insert all the specified values at the head of the list stored at key.
///
/// https://redis.io/docs/latest/commands/lpush
#[derive(Clone, Debug, PartialEq)]
pub struct LPush {
    pub key: BulkString,
    pub elements: Vec<BulkString>,
}

impl LPush {
    pub fn from_args(mut args: impl Iterator<Item = BulkString>) -> Result<Self, Error> {
        let key = args.next().ok_or(Error::MissingArgument)?;
        let elements: Vec<_> = args.collect();
        if elements.is_empty() {
            return Err(Error::MissingArgument);
        }
        Ok(Self { key, elements })
    }
}

/// Inserts element in the list stored at key either before or after the reference value pivot.
///
/// https://redis.io/docs/latest/commands/linsert
#[derive(Clone, Debug, PartialEq)]
pub struct LInsert {
    pub key: BulkString,
    pub pos: BulkString,
    pub pivot: BulkString,
    pub element: BulkString,
}

impl LInsert {
    pub fn from_args(mut args: impl Iterator<Item = BulkString>) -> Result<Self, Error> {
        Ok(Self {
            key: args.next().ok_or(Error::MissingArgument)?,
            pos: args.next().ok_or(Error::MissingArgument)?,
            pivot: args.next().ok_or(Error::MissingArgument)?,
            element: args.next().ok_or(Error::MissingArgument)?,
        })
    }
}

/// Removes and returns the first elements of the list stored at key.
///
/// https://redis.io/docs/latest/commands/lpop
#[derive(Clone, Debug, PartialEq)]
pub struct LPop {
    pub key: BulkString,
    pub count: Option<BulkString>,
}

impl LPop {
    pub fn from_args(mut args: impl Iterator<Item = BulkString>) -> Result<Self, Error> {
        let key = args.next().ok_or(Error::MissingArgument)?;
        let count = args.next();
        Ok(Self { key, count })
    }
}

impl From<LPop> for Vec<BulkString> {
    fn from(cmd: LPop) -> Self {
        let mut args = vec![BulkString::from("LPOP"), cmd.key];

        if let Some(count) = cmd.count {
            args.push(count);
        }

        args
    }
}

/// Returns the element at index index in the list stored at key. The index is zero-based, so 0 means the first element, 1 the second element and so on. Negative indices can be used to designate elements starting at the tail of the list. Here, -1 means the last element, -2 means the penultimate and so forth.
///
/// https://redis.io/docs/latest/commands/lindex
#[derive(Clone, Debug, PartialEq)]
pub struct LIndex {
    pub key: BulkString,
    pub index: BulkString,
}

impl LIndex {
    pub fn from_args(mut args: impl Iterator<Item = BulkString>) -> Result<Self, Error> {
        Ok(Self {
            key: args.next().ok_or(Error::MissingArgument)?,
            index: args.next().ok_or(Error::MissingArgument)?,
        })
    }
}

/// Returns the length of the list stored at key. If key does not exist, it is interpreted as an empty list and 0 is returned. An error is returned when the value stored at key is not a list.
///
/// https://redis.io/docs/latest/commands/llen
#[derive(Clone, Debug, PartialEq)]
pub struct LLen {
    pub key: BulkString,
}

impl LLen {
    pub fn from_args(mut args: impl Iterator<Item = BulkString>) -> Result<Self, Error> {
        Ok(Self {
            key: args.next().ok_or(Error::MissingArgument)?,
        })
    }
}

/// Returns the specified elements of the list stored at key. The offsets start and stop are zero-based indexes, with 0 being the first element of the list (the head of the list), 1 being the next element and so on.
///
/// https://redis.io/docs/latest/commands/lrange
#[derive(Clone, Debug, PartialEq)]
pub struct LRange {
    pub key: BulkString,
    pub start: BulkString,
    pub stop: BulkString,
}

impl LRange {
    pub fn from_args(mut args: impl Iterator<Item = BulkString>) -> Result<Self, Error> {
        Ok(Self {
            key: args.next().ok_or(Error::MissingArgument)?,
            start: args.next().ok_or(Error::MissingArgument)?,
            stop: args.next().ok_or(Error::MissingArgument)?,
        })
    }
}
