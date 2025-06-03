use std::fmt;

use crate::Error;
use redis_resp::BulkString;

/// Comandos de Pub/Sub.
#[derive(Clone, Debug, PartialEq)]
pub enum PubSubCommand {
    Subscribe(Subscribe),
    Unsubscribe(Unsubscribe),
    Publish(Publish),
    PubSubChannels(PubSubChannels),
    PubSubNumSub(PubSubNumSub),
}

impl From<PubSubCommand> for Vec<BulkString> {
    fn from(cmd: PubSubCommand) -> Self {
        match cmd {
            PubSubCommand::Subscribe(cmd) => cmd.into(),
            PubSubCommand::Unsubscribe(cmd) => cmd.into(),
            PubSubCommand::Publish(cmd) => cmd.into(),
            PubSubCommand::PubSubChannels(cmd) => cmd.into(),
            PubSubCommand::PubSubNumSub(cmd) => cmd.into(),
        }
    }
}

impl fmt::Display for PubSubCommand {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            PubSubCommand::Subscribe(cmd) => write!(f, "{cmd}"),
            PubSubCommand::Unsubscribe(cmd) => write!(f, "{cmd}"),
            PubSubCommand::Publish(cmd) => write!(f, "{cmd}"),
            PubSubCommand::PubSubChannels(cmd) => write!(f, "{cmd}"),
            PubSubCommand::PubSubNumSub(cmd) => write!(f, "{cmd}"),
        }
    }
}

/// Subscribes the client to the specified channels.
///
/// https://redis.io/docs/latest/commands/subscribe
#[derive(Clone, Debug, PartialEq)]
pub struct Subscribe {
    pub channels: Vec<BulkString>,
}

impl Subscribe {
    pub fn from_args(args: impl Iterator<Item = BulkString>) -> Result<Self, Error> {
        let args: Vec<_> = args.collect();

        if args.is_empty() {
            return Err(Error::MissingArgument);
        }

        Ok(Self { channels: args })
    }
}

impl From<Subscribe> for PubSubCommand {
    fn from(cmd: Subscribe) -> Self {
        PubSubCommand::Subscribe(cmd)
    }
}

impl From<Subscribe> for Vec<BulkString> {
    fn from(cmd: Subscribe) -> Self {
        let mut cmd_bs = vec![BulkString::from("SUBSCRIBE")];
        cmd_bs.extend(cmd.channels);
        cmd_bs
    }
}

impl fmt::Display for Subscribe {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "SUBSCRIBE")?;

        for c in &self.channels {
            write!(f, " {c}")?;
        }

        Ok(())
    }
}

/// Unsubscribes the client from the given channels, or from all of them if none is given.
///
/// https://redis.io/docs/latest/commands/unsubscribe
#[derive(Clone, Debug, PartialEq)]
pub struct Unsubscribe {
    pub channels: Vec<BulkString>,
}

impl Unsubscribe {
    pub fn from_args(args: impl Iterator<Item = BulkString>) -> Result<Self, Error> {
        Ok(Self {
            channels: args.collect(),
        })
    }
}

impl From<Unsubscribe> for PubSubCommand {
    fn from(cmd: Unsubscribe) -> Self {
        PubSubCommand::Unsubscribe(cmd)
    }
}

impl From<Unsubscribe> for Vec<BulkString> {
    fn from(cmd: Unsubscribe) -> Self {
        let mut cmd_bs = vec![BulkString::from("UNSUBSCRIBE")];
        cmd_bs.extend(cmd.channels);
        cmd_bs
    }
}

impl fmt::Display for Unsubscribe {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "UNSUBSCRIBE")?;

        for c in &self.channels {
            write!(f, " {c}")?;
        }

        Ok(())
    }
}

/// Posts a message to the given channel.
///
/// In a Redis Cluster clients can publish to every node. The cluster makes sure that published messages are forwarded as needed, so clients can subscribe to any channel by connecting to any one of the nodes.
///
/// https://redis.io/docs/latest/commands/publish
#[derive(Clone, Debug, PartialEq)]
pub struct Publish {
    pub channel: BulkString,
    pub message: BulkString,
}

impl Publish {
    pub fn from_args(mut args: impl Iterator<Item = BulkString>) -> Result<Self, Error> {
        Ok(Self {
            channel: args.next().ok_or(Error::MissingArgument)?,
            message: args.next().ok_or(Error::MissingArgument)?,
        })
    }
}

impl From<Publish> for PubSubCommand {
    fn from(cmd: Publish) -> Self {
        PubSubCommand::Publish(cmd)
    }
}

impl From<Publish> for Vec<BulkString> {
    fn from(cmd: Publish) -> Self {
        vec![BulkString::from("PUBLISH"), cmd.channel, cmd.message]
    }
}

impl fmt::Display for Publish {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut message = self.message.to_string();
        message.truncate(20);

        if self.message.len() > 20 {
            message.push_str("...");
        }

        write!(f, "PUBLISH {} {}", self.channel, message)
    }
}

/// Lists the currently active channels.
///
/// https://redis.io/docs/latest/commands/pubsub-channels
#[derive(Clone, Debug, PartialEq)]
pub struct PubSubChannels {
    pub pattern: Option<BulkString>,
}

impl PubSubChannels {
    pub fn from_args(args: impl Iterator<Item = BulkString>) -> Result<Self, Error> {
        let mut args = args.into_iter();
        let pattern = args.next();
        Ok(Self { pattern })
    }
}

impl From<PubSubChannels> for PubSubCommand {
    fn from(cmd: PubSubChannels) -> Self {
        PubSubCommand::PubSubChannels(cmd)
    }
}

impl From<PubSubChannels> for Vec<BulkString> {
    fn from(cmd: PubSubChannels) -> Self {
        let mut cmd_bs = vec![BulkString::from("PUBSUB"), BulkString::from("CHANNELS")];

        if let Some(pattern) = cmd.pattern {
            cmd_bs.push(pattern);
        }

        cmd_bs
    }
}

impl fmt::Display for PubSubChannels {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "PUBSUB CHANNELS")?;

        if let Some(pattern) = &self.pattern {
            write!(f, " {pattern}")?;
        }

        Ok(())
    }
}

/// Returns the number of subscribers (exclusive of clients subscribed to patterns) for the specified channels.
///
/// https://redis.io/docs/latest/commands/pubsub-numsub
#[derive(Clone, Debug, PartialEq)]
pub struct PubSubNumSub {
    pub channels: Vec<BulkString>,
}

impl PubSubNumSub {
    pub fn from_args(args: impl Iterator<Item = BulkString>) -> Result<Self, Error> {
        Ok(Self {
            channels: args.collect(),
        })
    }
}

impl From<PubSubNumSub> for PubSubCommand {
    fn from(cmd: PubSubNumSub) -> Self {
        PubSubCommand::PubSubNumSub(cmd)
    }
}

impl From<PubSubNumSub> for Vec<BulkString> {
    fn from(cmd: PubSubNumSub) -> Self {
        let mut cmd_bs = vec![BulkString::from("PUBSUB"), BulkString::from("NUMSUB")];
        cmd_bs.extend(cmd.channels);
        cmd_bs
    }
}

impl fmt::Display for PubSubNumSub {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "PUBSUB NUMSUB")?;

        for c in &self.channels {
            write!(f, " {c}")?;
        }

        Ok(())
    }
}
