use std::fmt;

use redis_resp::BulkString;

use crate::Error;

/// Add the specified members to the set stored at key. Specified members that are already a member of this set are ignored. If key does not exist, a new set is created before adding the specified members.
///
/// https://redis.io/docs/latest/commands/sadd
#[derive(Clone, Debug, PartialEq)]
pub struct SAdd {
    pub key: BulkString,
    pub members: Vec<BulkString>,
}

impl SAdd {
    pub fn from_args(mut args: impl Iterator<Item = BulkString>) -> Result<Self, Error> {
        let key = args.next().ok_or(Error::MissingArgument)?;
        let members: Vec<BulkString> = args.collect();

        if members.is_empty() {
            return Err(Error::MissingArgument);
        }

        Ok(Self { key, members })
    }
}

impl From<SAdd> for Vec<BulkString> {
    fn from(cmd: SAdd) -> Self {
        let mut args = vec![BulkString::from("SADD"), cmd.key];
        args.extend(cmd.members);
        args
    }
}

impl fmt::Display for SAdd {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "SADD {}", self.key)?;

        for m in &self.members {
            write!(f, " {}", m)?;
        }

        Ok(())
    }
}

/// Remove the specified members from the set stored at key. Specified members that are not a member of this set are ignored. If key does not exist, it is treated as an empty set and this command returns 0.
///
/// https://redis.io/docs/latest/commands/srem
#[derive(Clone, Debug, PartialEq)]
pub struct SRem {
    pub key: BulkString,
    pub members: Vec<BulkString>,
}

impl SRem {
    pub fn from_args(mut args: impl Iterator<Item = BulkString>) -> Result<Self, Error> {
        Ok(Self {
            key: args.next().ok_or(Error::MissingArgument)?,
            members: args.collect(),
        })
    }
}

impl From<SRem> for Vec<BulkString> {
    fn from(cmd: SRem) -> Self {
        let mut args = vec![BulkString::from("SREM"), cmd.key];
        args.extend(cmd.members);
        args
    }
}

impl fmt::Display for SRem {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "SREM {}", self.key)?;

        for m in &self.members {
            write!(f, " {}", m)?;
        }

        Ok(())
    }
}

/// Returns the set cardinality (number of elements) of the set stored at key.
///
/// https://redis.io/docs/latest/commands/scard
#[derive(Clone, Debug, PartialEq)]
pub struct SCard {
    pub key: BulkString,
}

impl SCard {
    pub fn from_args(mut args: impl Iterator<Item = BulkString>) -> Result<Self, Error> {
        Ok(Self {
            key: args.next().ok_or(Error::MissingArgument)?,
        })
    }
}

impl From<SCard> for Vec<BulkString> {
    fn from(cmd: SCard) -> Self {
        vec![BulkString::from("SCARD"), cmd.key]
    }
}

impl fmt::Display for SCard {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "SCARD {}", self.key)
    }
}

/// Returns if member is a member of the set stored at key.
///
/// https://redis.io/docs/latest/commands/sismember
#[derive(Clone, Debug, PartialEq)]
pub struct SIsMember {
    pub key: BulkString,
    pub member: BulkString,
}

impl SIsMember {
    pub fn from_args(mut args: impl Iterator<Item = BulkString>) -> Result<Self, Error> {
        Ok(Self {
            key: args.next().ok_or(Error::MissingArgument)?,
            member: args.next().ok_or(Error::MissingArgument)?,
        })
    }
}

impl From<SIsMember> for Vec<BulkString> {
    fn from(cmd: SIsMember) -> Self {
        vec![BulkString::from("SISMEMBER"), cmd.key, cmd.member]
    }
}

impl fmt::Display for SIsMember {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "SISMEMBER {} {}", self.key, self.member)
    }
}

/// Returns all the members of the set value stored at key.
///
/// https://redis.io/docs/latest/commands/smembers
#[derive(Clone, Debug, PartialEq)]
pub struct SMembers {
    pub key: BulkString,
}

impl SMembers {
    pub fn from_args(mut args: impl Iterator<Item = BulkString>) -> Result<Self, Error> {
        Ok(Self {
            key: args.next().ok_or(Error::MissingArgument)?,
        })
    }
}

impl From<SMembers> for Vec<BulkString> {
    fn from(cmd: SMembers) -> Self {
        vec![BulkString::from("SMEMBERS"), cmd.key]
    }
}

impl fmt::Display for SMembers {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "SMEMBERS {}", self.key)
    }
}
