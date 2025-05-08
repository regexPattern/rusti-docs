use resp::{BulkString, RespDataType, SimpleString};

use super::{Shard, data_type::RedisDataType};

impl Shard {
    // https://redis.io/docs/latest/commands/get
    pub(super) fn get(&self, key: &BulkString) -> Result<Vec<u8>, u16> {
        let slot = self.get_hash_slot(key)?;

        Ok(match slot.get(key) {
            Some(RedisDataType::String(value)) => value.into(),
            _ => RespDataType::Null.into(),
        })
    }

    // https://redis.io/docs/latest/commands/set
    pub(super) fn set(&mut self, key: BulkString, value: BulkString) -> Result<Vec<u8>, u16> {
        let slot = self.get_hash_slot_mut(&key)?;
        let value = RedisDataType::String(value);

        Ok(match slot.insert(key, value) {
            Some(RedisDataType::String(prev_value)) => prev_value.into(),
            _ => SimpleString::from("OK").into(),
        })
    }
}
