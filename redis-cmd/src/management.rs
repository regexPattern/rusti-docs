use std::fmt;

use redis_resp::BulkString;

use crate::Error;

#[derive(Clone, Debug, PartialEq)]
pub enum ManagementCommand {
    Sync(Sync),
}

impl From<ManagementCommand> for Vec<BulkString> {
    fn from(cmd: ManagementCommand) -> Self {
        match cmd {
            ManagementCommand::Sync(cmd) => cmd.into(),
        }
    }
}

impl fmt::Display for ManagementCommand {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ManagementCommand::Sync(cmd) => write!(f, "{cmd}"),
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct Sync;

impl Sync {
    pub fn from_args(_: impl Iterator<Item = BulkString>) -> Result<Self, Error> {
        Ok(Sync)
    }
}

impl From<Sync> for Vec<BulkString> {
    fn from(cmd: Sync) -> Self {
        vec!["SYNC".into()]
    }
}

impl From<Sync> for ManagementCommand {
    fn from(cmd: Sync) -> Self {
        ManagementCommand::Sync(cmd)
    }
}

impl fmt::Display for Sync {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "SYNC")
    }
}
