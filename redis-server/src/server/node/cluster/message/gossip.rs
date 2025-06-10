use std::{
    fmt,
    net::Ipv4Addr,
    time::{Duration, SystemTime, UNIX_EPOCH},
};

use chrono::{DateTime, Local};

use crate::server::node::cluster::{CLUSTER_SLOTS, NodeId, flags::Flags, node::ClusterNode};

use super::MessageData;

const CLUSTERMSG_KIND_MEET: u8 = 0;
const CLUSTERMSG_KIND_PING: u8 = 1;
const CLUSTERMSG_KIND_PONG: u8 = 2;

#[derive(PartialEq, Debug)]
pub struct GossipData {
    pub kind: GossipKind,
    pub nodes: Vec<GossipNode>,
}

impl From<&[u8]> for GossipData {
    fn from(bytes: &[u8]) -> Self {
        let kind = match bytes[0] {
            CLUSTERMSG_KIND_MEET => GossipKind::Meet,
            CLUSTERMSG_KIND_PING => GossipKind::Ping,
            CLUSTERMSG_KIND_PONG => GossipKind::Pong,
            _ => unreachable!(),
        };

        let nodes = bytes[1..]
            .chunks(std::mem::size_of::<GossipNode>())
            .map(GossipNode::from)
            .collect();

        Self { kind, nodes }
    }
}

impl From<&GossipData> for Vec<u8> {
    fn from(data: &GossipData) -> Self {
        let mut bytes = vec![match data.kind {
            GossipKind::Meet => CLUSTERMSG_KIND_MEET,
            GossipKind::Ping => CLUSTERMSG_KIND_PING,
            GossipKind::Pong => CLUSTERMSG_KIND_PONG,
        }];

        for node in &data.nodes {
            bytes.extend(Vec::from(node));
        }

        bytes
    }
}

impl From<GossipData> for MessageData {
    fn from(data: GossipData) -> Self {
        MessageData::Gossip(data)
    }
}

impl fmt::Display for GossipData {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self.kind {
            GossipKind::Meet => write!(f, "MEET"),
            GossipKind::Ping => write!(f, "PING"),
            GossipKind::Pong => write!(f, "PONG"),
        }
    }
}

#[derive(PartialEq, Debug)]
pub enum GossipKind {
    Meet,
    Ping,
    Pong,
}

#[derive(PartialEq)]
pub struct GossipNode {
    pub id: NodeId,
    pub ip: Ipv4Addr,
    pub port: u16,
    pub cluster_port: u16,
    pub flags: Flags,
    pub ping_sent: SystemTime,
    pub pong_received: SystemTime,
}

impl From<&[u8]> for GossipNode {
    fn from(bytes: &[u8]) -> Self {
        let id = {
            let mut id = [0u8; 20];
            id.copy_from_slice(&bytes[0..20]);
            id
        };

        let ip = Ipv4Addr::from_bits(u32::from_be_bytes([
            bytes[20], bytes[21], bytes[22], bytes[23],
        ]));
        let port = u16::from_be_bytes([bytes[24], bytes[25]]);
        let cluster_port = u16::from_be_bytes([bytes[26], bytes[27]]);
        let flags = Flags(u16::from_be_bytes([bytes[28], bytes[29]]));

        let ping_sent = UNIX_EPOCH
            + Duration::from_secs(u64::from_be_bytes([
                bytes[30], bytes[31], bytes[32], bytes[33], bytes[34], bytes[35], bytes[36],
                bytes[37],
            ]));

        let pong_received = UNIX_EPOCH
            + Duration::from_secs(u64::from_be_bytes([
                bytes[38], bytes[39], bytes[40], bytes[41], bytes[42], bytes[43], bytes[44],
                bytes[45],
            ]));

        Self {
            id,
            ip,
            port,
            cluster_port,
            flags,
            ping_sent,
            pong_received,
        }
    }
}

impl From<&GossipNode> for Vec<u8> {
    fn from(node: &GossipNode) -> Self {
        let mut bytes = Vec::new();

        bytes.extend(node.id);
        bytes.extend(node.ip.to_bits().to_be_bytes());
        bytes.extend(node.port.to_be_bytes());
        bytes.extend(node.cluster_port.to_be_bytes());
        bytes.extend(node.flags.0.to_be_bytes());
        bytes.extend(
            node.ping_sent
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_secs()
                .to_be_bytes(),
        );
        bytes.extend(
            node.pong_received
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_secs()
                .to_be_bytes(),
        );

        bytes
    }
}

impl From<&GossipNode> for ClusterNode {
    fn from(node: &GossipNode) -> Self {
        ClusterNode {
            id: node.id,
            ip: node.ip,
            port: node.port,
            cluster_port: node.cluster_port,
            flags: node.flags,
            ping_sent: node.ping_sent,
            pong_received: node.pong_received,
            config_epoch: 0,
            master_id: None,
            slots: [0; CLUSTER_SLOTS / 8],
        }
    }
}

impl From<&ClusterNode> for GossipNode {
    fn from(node: &ClusterNode) -> Self {
        GossipNode {
            id: node.id,
            ip: node.ip,
            port: node.port,
            cluster_port: node.cluster_port,
            flags: node.flags,
            ping_sent: node.ping_sent,
            pong_received: node.pong_received,
        }
    }
}

impl fmt::Debug for GossipNode {
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
            .finish()
    }
}

#[cfg(test)]
mod tests {
    use rand::Rng;

    use crate::server::node::cluster::flags::{FLAG_MASTER, FLAG_MYSELF};

    use super::*;

    #[test]
    fn se_serializa_gossip_node_correctamente() {
        let mut id = [0_u8; 20];
        rand::rng().fill(&mut id);

        let ping_secs: u64 = 123_456;
        let pong_secs: u64 = 654_321;

        let gossip_node = GossipNode {
            id,
            ip: Ipv4Addr::new(127, 0, 0, 2),
            port: 6379,
            cluster_port: 16379,
            flags: Flags(FLAG_MYSELF | FLAG_MASTER),
            ping_sent: UNIX_EPOCH + Duration::from_secs(ping_secs),
            pong_received: UNIX_EPOCH + Duration::from_secs(pong_secs),
        };

        let bytes = Vec::from(&gossip_node);

        assert_eq!(&bytes[0..20], gossip_node.id);
        assert_eq!(&bytes[20..24], gossip_node.ip.to_bits().to_be_bytes());
        assert_eq!(&bytes[24..26], gossip_node.port.to_be_bytes());
        assert_eq!(&bytes[26..28], gossip_node.cluster_port.to_be_bytes());
        assert_eq!(&bytes[28..30], gossip_node.flags.0.to_be_bytes());
        assert_eq!(&bytes[30..38], ping_secs.to_be_bytes());
        assert_eq!(&bytes[38..46], pong_secs.to_be_bytes());
    }

    #[test]
    fn se_deserializa_gossiped_node_correctamente() {
        let mut bytes = Vec::new();

        let mut id = [0u8; 20];
        hex::decode_to_slice("a1b2c3d4e5f67890abcdef0123456789abcdef02", &mut id).unwrap();

        let ip = Ipv4Addr::new(127, 0, 0, 2);
        let port: u16 = 6379;
        let cluster_port: u16 = 16379;
        let flags = Flags(FLAG_MYSELF | FLAG_MASTER);

        let ping_secs: u64 = 123_456;
        let pong_secs: u64 = 654_321;

        bytes.extend(id);
        bytes.extend(ip.to_bits().to_be_bytes());
        bytes.extend(port.to_be_bytes());
        bytes.extend(cluster_port.to_be_bytes());
        bytes.extend(flags.0.to_be_bytes());
        bytes.extend(ping_secs.to_be_bytes());
        bytes.extend(pong_secs.to_be_bytes());

        let gossiped_node = GossipNode {
            id,
            ip,
            port,
            cluster_port,
            flags,
            ping_sent: UNIX_EPOCH + Duration::from_secs(ping_secs),
            pong_received: UNIX_EPOCH + Duration::from_secs(pong_secs),
        };

        assert_eq!(gossiped_node, GossipNode::from(bytes.as_slice()));
    }
}
