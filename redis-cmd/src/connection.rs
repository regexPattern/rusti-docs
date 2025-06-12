use std::fmt;

use redis_resp::BulkString;

use crate::Error;

#[derive(Clone, Debug, PartialEq)]
pub enum ConnectionCommand {
    Auth(Auth),
    Ping(Ping),
}

impl From<ConnectionCommand> for Vec<BulkString> {
    fn from(cmd: ConnectionCommand) -> Self {
        match cmd {
            ConnectionCommand::Auth(cmd) => cmd.into(),
            ConnectionCommand::Ping(cmd) => cmd.into(),
        }
    }
}

impl fmt::Display for ConnectionCommand {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ConnectionCommand::Auth(cmd) => write!(f, "{cmd}"),
            ConnectionCommand::Ping(cmd) => write!(f, "{cmd}"),
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct Auth {
    pub username: Option<BulkString>,
    pub password: BulkString,
}

impl Auth {
    pub fn from_args(mut args: impl Iterator<Item = BulkString>) -> Result<Self, Error> {
        Ok(Auth {
            username: args.next(),
            password: args.next().ok_or(Error::MissingArgument)?,
        })
    }
}

impl From<Auth> for Vec<BulkString> {
    fn from(cmd: Auth) -> Self {
        let mut cmd_bs = vec!["AUTH".into()];
        if let Some(username) = cmd.username {
            cmd_bs.push(username);
        }
        cmd_bs.push(cmd.password);
        cmd_bs
    }
}

impl From<Auth> for ConnectionCommand {
    fn from(cmd: Auth) -> Self {
        ConnectionCommand::Auth(cmd)
    }
}

impl fmt::Display for Auth {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "AUTH ")?;
        if let Some(username) = &self.username {
            write!(f, "{username} ")?;
        }
        write!(f, "*****")
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct Ping {
    pub message: Option<BulkString>,
}

impl Ping {
    pub fn from_args(mut args: impl Iterator<Item = BulkString>) -> Result<Self, Error> {
        Ok(Ping {
            message: args.next(),
        })
    }
}

impl From<Ping> for Vec<BulkString> {
    fn from(cmd: Ping) -> Self {
        let mut cmd_bs = vec!["PING".into()];
        if let Some(message) = cmd.message {
            cmd_bs.push(message);
        }
        cmd_bs
    }
}

impl From<Ping> for ConnectionCommand {
    fn from(cmd: Ping) -> Self {
        ConnectionCommand::Ping(cmd)
    }
}

impl fmt::Display for Ping {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "PING")?;
        if let Some(message) = &self.message {
            write!(f, " {message}")?;
        }
        Ok(())
    }
}
