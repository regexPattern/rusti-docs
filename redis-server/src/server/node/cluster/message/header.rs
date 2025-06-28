use std::{fmt, net::Ipv4Addr, time::UNIX_EPOCH};

use crate::server::node::cluster::{
    ClusterNode, NodeId,
    flags::{self, Flags},
};

#[derive(Copy, Clone, PartialEq)]
pub struct MessageHeader {
    pub id: NodeId,
    pub ip: Ipv4Addr,
    pub port: u16,
    pub cluster_port: u16,
    pub flags: Flags,
    pub config_epoch: u64,
    pub slots: (u16, u16),
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

        let slots = {
            let start = u16::from_be_bytes([bytes[38], bytes[39]]);
            let end = u16::from_be_bytes([bytes[40], bytes[41]]);
            (start, end)
        };

        let master_id = {
            let mut id = [0u8; 20];
            id.copy_from_slice(&bytes[42..62]);
            if id != [0u8; 20] { Some(id) } else { None }
        };

        Self {
            id,
            ip,
            port,
            cluster_port,
            flags,
            config_epoch,
            slots,
            master_id,
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
        bytes.extend(header.slots.0.to_be_bytes());
        bytes.extend(header.slots.1.to_be_bytes());
        bytes.extend(header.master_id.unwrap_or_default());

        bytes
    }
}

impl From<&ClusterNode> for MessageHeader {
    fn from(node: &ClusterNode) -> Self {
        Self {
            id: node.id,
            ip: node.ip,
            port: node.port,
            cluster_port: node.cluster_port,
            flags: Flags(node.flags.0 & !flags::FLAG_MYSELF),
            config_epoch: node.config_epoch,
            slots: node.slots,
            master_id: node.master_id,
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
            pong_received: UNIX_EPOCH,
            config_epoch: header.config_epoch,
            slots: header.slots,
            master_id: header.master_id,
        }
    }
}

impl fmt::Debug for MessageHeader {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("MessageHeader")
            .field("id", &hex::encode(self.id))
            .field("ip", &self.ip.to_string())
            .field("port", &self.port)
            .field("cluster_port", &self.cluster_port)
            .field("flags", &self.flags.to_string())
            .field("config_epoch", &self.config_epoch)
            .field("slots", &self.slots)
            .field("master_id", &self.master_id.map(hex::encode))
            .finish()
    }
}

#[cfg(test)]
mod tests {
    use std::net::Ipv4Addr;

    use super::*;
    use crate::server::node::cluster::flags::{self, Flags};

    #[test]
    fn message_header_sin_master_id_se_serializa_y_deserializa_correctamente() {
        let header = MessageHeader {
            id: [0; 20],
            ip: Ipv4Addr::new(0, 0, 0, 0),
            port: 7000,
            cluster_port: 17000,
            flags: Flags(flags::FLAG_MYSELF | flags::FLAG_MASTER),
            config_epoch: 0,
            slots: (0, 0),
            master_id: None,
        };

        let bytes: Vec<u8> = Vec::from(&header);

        assert_eq!(MessageHeader::from(bytes.as_slice()), header);
    }

    #[test]
    fn message_header_con_master_id_se_serializa_y_deserializa_correctamente() {
        let header = MessageHeader {
            id: [0; 20],
            ip: Ipv4Addr::new(0, 0, 0, 0),
            port: 7000,
            cluster_port: 17000,
            flags: Flags(flags::FLAG_MYSELF | flags::FLAG_MASTER),
            config_epoch: 0,
            slots: (0, 0),
            master_id: Some([1; 20]),
        };

        let bytes: Vec<u8> = Vec::from(&header);

        assert_eq!(MessageHeader::from(bytes.as_slice()), header);
    }
}
