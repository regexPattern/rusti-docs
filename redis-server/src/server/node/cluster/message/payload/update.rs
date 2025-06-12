use crate::server::node::cluster::NodeId;

use super::MessagePayload;
use std::fmt;

#[derive(PartialEq, Debug)]
pub struct UpdatePayload {
    id: NodeId,
    config_epoch: u64,
    slots: (u16, u16),
}

impl From<&[u8]> for UpdatePayload {
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
            let start = u16::from_be_bytes([bytes[28], bytes[29]]);
            let end = u16::from_be_bytes([bytes[30], bytes[31]]);
            (start, end)
        };

        Self {
            id,
            config_epoch,
            slots,
        }
    }
}

impl From<UpdatePayload> for MessagePayload {
    fn from(msg: UpdatePayload) -> Self {
        MessagePayload::Update(msg)
    }
}

impl From<&UpdatePayload> for Vec<u8> {
    fn from(payload: &UpdatePayload) -> Self {
        let mut bytes = Vec::new();

        bytes.extend(payload.id);
        bytes.extend(payload.config_epoch.to_be_bytes());
        bytes.extend(payload.slots.0.to_be_bytes());
        bytes.extend(payload.slots.1.to_be_bytes());

        bytes
    }
}

impl fmt::Display for UpdatePayload {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "Update(id={:x?}, config_epoch={}, slots=[...])",
            self.id, self.config_epoch
        )
    }
}
