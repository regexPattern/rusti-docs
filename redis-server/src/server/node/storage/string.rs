use redis_resp::{BulkString, RespDataType, SimpleError, SimpleString};

use super::{StorageActor, data_type::RedisDataType, error::OperationError};

impl StorageActor {
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

    // https://redis.io/docs/latest/commands/append
    pub(super) fn append(&mut self, key: BulkString, value: BulkString) -> Result<Vec<u8>, u16> {
        let slot = self.get_hash_slot_mut(&key)?;

        let existing_value = match slot.get_mut(&key) {
            Some(RedisDataType::String(existing_value)) => existing_value,
            Some(_) => return Ok(SimpleError::from(OperationError::WrongDataType).into()),
            None => {
                slot.insert(key.clone(), RedisDataType::String(value.clone()));
                return Ok(value.len().to_string().into_bytes());
            }
        };
        let mut existing_value = existing_value.to_string();
        let value = value.to_string();
        existing_value.push_str(&value);
        let updated_key = BulkString::from(existing_value.to_string());
        slot.insert(key, RedisDataType::String(updated_key));
        Ok(BulkString::from(existing_value.len().to_string()).into())
    }

    pub(super) fn decr(&mut self, key: BulkString) -> Result<Vec<u8>, u16> {
        let slot = self.get_hash_slot_mut(&key)?;

        let existing_value = match slot.get_mut(&key) {
            Some(RedisDataType::String(existing_value)) => existing_value,
            Some(_) => return Ok(SimpleError::from(OperationError::WrongDataType).into()),
            None => {
                slot.insert(key.clone(), RedisDataType::String(BulkString::from("-1")));
                return Ok(BulkString::from("-1").into());
            }
        };
        let existing_value = existing_value.to_string();

        let new_value = match existing_value.parse::<i64>() {
            Ok(value) => value - 1,
            Err(_) => return Ok(SimpleError::from(OperationError::ValueNotAnInteger).into()),
        };
        slot.insert(
            key,
            RedisDataType::String(BulkString::from(new_value.to_string())),
        );

        Ok(BulkString::from(new_value.to_string()).into())
    }
    pub(super) fn incr(&mut self, key: BulkString) -> Result<Vec<u8>, u16> {
        let slot = self.get_hash_slot_mut(&key)?;

        let existing_value = match slot.get_mut(&key) {
            Some(RedisDataType::String(existing_value)) => existing_value,
            Some(_) => return Ok(SimpleError::from(OperationError::WrongDataType).into()),
            None => {
                slot.insert(key.clone(), RedisDataType::String(BulkString::from("1")));
                return Ok(BulkString::from("1").into());
            }
        };
        let existing_value = existing_value.to_string();

        let new_value = match existing_value.parse::<i64>() {
            Ok(value) => value + 1,
            Err(_) => return Ok(SimpleError::from(OperationError::ValueNotAnInteger).into()),
        };
        slot.insert(
            key,
            RedisDataType::String(BulkString::from(new_value.to_string())),
        );

        Ok(BulkString::from(new_value.to_string()).into())
    }
}
