use std::{
    fmt,
    net::Ipv4Addr,
    time::{SystemTime, UNIX_EPOCH},
};

use chrono::{DateTime, Local};

use crate::server::node::cluster::flags::FLAG_MYSELF;

use super::{CLUSTER_SLOTS, NodeId, flags::Flags, message::MessageHeader};

pub struct ClusterNode {
    pub id: NodeId,
    pub ip: Ipv4Addr,
    pub port: u16,
    pub cluster_port: u16,
    pub flags: Flags,
    pub ping_sent: SystemTime,
    pub pong_received: SystemTime,
    pub config_epoch: u64,
    pub master_id: Option<NodeId>,
    pub slots: [u8; CLUSTER_SLOTS / 8],
}

impl From<&ClusterNode> for MessageHeader {
    fn from(node: &ClusterNode) -> Self {
        Self {
            id: node.id,
            ip: node.ip,
            port: node.port,
            cluster_port: node.cluster_port,
            flags: Flags(node.flags.0 & !FLAG_MYSELF),
            config_epoch: node.config_epoch,
            master_id: node.master_id,
            slots: node.slots,
        }
    }
}

impl From<&MessageHeader> for ClusterNode {
    fn from(header: &MessageHeader) -> Self {
        Self {
            id: header.id,
            ip: header.ip,
            port: header.port,
            cluster_port: header.cluster_port,
            flags: header.flags,
            ping_sent: UNIX_EPOCH,
            pong_received: SystemTime::now(),
            config_epoch: header.config_epoch,
            master_id: header.master_id,
            slots: header.slots,
        }
    }
}

impl fmt::Debug for ClusterNode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let ping_sent: DateTime<Local> = self.ping_sent.into();
        let pong_received: DateTime<Local> = self.pong_received.into();

        f.debug_struct("ClusterNode")
            .field("id", &hex::encode(&self.id))
            .field("ip", &self.ip.to_string())
            .field("port", &self.port)
            .field("cluster_port", &self.cluster_port)
            .field("flags", &self.flags.to_string())
            .field(
                "ping_sent",
                &ping_sent.format("%d-%m-%Y %H:%M:%S").to_string(),
            )
            .field(
                "pong_received",
                &pong_received.format("%d-%m-%Y %H:%M:%S").to_string(),
            )
            .field("config_epoch", &self.config_epoch)
            .field("master_id", &self.master_id.map(|id| hex::encode(id)))
            .field("slots", &"TODO")
            .finish()
    }
}
