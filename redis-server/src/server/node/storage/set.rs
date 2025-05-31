use std::collections::HashSet;

use redis_resp::{BulkString, Integer, RespDataType, Set, SimpleError};

use super::{StorageActor, data_type::RedisDataType, error::OperationError};

impl StorageActor {
    // https://redis.io/docs/latest/commands/sadd
    pub(super) fn sadd(&mut self, key: BulkString, members: Vec<BulkString>) -> Vec<u8> {
        if members.is_empty() {
            return SimpleError::from(OperationError::WrongNumberOfArgs).into();
        }

        let set = self
            .data
            .entry(key)
            .or_insert(RedisDataType::Set(HashSet::new()));

        let set = match set {
            RedisDataType::Set(set) => set,
            _ => return SimpleError::from(OperationError::WrongDataType).into(),
        };

        let added = members.len() as i64;

        set.extend(members);

        Integer::from(added).into()
    }

    // https://redis.io/docs/latest/commands/smembers
    pub(super) fn smembers(&self, key: &BulkString) -> Vec<u8> {
        match self.data.get(key) {
            Some(RedisDataType::Set(set)) => Set::to_resp_vec(set),
            Some(_) => SimpleError::from(OperationError::WrongDataType).into(),
            None => RespDataType::Null.into(),
        }
    }

    // https://redis.io/docs/latest/commands/srem
    pub(super) fn srem(&mut self, key: BulkString, members: Vec<BulkString>) -> Vec<u8> {
        if members.is_empty() {
            return SimpleError::from(OperationError::WrongNumberOfArgs).into();
        }

        let set = match self.data.get_mut(&key) {
            Some(RedisDataType::Set(set)) => set,
            Some(_) => return SimpleError::from(OperationError::WrongDataType).into(),
            None => return Integer::from(0).into(),
        };

        let mut removed_count = 0;

        for member in members {
            if set.remove(&member) {
                removed_count += 1;
            }
        }

        Integer::from(removed_count).into()
    }

    // https://redis.io/docs/latest/commands/scard
    pub(super) fn scard(&self, key: &BulkString) -> Vec<u8> {
        match self.data.get(key) {
            Some(RedisDataType::Set(set)) => Integer::from(set.len() as i64).into(),
            Some(_) => SimpleError::from(OperationError::WrongDataType).into(),
            None => Integer::from(0).into(),
        }
    }

    // https://redis.io/docs/latest/commands/sismember
    pub(super) fn sismember(&self, key: &BulkString, member: &BulkString) -> Vec<u8> {
        match self.data.get(key) {
            Some(RedisDataType::Set(set)) => {
                let reply = if set.contains(member) { 1 } else { 0 };
                Integer::from(reply).into()
            }
            Some(_) => SimpleError::from(OperationError::WrongDataType).into(),
            None => Integer::from(0).into(),
        }
    }
}
