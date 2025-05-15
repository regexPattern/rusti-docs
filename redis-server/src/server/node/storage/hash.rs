use std::collections::HashMap;

use redis_resp::{BulkString, Integer, Map, RespDataType, SimpleError};

use super::{StorageActor, data_type::RedisDataType, error::OperationError};

impl StorageActor {
    // https://redis.io/docs/latest/commands/hset
    pub(super) fn hset(
        &mut self,
        key: BulkString,
        field_value_pairs: Vec<BulkString>,
    ) -> Result<Vec<u8>, u16> {
        let slot = self.get_hash_slot_mut(&key)?;

        if field_value_pairs.is_empty() || field_value_pairs.len() % 2 != 0 {
            return Ok(SimpleError::from(OperationError::WrongNumberOfArgs).into());
        }

        let hash = slot
            .entry(key)
            .or_insert(RedisDataType::Hash(HashMap::new()));

        let hash = match hash {
            RedisDataType::Hash(hash) => hash,
            _ => return Ok(SimpleError::from(OperationError::WrongDataType).into()),
        };

        let mut added = 0;
        let mut pairs = field_value_pairs.into_iter();

        while let (Some(key), Some(value)) = (pairs.next(), pairs.next()) {
            hash.insert(key, value);
            added += 1;
        }

        Ok(Integer::from(added).into())
    }

    // https://redis.io/docs/latest/commands/hget
    pub(super) fn hget(&self, key: &BulkString, field: &BulkString) -> Result<Vec<u8>, u16> {
        let slot = self.get_hash_slot(key)?;

        let hash = match slot.get(key) {
            Some(RedisDataType::Hash(hash)) => hash,
            Some(_) => return Ok(SimpleError::from(OperationError::WrongDataType).into()),
            None => return Ok(RespDataType::Null.into()),
        };

        Ok(match hash.get(field) {
            Some(value) => value.into(),
            _ => RespDataType::Null.into(),
        })
    }

    // https://redis.io/docs/latest/commands/hdel
    pub(super) fn hdel(&mut self, key: &BulkString, fields: &[BulkString]) -> Result<Vec<u8>, u16> {
        let slot = self.get_hash_slot_mut(key)?;

        let hash = match slot.get_mut(key) {
            Some(RedisDataType::Hash(hash)) => hash,
            Some(_) => return Ok(SimpleError::from(OperationError::WrongDataType).into()),
            None => return Ok(RespDataType::Null.into()),
        };

        let mut deleted = 0;

        for f in fields {
            if hash.remove(f).is_some() {
                deleted += 1;
            }
        }

        Ok(Integer::from(deleted).into())
    }

    // https://redis.io/docs/latest/commands/hgetall
    pub(super) fn hgetall(&self, key: &BulkString) -> Result<Vec<u8>, u16> {
        let slot = self.get_hash_slot(key)?;

        let hash = match slot.get(key) {
            Some(RedisDataType::Hash(hash)) => hash,
            Some(_) => return Ok(SimpleError::from(OperationError::WrongDataType).into()),
            None => return Ok(RespDataType::Null.into()),
        };

        Ok(Map::to_resp_vec(hash))
    }

    // https://redis.io/docs/latest/commands/hkeys
    pub(super) fn hkeys(&self, key: &BulkString) -> Result<Vec<u8>, u16> {
        let slot = self.get_hash_slot(key)?;

        let hash = match slot.get(key) {
            Some(RedisDataType::Hash(hash)) => hash,
            Some(_) => return Ok(SimpleError::from(OperationError::WrongDataType).into()),
            None => return Ok(RespDataType::Null.into()),
        };

        let keys: Vec<RespDataType> = hash.keys().cloned().map(RespDataType::from).collect();
        Ok(redis_resp::Array::from(keys).into())
    }

    // https://redis.io/docs/latest/commands/hvals
    pub(super) fn hvals(&self, key: &BulkString) -> Result<Vec<u8>, u16> {
        let slot = self.get_hash_slot(key)?;

        let hash = match slot.get(key) {
            Some(RedisDataType::Hash(hash)) => hash,
            Some(_) => return Ok(SimpleError::from(OperationError::WrongDataType).into()),
            None => return Ok(RespDataType::Null.into()),
        };

        let values: Vec<RespDataType> = hash.values().cloned().map(RespDataType::from).collect();
        Ok(redis_resp::Array::from(values).into())
    }

    // https://redis.io/docs/latest/commands/hexists
    pub(super) fn hexists(&self, key: &BulkString, field: &BulkString) -> Result<Vec<u8>, u16> {
        let slot = self.get_hash_slot(key)?;

        let hash = match slot.get(key) {
            Some(RedisDataType::Hash(hash)) => hash,
            Some(_) => return Ok(SimpleError::from(OperationError::WrongDataType).into()),
            None => return Ok(Integer::from(0).into()),
        };
        if hash.contains_key(field) {
            Ok(Integer::from(1).into())
        } else {
            Ok(Integer::from(0).into())
        }
    }
}
