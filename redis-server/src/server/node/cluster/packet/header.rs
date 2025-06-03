use std::net::Ipv4Addr;

use crate::server::node::cluster::{NodeId, node::ClusterNode};

use super::error::Error;

pub const HEADER_BYTES: usize = 36;

#[derive(Debug)]
pub struct PacketHeader {
    pub id: NodeId,
    pub ip: Ipv4Addr,
    pub port: u16,
    pub cluster_port: u16,
    pub current_epoch: u64,
}

//de dodcu ofical
//FALTAN:
// -> The currentEpoch anigEpoch
// fields of the sending node that are used to mount the distributed algorithms  used by Redis Cluster
// (this is explained in detail in the next sections). If The node is a replica the configEpoch is
// the last known configEpoch of its master.

// -> The node flags,
// indicating if the node is a replica, a master, and other single-bit node information.
// -> A bitmap
// of the hash slots served by THE SENDING NODE node, or if the node is a replica,
// a bitmap of the slots served by its master.
// -> The state of the cluster from the point of view of the sender (down or ok).
// -> The master node ID of the sending node, if it is a replica.(ok)

// (ok?)The sender TCP base port that is the port used by Redis to accept client commands.
//(ok?) The cluster port that is the port used by Redis for node-to-node communication.

impl TryFrom<&[u8]> for PacketHeader {
    type Error = Error;

    fn try_from(bytes: &[u8]) -> Result<Self, Self::Error> {
        if bytes.len() != 36 {
            return Err(Error::PacketDataKind);
        }

        let id = {
            let mut id = [0u8; 20];
            id.copy_from_slice(&bytes[0..20]);
            id
        };

        let ip = Ipv4Addr::new(bytes[20], bytes[21], bytes[22], bytes[23]);
        let port = u16::from_be_bytes([bytes[24], bytes[25]]);
        let cluster_port = u16::from_be_bytes([bytes[26], bytes[27]]);
        let current_epoch = u64::from_be_bytes([
            bytes[28], bytes[29], bytes[30], bytes[31], bytes[32], bytes[33], bytes[34], bytes[35],
        ]);

        Ok(Self {
            id,
            ip,
            port,
            current_epoch,
            cluster_port,
        })
    }
}

impl From<PacketHeader> for Vec<u8> {
    fn from(packet: PacketHeader) -> Self {
        let mut bytes = Vec::with_capacity(HEADER_BYTES);

        bytes.extend_from_slice(&packet.id);
        bytes.extend_from_slice(&packet.ip.octets());
        bytes.extend_from_slice(&packet.port.to_be_bytes());
        bytes.extend_from_slice(&packet.cluster_port.to_be_bytes());
        bytes.extend_from_slice(&packet.current_epoch.to_be_bytes());

        bytes
    }
}

impl From<&ClusterNode> for PacketHeader {
    fn from(node: &ClusterNode) -> Self {
        Self {
            id: node.id,
            ip: node.ip,
            port: node.port,
            current_epoch: 0,
            cluster_port: node.cluster_port,
        }
    }
}
