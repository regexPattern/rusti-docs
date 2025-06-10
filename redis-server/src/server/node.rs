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
use redis_cmd::management::ManagementCommand;
use redis_resp::SimpleError;
use storage::StorageActor;

use crate::config::ClusterConfig;
use crate::server::node::cluster::message::PublishData;
use crate::server::node::replication::ReplicationAction;

// use crate::server::node::pub_sub::PubSubAction;

use crate::server::node::storage::StorageAction;
use cluster::{ClusterAction, ClusterState};
use replication::ReplicationActor;

#[derive(Debug)]
pub struct Node {
    broker_tx: Sender<PubSubEnvelope>,
    storage_tx: Sender<storage::StorageAction>,
    replication_tx: Sender<ReplicationAction>,
    cluster_tx: Option<Sender<ClusterAction>>,
    log_tx: Sender<Log>,
    pub_sub_tx: Sender<PublishData>,
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
        let cluster_tx = ClusterState::start(config, self.storage_tx.clone(),
        self.pub_sub_tx.clone(), self.log_tx.clone());
        
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
        let (client_tx, client_rx) = mpsc::channel();

        match cmd {
            Command::Storage(cmd) => {
                self.storage_tx
                    .send(StorageAction::ClientCommand { cmd, client_tx })
                    .unwrap();

                if let Ok(reply) = client_rx.recv() {
                    client
                        .write_all(&reply)
                        .map_err(InternalError::StreamWrite)?;
                }
            }
            Command::PubSub(cmd) => {
                self.broker_tx.send(PubSubEnvelope { client: client.try_clone().unwrap(), cmd, cluster_tx: self.cluster_tx.clone().unwrap() })?;

            }
            Command::Cluster(cmd) => {
                if let Some(cluster_tx) = &self.cluster_tx {
                    cluster_tx.send(ClusterAction::ClientCommand { cmd, client_tx })?;

                    if let Ok(reply) = client_rx.recv() {
                        client
                            .write_all(&reply)
                            .map_err(InternalError::StreamWrite)?;
                    }
                } else {
                    self.log_tx
                        .send(log::warn!(
                            "recibido comando de cluster sin tener modo cluster activado"
                        ))
                        .unwrap();
                }
            }
            Command::Management(cmd) => match cmd {
                ManagementCommand::Sync(_) => {
                    self.replication_tx
                        .send(ReplicationAction::SyncReplica { client })
                        .unwrap();
                }
            },
        };

        Ok(())
    }
}
