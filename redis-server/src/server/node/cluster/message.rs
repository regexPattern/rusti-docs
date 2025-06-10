pub mod fail;
pub mod failover;
pub mod gossip;
pub mod publish;
pub mod update;

use std::{fmt, net::Ipv4Addr};

use fail::FailData;
use failover::FailOverData;
use gossip::GossipData;
pub use publish::PublishData;
use update::UpdateData;

use super::{CLUSTER_SLOTS, NodeId, flags::Flags};

const CLUSTERMSG_KIND_GOSSIP: u8 = 0;
const CLUSTERMSG_KIND_FAIL: u8 = 1;
const CLUSTERMSG_KIND_PUBLISH: u8 = 2;
const CLUSTERMSG_KIND_FAILOVER_AUTH_REQUEST: u8 = 3;
const CLUSTERMSG_TYPE_FAILOVER_AUTH_ACK: u8 = 4;
const CLUSTERMSG_TYPE_UPDATE: u8 = 5;

#[derive(Debug)]
pub struct Message {
    pub header: MessageHeader,
    pub data: MessageData,
}

impl From<&[u8]> for Message {
    fn from(bytes: &[u8]) -> Self {
        let header = MessageHeader::from(&bytes[0..(38 + CLUSTER_SLOTS / 8)]);
        let data = MessageData::from(&bytes[(38 + CLUSTER_SLOTS / 8)..]);
        Self { header, data }
    }
}

impl From<&Message> for Vec<u8> {
    fn from(msg: &Message) -> Self {
        let mut bytes = Vec::from(&msg.header);
        bytes.extend(Vec::from(&msg.data));
        bytes
    }
}

#[derive(PartialEq)]
pub struct MessageHeader {
    pub id: NodeId,
    pub ip: Ipv4Addr,
    pub port: u16,
    pub cluster_port: u16,
    pub flags: Flags,
    pub config_epoch: u64,
    pub slots: [u8; CLUSTER_SLOTS / 8],
    pub master_id: Option<NodeId>,
}

impl From<&[u8]> for MessageHeader {
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
        let config_epoch = u64::from_be_bytes([
            bytes[30], bytes[31], bytes[32], bytes[33], bytes[34], bytes[35], bytes[36], bytes[37],
        ]);
        let master_id = None;

        let slots = {
            let mut slots = [0u8; CLUSTER_SLOTS / 8];
            slots.copy_from_slice(&bytes[38..]);
            slots
        };

        Self {
            id,
            ip,
            port,
            cluster_port,
            flags,
            config_epoch,
            master_id,
            slots,
        }
    }
}

impl From<&MessageHeader> for Vec<u8> {
    fn from(header: &MessageHeader) -> Self {
        let mut bytes = Vec::new();

        bytes.extend(header.id);
        bytes.extend(header.ip.to_bits().to_be_bytes());
        bytes.extend(header.port.to_be_bytes());
        bytes.extend(header.cluster_port.to_be_bytes());
        bytes.extend(header.flags.0.to_be_bytes());
        bytes.extend(header.config_epoch.to_be_bytes());
        bytes.extend(header.slots);

        bytes
    }
}

#[derive(PartialEq, Debug)]
pub enum MessageData {
    Gossip(GossipData),
    Fail(FailData),
    Publish(PublishData),
    FailOver(FailOverData),
    Update(UpdateData),
}

impl From<&[u8]> for MessageData {
    fn from(bytes: &[u8]) -> Self {
        match bytes[0] {
            CLUSTERMSG_KIND_GOSSIP => GossipData::from(&bytes[1..]).into(),
            CLUSTERMSG_KIND_FAIL => FailData::from(&bytes[1..]).into(),
            CLUSTERMSG_KIND_PUBLISH => PublishData::try_from(&bytes[1..])
                .expect("Invalid PublishData bytes")
                .into(),
            CLUSTERMSG_KIND_FAILOVER_AUTH_REQUEST => FailOverData::from(&bytes[1..]).into(),
            CLUSTERMSG_TYPE_FAILOVER_AUTH_ACK => FailOverData::from(&bytes[1..]).into(),
            CLUSTERMSG_TYPE_UPDATE => UpdateData::from(&bytes[1..]).into(),
            _ => unreachable!(),
        }
    }
}

impl From<&MessageData> for Vec<u8> {
    fn from(data: &MessageData) -> Self {
        match data {
            MessageData::Gossip(data) => {
                let mut bytes = vec![CLUSTERMSG_KIND_GOSSIP];
                bytes.extend(Vec::from(data));
                bytes
            }
            MessageData::Fail(data) => {
                let mut bytes = vec![CLUSTERMSG_KIND_FAIL];
                bytes.extend(Vec::from(data));
                bytes
            }
            MessageData::Publish(data) => {
                let mut bytes = vec![CLUSTERMSG_KIND_PUBLISH];
                bytes.extend(Vec::from(data));
                bytes
            }
            MessageData::FailOver(data) => {
                let kind_byte = match data.kind {
                    failover::FailOverKind::AuthRequest => CLUSTERMSG_KIND_FAILOVER_AUTH_REQUEST,
                    failover::FailOverKind::AuthAck => CLUSTERMSG_TYPE_FAILOVER_AUTH_ACK,
                };
                let mut bytes = vec![kind_byte];
                bytes.extend(Vec::from(data));
                bytes
            }
            MessageData::Update(data) => {
                let mut bytes = vec![CLUSTERMSG_TYPE_UPDATE];
                bytes.extend(Vec::from(data));
                bytes
            }
        }
    }
}

impl fmt::Display for MessageData {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            MessageData::Gossip(data) => write!(f, "{data}"),
            MessageData::Fail(data) => write!(f, "{data}"),
            MessageData::Publish(data) => write!(f, "{data}"),
            MessageData::FailOver(data) => write!(f, "{data}"),
            MessageData::Update(data) => write!(f, "{data}"),
        }
    }
}

impl fmt::Debug for MessageHeader {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("MessageHeader")
            .field("id", &hex::encode(&self.id))
            .field("ip", &self.ip.to_string())
            .field("port", &self.port)
            .field("cluster_port", &self.cluster_port)
            .field("flags", &self.flags.to_string())
            .field("config_epoch", &self.config_epoch)
            .field("master_id", &self.master_id)
            .field("slots", &"TODO")
            .finish()
    }
}

#[cfg(test)]
mod tests {
    use std::net::Ipv4Addr;

    use crate::server::node::cluster::{
        CLUSTER_SLOTS,
        flags::{FLAG_MASTER, FLAG_MYSELF, Flags},
        message::MessageHeader,
    };

    #[test]
    fn se_deserializa_header_correctamente() {
        let mut bytes = Vec::new();

        let mut id = [0u8; 20];
        hex::decode_to_slice("a1b2c3d4e5f67890abcdef0123456789abcdef02", &mut id).unwrap();

        let ip = Ipv4Addr::new(127, 0, 0, 1);
        let port: u16 = 6379;
        let cluster_port: u16 = 16379;
        let flags = Flags(FLAG_MYSELF | FLAG_MASTER);
        let config_epoch: u64 = 0;

        let mut slots = [0u8; CLUSTER_SLOTS / 8];
        for i in 0..CLUSTER_SLOTS / 16 {
            slots[i] = u8::MAX;
        }

        bytes.extend(id);
        bytes.extend(ip.to_bits().to_be_bytes());
        bytes.extend(port.to_be_bytes());
        bytes.extend(cluster_port.to_be_bytes());
        bytes.extend(flags.0.to_be_bytes());
        bytes.extend(config_epoch.to_be_bytes());
        bytes.extend(slots);

        let header = MessageHeader {
            id,
            ip,
            port,
            cluster_port,
            flags,
            config_epoch,
            slots,
            master_id: None,
        };

        assert_eq!(header, MessageHeader::from(bytes.as_slice()));
    }
}
