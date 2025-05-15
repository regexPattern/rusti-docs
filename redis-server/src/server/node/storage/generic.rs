use super::StorageActor;
use redis_resp::{BulkString, Integer};

impl StorageActor {
    pub(super) fn del(&mut self, key: BulkString, keys: Vec<BulkString>) -> Result<Vec<u8>, u16> {
        let mut removed = 0;

        let hash = Self::hash_key(&key);
        if let Some(slot) = self.hash_slots.get_mut(&hash) {
            if slot.remove(&key).is_some() {
                removed += 1;
            }
        }

        for key in keys {
            let hash = Self::hash_key(&key);
            if let Some(slot) = self.hash_slots.get_mut(&hash) {
                if slot.remove(&key).is_some() {
                    removed += 1;
                }
            }
        }

        Ok(Integer::from(removed).into())
    }
}
