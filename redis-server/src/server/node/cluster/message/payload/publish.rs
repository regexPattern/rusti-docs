use std::{convert::TryFrom, fmt};

use redis_resp::{BulkString, Error};

use super::MessagePayload;

#[derive(PartialEq, Debug, Clone)]
pub struct PublishPayload {
    pub channel: BulkString,
    pub message: BulkString,
}

impl TryFrom<&[u8]> for PublishPayload {
    type Error = Error;

    fn try_from(bytes: &[u8]) -> Result<Self, Self::Error> {
        let channel = BulkString::try_from(bytes)?;
        let channel_len = Vec::<u8>::from(&channel).len();

        let message = BulkString::try_from(&bytes[channel_len..])?;

        Ok(PublishPayload { channel, message })
    }
}

impl From<&PublishPayload> for Vec<u8> {
    fn from(payload: &PublishPayload) -> Self {
        let mut bytes = Vec::new();
        bytes.extend(Vec::from(&payload.channel));
        bytes.extend(Vec::from(&payload.message));
        bytes
    }
}

impl fmt::Display for PublishPayload {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "PUBLISH {} {}", self.channel, self.message)
    }
}

impl From<PublishPayload> for MessagePayload {
    fn from(payload: PublishPayload) -> Self {
        MessagePayload::Publish(payload)
    }
}
