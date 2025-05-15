use std::collections::HashSet;

use redis_resp::{BulkString, Integer, RespDataType, Set, SimpleError};

use super::{StorageActor, data_type::RedisDataType, error::OperationError};

impl StorageActor {
    // https://redis.io/docs/latest/commands/sadd
    pub(super) fn sadd(
        &mut self,
        key: BulkString,
        members: Vec<BulkString>,
    ) -> Result<Vec<u8>, u16> {
        let slot = self.get_hash_slot_mut(&key)?;

        if members.is_empty() {
            return Ok(SimpleError::from(OperationError::WrongNumberOfArgs).into());
        }

        let set = slot
            .entry(key)
            .or_insert(RedisDataType::Set(HashSet::new()));

        let set = match set {
            RedisDataType::Set(set) => set,
            _ => return Ok(SimpleError::from(OperationError::WrongDataType).into()),
        };

        let added = members.len() as i64;

        set.extend(members);

        Ok(Integer::from(added).into())
    }

    // https://redis.io/docs/latest/commands/smembers
    pub(super) fn smembers(&self, key: &BulkString) -> Result<Vec<u8>, u16> {
        let slot = self.get_hash_slot(key)?;

        let set = match slot.get(key) {
            Some(RedisDataType::Set(set)) => set,
            Some(_) => return Ok(SimpleError::from(OperationError::WrongDataType).into()),
            None => return Ok(RespDataType::Null.into()),
        };

        Ok(Set::to_resp_vec(set))
    }

    // https://redis.io/docs/latest/commands/srem
    pub(super) fn srem(
        &mut self,
        key: BulkString,
        members: Vec<BulkString>,
    ) -> Result<Vec<u8>, u16> {
        let slot = self.get_hash_slot_mut(&key)?;

        if members.is_empty() {
            return Ok(SimpleError::from(OperationError::WrongNumberOfArgs).into());
        }

        let set = match slot.get_mut(&key) {
            Some(RedisDataType::Set(set)) => set,
            Some(_) => return Ok(SimpleError::from(OperationError::WrongDataType).into()),
            None => return Ok(Integer::from(0).into()),
        };

        let mut removed_count = 0;

        for member in members {
            if set.remove(&member) {
                removed_count += 1;
            }
        }

        Ok(Integer::from(removed_count).into())
    }

    // https://redis.io/docs/latest/commands/scard
    pub(super) fn scard(&self, key: &BulkString) -> Result<Vec<u8>, u16> {
        let slot = self.get_hash_slot(key)?;

        let set = match slot.get(key) {
            Some(RedisDataType::Set(set)) => set,
            Some(_) => return Ok(SimpleError::from(OperationError::WrongDataType).into()),
            None => return Ok(Integer::from(0).into()),
        };

        let count = set.len() as i64;

        Ok(Integer::from(count).into())
    }

    // https://redis.io/docs/latest/commands/sismember
    pub(super) fn sismember(&self, key: &BulkString, member: &BulkString) -> Result<Vec<u8>, u16> {
        let slot = self.get_hash_slot(key)?;

        let set = match slot.get(key) {
            Some(RedisDataType::Set(set)) => set,
            Some(_) => return Ok(SimpleError::from(OperationError::WrongDataType).into()),
            None => return Ok(Integer::from(0).into()),
        };

        let reply = if set.contains(member) { 1 } else { 0 };

        Ok(Integer::from(reply).into())
    }
}
