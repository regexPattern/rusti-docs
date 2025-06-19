mod cluster;
mod error;
mod pub_sub;
mod replication;
mod storage;

use std::io::{BufReader, Write};
use std::path::PathBuf;
use std::{
    io::BufRead,
    net::TcpStream,
    sync::mpsc::{self, Sender},
};

pub use error::InternalError;
use log::Log;
use pub_sub::{PubSubBroker, PubSubEnvelope};
use redis_cmd::Command;
use redis_cmd::connection::ConnectionCommand;
use redis_cmd::server::ServerCommand;
use redis_resp::{SimpleError, SimpleString};
use storage::StorageActor;

use crate::config::ClusterConfig;
use crate::server::node::cluster::message::payload::PublishPayload;
use crate::server::node::replication::ReplicationAction;

use crate::server::node::storage::StorageAction;
use cluster::{ClusterAction, ClusterActor};
use replication::ReplicationActor;

#[derive(Debug)]
pub struct Node {
    broker_tx: Sender<PubSubEnvelope>,
    storage_tx: Sender<storage::StorageAction>,
    replication_tx: Sender<ReplicationAction>,
    cluster_tx: Option<Sender<ClusterAction>>,
    pub_sub_tx: Sender<PublishPayload>,
    log_tx: Sender<Log>,
}

impl Node {
    pub fn start(append_file_path: PathBuf, log_tx: Sender<Log>) -> Result<Self, InternalError> {
        let (broker_tx, pub_sub_tx) = PubSubBroker::start(log_tx.clone());

        let (storage_tx, storage_rx) = mpsc::channel();

        let replication_tx = ReplicationActor::start(storage_tx.clone(), log_tx.clone());

        StorageActor::start(
            append_file_path,
            storage_rx,
            replication_tx.clone(),
            log_tx.clone(),
        )?;

        Ok(Self {
            broker_tx,
            storage_tx,
            replication_tx,
            cluster_tx: None,
            log_tx,
            pub_sub_tx: pub_sub_tx,
        })
    }

    pub fn enable_cluster_mode(&mut self, config: ClusterConfig) -> Result<(), ()> {
        let cluster_tx = ClusterActor::start(
            config,
            self.storage_tx.clone(),
            self.pub_sub_tx.clone(),
            self.log_tx.clone(),
        );

        self.cluster_tx = Some(cluster_tx);

        Ok(())
    }

    pub fn handle_client(&self, mut client: TcpStream) -> Result<(), InternalError> {
        let mut reader = BufReader::new(&mut client);

        let bytes = match reader.fill_buf() {
            Ok(bytes) if !bytes.is_empty() => bytes,
            Ok(_) => {
                self.log_tx.send(log::debug!("cliente desconectado"))?;
                return Ok(());
            }
            Err(err) => {
                return Err(InternalError::StreamRead(err));
            }
        };

        self.log_tx
            .send(log::debug!("recibidos {} bytes del cliente", bytes.len()))?;

        let cmd = Command::try_from(bytes);

        match cmd {
            Ok(cmd) => {
                self.log_tx.send(log::info!("procesando comando {cmd}"))?;
                self.execute_command(client, cmd)?;
                Ok(())
            }
            Err(err) => {
                self.log_tx
                    .send(log::info!("comando recibido es inválido {err}"))?;

                client
                    .write_all(&Vec::from(SimpleError::from(err)))
                    .map_err(InternalError::StreamWrite)?;

                Ok(())
            }
        }
    }

    fn execute_command(&self, mut client: TcpStream, cmd: Command) -> Result<(), InternalError> {
        let (reply_tx, reply_rx) = mpsc::channel();

        match cmd {
            Command::Storage(cmd) => {
                let (redir_tx, redir_rx) = mpsc::channel();

                if let Some(cluster_tx) = &self.cluster_tx {
                    cluster_tx.send(ClusterAction::RedirectToHoldingNode {
                        key: cmd.key().to_owned(),
                        redir_tx,
                    })?;

                    if let Ok(redir) = redir_rx.recv() {
                        if let Some(moved_err) = redir {
                            self.log_tx
                                .send(log::info!("redirigiendo cliente a nodo correspondiente"))
                                .unwrap();

                            client
                                .write_all(&moved_err)
                                .map_err(InternalError::StreamWrite)?;

                            return Ok(());
                        }
                    }
                }

                self.storage_tx
                    .send(StorageAction::ClientCommand { cmd, reply_tx })
                    .unwrap();

                if let Ok(reply) = reply_rx.recv() {
                    client
                        .write_all(&reply)
                        .map_err(InternalError::StreamWrite)?;
                }
            }
            Command::PubSub(cmd) => {
                self.broker_tx.send(PubSubEnvelope {
                    client: client.try_clone().unwrap(),
                    cmd,
                    cluster_tx: self.cluster_tx.clone().unwrap(),
                })?;
            }
            Command::Cluster(cmd) => {
                if let Some(cluster_tx) = &self.cluster_tx {
                    cluster_tx.send(ClusterAction::ClientCommand { cmd, reply_tx })?;

                    if let Ok(reply) = reply_rx.recv() {
                        client
                            .write_all(&reply)
                            .map_err(InternalError::StreamWrite)?;
                    }
                } else {
                    self.log_tx.send(log::warn!(
                        "recibido comando de cluster sin tener modo cluster activado"
                    ))?;
                }
            }
            Command::Server(cmd) => match cmd {
                ServerCommand::Sync(_) => {
                    self.replication_tx
                        .send(ReplicationAction::SyncReplica { stream: client })
                        .unwrap();
                }
            },
            Command::Connection(cmd) => match cmd {
                ConnectionCommand::Auth(_cmd) => todo!(),
                ConnectionCommand::Ping(cmd) => {
                    let reply = if let Some(message) = cmd.message {
                        Vec::from(message)
                    } else {
                        Vec::from(SimpleString::from("PONG"))
                    };

                    client
                        .write_all(&reply)
                        .map_err(InternalError::StreamWrite)?;
                }
            },
        };

        Ok(())
    }
}
