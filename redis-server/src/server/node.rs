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
use rustls::{ServerConnection, StreamOwned};
use storage::StorageActor;

use crate::config::ClusterConfig;
use crate::server::node::cluster::message::payload::PublishPayload;
use crate::server::node::replication::ReplicationAction;

use crate::server::node::storage::StorageAction;
use cluster::{ClusterAction, ClusterActor};
use replication::ReplicationActor;

#[derive(Debug)]
pub struct Node {
    pub_sub_tx: Sender<PubSubEnvelope>,
    storage_tx: Sender<storage::StorageAction>,
    replication_tx: Sender<ReplicationAction>,
    cluster_tx: Option<Sender<ClusterAction>>,
    cluster_pub_sub_tx: Sender<PublishPayload>,
    log_tx: Sender<Log>,
}

impl Node {
    pub fn start(append_file_path: PathBuf, log_tx: Sender<Log>) -> Result<Self, InternalError> {
        let (pub_sub_tx, cluster_pub_sub_tx) = PubSubBroker::start(log_tx.clone());

        let (storage_tx, storage_rx) = mpsc::channel();

        let replication_tx = ReplicationActor::start(storage_tx.clone(), log_tx.clone());

        StorageActor::start(
            append_file_path,
            storage_rx,
            replication_tx.clone(),
            log_tx.clone(),
        )?;

        Ok(Self {
            pub_sub_tx,
            storage_tx,
            replication_tx,
            cluster_tx: None,
            log_tx,
            cluster_pub_sub_tx,
        })
    }

    pub fn enable_cluster_mode(&mut self, config: ClusterConfig) -> Result<(), ()> {
        let cluster_tx = ClusterActor::start(
            config,
            self.storage_tx.clone(),
            self.cluster_pub_sub_tx.clone(),
            self.log_tx.clone(),
        );

        self.cluster_tx = Some(cluster_tx);

        Ok(())
    }

    pub fn handle_client(
        &self,
        mut stream: StreamOwned<ServerConnection, TcpStream>,
    ) -> Result<(), InternalError> {
        let mut reader = BufReader::new(&mut stream);

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
                self.execute_command(stream, cmd)?;
                Ok(())
            }
            Err(err) => {
                self.log_tx
                    .send(log::info!("comando recibido es inválido {err}"))?;

                stream
                    .write_all(&Vec::from(SimpleError::from(err)))
                    .map_err(InternalError::StreamWrite)?;

                Ok(())
            }
        }
    }

    fn execute_command(
        &self,
        mut stream: StreamOwned<ServerConnection, TcpStream>,
        cmd: Command,
    ) -> Result<(), InternalError> {
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

                            stream
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
                    stream
                        .write_all(&reply)
                        .map_err(InternalError::StreamWrite)?;
                }
            }
            Command::PubSub(cmd) => {
                self.pub_sub_tx.send(PubSubEnvelope {
                    stream,
                    cmd,
                    cluster_tx: self.cluster_tx.clone().unwrap(),
                })?;
            }
            Command::Cluster(cmd) => {
                if let Some(cluster_tx) = &self.cluster_tx {
                    cluster_tx.send(ClusterAction::ClientCommand { cmd, reply_tx })?;

                    if let Ok(reply) = reply_rx.recv() {
                        stream
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
                        .send(ReplicationAction::SyncReplica {
                            stream: stream.into_parts().1,
                        })
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

                    stream
                        .write_all(&reply)
                        .map_err(InternalError::StreamWrite)?;
                }
            },
        };

        Ok(())
    }
}
