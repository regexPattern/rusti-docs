use std::collections::HashMap;

use redis_resp::{BulkString, Integer, Map, RespDataType, SimpleError};

use super::{StorageActor, data_type::RedisDataType, error::OperationError};

impl StorageActor {
    // https://redis.io/docs/latest/commands/hset
    pub(super) fn hset(&mut self, key: BulkString, field_value_pairs: Vec<BulkString>) -> Vec<u8> {
        if field_value_pairs.is_empty() || field_value_pairs.len() % 2 != 0 {
            return SimpleError::from(OperationError::WrongNumberOfArgs).into();
        }

        let hash = self
            .data
            .entry(key)
            .or_insert(RedisDataType::Hash(HashMap::new()));

        let hash = match hash {
            RedisDataType::Hash(hash) => hash,
            _ => return SimpleError::from(OperationError::WrongDataType).into(),
        };

        let mut added = 0;
        let mut pairs = field_value_pairs.into_iter();

        while let (Some(key), Some(value)) = (pairs.next(), pairs.next()) {
            hash.insert(key, value);
            added += 1;
        }

        Integer::from(added).into()
    }

    // https://redis.io/docs/latest/commands/hget
    pub(super) fn hget(&self, key: &BulkString, field: &BulkString) -> Vec<u8> {
        let hash = match self.data.get(key) {
            Some(RedisDataType::Hash(hash)) => hash,
            Some(_) => return SimpleError::from(OperationError::WrongDataType).into(),
            None => return RespDataType::Null.into(),
        };

        match hash.get(field) {
            Some(value) => value.into(),
            _ => RespDataType::Null.into(),
        }
    }

    // https://redis.io/docs/latest/commands/hdel
    pub(super) fn hdel(&mut self, key: &BulkString, fields: &[BulkString]) -> Vec<u8> {
        let hash = match self.data.get_mut(key) {
            Some(RedisDataType::Hash(hash)) => hash,
            Some(_) => return SimpleError::from(OperationError::WrongDataType).into(),
            None => return RespDataType::Null.into(),
        };

        let mut deleted = 0;

        for f in fields {
            if hash.remove(f).is_some() {
                deleted += 1;
            }
        }

        Integer::from(deleted).into()
    }

    // https://redis.io/docs/latest/commands/hgetall
    pub(super) fn hgetall(&self, key: &BulkString) -> Vec<u8> {
        let hash = match self.data.get(key) {
            Some(RedisDataType::Hash(hash)) => hash,
            Some(_) => return SimpleError::from(OperationError::WrongDataType).into(),
            None => return RespDataType::Null.into(),
        };

        Map::to_resp_vec(hash)
    }

    // https://redis.io/docs/latest/commands/hkeys
    pub(super) fn hkeys(&self, key: &BulkString) -> Vec<u8> {
        let hash = match self.data.get(key) {
            Some(RedisDataType::Hash(hash)) => hash,
            Some(_) => return SimpleError::from(OperationError::WrongDataType).into(),
            None => return RespDataType::Null.into(),
        };

        let keys: Vec<RespDataType> = hash.keys().cloned().map(RespDataType::from).collect();

        redis_resp::Array::from(keys).into()
    }

    // https://redis.io/docs/latest/commands/hvals
    pub(super) fn hvals(&self, key: &BulkString) -> Vec<u8> {
        let hash = match self.data.get(key) {
            Some(RedisDataType::Hash(hash)) => hash,
            Some(_) => return SimpleError::from(OperationError::WrongDataType).into(),
            None => return RespDataType::Null.into(),
        };

        let values: Vec<RespDataType> = hash.values().cloned().map(RespDataType::from).collect();

        redis_resp::Array::from(values).into()
    }

    // https://redis.io/docs/latest/commands/hexists
    pub(super) fn hexists(&self, key: &BulkString, field: &BulkString) -> Vec<u8> {
        let hash = match self.data.get(key) {
            Some(RedisDataType::Hash(hash)) => hash,
            Some(_) => return SimpleError::from(OperationError::WrongDataType).into(),
            None => return Integer::from(0).into(),
        };

        if hash.contains_key(field) {
            Integer::from(1).into()
        } else {
            Integer::from(0).into()
        }
    }
}
