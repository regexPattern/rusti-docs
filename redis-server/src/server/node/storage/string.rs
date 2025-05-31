use redis_resp::{BulkString, Integer, RespDataType, SimpleError, SimpleString};

use super::{StorageActor, data_type::RedisDataType, error::OperationError};

impl StorageActor {
    // https://redis.io/docs/latest/commands/get
    pub(super) fn get(&self, key: &BulkString) -> Vec<u8> {
        match self.data.get(key) {
            Some(RedisDataType::String(value)) => value.into(),
            _ => RespDataType::Null.into(),
        }
    }

    // https://redis.io/docs/latest/commands/set
    pub(super) fn set(&mut self, key: BulkString, value: BulkString) -> Vec<u8> {
        let value = RedisDataType::String(value);

        match self.data.insert(key, value) {
            Some(RedisDataType::String(prev_value)) => prev_value.into(),
            _ => SimpleString::from("OK").into(),
        }
    }

    // https://redis.io/docs/latest/commands/append
    pub(super) fn append(&mut self, key: BulkString, value: BulkString) -> Vec<u8> {
        let prev_value = self
            .data
            .entry(key)
            .or_insert(RedisDataType::String("".into()));

        if let RedisDataType::String(prev_value) = prev_value {
            prev_value.push_bs(&value);
            Integer::from(value.len() as i64).into()
        } else {
            SimpleError::from(OperationError::WrongDataType).into()
        }
    }

    // https://redis.io/docs/latest/commands/decr
    pub(super) fn decr(&mut self, key: BulkString) -> Vec<u8> {
        let prev_value = self
            .data
            .entry(key)
            .or_insert(RedisDataType::String("".into()));

        if let RedisDataType::String(prev_value) = prev_value {
            let new_value = match prev_value.parse::<i64>() {
                Ok(v) => v - 1,
                _ => return SimpleError::from(OperationError::ValueNotAnInteger).into(),
            };

            *prev_value = BulkString::from(new_value.to_string());

            Integer::from(new_value).into()
        } else {
            SimpleError::from(OperationError::WrongDataType).into()
        }
    }

    // https://redis.io/docs/latest/commands/incr
    pub(super) fn incr(&mut self, key: BulkString) -> Vec<u8> {
        let prev_value = self
            .data
            .entry(key)
            .or_insert(RedisDataType::String("".into()));

        if let RedisDataType::String(prev_value) = prev_value {
            let new_value = match prev_value.parse::<i64>() {
                Ok(v) => v + 1,
                _ => return SimpleError::from(OperationError::ValueNotAnInteger).into(),
            };

            *prev_value = BulkString::from(new_value.to_string());

            Integer::from(new_value).into()
        } else {
            SimpleError::from(OperationError::WrongDataType).into()
        }
    }
}
