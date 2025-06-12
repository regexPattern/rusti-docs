pub mod header;
pub mod payload;

pub use header::MessageHeader;
pub use payload::*;

#[derive(PartialEq, Debug)]
pub struct Message {
    pub header: MessageHeader,
    pub payload: MessagePayload,
}

impl Message {
    pub fn from(bytes: &[u8]) -> (Self, usize) {
        let consumed = usize::from_be_bytes([
            bytes[0], bytes[1], bytes[2], bytes[3], bytes[4], bytes[5], bytes[6], bytes[7],
        ]);

        let header = MessageHeader::from(&bytes[8..70]);
        let payload = MessagePayload::from(&bytes[70..(8 + consumed)]);

        (Self { header, payload }, 8 + consumed)
    }
}

impl From<&Message> for Vec<u8> {
    fn from(msg: &Message) -> Self {
        let header = Vec::from(&msg.header);
        let payload = Vec::from(&msg.payload);
        let length = header.len() + payload.len();

        let mut bytes = Vec::with_capacity(2 + length);

        bytes.extend(length.to_be_bytes());
        bytes.extend(header);
        bytes.extend(payload);

        bytes
    }
}

#[cfg(test)]
mod tests {
    use std::net::Ipv4Addr;

    use crate::server::node::cluster::flags::{self, Flags};

    use super::*;

    #[test]
    fn message_se_serializa_y_deserializa_correctamente() {
        let header = MessageHeader {
            id: [0; 20],
            ip: Ipv4Addr::new(127, 0, 0, 1),
            port: 7000,
            cluster_port: 17000,
            flags: Flags(flags::FLAG_MYSELF | flags::FLAG_MASTER),
            config_epoch: 0,
            slots: (0, 0),
            master_id: None,
        };

        let payload = MessagePayload::Gossip(GossipPayload {
            kind: GossipKind::Meet,
            nodes: vec![GossipNode {
                id: [1; 20],
                ip: Ipv4Addr::new(127, 0, 0, 1),
                port: 7001,
                cluster_port: 17001,
                flags: Flags(flags::FLAG_MASTER),
            }],
        });

        let msg = Message { header, payload };
        let bytes: Vec<u8> = Vec::from(&msg);

        assert_eq!(Message::from(&bytes).0, msg);
    }
}
