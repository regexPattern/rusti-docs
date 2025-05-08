use std::collections::{HashMap, HashSet, LinkedList};

use resp::BulkString;

#[derive(Debug)]
pub enum RedisDataType {
    String(BulkString),
    Hash(HashMap<BulkString, BulkString>),
    List(LinkedList<BulkString>),
    Set(HashSet<BulkString>),
}
