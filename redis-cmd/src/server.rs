use std::fmt;

use redis_resp::BulkString;

use crate::Error;

#[derive(Clone, Debug, PartialEq)]
pub enum ServerCommand {
    Sync(Sync),
}

impl From<ServerCommand> for Vec<BulkString> {
    fn from(cmd: ServerCommand) -> Self {
        match cmd {
            ServerCommand::Sync(cmd) => cmd.into(),
        }
    }
}

impl fmt::Display for ServerCommand {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ServerCommand::Sync(cmd) => write!(f, "{cmd}"),
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
    fn from(_: Sync) -> Self {
        vec!["SYNC".into()]
    }
}

impl From<Sync> for ServerCommand {
    fn from(cmd: Sync) -> Self {
        ServerCommand::Sync(cmd)
    }
}

impl fmt::Display for Sync {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "SYNC")
    }
}
