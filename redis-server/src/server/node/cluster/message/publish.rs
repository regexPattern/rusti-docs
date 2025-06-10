use super::MessageData;

use std::fmt;
use redis_resp::{BulkString, Error};
use std::convert::TryFrom;

#[derive(PartialEq, Debug, Clone)]
pub struct PublishData {
    pub channel: BulkString,
    pub message: BulkString,
}

impl TryFrom<&[u8]> for PublishData {
    type Error = Error;

    fn try_from(bytes: &[u8]) -> Result<Self, Self::Error> {
        let channel = BulkString::try_from(bytes)?;
        let channel_len = Vec::<u8>::from(&channel).len();

        let message = BulkString::try_from(&bytes[channel_len..])?;

        Ok(PublishData { channel, message })
    }
}

impl From<&PublishData> for Vec<u8> {
    fn from(data: &PublishData) -> Self {
        let mut bytes = Vec::new();
        bytes.extend(Vec::from(&data.channel));
        bytes.extend(Vec::from(&data.message));
        bytes
    }
}

impl fmt::Display for PublishData {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "PUBLISH {} {}", self.channel, self.message)
    }
}

impl From<PublishData> for MessageData {
    fn from(data: PublishData) -> Self {
        MessageData::Publish(data)
    }
}