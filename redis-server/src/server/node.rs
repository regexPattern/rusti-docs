mod error;
mod pub_sub;
mod storage;

use std::io::Write;
use std::path::PathBuf;
use std::{
    io::{BufRead, BufReader},
    net::TcpStream,
    sync::mpsc::{self, Sender},
    thread,
};

pub use error::InternalError;
use log::LogMsg;
use pub_sub::{PubSubBroker, PubSubEnvelope};
use redis_cmd::Command;
use redis_resp::SimpleError;
use storage::{StorageActor, StorageEnvelope};

#[derive(Debug)]
pub struct Node {
    broker_tx: Sender<PubSubEnvelope>,
    storage_tx: Sender<StorageEnvelope>,
    logger_tx: Sender<LogMsg>,
}

impl Node {
    pub fn start(
        append_file_path: PathBuf,
        logger_tx: Sender<LogMsg>,
    ) -> Result<Self, InternalError> {
        let (broker_tx, broker_rx) = mpsc::channel();
        let (storage_tx, storage_rx) = mpsc::channel();

        let mut pub_sub_broker = PubSubBroker::start(logger_tx.clone());
        let mut storage_actor = StorageActor::start(append_file_path, logger_tx.clone())?;

        let logger_tx_clone = logger_tx.clone();

        thread::spawn(move || {
            while let Ok(envel) = broker_rx.recv() {
                if let Err(err) = pub_sub_broker.process(envel) {
                    logger_tx_clone.send(log::error!("{err}")).unwrap();
                    break;
                }
            }
        });

        thread::spawn(move || {
            while let Ok(envel) = storage_rx.recv() {
                storage_actor.process(envel).unwrap();
            }
        });

        Ok(Self {
            broker_tx,
            storage_tx,
            logger_tx,
        })
    }

    pub fn handle_client_conn(&self, client_conn: TcpStream) -> Result<(), InternalError> {
        let mut reader = BufReader::new(client_conn);

        let bytes = match reader.fill_buf() {
            Ok(bytes) if !bytes.is_empty() => bytes,
            Ok(_) => {
                self.logger_tx.send(log::debug!("cliente desconectado"))?;
                return Ok(());
            }
            Err(err) => {
                return Err(InternalError::StreamRead(err));
            }
        };

        self.logger_tx
            .send(log::debug!("recibidos {} bytes del cliente", bytes.len()))?;

        let cmd = Command::try_from(bytes);

        let length = bytes.len();
        reader.consume(length);

        let mut client_conn = reader.into_inner();

        match cmd {
            Ok(cmd) => {
                self.execute_command(client_conn, cmd)?;
                Ok(())
            }
            Err(err) => {
                client_conn
                    .write_all(&Vec::from(SimpleError::from(err)))
                    .map_err(InternalError::StreamWrite)?;

                Ok(())
            }
        }
    }

    fn execute_command(
        &self,
        mut client_conn: TcpStream,
        cmd: Command,
    ) -> Result<(), InternalError> {
        let (reply_tx, reply_rx) = mpsc::channel();

        match cmd {
            Command::Storage(cmd) => {
                self.storage_tx
                    .send(storage::StorageEnvelope { cmd, reply_tx })?;

                while let Ok(reply) = reply_rx.recv() {
                    let reply = match reply {
                        Ok(reply) => reply,
                        Err(_hash) => todo!("traer valor del nodo del cluster que tiene el hash"),
                    };

                    client_conn
                        .write_all(&reply)
                        .map_err(InternalError::StreamWrite)?;
                }
            }
            Command::PubSub(cmd) => {
                self.broker_tx
                    .send(pub_sub::PubSubEnvelope { client_conn, cmd })?;
            }
            Command::Cluster(_cmd) => todo!("implementar la ejecucion de comandos del cluster"),
        };

        Ok(())
    }
}
