use super::StorageActor;
use redis_resp::{BulkString, Integer};

impl StorageActor {
    // https://redis.io/docs/latest/commands/del
    pub(super) fn del(
        &mut self,
        key: BulkString,
        mut keys: Vec<BulkString>,
    ) -> Result<Vec<u8>, u16> {
        let mut removed = 0;

        keys.insert(0, key);

        for key in keys {
            let slot = self.get_hash_slot_mut(&key)?;
            if slot.remove(&key).is_some() {
                removed += 1;
            }
        }

        Ok(Integer::from(removed).into())
    }
}
