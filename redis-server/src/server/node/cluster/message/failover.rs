use super::MessageData;
use std::fmt;

const CLUSTERMSG_KIND_FAILOVER_AUTH_REQUEST: u8 = 3;
const CLUSTERMSG_TYPE_FAILOVER_AUTH_ACK: u8 = 4;

#[derive(PartialEq, Debug, Clone)]
pub enum FailOverKind {
    AuthRequest,
    AuthAck,
}

#[derive(PartialEq, Debug, Clone)]
pub struct FailOverData {
    pub kind: FailOverKind,
}

impl From<&[u8]> for FailOverData {
    fn from(bytes: &[u8]) -> Self {
        let kind = match bytes[0] {
            CLUSTERMSG_KIND_FAILOVER_AUTH_REQUEST => FailOverKind::AuthRequest,
            CLUSTERMSG_TYPE_FAILOVER_AUTH_ACK => FailOverKind::AuthAck,
            _ => unreachable!(),
        };
        FailOverData { kind }
    }
}

impl From<&FailOverData> for Vec<u8> {
    fn from(data: &FailOverData) -> Self {
        let kind_byte = match data.kind {
            FailOverKind::AuthRequest => CLUSTERMSG_KIND_FAILOVER_AUTH_REQUEST,
            FailOverKind::AuthAck => CLUSTERMSG_TYPE_FAILOVER_AUTH_ACK,
        };
        vec![kind_byte]
    }
}

impl From<FailOverData> for MessageData {
    fn from(data: FailOverData) -> Self {
        MessageData::FailOver(data)
    }
}

impl From<&FailOverData> for MessageData {
    fn from(data: &FailOverData) -> Self {
        MessageData::FailOver(data.clone())
    }
}

impl fmt::Display for FailOverData {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self.kind {
            FailOverKind::AuthRequest => write!(f, "AUTH_REQUEST"),
            FailOverKind::AuthAck => write!(f, "AUTH_ACK"),
        }
    }
}

