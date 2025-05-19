use std::fmt;

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

impl From<LPush> for Vec<BulkString> {
    fn from(cmd: LPush) -> Self {
        let mut args = vec![BulkString::from("LPUSH"), cmd.key];
        args.extend(cmd.elements);
        args
    }
}

impl fmt::Display for LPush {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "LPUSH {}", self.key)?;

        for e in &self.elements {
            write!(f, " {e}")?;
        }

        Ok(())
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

impl From<LInsert> for Vec<BulkString> {
    fn from(cmd: LInsert) -> Self {
        vec![
            BulkString::from("LINSERT"),
            cmd.key,
            cmd.pos,
            cmd.pivot,
            cmd.element,
        ]
    }
}

impl fmt::Display for LInsert {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "LINSERT {} {} {} {}",
            self.key, self.pos, self.pivot, self.element
        )
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

impl fmt::Display for LPop {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "LPOP {}", self.key)?;

        if let Some(count) = &self.count {
            write!(f, " {count}")?;
        }

        Ok(())
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

impl From<LIndex> for Vec<BulkString> {
    fn from(cmd: LIndex) -> Self {
        vec![BulkString::from("LINDEX"), cmd.key, cmd.index]
    }
}

impl fmt::Display for LIndex {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "LINDEX {} {}", self.key, self.index)
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

impl From<LLen> for Vec<BulkString> {
    fn from(cmd: LLen) -> Self {
        vec![BulkString::from("LLEN"), cmd.key]
    }
}

impl fmt::Display for LLen {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "LLEN {}", self.key)
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

impl From<LRange> for Vec<BulkString> {
    fn from(cmd: LRange) -> Self {
        vec![BulkString::from("LRANGE"), cmd.key, cmd.start, cmd.stop]
    }
}

impl fmt::Display for LRange {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "LRANGE {} {} {}", self.key, self.start, self.stop)
    }
}
