use std::fmt;

use super::MessagePayload;

const FAILOVER_PAYLOAD_KIND_AUTH_REQUEST: u8 = 3;
const FAILOVER_PAYLOAD_KING_AUTH_ACK: u8 = 4;

#[derive(Copy, Clone, PartialEq, Debug)]
pub struct FailOverPayload {
    pub kind: FailOverKind,
}

#[derive(Copy, Clone, PartialEq, Debug)]
pub enum FailOverKind {
    AuthRequest,
    AuthAck,
}

impl From<&[u8]> for FailOverPayload {
    fn from(bytes: &[u8]) -> Self {
        let kind = match bytes[0] {
            FAILOVER_PAYLOAD_KIND_AUTH_REQUEST => FailOverKind::AuthRequest,
            FAILOVER_PAYLOAD_KING_AUTH_ACK => FailOverKind::AuthAck,
            _ => unreachable!(),
        };

        Self { kind }
    }
}

impl From<&FailOverPayload> for Vec<u8> {
    fn from(payload: &FailOverPayload) -> Self {
        vec![match payload.kind {
            FailOverKind::AuthRequest => FAILOVER_PAYLOAD_KIND_AUTH_REQUEST,
            FailOverKind::AuthAck => FAILOVER_PAYLOAD_KING_AUTH_ACK,
        }]
    }
}

impl From<FailOverPayload> for MessagePayload {
    fn from(payload: FailOverPayload) -> Self {
        MessagePayload::FailOver(payload)
    }
}

impl fmt::Display for FailOverPayload {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self.kind {
            FailOverKind::AuthRequest => write!(f, "AUTH_REQUEST"),
            FailOverKind::AuthAck => write!(f, "AUTH_ACK"),
        }
    }
}
