use std::collections::LinkedList;

use redis_resp::{Array, BulkString, Integer, SimpleError};

use super::{StorageActor, data_type::RedisDataType, error::OperationError};

impl StorageActor {
    // https://redis.io/docs/latest/commands/lpush
    pub(super) fn lpush(&mut self, key: BulkString, elements: Vec<BulkString>) -> Vec<u8> {
        let list = self
            .data
            .entry(key)
            .or_insert(RedisDataType::List(LinkedList::new()));

        let list = match list {
            RedisDataType::List(list) => list,
            _ => return SimpleError::from(OperationError::WrongDataType).into(),
        };

        let added = elements.len() as i64;

        for e in elements {
            list.push_front(e);
        }

        Integer::from(added).into()
    }

    // https://redis.io/docs/latest/commands/llen
    pub(super) fn llen(&self, key: &BulkString) -> Vec<u8> {
        match self.data.get(key) {
            Some(RedisDataType::List(list)) => Integer::from(list.len() as i64).into(),
            Some(_) => SimpleError::from(OperationError::WrongDataType).into(),
            None => Integer::from(0).into(),
        }
    }

    // https://redis.io/docs/latest/commands/lrange
    pub(super) fn lrange(
        &self,
        key: &BulkString,
        start: &BulkString,
        stop: &BulkString,
    ) -> Vec<u8> {
        let list = match self.data.get(key) {
            Some(RedisDataType::List(list)) => list,
            Some(_) => return SimpleError::from(OperationError::WrongDataType).into(),
            None => return Array::to_resp_vec(&[]),
        };

        let (start, stop): (&str, &str) = (start.into(), stop.into());
        let (start, stop) = match (start.parse(), stop.parse()) {
            (Ok(start), Ok(stop)) => (start, stop),
            _ => return SimpleError::from(OperationError::WrongDataType).into(),
        };

        let slice = &slice_wrap_iter(list.iter(), start, stop, list.len());

        Array::to_resp_vec(slice)
    }

    // https://redis.io/docs/latest/commands/lpop
    pub(super) fn lpop(&mut self, key: &BulkString, count: Option<BulkString>) -> Vec<u8> {
        let list = match self.data.get_mut(key) {
            Some(RedisDataType::List(list)) => list,
            Some(_) => return SimpleError::from(OperationError::WrongDataType).into(),
            None => return Array::to_resp_vec(&[]),
        };

        if list.is_empty() {
            self.data.remove(key);
            return Array::to_resp_vec(&[]);
        }
        let count_str = count.unwrap_or(BulkString::from("1"));
        let count_str: String = count_str.into();
        let n: i64 = match count_str.parse::<i64>() {
            Ok(value) if value >= 0 => value,
            _ => return SimpleError::from(OperationError::ValueNotAnInteger).into(),
        };

        let mut popped = Vec::new();
        for _ in 0..n {
            if let Some(val) = list.pop_front() {
                popped.push(val);
            } else {
                break;
            }
        }

        if n > 1 {
            Array::to_resp_vec(&popped.iter().collect::<Vec<_>>())
        } else {
            popped
                .into_iter()
                .next()
                .map_or(BulkString::from("").into(), Into::into)
        }
    }

    // https://redis.io/docs/latest/commands/lindex
    pub(super) fn lindex(&self, key: BulkString, index: BulkString) -> Vec<u8> {
        let list = match self.data.get(&key) {
            Some(RedisDataType::List(list)) => list,
            Some(_) => return SimpleError::from(OperationError::WrongDataType).into(),
            None => return BulkString::from("").into(),
        };

        let index: String = index.into();
        let index: i64 = match index.parse() {
            Ok(value) => value,
            _ => return SimpleError::from(OperationError::ValueNotAnInteger).into(),
        };

        let mapped_index = map_index(index, list.len());
        if let Some(i) = mapped_index {
            if let Some(element) = list.iter().nth(i) {
                return element.clone().into();
            }
        }

        BulkString::from("").into()
    }

    // https://redis.io/docs/latest/commands/linsert
    pub(super) fn linsert(
        &mut self,
        key: BulkString,
        position: BulkString,
        pivot: BulkString,
        element: BulkString,
    ) -> Vec<u8> {
        let list = match self.data.get_mut(&key) {
            Some(RedisDataType::List(list)) => list,
            Some(_) => return SimpleError::from(OperationError::WrongDataType).into(),
            None => return Integer::from(-1).into(),
        };

        let position: String = position.into();
        let before = match position.as_str() {
            "BEFORE" => true,
            "AFTER" => false,
            _ => return SimpleError::from(OperationError::WrongDataType).into(),
        };

        let mut inserted = false;
        let mut new_list = LinkedList::new();

        while let Some(item) = list.pop_front() {
            if item == pivot {
                if before {
                    new_list.push_back(element.clone());
                }
                new_list.push_back(item.clone());
                if !before {
                    new_list.push_back(element.clone());
                }
                inserted = true;
            } else {
                new_list.push_back(item);
            }
        }

        *list = new_list;

        if inserted {
            Integer::from(list.len() as i64).into()
        } else {
            Integer::from(-1).into()
        }
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
