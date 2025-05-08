mod generic;
mod hash;
mod list;
mod set;
mod string;

pub use generic::*;
pub use hash::*;
pub use list::*;
use resp::BulkString;
pub use set::*;
pub use string::*;

/// Comandos de storage.
#[derive(Debug, PartialEq)]
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

impl From<StorageCommand> for Vec<BulkString> {
    fn from(cmd: StorageCommand) -> Self {
        match cmd {
            StorageCommand::Del(cmd) => todo!(),
            StorageCommand::HSet(cmd) => todo!(),
            StorageCommand::HGet(cmd) => todo!(),
            StorageCommand::HDel(cmd) => todo!(),
            StorageCommand::HGetAll(cmd) => todo!(),
            StorageCommand::HKeys(cmd) => todo!(),
            StorageCommand::HVals(cmd) => todo!(),
            StorageCommand::HExists(cmd) => todo!(),
            StorageCommand::LPush(cmd) => todo!(),
            StorageCommand::LInsert(cmd) => todo!(),
            StorageCommand::LPop(cmd) => todo!(),
            StorageCommand::LIndex(cmd) => todo!(),
            StorageCommand::LLen(cmd) => todo!(),
            StorageCommand::LRange(cmd) => todo!(),
            StorageCommand::SAdd(cmd) => todo!(),
            StorageCommand::SRem(cmd) => todo!(),
            StorageCommand::SCard(cmd) => todo!(),
            StorageCommand::SIsMember(cmd) => todo!(),
            StorageCommand::SMembers(cmd) => todo!(),
            StorageCommand::Set(cmd) => cmd.into(),
            StorageCommand::Get(cmd) => cmd.into(),
            StorageCommand::Append(cmd) => todo!(),
            StorageCommand::Decr(cmd) => todo!(),
            StorageCommand::Incr(cmd) => todo!(),
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
