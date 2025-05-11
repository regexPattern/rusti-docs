use std::collections::LinkedList;

use redis_resp::{Array, BulkString, Integer, SimpleError};

use super::{Shard, data_type::RedisDataType, error::Error};

impl Shard {
    // https://redis.io/docs/latest/commands/lpush
    pub(super) fn lpush(
        &mut self,
        key: BulkString,
        elements: Vec<BulkString>,
    ) -> Result<Vec<u8>, u16> {
        let slot = self.get_hash_slot_mut(&key)?;

        let list = slot
            .entry(key)
            .or_insert(RedisDataType::List(LinkedList::new()));

        let list = match list {
            RedisDataType::List(list) => list,
            _ => return Ok(SimpleError::from(Error::WrongType).into()),
        };

        let added = elements.len() as i64;

        for e in elements {
            list.push_front(e);
        }

        Ok(Integer::from(added).into())
    }

    // https://redis.io/docs/latest/commands/llen
    pub(super) fn llen(&self, key: &BulkString) -> Result<Vec<u8>, u16> {
        let slot = self.get_hash_slot(key)?;

        let list = match slot.get(key) {
            Some(RedisDataType::List(list)) => list,
            Some(_) => return Ok(SimpleError::from(Error::WrongType).into()),
            None => return Ok(Integer::from(0).into()),
        };

        Ok(Integer::from(list.len() as i64).into())
    }

    // https://redis.io/docs/latest/commands/lrange
    pub(super) fn lrange(
        &self,
        key: &BulkString,
        start: &BulkString,
        stop: &BulkString,
    ) -> Result<Vec<u8>, u16> {
        let slot = self.get_hash_slot(key)?;

        let list = match slot.get(key) {
            Some(RedisDataType::List(list)) => list,
            Some(_) => return Ok(SimpleError::from(Error::WrongType).into()),
            None => return Ok(Array::to_resp_vec(&[])),
        };

        let (start, stop): (&str, &str) = (start.into(), stop.into());
        let (start, stop) = match (start.parse(), stop.parse()) {
            (Ok(start), Ok(stop)) => (start, stop),
            _ => return Ok(SimpleError::from(Error::ValueNotAnInteger).into()),
        };

        let slice = &slice_wrap_iter(list.iter(), start, stop, list.len());

        Ok(Array::to_resp_vec(slice))
    }
}

fn slice_wrap_iter<'l, L>(list: L, start: i64, end: i64, len: usize) -> Vec<&'l BulkString>
where
    L: Iterator<Item = &'l BulkString>,
{
    match (map_index(start, len), map_index(end, len)) {
        (Some(s), Some(e)) if s <= e => list.skip(s).take(e - s + 1).collect(),
        _ => Vec::new(),
    }
}

fn map_index(index: i64, len: usize) -> Option<usize> {
    if index >= 0 {
        let u = index as usize;
        if u < len { Some(u) } else { None }
    } else {
        let abs = (-index) as usize;
        if abs == 0 || abs > len {
            None
        } else {
            Some(len - abs)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn map_index_positivo_dentro_de_limites() {
        assert_eq!(map_index(0, 3), Some(0));
        assert_eq!(map_index(2, 3), Some(2));
    }

    #[test]
    fn map_index_positivo_fuera_de_limites() {
        assert_eq!(map_index(3, 3), None);
        assert_eq!(map_index(5, 3), None);
    }

    #[test]
    fn map_index_negativo_dentro_de_limites() {
        assert_eq!(map_index(-1, 3), Some(2));
        assert_eq!(map_index(-3, 3), Some(0));
    }

    #[test]
    fn map_index_negativo_fuera_de_limites() {
        assert_eq!(map_index(-4, 3), None);
    }

    #[test]
    fn slice_wrap_iter_subsegmento_basico() {
        let list = LinkedList::from([
            BulkString::from("a"),
            BulkString::from("b"),
            BulkString::from("c"),
        ]);

        let slice = slice_wrap_iter(list.iter(), 1, 2, list.len());

        assert_eq!(slice, [&BulkString::from("b"), &BulkString::from("c")]);
    }

    #[test]
    fn slice_wrap_iter_indices_negativos() {
        let list = LinkedList::from([
            BulkString::from("a"),
            BulkString::from("b"),
            BulkString::from("c"),
            BulkString::from("d"),
        ]);

        let slice = slice_wrap_iter(list.iter(), -2, -1, list.len());

        assert_eq!(slice, [&BulkString::from("c"), &BulkString::from("d")]);
    }

    #[test]
    fn slice_wrap_iter_inicio_mayor_que_final() {
        let list = LinkedList::from([BulkString::from("a"), BulkString::from("b")]);

        let slice = slice_wrap_iter(list.iter(), 2, 0, list.len());

        assert!(slice.is_empty());
    }

    #[test]
    fn slice_wrap_iter_fuera_de_limites() {
        let list = LinkedList::from([BulkString::from("a")]);

        let slice = slice_wrap_iter(list.iter(), 0, 5, list.len());

        assert!(slice.is_empty());
    }
}
