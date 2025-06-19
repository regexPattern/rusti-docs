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
    AddSlots(AddSlots),
    AddSlotsRange(AddSlotsRange),
    Replicate(Replicate),
    MyId(MyId),
    SetConfigEpoch(SetConfigEpoch),
    SaveConfig(SaveConfig),
    KeySlot(KeySlot),
}

impl From<ClusterCommand> for Vec<BulkString> {
    fn from(cmd: ClusterCommand) -> Self {
        match cmd {
            ClusterCommand::FailOver(cmd) => cmd.into(),
            ClusterCommand::Info(cmd) => cmd.into(),
            ClusterCommand::Meet(cmd) => cmd.into(),
            ClusterCommand::Nodes(cmd) => cmd.into(),
            ClusterCommand::Shards(cmd) => cmd.into(),
            ClusterCommand::AddSlots(cmd) => cmd.into(),
            ClusterCommand::AddSlotsRange(cmd) => cmd.into(),
            ClusterCommand::Replicate(cmd) => cmd.into(),
            ClusterCommand::MyId(cmd) => cmd.into(),
            ClusterCommand::SetConfigEpoch(cmd) => cmd.into(),
            ClusterCommand::SaveConfig(cmd) => cmd.into(),
            ClusterCommand::KeySlot(cmd) => cmd.into(),
        }
    }
}

impl fmt::Display for ClusterCommand {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ClusterCommand::FailOver(cmd) => write!(f, "{cmd}"),
            ClusterCommand::Info(cmd) => write!(f, "{cmd}"),
            ClusterCommand::Meet(cmd) => write!(f, "{cmd}"),
            ClusterCommand::Nodes(cmd) => write!(f, "{cmd}"),
            ClusterCommand::Shards(cmd) => write!(f, "{cmd}"),
            ClusterCommand::AddSlots(cmd) => write!(f, "{cmd}"),
            ClusterCommand::AddSlotsRange(cmd) => write!(f, "{cmd}"),
            ClusterCommand::Replicate(cmd) => write!(f, "{cmd}"),
            ClusterCommand::MyId(cmd) => write!(f, "{cmd}"),
            ClusterCommand::SetConfigEpoch(cmd) => write!(f, "{cmd}"),
            ClusterCommand::SaveConfig(cmd) => write!(f, "{cmd}"),
            ClusterCommand::KeySlot(cmd) => write!(f, "{cmd}"),
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
        if let Some(cluster_port) = cmd.cluster_port {
            cmd_bs.push(cluster_port);
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

        if let Some(cluster_port) = &self.cluster_port {
            write!(f, " {}", cluster_port)?;
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

impl From<FailOver> for Vec<BulkString> {
    fn from(_: FailOver) -> Self {
        vec!["FAILOVER".into()]
    }
}

impl From<FailOver> for ClusterCommand {
    fn from(cmd: FailOver) -> Self {
        ClusterCommand::FailOver(cmd)
    }
}

impl fmt::Display for FailOver {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "CLUSTER FAILOVER")
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

impl From<Info> for Vec<BulkString> {
    fn from(_: Info) -> Self {
        vec!["INFO".into()]
    }
}

impl From<Info> for ClusterCommand {
    fn from(cmd: Info) -> Self {
        ClusterCommand::Info(cmd)
    }
}

impl fmt::Display for Info {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "CLUSTER INFO")
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
    fn from(_: Nodes) -> Self {
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

impl From<Shards> for Vec<BulkString> {
    fn from(_: Shards) -> Self {
        vec!["SHARDS".into()]
    }
}

impl From<Shards> for ClusterCommand {
    fn from(cmd: Shards) -> Self {
        ClusterCommand::Shards(cmd)
    }
}

impl fmt::Display for Shards {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "CLUSTER SHARDS")
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct AddSlots {
    pub slot: BulkString,
    pub slots: Vec<BulkString>,
}

impl AddSlots {
    pub fn from_args(mut args: impl Iterator<Item = BulkString>) -> Result<Self, Error> {
        let slot = args.next().ok_or(Error::MissingArgument)?;
        let slots: Vec<BulkString> = args.collect();
        Ok(Self { slot, slots })
    }
}

impl From<AddSlots> for Vec<BulkString> {
    fn from(cmd: AddSlots) -> Self {
        let mut cmd_bs = vec!["ADDSLOTS".into(), cmd.slot];
        cmd_bs.extend(cmd.slots);
        cmd_bs
    }
}

impl From<AddSlots> for ClusterCommand {
    fn from(cmd: AddSlots) -> Self {
        ClusterCommand::AddSlots(cmd)
    }
}

impl fmt::Display for AddSlots {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "CLUSTER ADDSLOTS {}", self.slot)?;

        for slot in &self.slots {
            write!(f, " {slot}")?;
        }

        Ok(())
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct AddSlotsRange {
    pub start: BulkString,
    pub end: BulkString,
}

impl AddSlotsRange {
    pub fn from_args(mut args: impl Iterator<Item = BulkString>) -> Result<Self, Error> {
        Ok(Self {
            start: args.next().ok_or(Error::MissingArgument)?,
            end: args.next().ok_or(Error::MissingArgument)?,
        })
    }
}

impl From<AddSlotsRange> for Vec<BulkString> {
    fn from(cmd: AddSlotsRange) -> Self {
        vec!["ADDSLOTSRANGE".into(), cmd.start, cmd.end]
    }
}

impl From<AddSlotsRange> for ClusterCommand {
    fn from(cmd: AddSlotsRange) -> Self {
        ClusterCommand::AddSlotsRange(cmd)
    }
}

impl fmt::Display for AddSlotsRange {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "CLUSTER ADDSLOTSRANGE {} {}", self.start, self.end)
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct Replicate {
    pub node_id: BulkString,
}

impl Replicate {
    pub fn from_args(mut args: impl Iterator<Item = BulkString>) -> Result<Self, Error> {
        let node_id = args.next().ok_or(Error::MissingArgument)?;
        Ok(Self { node_id })
    }
}

impl From<Replicate> for Vec<BulkString> {
    fn from(cmd: Replicate) -> Self {
        vec!["REPLICATE".into(), cmd.node_id]
    }
}

impl From<Replicate> for ClusterCommand {
    fn from(cmd: Replicate) -> Self {
        ClusterCommand::Replicate(cmd)
    }
}

impl fmt::Display for Replicate {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "CLUSTER REPLICATE {}", self.node_id)
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct MyId;

impl MyId {
    pub fn from_args(_: impl Iterator<Item = BulkString>) -> Result<Self, Error> {
        Ok(Self)
    }
}

impl From<MyId> for Vec<BulkString> {
    fn from(_: MyId) -> Self {
        vec!["MYID".into()]
    }
}

impl From<MyId> for ClusterCommand {
    fn from(cmd: MyId) -> Self {
        ClusterCommand::MyId(cmd)
    }
}

impl fmt::Display for MyId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "CLUSTER MYID")
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct SetConfigEpoch {
    config_epoch: BulkString,
}

impl SetConfigEpoch {
    pub fn from_args(mut args: impl Iterator<Item = BulkString>) -> Result<Self, Error> {
        Ok(Self {
            config_epoch: args.next().ok_or(Error::MissingArgument)?,
        })
    }
}

impl From<SetConfigEpoch> for Vec<BulkString> {
    fn from(_: SetConfigEpoch) -> Self {
        vec!["SET-CONFIG-EPOCH".into()]
    }
}

impl From<SetConfigEpoch> for ClusterCommand {
    fn from(cmd: SetConfigEpoch) -> Self {
        ClusterCommand::SetConfigEpoch(cmd)
    }
}

impl fmt::Display for SetConfigEpoch {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "CLUSTER SET-CONFIG-EPOCH")
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct SaveConfig;

impl SaveConfig {
    pub fn from_args(_: impl Iterator<Item = BulkString>) -> Result<Self, Error> {
        Ok(Self)
    }
}

impl From<SaveConfig> for Vec<BulkString> {
    fn from(_: SaveConfig) -> Self {
        vec!["SAVECONFIG".into()]
    }
}

impl From<SaveConfig> for ClusterCommand {
    fn from(cmd: SaveConfig) -> Self {
        ClusterCommand::SaveConfig(cmd)
    }
}

impl fmt::Display for SaveConfig {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "CLUSTER SAVECONFIG")
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct KeySlot {
    pub key: BulkString,
}

impl KeySlot {
    pub fn from_args(mut args: impl Iterator<Item = BulkString>) -> Result<Self, Error> {
        Ok(Self {
            key: args.next().ok_or(Error::MissingArgument)?,
        })
    }
}

impl From<KeySlot> for Vec<BulkString> {
    fn from(cmd: KeySlot) -> Self {
        vec!["KEYSLOT".into(), cmd.key.into()]
    }
}

impl From<KeySlot> for ClusterCommand {
    fn from(cmd: KeySlot) -> Self {
        ClusterCommand::KeySlot(cmd)
    }
}

impl fmt::Display for KeySlot {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "CLUSTER KEYSLOT")
    }
}
