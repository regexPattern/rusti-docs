mod data_type;
mod error;
mod hash;
mod list;
mod set;
mod string;

use std::{
    collections::HashMap,
    fs::{File, OpenOptions},
    io::{Read, Write},
    path::PathBuf,
    sync::mpsc::{self, Sender},
    thread,
};

use crc16::{State, XMODEM};
use data_type::RedisDataType;
use error::Error;
use log::LogMsg;
use redis_cmd::{Command, storage::*};
use redis_resp::{Array, BulkString};

const CLUSTER_HASH_SLOTS: u16 = 16384;

#[derive(Debug)]
pub struct StorageActor {
    hash_slots: HashMap<u16, HashMap<BulkString, RedisDataType>>,
    persistence_tx: Sender<StorageCommand>,
}

#[derive(Debug)]
pub struct StorageEnvelope {
    pub cmd: StorageCommand,
    pub reply_tx: Sender<Result<Vec<u8>, u16>>,
}

impl StorageActor {
    pub fn start(append_file_path: PathBuf, logger_tx: Sender<LogMsg>) -> Result<Self, Error> {
        let (persistence_tx, persistence_rx): (
            Sender<StorageCommand>,
            mpsc::Receiver<StorageCommand>,
        ) = mpsc::channel();

        let mut hash_slots = HashMap::new();

        for i in 0..CLUSTER_HASH_SLOTS {
            hash_slots.insert(i, HashMap::new());
        }

        let mut persistence_file = OpenOptions::new()
            .read(true)
            .append(true)
            .create(true)
            .open(&append_file_path)
            .unwrap();

        let mut storage_actor = Self {
            hash_slots,
            persistence_tx,
        };

        storage_actor.rebuild_from_persistence_file(&mut persistence_file);

        thread::spawn(move || {
            logger_tx
                .send(log::info!(
                    "se persistirán los comandos al archivo {:?}",
                    append_file_path
                ))
                .unwrap();

            for cmd in persistence_rx {
                let bytes = Vec::from(Command::Storage(cmd));

                persistence_file.write_all(&bytes).unwrap();
                persistence_file.flush().unwrap();

                logger_tx.send(log::info!("persistiendo comando")).unwrap();
            }
        });

        Ok(storage_actor)
    }

    fn rebuild_from_persistence_file(&mut self, file: &mut File) {
        let mut bytes = Vec::new();

        file.read_to_end(&mut bytes).unwrap();

        let mut remaining_bytes = bytes.as_slice();

        while !remaining_bytes.is_empty() {
            let result = Array::parse_incremental(remaining_bytes).unwrap();

            let cmd = Command::try_from(result.0).unwrap();

            if let Command::Storage(cmd) = cmd {
                self.apply(cmd).unwrap();
            }

            remaining_bytes = result.1;
        }
    }

    pub fn process(&mut self, envel: StorageEnvelope) {
        let reply = self.apply(envel.cmd.clone());

        if reply.is_ok() {
            self.persistence_tx.send(envel.cmd).unwrap();
        }

        envel.reply_tx.send(reply).unwrap();
    }

    fn apply(&mut self, cmd: StorageCommand) -> Result<Vec<u8>, u16> {
        match cmd {
            StorageCommand::Del(Del { keys }) => todo!(),

            StorageCommand::HSet(HSet {
                key,
                field_value_pairs,
            }) => self.hset(key, field_value_pairs),
            StorageCommand::HGet(HGet { key, field }) => self.hget(&key, &field),
            StorageCommand::HDel(HDel { key, fields }) => self.hdel(&key, &fields),
            StorageCommand::HGetAll(HGetAll { key }) => self.hgetall(&key),
            StorageCommand::HKeys(HKeys { key }) => self.hkeys(&key),
            StorageCommand::HVals(HVals { key }) => self.hvals(&key),
            StorageCommand::HExists(HExists { key, field }) => self.hexists(&key, &field),

            StorageCommand::LPush(LPush { key, elements }) => self.lpush(key, elements),
            StorageCommand::LInsert(LInsert {
                key,
                pos,
                pivot,
                element,
            }) => todo!(),
            StorageCommand::LPop(LPop { key, count }) => self.lpop(&key, count),
            StorageCommand::LIndex(LIndex { key, index }) => self.lindex(key, index),
            StorageCommand::LLen(LLen { key }) => self.llen(&key),
            StorageCommand::LRange(LRange { key, start, stop }) => self.lrange(&key, &start, &stop),

            StorageCommand::SAdd(SAdd { key, members }) => self.sadd(key, members),
            StorageCommand::SRem(SRem { key, members }) => self.srem(key, members),
            StorageCommand::SCard(SCard { key }) => self.scard(&key),
            StorageCommand::SIsMember(SIsMember { key, member }) => self.sismember(&key, &member),
            StorageCommand::SMembers(SMembers { key }) => self.smembers(&key),

            StorageCommand::Set(Set { key, value }) => self.set(key, value),
            StorageCommand::Get(Get { key }) => self.get(&key),

            StorageCommand::Append(Append { key, value }) => self.append(key, value),
            StorageCommand::Decr(Decr { key }) => self.decr(key),
            StorageCommand::Incr(Incr { key }) => self.incr(key),
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
