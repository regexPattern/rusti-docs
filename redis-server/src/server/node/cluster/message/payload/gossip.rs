use std::{fmt, net::Ipv4Addr, time::UNIX_EPOCH};

use crate::server::node::cluster::{ClusterNode, NodeId, flags::Flags};

use super::MessagePayload;

const GOSSIP_PAYLOAD_KIND_MEET: u8 = 0;
const GOSSIP_PAYLOAD_KIND_PING: u8 = 1;
const GOSSIP_PAYLOAD_KIND_PONG: u8 = 2;

#[derive(PartialEq, Debug)]
pub struct GossipPayload {
    pub kind: GossipKind,
    pub nodes: Vec<GossipNode>,
}

impl From<&[u8]> for GossipPayload {
    fn from(bytes: &[u8]) -> Self {
        let kind = match bytes[0] {
            GOSSIP_PAYLOAD_KIND_MEET => GossipKind::Meet,
            GOSSIP_PAYLOAD_KIND_PING => GossipKind::Ping,
            GOSSIP_PAYLOAD_KIND_PONG => GossipKind::Pong,
            _ => unreachable!(),
        };

        let nodes = bytes[1..].chunks_exact(30).map(GossipNode::from).collect();

        Self { kind, nodes }
    }
}

impl From<&GossipPayload> for Vec<u8> {
    fn from(payload: &GossipPayload) -> Self {
        let mut bytes = vec![match payload.kind {
            GossipKind::Meet => GOSSIP_PAYLOAD_KIND_MEET,
            GossipKind::Ping => GOSSIP_PAYLOAD_KIND_PING,
            GossipKind::Pong => GOSSIP_PAYLOAD_KIND_PONG,
        }];

        for node in &payload.nodes {
            bytes.extend(Vec::from(node));
        }

        bytes
    }
}

impl From<GossipPayload> for MessagePayload {
    fn from(payload: GossipPayload) -> Self {
        MessagePayload::Gossip(payload)
    }
}

impl fmt::Display for GossipPayload {
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

#[derive(Copy, Clone, PartialEq)]
pub struct GossipNode {
    pub id: NodeId,
    pub ip: Ipv4Addr,
    pub port: u16,
    pub cluster_port: u16,
    pub flags: Flags,
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

        Self {
            id,
            ip,
            port,
            cluster_port,
            flags,
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
            ping_sent: UNIX_EPOCH,
            pong_received: UNIX_EPOCH,
            config_epoch: 0,
            slots: (0, 0),
            master_id: None,
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
        }
    }
}

impl fmt::Debug for GossipNode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("ClusterNode")
            .field("id", &hex::encode(&self.id))
            .field("ip", &self.ip.to_string())
            .field("port", &self.port)
            .field("cluster_port", &self.cluster_port)
            .field("flags", &self.flags.to_string())
            .finish()
    }
}

#[cfg(test)]
mod tests {
    use crate::server::node::cluster::flags;

    use super::*;

    #[test]
    fn gossiped_node_se_serializa_y_deserializa_correctamente() {
        let gossiped_node = GossipNode {
            id: [1; 20],
            ip: Ipv4Addr::new(127, 0, 0, 1),
            port: 7000,
            cluster_port: 17000,
            flags: Flags(flags::FLAG_MYSELF | flags::FLAG_MASTER),
        };

        let bytes = Vec::from(&gossiped_node);

        assert_eq!(GossipNode::from(bytes.as_slice()), gossiped_node);
    }

    #[test]
    fn gossip_payload_sin_gossiped_nodes_se_serializa_y_deserializa_correctamente() {
        let payload = GossipPayload {
            kind: GossipKind::Meet,
            nodes: vec![],
        };

        let bytes = Vec::from(&payload);

        assert_eq!(GossipPayload::from(bytes.as_slice()), payload);
    }

    #[test]
    fn gossip_payload_con_gossiped_nodes_se_serializa_y_deserializa_correctamente() {
        let payload = GossipPayload {
            kind: GossipKind::Meet,
            nodes: vec![
                GossipNode {
                    id: [1; 20],
                    ip: Ipv4Addr::new(127, 0, 0, 1),
                    port: 7000,
                    cluster_port: 17000,
                    flags: Flags(flags::FLAG_MASTER),
                },
                GossipNode {
                    id: [2; 20],
                    ip: Ipv4Addr::new(127, 0, 0, 1),
                    port: 7001,
                    cluster_port: 17001,
                    flags: Flags(flags::FLAG_MASTER),
                },
            ],
        };

        let bytes = Vec::from(&payload);

        assert_eq!(GossipPayload::from(bytes.as_slice()), payload);
    }

    #[test]
    fn gossip_payload_de_meet_se_serializa_y_deserializa_correctamente() {
        let payload = GossipPayload {
            kind: GossipKind::Meet,
            nodes: vec![GossipNode {
                id: [1; 20],
                ip: Ipv4Addr::new(127, 0, 0, 1),
                port: 7000,
                cluster_port: 17000,
                flags: Flags(flags::FLAG_MASTER),
            }],
        };

        let bytes = Vec::from(&payload);

        assert_eq!(GossipPayload::from(bytes.as_slice()), payload);
    }

    #[test]
    fn gossip_payload_de_ping_se_serializa_y_deserializa_correctamente() {
        let payload = GossipPayload {
            kind: GossipKind::Ping,
            nodes: vec![GossipNode {
                id: [1; 20],
                ip: Ipv4Addr::new(127, 0, 0, 1),
                port: 7000,
                cluster_port: 17000,
                flags: Flags(flags::FLAG_MASTER),
            }],
        };

        let bytes = Vec::from(&payload);

        assert_eq!(GossipPayload::from(bytes.as_slice()), payload);
    }

    #[test]
    fn gossip_payload_de_pong_se_serializa_y_deserializa_correctamente() {
        let payload = GossipPayload {
            kind: GossipKind::Pong,
            nodes: vec![GossipNode {
                id: [1; 20],
                ip: Ipv4Addr::new(127, 0, 0, 1),
                port: 7000,
                cluster_port: 17000,
                flags: Flags(flags::FLAG_MASTER),
            }],
        };

        let bytes = Vec::from(&payload);

        assert_eq!(GossipPayload::from(bytes.as_slice()), payload);
    }
}
