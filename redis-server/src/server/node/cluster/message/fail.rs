use crate::server::node::cluster::NodeId;
use std::fmt;

use super::MessageData;

#[derive(PartialEq, Debug)]
pub struct FailData {
    pub id: NodeId,
}

impl From<&[u8]> for FailData {
    fn from(bytes: &[u8]) -> Self {
        let mut id = [0u8; 20];
        id.copy_from_slice(&bytes[0..20]);
        Self { id }
    }
}

impl From<&FailData> for Vec<u8> {
    fn from(data: &FailData) -> Self {
        data.id.to_vec()
    }
}

impl From<FailData> for MessageData {
    fn from(msg: FailData) -> Self {
        MessageData::Fail(msg)
    }
}

impl fmt::Display for FailData {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "FAIL {}", hex::encode(self.id))
    }
}
