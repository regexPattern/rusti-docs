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
    sync::mpsc::{self, Receiver, Sender},
    thread,
};

use data_type::RedisDataType;
pub use error::InternalError;
use log::Log;
use redis_cmd::{Command, storage::*};
use redis_resp::{Array, BulkString};

use crate::server::node::replication::ReplicationAction;

#[derive(Debug)]
pub struct StorageActor {
    data: HashMap<BulkString, RedisDataType>,
    persistence_tx: Sender<StorageCommand>,
}

#[derive(Debug)]
pub enum StorageAction {
    ClientCommand {
        cmd: StorageCommand,
        reply_tx: Sender<Vec<u8>>,
    },
    DumpHistory {
        history_tx: Sender<Vec<StorageCommand>>,
    },
}

impl StorageActor {
    pub fn start(
        append_file_path: PathBuf,
        actions_rx: Receiver<StorageAction>,
        replication_tx: Sender<ReplicationAction>,
        logger_tx: Sender<Log>,
    ) -> Result<(), InternalError> {
        let (commands_tx, commands_rx) = mpsc::channel();

        let mut persistence_file = OpenOptions::new()
            .read(true)
            .append(true)
            .create(true)
            .open(&append_file_path)
            .map_err(InternalError::PersistenceFileOpen)?;

        logger_tx.send(log::info!(
            "persistiendo base de datos al archivo {:?}",
            append_file_path
        ))?;

        let mut storage_actor = Self {
            data: HashMap::new(),
            persistence_tx: commands_tx,
        };

        let history = Self::load_history(&mut persistence_file);

        for cmd in &history {
            storage_actor.apply(cmd.clone());
        }

        if history.len() > 0 {
            logger_tx.send(log::info!(
                "recuperados {} comandos del archivo de persistencia",
                history.len()
            ))?;

            logger_tx.send(log::info!("restablecido estado de la base de datos"))?;
        }

        {
            thread::spawn(move || {
                for action in actions_rx {
                    match action {
                        StorageAction::ClientCommand { cmd, reply_tx } => {
                            let reply = storage_actor.process_command(cmd).unwrap();
                            let _ = reply_tx.send(reply);
                        }
                        StorageAction::DumpHistory { history_tx } => {
                            let mut file = OpenOptions::new()
                                .read(true)
                                .open(&append_file_path)
                                .unwrap();

                            history_tx.send(Self::load_history(&mut file)).unwrap();
                        }
                    }
                }
            });
        }

        {
            thread::spawn(move || {
                for cmd in commands_rx {
                    let bytes = Vec::from(Command::Storage(cmd));
                    if let Err(err) = Self::persist(&mut persistence_file, &bytes) {
                        logger_tx.send(log::error!("{err}")).unwrap();
                    }
                    replication_tx
                        .send(ReplicationAction::BroadcastCommand { bytes })
                        .unwrap();
                }
            });
        }

        Ok(())
    }

    fn load_history(file: &mut File) -> Vec<StorageCommand> {
        let mut history = Vec::new();
        let mut bytes = Vec::new();

        file.read_to_end(&mut bytes)
            .map_err(InternalError::PersistenceFileRead)
            .unwrap();

        let mut remaining_bytes = bytes.as_slice();

        while !remaining_bytes.is_empty() {
            let result = Array::parse_incremental(remaining_bytes)
                .map_err(InternalError::PersistenceFileFormat)
                .unwrap();

            let cmd = Command::try_from(result.0)
                .map_err(InternalError::PersistenceFileCommand)
                .unwrap();

            if let Command::Storage(cmd) = cmd {
                history.push(cmd);
            }

            remaining_bytes = result.1;
        }

        history
    }

    fn persist(file: &mut File, bytes: &[u8]) -> Result<(), InternalError> {
        file.write_all(bytes)
            .map_err(InternalError::PersistenceFileWrite)?;
        file.sync_all().map_err(InternalError::PersistenceFileWrite)
    }

    pub fn process_command(&mut self, cmd: StorageCommand) -> Result<Vec<u8>, InternalError> {
        let reply = self.apply(cmd.clone());
        self.persistence_tx.send(cmd).unwrap();
        Ok(reply)
    }

    pub fn apply(&mut self, cmd: StorageCommand) -> Vec<u8> {
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
}
