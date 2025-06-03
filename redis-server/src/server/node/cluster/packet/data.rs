use std::{fmt, net::Ipv4Addr};

use crate::server::node::cluster::{CLUSTER_SLOTS, NodeId, node::GossipedNode};

use super::error::Error;

const KIND_FAIL: u8 = 0;
const KIND_GOSSIP: u8 = 1;
const SUB_KIND_GOSSIP_MEET: u8 = 0;
const SUB_KIND_GOSSIP_PING: u8 = 1;
const SUB_KIND_GOSSIP_PONG: u8 = 2;
const KIND_UPDATE: u8 = 2;

const PEER_BYTES: usize = 28;

#[allow(clippy::large_enum_variant)]
#[derive(Debug)]
pub enum PacketData {
    Fail(FailData),
    Gossip(GossipData),
    Update(UpdateData),
}

impl TryFrom<&[u8]> for PacketData {
    type Error = Error;

    fn try_from(bytes: &[u8]) -> Result<Self, Self::Error> {
        Ok(match bytes[0] {
            KIND_FAIL => Self::Fail(FailData::try_from(&bytes[1..])?),
            KIND_GOSSIP => Self::Gossip(GossipData::try_from(&bytes[1..])?),
            KIND_UPDATE => Self::Update(UpdateData::try_from(&bytes[1..])?),
            _ => return Err(Error::PacketDataKind),
        })
    }
}

impl From<PacketData> for Vec<u8> {
    fn from(packet: PacketData) -> Self {
        match packet {
            PacketData::Fail(data) => {
                let mut bytes = vec![KIND_FAIL];
                bytes.extend(Vec::from(data));
                bytes
            }
            PacketData::Gossip(data) => {
                let mut bytes = vec![KIND_GOSSIP];
                bytes.extend(Vec::from(data));
                bytes
            }
            PacketData::Update(data) => {
                let mut bytes = vec![KIND_UPDATE];
                bytes.extend(Vec::from(data));
                bytes
            }
        }
    }
}

#[derive(Debug)]
pub struct GossipData {
    pub kind: GossipDataKind,
    pub cluster_view: Vec<GossipedNode>,
}

impl TryFrom<&[u8]> for GossipData {
    type Error = Error;

    fn try_from(bytes: &[u8]) -> Result<Self, Self::Error> {
        let kind = GossipDataKind::try_from(bytes[0])?;

        let mut cluster_view = Vec::new();
        let mut offset = 1;

        while offset + PEER_BYTES <= bytes.len() {
            let peer = GossipedNode::from(&bytes[offset..offset + PEER_BYTES]);
            cluster_view.push(peer);
            offset += PEER_BYTES;
        }

        Ok(Self { kind, cluster_view })
    }
}

impl From<GossipData> for Vec<u8> {
    fn from(data: GossipData) -> Self {
        let mut bytes = vec![u8::from(data.kind)];

        for data in data.cluster_view {
            bytes.extend(Vec::from(data));
        }

        bytes
    }
}

#[derive(Debug, PartialEq)]
pub enum GossipDataKind {
    Meet,
    Ping,
    Pong,
}

impl TryFrom<u8> for GossipDataKind {
    type Error = Error;

    fn try_from(byte: u8) -> Result<Self, Self::Error> {
        Ok(match byte {
            SUB_KIND_GOSSIP_MEET => Self::Meet,
            SUB_KIND_GOSSIP_PING => Self::Ping,
            SUB_KIND_GOSSIP_PONG => Self::Pong,
            _ => return Err(Error::GossipDataKind),
        })
    }
}

impl From<GossipDataKind> for u8 {
    fn from(kind: GossipDataKind) -> u8 {
        match kind {
            GossipDataKind::Meet => SUB_KIND_GOSSIP_MEET,
            GossipDataKind::Ping => SUB_KIND_GOSSIP_PING,
            GossipDataKind::Pong => SUB_KIND_GOSSIP_PONG,
        }
    }
}

#[derive(Debug, PartialEq)]
pub struct FailData {
    pub id: NodeId,
}

impl TryFrom<&[u8]> for FailData {
    type Error = Error;

    fn try_from(bytes: &[u8]) -> Result<Self, Self::Error> {
        if bytes.len() != 20 {
            return Err(Error::PacketDataLength);
        }

        let mut id = [0u8; 20];
        id.copy_from_slice(&bytes[0..20]);

        Ok(FailData { id })
    }
}

impl From<FailData> for Vec<u8> {
    fn from(data: FailData) -> Self {
        data.id.to_vec()
    }
}

#[derive(Debug, PartialEq)]
pub struct UpdateData {
    pub id: NodeId,
    pub config_epoch: u64,
    pub slots: [u8; CLUSTER_SLOTS / 8],
}

impl TryFrom<&[u8]> for UpdateData {
    type Error = Error;

    fn try_from(bytes: &[u8]) -> Result<Self, Self::Error> {
        if bytes.len() != 2076 {
            return Err(Error::PacketDataLength);
        }

        let id = {
            let mut id = [0u8; 20];
            id.copy_from_slice(&bytes[0..20]);
            id
        };

        let config_epoch = u64::from_be_bytes([
            bytes[20], bytes[21], bytes[22], bytes[23], bytes[24], bytes[25], bytes[26], bytes[27],
        ]);

        let slots = {
            let mut slots = [0u8; CLUSTER_SLOTS / 8];
            slots.copy_from_slice(&bytes[28..]);
            slots
        };

        Ok(UpdateData {
            id,
            config_epoch,
            slots,
        })
    }
}

impl From<UpdateData> for Vec<u8> {
    fn from(data: UpdateData) -> Self {
        let mut bytes = Vec::with_capacity(2076);

        bytes.extend(data.id);
        bytes.extend(data.config_epoch.to_be_bytes());
        bytes.extend(data.slots);

        bytes
    }
}

impl From<&[u8]> for GossipedNode {
    fn from(bytes: &[u8]) -> Self {
        let id = {
            let mut id = [0u8; 20];
            id.copy_from_slice(&bytes[0..20]);
            id
        };

        let ip = Ipv4Addr::new(bytes[20], bytes[21], bytes[22], bytes[23]);
        let port = u16::from_be_bytes([bytes[24], bytes[25]]);
        let cluster_port = u16::from_be_bytes([bytes[26], bytes[27]]);

        Self {
            id,
            ip,
            port,
            cluster_port,
        }
    }
}

impl From<GossipedNode> for Vec<u8> {
    fn from(node: GossipedNode) -> Vec<u8> {
        let mut bytes = Vec::with_capacity(PEER_BYTES);

        bytes.extend_from_slice(&node.id);
        bytes.extend_from_slice(&node.ip.octets());
        bytes.extend_from_slice(&node.port.to_be_bytes());
        bytes.extend_from_slice(&node.cluster_port.to_be_bytes());

        bytes
    }
}

impl fmt::Display for PacketData {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            PacketData::Fail(_) => write!(f, "FAIL"),
            PacketData::Gossip(data) => match data.kind {
                GossipDataKind::Meet => write!(f, "MEET"),
                GossipDataKind::Ping => write!(f, "PING"),
                GossipDataKind::Pong => write!(f, "PONG"),
            },
            PacketData::Update(_) => write!(f, "UPDATE"),
        }
    }
}
