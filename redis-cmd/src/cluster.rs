use std::fmt;

use crate::Error;
use redis_resp::BulkString;

/// Comandos de cluster.
#[derive(Clone, Debug, PartialEq)]
pub enum ClusterCommand {
    FailOver(FailOver),
    Info(Info),
    Meet(Meet),
    Nodes(Nodes),
    Shards(Shards),
}

impl From<ClusterCommand> for Vec<BulkString> {
    fn from(cmd: ClusterCommand) -> Self {
        match cmd {
            ClusterCommand::FailOver(cmd) => todo!(),
            ClusterCommand::Info(cmd) => todo!(),
            ClusterCommand::Meet(cmd) => cmd.into(),
            ClusterCommand::Nodes(cmd) => cmd.into(),
            ClusterCommand::Shards(cmd) => todo!(),
        }
    }
}

impl fmt::Display for ClusterCommand {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ClusterCommand::FailOver(cmd) => todo!(),
            ClusterCommand::Info(cmd) => todo!(),
            ClusterCommand::Meet(cmd) => write!(f, "{cmd}"),
            ClusterCommand::Nodes(cmd) => write!(f, "{cmd}"),
            ClusterCommand::Shards(cmd) => todo!(),
        }
    }
}

/// Connect different Redis nodes with cluster support enabled, into a working cluster.
///
/// https://redis.io/docs/latest/commands/cluster-meet
#[derive(Clone, Debug, PartialEq)]
pub struct Meet {
    pub ip: BulkString,
    pub port: BulkString,
    pub cluster_port: Option<BulkString>,
}

impl Meet {
    pub fn from_args(mut args: impl Iterator<Item = BulkString>) -> Result<Self, Error> {
        Ok(Self {
            ip: args.next().ok_or(Error::MissingArgument)?,
            port: args.next().ok_or(Error::MissingArgument)?,
            cluster_port: args.next(),
        })
    }
}

impl From<Meet> for Vec<BulkString> {
    fn from(cmd: Meet) -> Self {
        let mut cmd_bs = vec!["MEET".into(), cmd.ip, cmd.port];
        if let Some(port) = cmd.cluster_port {
            cmd_bs.push(port);
        }
        cmd_bs
    }
}

impl From<Meet> for ClusterCommand {
    fn from(cmd: Meet) -> Self {
        ClusterCommand::Meet(cmd)
    }
}

impl fmt::Display for Meet {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "CLUSTER MEET {} {}", self.ip, self.port)?;

        if let Some(port) = &self.cluster_port {
            write!(f, " {}", port)?;
        }

        Ok(())
    }
}

/// This command, that can only be sent to a Redis Cluster replica node, forces the replica to start a manual failover of its master instance.
///
/// https://redis.io/docs/latest/commands/cluster-failover
#[derive(Clone, Debug, PartialEq)]
pub struct FailOver {
    pub option: Option<BulkString>,
}

impl FailOver {
    pub fn from_args(mut args: impl Iterator<Item = BulkString>) -> Result<Self, Error> {
        let option = args.next();
        Ok(Self { option })
    }
}

impl From<FailOver> for ClusterCommand {
    fn from(cmd: FailOver) -> Self {
        ClusterCommand::FailOver(cmd)
    }
}

/// Provides INFO style information about Redis Cluster vital parameters.
///
/// https://redis.io/docs/latest/commands/cluster-info
#[derive(Clone, Debug, PartialEq)]
pub struct Info;

impl Info {
    pub fn from_args(_args: impl Iterator<Item = BulkString>) -> Result<Self, Error> {
        Ok(Info)
    }
}

impl From<Info> for ClusterCommand {
    fn from(cmd: Info) -> Self {
        ClusterCommand::Info(cmd)
    }
}

/// Each node in a Redis Cluster has its view of the current cluster configuration, given by the set of known nodes, the state of the connection we have with such nodes, their flags, properties and assigned slots, and so forth.
///
/// https://redis.io/docs/latest/commands/cluster-nodes
#[derive(Clone, Debug, PartialEq)]
pub struct Nodes;

impl Nodes {
    pub fn from_args(_args: impl Iterator<Item = BulkString>) -> Result<Self, Error> {
        Ok(Nodes)
    }
}

impl From<Nodes> for Vec<BulkString> {
    fn from(cmd: Nodes) -> Self {
        vec!["NODES".into()]
    }
}

impl From<Nodes> for ClusterCommand {
    fn from(cmd: Nodes) -> Self {
        ClusterCommand::Nodes(cmd)
    }
}

impl fmt::Display for Nodes {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "CLUSTER NODES")
    }
}

/// Returns details about the shards of the cluster.
///
/// https://redis.io/docs/latest/commands/cluster-shards
#[derive(Clone, Debug, PartialEq)]
pub struct Shards;

impl Shards {
    pub fn from_args(_args: impl Iterator<Item = BulkString>) -> Result<Self, Error> {
        Ok(Shards)
    }
}

impl From<Shards> for ClusterCommand {
    fn from(cmd: Shards) -> Self {
        ClusterCommand::Shards(cmd)
    }
}
