mod generic;
mod hash;
mod list;
mod set;
mod string;

use std::fmt;

pub use generic::*;
pub use hash::*;
pub use list::*;
use redis_resp::BulkString;
pub use set::*;
pub use string::*;

/// Comandos de storage.
#[derive(Clone, Debug, PartialEq)]
pub enum StorageCommand {
    Del(Del),

    HSet(HSet),
    HGet(HGet),
    HDel(HDel),
    HGetAll(HGetAll),
    HKeys(HKeys),
    HVals(HVals),
    HExists(HExists),

    LPush(LPush),
    LInsert(LInsert),
    LPop(LPop),
    LIndex(LIndex),
    LLen(LLen),
    LRange(LRange),

    SAdd(SAdd),
    SRem(SRem),
    SCard(SCard),
    SIsMember(SIsMember),
    SMembers(SMembers),

    Set(Set),
    Get(Get),
    Append(Append),
    Decr(Decr),
    Incr(Incr),
}

impl StorageCommand {
    pub fn key(&self) -> &BulkString {
        match self {
            StorageCommand::Del(del) => (&del.key).into(),
            StorageCommand::HSet(hset) => (&hset.key).into(),
            StorageCommand::HGet(hget) => (&hget.key).into(),
            StorageCommand::HDel(hdel) => (&hdel.key).into(),
            StorageCommand::HGetAll(hget_all) => (&hget_all.key).into(),
            StorageCommand::HKeys(hkeys) => (&hkeys.key).into(),
            StorageCommand::HVals(hvals) => (&hvals.key).into(),
            StorageCommand::HExists(hexists) => (&hexists.key).into(),
            StorageCommand::LPush(lpush) => (&lpush.key).into(),
            StorageCommand::LInsert(linsert) => (&linsert.key).into(),
            StorageCommand::LPop(lpop) => (&lpop.key).into(),
            StorageCommand::LIndex(lindex) => (&lindex.key).into(),
            StorageCommand::LLen(llen) => (&llen.key).into(),
            StorageCommand::LRange(lrange) => (&lrange.key).into(),
            StorageCommand::SAdd(sadd) => (&sadd.key).into(),
            StorageCommand::SRem(srem) => (&srem.key).into(),
            StorageCommand::SCard(scard) => (&scard.key).into(),
            StorageCommand::SIsMember(sis_member) => (&sis_member.key).into(),
            StorageCommand::SMembers(smembers) => (&smembers.key).into(),
            StorageCommand::Set(set) => (&set.key).into(),
            StorageCommand::Get(get) => (&get.key).into(),
            StorageCommand::Append(append) => (&append.key).into(),
            StorageCommand::Decr(decr) => (&decr.key).into(),
            StorageCommand::Incr(incr) => (&incr.key).into(),
        }
    }
}

impl From<StorageCommand> for Vec<BulkString> {
    fn from(cmd: StorageCommand) -> Self {
        match cmd {
            StorageCommand::Del(cmd) => cmd.into(),
            StorageCommand::HSet(cmd) => cmd.into(),
            StorageCommand::HGet(cmd) => cmd.into(),
            StorageCommand::HDel(cmd) => cmd.into(),
            StorageCommand::HGetAll(cmd) => cmd.into(),
            StorageCommand::HKeys(cmd) => cmd.into(),
            StorageCommand::HVals(cmd) => cmd.into(),
            StorageCommand::HExists(cmd) => cmd.into(),
            StorageCommand::LPush(cmd) => cmd.into(),
            StorageCommand::LInsert(cmd) => cmd.into(),
            StorageCommand::LPop(cmd) => cmd.into(),
            StorageCommand::LIndex(cmd) => cmd.into(),
            StorageCommand::LLen(cmd) => cmd.into(),
            StorageCommand::LRange(cmd) => cmd.into(),
            StorageCommand::SAdd(cmd) => cmd.into(),
            StorageCommand::SRem(cmd) => cmd.into(),
            StorageCommand::SCard(cmd) => cmd.into(),
            StorageCommand::SIsMember(cmd) => cmd.into(),
            StorageCommand::SMembers(cmd) => cmd.into(),
            StorageCommand::Set(cmd) => cmd.into(),
            StorageCommand::Get(cmd) => cmd.into(),
            StorageCommand::Append(cmd) => cmd.into(),
            StorageCommand::Decr(cmd) => cmd.into(),
            StorageCommand::Incr(cmd) => cmd.into(),
        }
    }
}

impl fmt::Display for StorageCommand {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            StorageCommand::Del(cmd) => write!(f, "{cmd}"),
            StorageCommand::HSet(cmd) => write!(f, "{cmd}"),
            StorageCommand::HGet(cmd) => write!(f, "{cmd}"),
            StorageCommand::HDel(cmd) => write!(f, "{cmd}"),
            StorageCommand::HGetAll(cmd) => write!(f, "{cmd}"),
            StorageCommand::HKeys(cmd) => write!(f, "{cmd}"),
            StorageCommand::HVals(cmd) => write!(f, "{cmd}"),
            StorageCommand::HExists(cmd) => write!(f, "{cmd}"),
            StorageCommand::LPush(cmd) => write!(f, "{cmd}"),
            StorageCommand::LInsert(cmd) => write!(f, "{cmd}"),
            StorageCommand::LPop(cmd) => write!(f, "{cmd}"),
            StorageCommand::LIndex(cmd) => write!(f, "{cmd}"),
            StorageCommand::LLen(cmd) => write!(f, "{cmd}"),
            StorageCommand::LRange(cmd) => write!(f, "{cmd}"),
            StorageCommand::SAdd(cmd) => write!(f, "{cmd}"),
            StorageCommand::SRem(cmd) => write!(f, "{cmd}"),
            StorageCommand::SCard(cmd) => write!(f, "{cmd}"),
            StorageCommand::SIsMember(cmd) => write!(f, "{cmd}"),
            StorageCommand::SMembers(cmd) => write!(f, "{cmd}"),
            StorageCommand::Set(cmd) => write!(f, "{cmd}"),
            StorageCommand::Get(cmd) => write!(f, "{cmd}"),
            StorageCommand::Append(cmd) => write!(f, "{cmd}"),
            StorageCommand::Decr(cmd) => write!(f, "{cmd}"),
            StorageCommand::Incr(cmd) => write!(f, "{cmd}"),
        }
    }
}

impl From<Del> for StorageCommand {
    fn from(cmd: Del) -> Self {
        Self::Del(cmd)
    }
}

impl From<HSet> for StorageCommand {
    fn from(cmd: HSet) -> Self {
        Self::HSet(cmd)
    }
}

impl From<HGet> for StorageCommand {
    fn from(cmd: HGet) -> Self {
        Self::HGet(cmd)
    }
}

impl From<HDel> for StorageCommand {
    fn from(cmd: HDel) -> Self {
        Self::HDel(cmd)
    }
}

impl From<HGetAll> for StorageCommand {
    fn from(cmd: HGetAll) -> Self {
        Self::HGetAll(cmd)
    }
}

impl From<HKeys> for StorageCommand {
    fn from(cmd: HKeys) -> Self {
        Self::HKeys(cmd)
    }
}

impl From<HVals> for StorageCommand {
    fn from(cmd: HVals) -> Self {
        Self::HVals(cmd)
    }
}

impl From<HExists> for StorageCommand {
    fn from(cmd: HExists) -> Self {
        Self::HExists(cmd)
    }
}

impl From<LPush> for StorageCommand {
    fn from(cmd: LPush) -> Self {
        Self::LPush(cmd)
    }
}

impl From<LInsert> for StorageCommand {
    fn from(cmd: LInsert) -> Self {
        Self::LInsert(cmd)
    }
}

impl From<LPop> for StorageCommand {
    fn from(cmd: LPop) -> Self {
        Self::LPop(cmd)
    }
}

impl From<LIndex> for StorageCommand {
    fn from(cmd: LIndex) -> Self {
        Self::LIndex(cmd)
    }
}

impl From<LLen> for StorageCommand {
    fn from(cmd: LLen) -> Self {
        Self::LLen(cmd)
    }
}

impl From<LRange> for StorageCommand {
    fn from(cmd: LRange) -> Self {
        Self::LRange(cmd)
    }
}

impl From<SAdd> for StorageCommand {
    fn from(cmd: SAdd) -> Self {
        Self::SAdd(cmd)
    }
}

impl From<SRem> for StorageCommand {
    fn from(cmd: SRem) -> Self {
        Self::SRem(cmd)
    }
}

impl From<SCard> for StorageCommand {
    fn from(cmd: SCard) -> Self {
        Self::SCard(cmd)
    }
}

impl From<SIsMember> for StorageCommand {
    fn from(cmd: SIsMember) -> Self {
        Self::SIsMember(cmd)
    }
}

impl From<SMembers> for StorageCommand {
    fn from(cmd: SMembers) -> Self {
        Self::SMembers(cmd)
    }
}

impl From<Set> for StorageCommand {
    fn from(cmd: Set) -> Self {
        Self::Set(cmd)
    }
}

impl From<Get> for StorageCommand {
    fn from(cmd: Get) -> Self {
        Self::Get(cmd)
    }
}

impl From<Append> for StorageCommand {
    fn from(cmd: Append) -> Self {
        Self::Append(cmd)
    }
}

impl From<Decr> for StorageCommand {
    fn from(cmd: Decr) -> Self {
        Self::Decr(cmd)
    }
}

impl From<Incr> for StorageCommand {
    fn from(cmd: Incr) -> Self {
        Self::Incr(cmd)
    }
}
