mod data_type;
mod error;
mod hash;
mod list;
mod set;
mod string;

use std::{collections::HashMap, sync::mpsc::Sender};

use crc16::{State, XMODEM};
use data_type::RedisDataType;
use redis_cmd::storage::*;
use redis_resp::BulkString;

const CLUSTER_HASH_SLOTS: u16 = 16384;

#[derive(Debug)]
pub struct Shard {
    hash_slots: HashMap<u16, HashMap<BulkString, RedisDataType>>,
}

#[derive(Debug)]
pub struct StorageEnvelope {
    pub cmd: StorageCommand,
    pub reply_tx: Sender<Result<Vec<u8>, u16>>,
}

impl Shard {
    pub fn new() -> Self {
        let mut hash_slots = HashMap::new();

        for i in 0..CLUSTER_HASH_SLOTS {
            hash_slots.insert(i, HashMap::new());
        }

        Self { hash_slots }
    }

    pub fn process(&mut self, envel: StorageEnvelope) {
        let reply = match envel.cmd {
            StorageCommand::Del(Del { keys }) => todo!(),

            StorageCommand::HSet(HSet {
                key,
                field_value_pairs,
            }) => self.hset(key, field_value_pairs),
            StorageCommand::HGet(HGet { key, field }) => self.hget(&key, &field),
            StorageCommand::HDel(HDel { key, fields }) => self.hdel(&key, &fields),
            StorageCommand::HGetAll(HGetAll { key }) => self.hgetall(&key),
            StorageCommand::HKeys(HKeys { key }) => todo!(),
            StorageCommand::HVals(HVals { key }) => todo!(),
            StorageCommand::HExists(HExists { key, field }) => todo!(),

            StorageCommand::LPush(LPush { key, elements }) => self.lpush(key, elements),
            StorageCommand::LInsert(LInsert {
                key,
                pos,
                pivot,
                element,
            }) => todo!(),
            StorageCommand::LPop(LPop { key, count }) => todo!(),
            StorageCommand::LIndex(LIndex { key, index }) => todo!(),
            StorageCommand::LLen(LLen { key }) => self.llen(&key),
            StorageCommand::LRange(LRange { key, start, stop }) => self.lrange(&key, &start, &stop),

            StorageCommand::SAdd(SAdd { key, members }) => self.sadd(key, members),
            StorageCommand::SRem(SRem { key, members }) => self.srem(key, members),
            StorageCommand::SCard(SCard { key }) => self.scard(&key),
            StorageCommand::SIsMember(SIsMember { key, member }) => self.sismember(&key, &member),
            StorageCommand::SMembers(SMembers { key }) => self.smembers(&key),

            StorageCommand::Set(Set { key, value }) => self.set(key, value),
            StorageCommand::Get(Get { key }) => self.get(&key),
            StorageCommand::Append(Append { key, value }) => todo!(),
            StorageCommand::Decr(Decr { key }) => todo!(),
            StorageCommand::Incr(Incr { key }) => todo!(),
        };

        envel.reply_tx.send(reply).unwrap();
    }

    fn hash_key(key: &BulkString) -> u16 {
        let key: &str = key.into();
        let hash = State::<XMODEM>::calculate(key.as_bytes());
        hash % CLUSTER_HASH_SLOTS
    }

    fn get_hash_slot(&self, key: &BulkString) -> Result<&HashMap<BulkString, RedisDataType>, u16> {
        let hash = Self::hash_key(key);
        self.hash_slots.get(&hash).ok_or(hash)
    }

    fn get_hash_slot_mut(
        &mut self,
        key: &BulkString,
    ) -> Result<&mut HashMap<BulkString, RedisDataType>, u16> {
        let hash = Self::hash_key(key);
        self.hash_slots.get_mut(&hash).ok_or(hash)
    }
}
