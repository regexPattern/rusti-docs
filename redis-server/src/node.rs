mod error;
mod pub_sub;

use std::io::Write;
use std::path::PathBuf;
use std::{
    io::{BufRead, BufReader},
    net::TcpStream,
    sync::mpsc::{self, Sender},
    thread,
};

use error::Error;
use log::LogMsg;
use pub_sub::PubSubBroker;
use redis_cmd::Command;
use redis_resp::SimpleError;

use crate::storage::{self, StorageActor};

#[derive(Debug)]
pub struct Node {
    broker_tx: Sender<pub_sub::PubSubEnvelope>,
    storage_tx: Sender<storage::StorageEnvelope>,
    logger_tx: Sender<LogMsg>,
}

impl Node {
    pub fn start(append_file_path: PathBuf, logger_tx: Sender<LogMsg>) -> Result<Self, Error> {
        let (broker_tx, broker_rx) = mpsc::channel::<pub_sub::PubSubEnvelope>();
        let (storage_tx, storage_rx) = mpsc::channel::<storage::StorageEnvelope>();

        let mut pub_sub_broker = PubSubBroker::start(logger_tx.clone());
        let mut storage_actor = StorageActor::start(append_file_path, logger_tx.clone()).unwrap();

        thread::spawn(move || {
            while let Ok(envel) = broker_rx.recv() {
                pub_sub_broker.process(envel).unwrap();
            }
        });

        thread::spawn(move || {
            while let Ok(envel) = storage_rx.recv() {
                storage_actor.process(envel);
            }
        });

        Ok(Self {
            broker_tx,
            storage_tx,
            logger_tx,
        })
    }

    pub fn handle_client_conn(&self, client_conn: TcpStream) -> Result<(), Error> {
        let mut reader = BufReader::new(client_conn);

        let bytes = match reader.fill_buf() {
            Ok(bytes) if !bytes.is_empty() => bytes,
            Ok(_) => todo!(),
            Err(_) => todo!(),
        };

        self.logger_tx
            .send(log::debug!("recibidos {} bytes", bytes.len()))?;

        let cmd = Command::try_from(bytes);

        let length = bytes.len();
        reader.consume(length);

        let mut client_conn = reader.into_inner();

        match cmd {
            Ok(cmd) => {
                self.execute_command(client_conn, cmd).unwrap();
                Ok(())
            }
            Err(err) => {
                client_conn
                    .write_all(&Vec::from(SimpleError::from(err)))
                    .unwrap();
                Ok(())
            }
        }
    }

    fn execute_command(&self, mut client_conn: TcpStream, cmd: Command) -> Result<(), ()> {
        let (reply_tx, reply_rx) = mpsc::channel();

        match cmd {
            Command::Storage(cmd) => {
                self.storage_tx
                    .send(storage::StorageEnvelope { cmd, reply_tx })
                    .unwrap();

                while let Ok(reply) = reply_rx.recv() {
                    let reply = match reply {
                        Ok(reply) => reply,
                        Err(_hash) => todo!("TRAER VALOR DEL NODO QUE TIENE EL HASH"),
                    };

                    client_conn.write_all(&reply).unwrap();
                }
            }
            Command::PubSub(cmd) => {
                self.broker_tx
                    .send(pub_sub::PubSubEnvelope { client_conn, cmd })
                    .unwrap();
            }
            Command::Cluster(_command) => todo!(),
        };

        Ok(())
    }
}
