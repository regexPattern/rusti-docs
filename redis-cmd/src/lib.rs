pub mod cluster;
pub mod connection;
mod error;
pub mod pub_sub;
pub mod server;
pub mod storage;

use std::fmt;

use cluster::*;
use connection::*;
pub use error::Error;
use pub_sub::*;
use redis_resp::{Array, RespDataType};
use server::*;
use storage::*;

#[derive(Clone, Debug, PartialEq)]
pub enum Command {
    Storage(StorageCommand),
    PubSub(PubSubCommand),
    Cluster(ClusterCommand),
    Server(ServerCommand),
    Connection(ConnectionCommand),
}

impl TryFrom<&[u8]> for Command {
    type Error = Error;

    fn try_from(bytes: &[u8]) -> Result<Self, Self::Error> {
        Command::try_from(Array::try_from(bytes)?)
    }
}

impl TryFrom<Array> for Command {
    type Error = Error;

    fn try_from(arr: Array) -> Result<Self, Self::Error> {
        let dt_arr = Vec::from(arr).into_iter();
        let mut args = Vec::with_capacity(dt_arr.len());

        for dt in dt_arr {
            if let RespDataType::BulkString(bs) = dt {
                args.push(bs);
            } else {
                return Err(Error::InvalidDataType);
            }
        }

        let mut args = args.into_iter();

        let cmd_bs = args.next().ok_or(Error::MissingCommand)?;
        let cmd_str = String::from(cmd_bs).to_ascii_uppercase();

        let cmd = match cmd_str.as_str() {
            "DEL" => Self::Storage(Del::from_args(args)?.into()),

            "SET" => Self::Storage(Set::from_args(args)?.into()),
            "GET" => Self::Storage(Get::from_args(args)?.into()),
            "APPEND" => Self::Storage(Append::from_args(args)?.into()),
            "DECR" => Self::Storage(Decr::from_args(args)?.into()),
            "INCR" => Self::Storage(Incr::from_args(args)?.into()),

            "HSET" => Self::Storage(HSet::from_args(args)?.into()),
            "HGET" => Self::Storage(HGet::from_args(args)?.into()),
            "HDEL" => Self::Storage(HDel::from_args(args)?.into()),
            "HGETALL" => Self::Storage(HGetAll::from_args(args)?.into()),
            "HKEYS" => Self::Storage(HKeys::from_args(args)?.into()),
            "HVALS" => Self::Storage(HVals::from_args(args)?.into()),
            "HEXISTS" => Self::Storage(HExists::from_args(args)?.into()),

            "LPUSH" => Self::Storage(LPush::from_args(args)?.into()),
            "LINSERT" => Self::Storage(LInsert::from_args(args)?.into()),
            "LPOP" => Self::Storage(LPop::from_args(args)?.into()),
            "LINDEX" => Self::Storage(LIndex::from_args(args)?.into()),
            "LLEN" => Self::Storage(LLen::from_args(args)?.into()),
            "LRANGE" => Self::Storage(LRange::from_args(args)?.into()),

            "SADD" => Self::Storage(SAdd::from_args(args)?.into()),
            "SREM" => Self::Storage(SRem::from_args(args)?.into()),
            "SCARD" => Self::Storage(SCard::from_args(args)?.into()),
            "SISMEMBER" => Self::Storage(SIsMember::from_args(args)?.into()),
            "SMEMBERS" => Self::Storage(SMembers::from_args(args)?.into()),

            "CLUSTER" => {
                let sub_cmd_bs = args.next().ok_or(Error::MissingCommand)?;
                let sub_cmd_str = String::from(sub_cmd_bs).to_ascii_uppercase();
                match sub_cmd_str.as_str() {
                    "MEET" => Self::Cluster(Meet::from_args(args)?.into()),
                    "NODES" => Self::Cluster(Nodes::from_args(args)?.into()),
                    "ADDSLOTS" => Self::Cluster(AddSlots::from_args(args)?.into()),
                    "ADDSLOTSRANGE" => Self::Cluster(AddSlotsRange::from_args(args)?.into()),
                    "REPLICATE" => Self::Cluster(Replicate::from_args(args)?.into()),
                    "KEYSLOT" => Self::Cluster(KeySlot::from_args(args)?.into()),
                    "MYID" => Self::Cluster(MyId::from_args(args)?.into()),
                    _ => return Err(Error::CommandNotSupported),
                }
            }

            "SUBSCRIBE" => Self::PubSub(Subscribe::from_args(args)?.into()),
            "PUBLISH" => Self::PubSub(Publish::from_args(args)?.into()),
            "UNSUBSCRIBE" => Self::PubSub(Unsubscribe::from_args(args)?.into()),
            "PUBSUB" => {
                let sub_cmd_bs = args.next().ok_or(Error::MissingCommand)?;
                let sub_cmd_str = String::from(sub_cmd_bs).to_ascii_uppercase();
                match sub_cmd_str.as_str() {
                    "CHANNELS" => Self::PubSub(PubSubChannels::from_args(args)?.into()),
                    "NUMSUB" => Self::PubSub(PubSubNumSub::from_args(args)?.into()),
                    _ => return Err(Error::CommandNotSupported),
                }
            }

            "SYNC" => Self::Server(Sync::from_args(args)?.into()),

            "AUTH" => Self::Connection(Auth::from_args(args)?.into()),
            "PING" => Self::Connection(Ping::from_args(args)?.into()),

            _ => return Err(Error::CommandNotSupported),
        };

        Ok(cmd)
    }
}

impl From<Command> for Vec<u8> {
    fn from(cmd: Command) -> Self {
        let cmd_bs: Vec<_> = match cmd {
            Command::Storage(cmd) => cmd.into(),
            Command::PubSub(cmd) => cmd.into(),
            Command::Server(cmd) => cmd.into(),
            Command::Connection(cmd) => cmd.into(),
            Command::Cluster(_) => unimplemented!(),
        };

        let cmd_dt: Vec<_> = cmd_bs.into_iter().map(RespDataType::from).collect();

        Array::from(cmd_dt).into()
    }
}

impl From<StorageCommand> for Command {
    fn from(cmd: StorageCommand) -> Self {
        Self::Storage(cmd)
    }
}

impl From<PubSubCommand> for Command {
    fn from(cmd: PubSubCommand) -> Self {
        Self::PubSub(cmd)
    }
}

impl From<ClusterCommand> for Command {
    fn from(cmd: ClusterCommand) -> Self {
        Self::Cluster(cmd)
    }
}

impl fmt::Display for Command {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Command::Storage(cmd) => write!(f, "{cmd}"),
            Command::PubSub(cmd) => write!(f, "{cmd}"),
            Command::Cluster(cmd) => write!(f, "{cmd}"),
            Command::Server(cmd) => write!(f, "{cmd}"),
            Command::Connection(cmd) => write!(f, "{cmd}"),
        }
    }
}
