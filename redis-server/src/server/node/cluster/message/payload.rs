mod fail;
mod failover;
mod gossip;
mod publish;

use std::fmt;

pub use fail::FailPayload;
pub use failover::{FailOverKind, FailOverPayload};
pub use gossip::{GossipKind, GossipNode, GossipPayload};
pub use publish::PublishPayload;

const PAYLOAD_KIND_GOSSIP: u8 = 0;
const PAYLOAD_KIND_FAIL: u8 = 1;
const PAYLOAD_KIND_PUBLISH: u8 = 2;
const PAYLOAD_KIND_FAILOVER_AUTH_REQUEST: u8 = 3;
const PAYLOAD_TYPE_FAILOVER_AUTH_ACK: u8 = 4;

#[derive(PartialEq, Debug)]
pub enum MessagePayload {
    Gossip(GossipPayload),
    Fail(FailPayload),
    Publish(PublishPayload),
    FailOver(FailOverPayload),
}

impl From<&[u8]> for MessagePayload {
    fn from(bytes: &[u8]) -> Self {
        match bytes[0] {
            PAYLOAD_KIND_GOSSIP => GossipPayload::from(&bytes[1..]).into(),
            PAYLOAD_KIND_FAIL => FailPayload::from(&bytes[1..]).into(),
            PAYLOAD_KIND_PUBLISH => PublishPayload::try_from(&bytes[1..]).unwrap().into(),
            PAYLOAD_KIND_FAILOVER_AUTH_REQUEST => FailOverPayload::from(&bytes[1..]).into(),
            PAYLOAD_TYPE_FAILOVER_AUTH_ACK => FailOverPayload::from(&bytes[1..]).into(),
            _ => unreachable!(),
        }
    }
}

impl From<&MessagePayload> for Vec<u8> {
    fn from(payload: &MessagePayload) -> Self {
        match payload {
            MessagePayload::Gossip(payload) => {
                let mut bytes = vec![PAYLOAD_KIND_GOSSIP];
                bytes.extend(Vec::from(payload));
                bytes
            }
            MessagePayload::Fail(payload) => {
                let mut bytes = vec![PAYLOAD_KIND_FAIL];
                bytes.extend(Vec::from(payload));
                bytes
            }
            MessagePayload::Publish(payload) => {
                let mut bytes = vec![PAYLOAD_KIND_PUBLISH];
                bytes.extend(Vec::from(payload));
                bytes
            }
            MessagePayload::FailOver(payload) => {
                let mut bytes = vec![match payload.kind {
                    FailOverKind::AuthRequest => PAYLOAD_KIND_FAILOVER_AUTH_REQUEST,
                    FailOverKind::AuthAck => PAYLOAD_TYPE_FAILOVER_AUTH_ACK,
                }];
                bytes.extend(Vec::from(payload));
                bytes
            }
        }
    }
}

impl fmt::Display for MessagePayload {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            MessagePayload::Gossip(payload) => write!(f, "{payload}"),
            MessagePayload::Fail(payload) => write!(f, "{payload}"),
            MessagePayload::Publish(payload) => write!(f, "{payload}"),
            MessagePayload::FailOver(payload) => write!(f, "{payload}"),
        }
    }
}
