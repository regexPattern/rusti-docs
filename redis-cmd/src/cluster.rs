use crate::Error;
use redis_resp::BulkString;

/// Comandos de cluster.
#[derive(Debug, PartialEq)]
pub enum ClusterCommand {
    FailOver(FailOver),
    Info(Info),
    Nodes(Nodes),
    Shards(Shards),
}

impl From<ClusterCommand> for Vec<BulkString> {
    fn from(cmd: ClusterCommand) -> Self {
        match cmd {
            ClusterCommand::FailOver(cmd) => todo!(),
            ClusterCommand::Info(cmd) => todo!(),
            ClusterCommand::Nodes(cmd) => todo!(),
            ClusterCommand::Shards(cmd) => todo!(),
        }
    }
}

/// This command, that can only be sent to a Redis Cluster replica node, forces the replica to start a manual failover of its master instance.
///
/// https://redis.io/docs/latest/commands/cluster-failover
#[derive(Debug, PartialEq)]
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
#[derive(Debug, PartialEq)]
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
#[derive(Debug, PartialEq)]
pub struct Nodes;
impl Nodes {
    pub fn from_args(_args: impl Iterator<Item = BulkString>) -> Result<Self, Error> {
        Ok(Nodes)
    }
}

impl From<Nodes> for ClusterCommand {
    fn from(cmd: Nodes) -> Self {
        ClusterCommand::Nodes(cmd)
    }
}

/// Returns details about the shards of the cluster.
///
/// https://redis.io/docs/latest/commands/cluster-shards
#[derive(Debug, PartialEq)]
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
