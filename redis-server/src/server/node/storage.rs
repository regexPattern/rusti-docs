mod data_type;
mod error;
mod generic;
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
pub use error::InternalError;
use log::LogMsg;
use redis_cmd::{Command, storage::*};
use redis_resp::{Array, BulkString};

const CLUSTER_HASH_SLOTS: u16 = 16384;

#[derive(Debug)]
pub struct StorageActor {
    data: HashMap<BulkString, RedisDataType>,
    persistence_tx: Sender<StorageCommand>,
}

#[derive(Debug)]
pub struct StorageEnvelope {
    pub cmd: StorageCommand,
    pub reply_tx: Sender<Vec<u8>>,
}

impl StorageActor {
    pub fn start(
        append_file_path: PathBuf,
        logger_tx: Sender<LogMsg>,
    ) -> Result<Sender<StorageEnvelope>, InternalError> {
        let (storage_tx, storage_rx) = mpsc::channel();
        let (persistence_tx, persistence_rx) = mpsc::channel();

        let mut persistence_file = OpenOptions::new()
            .read(true)
            .append(true)
            .create(true)
            .open(&append_file_path)
            .map_err(InternalError::PersistenceFileOpen)?;

        let mut storage_actor = Self {
            data: HashMap::new(),
            persistence_tx,
        };

        storage_actor.rebuild_from_persistence_file(&mut persistence_file, &logger_tx)?;

        let logger_tx_clone = logger_tx.clone();

        thread::spawn(move || {
            while let Ok(envel) = storage_rx.recv() {
                if let Err(err) = storage_actor.process(envel) {
                    logger_tx_clone.send(log::error!("{err}")).unwrap();
                    break;
                }
            }
        });

        logger_tx.send(log::info!(
            "persistiendo base de datos al archivo {:?}",
            append_file_path
        ))?;

        thread::spawn(move || {
            for cmd in persistence_rx {
                let bytes = Vec::from(Command::Storage(cmd));
                if let Err(err) = Self::write_to_persistence_file(&mut persistence_file, &bytes) {
                    let _ = logger_tx.send(log::error!("{err}"));
                }
            }
        });

        Ok(storage_tx)
    }

    fn rebuild_from_persistence_file(
        &mut self,
        file: &mut File,
        logger_tx: &Sender<LogMsg>,
    ) -> Result<(), InternalError> {
        let mut bytes = Vec::new();

        file.read_to_end(&mut bytes)
            .map_err(InternalError::PersistenceFileRead)?;

        let mut remaining_bytes = bytes.as_slice();
        let mut n_applied_cmds = 0;

        while !remaining_bytes.is_empty() {
            let result = Array::parse_incremental(remaining_bytes)
                .map_err(InternalError::PersistenceFileFormat)?;

            let cmd = Command::try_from(result.0).map_err(InternalError::PersistenceFileCommand)?;

            if let Command::Storage(cmd) = cmd {
                self.apply(cmd);
            }

            remaining_bytes = result.1;
            n_applied_cmds += 1;
        }

        if n_applied_cmds > 0 {
            logger_tx.send(log::debug!(
                "recuperados {n_applied_cmds} comandos del archivo de persistencia"
            ))?;

            logger_tx.send(log::info!("reestablecido estado de la base de datos"))?;
        }

        Ok(())
    }

    fn write_to_persistence_file(file: &mut File, bytes: &[u8]) -> Result<(), InternalError> {
        file.write_all(bytes)
            .map_err(InternalError::PersistenceFileWrite)?;
        file.sync_all().map_err(InternalError::PersistenceFileWrite)
    }

    pub fn process(&mut self, envel: StorageEnvelope) -> Result<(), InternalError> {
        let reply = self.apply(envel.cmd.clone());

        self.persistence_tx.send(envel.cmd).unwrap();
        envel.reply_tx.send(reply).unwrap();

        Ok(())
    }

    fn apply(&mut self, cmd: StorageCommand) -> Vec<u8> {
        match cmd {
            StorageCommand::Del(Del { key, keys }) => self.del(key, keys),

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
            }) => self.linsert(key, pos, pivot, element),
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
        }
    }

    fn _hash_key(key: &BulkString) -> u16 {
        let key: &str = key.into();
        let hash = State::<XMODEM>::calculate(key.as_bytes());
        hash % CLUSTER_HASH_SLOTS
    }
}
