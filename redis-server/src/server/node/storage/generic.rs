use redis_resp::{BulkString, Integer};

use super::StorageActor;

impl StorageActor {
    // https://redis.io/docs/latest/commands/del
    pub(super) fn del(&mut self, key: BulkString, mut keys: Vec<BulkString>) -> Vec<u8> {
        let mut removed = 0;

        keys.insert(0, key);

        for key in keys {
            if self.data.remove(&key).is_some() {
                removed += 1;
            }
        }

        Integer::from(removed).into()
    }
}
