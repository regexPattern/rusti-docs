use std::fmt;

use crate::server::node::cluster::NodeId;

use super::MessagePayload;

#[derive(Copy, Clone, PartialEq, Debug)]
pub struct FailPayload {
    pub id: NodeId,
}

impl From<&[u8]> for FailPayload {
    fn from(bytes: &[u8]) -> Self {
        let mut id = [0u8; 20];
        id.copy_from_slice(&bytes[0..20]);
        Self { id }
    }
}

impl From<&FailPayload> for Vec<u8> {
    fn from(payload: &FailPayload) -> Self {
        payload.id.to_vec()
    }
}

impl From<FailPayload> for MessagePayload {
    fn from(msg: FailPayload) -> Self {
        MessagePayload::Fail(msg)
    }
}

impl fmt::Display for FailPayload {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "FAIL")
    }
}
