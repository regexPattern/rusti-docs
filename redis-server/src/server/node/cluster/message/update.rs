use crate::server::node::cluster::{CLUSTER_SLOTS, NodeId};

use super::MessageData;
use std::fmt;

#[derive(PartialEq, Debug)]
pub struct UpdateData {
    id: NodeId,
    config_epoch: u64,
    slots: Box<[u8; CLUSTER_SLOTS / 8]>,
}

impl From<&[u8]> for UpdateData {
    fn from(bytes: &[u8]) -> Self {
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

        Self {
            id,
            config_epoch,
            slots: Box::new(slots),
        }
    }
}

impl From<UpdateData> for MessageData {
    fn from(msg: UpdateData) -> Self {
        MessageData::Update(msg)
    }
}

impl From<&UpdateData> for Vec<u8> {
    fn from(data: &UpdateData) -> Self {
        let mut bytes = Vec::new();
        bytes.extend_from_slice(&data.id);
        bytes.extend_from_slice(&data.config_epoch.to_be_bytes());
        bytes.extend_from_slice(&data.slots[..]);
        bytes
    }
}

impl fmt::Display for UpdateData {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "Update(id={:x?}, config_epoch={}, slots=[...])",
            self.id,
            self.config_epoch
        )
    }
}
