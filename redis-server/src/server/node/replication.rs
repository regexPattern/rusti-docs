use std::{
    io::Write,
    net::TcpStream,
    sync::mpsc::{self, Sender},
    thread,
};

use log::Log;
use redis_cmd::{
    Command,
    connection::{ConnectionCommand, Ping},
};

use crate::server::node::storage::StorageAction;

#[derive(Debug)]
pub struct ReplicationActor {
    replication_streams: Vec<TcpStream>,
}

#[derive(Debug)]
pub enum ReplicationAction {
    BroadcastCommand { bytes: Vec<u8> },
    SyncReplica { stream: TcpStream },
}

impl ReplicationActor {
    pub fn start(
        storage_tx: Sender<StorageAction>,
        log_tx: Sender<Log>,
    ) -> Sender<ReplicationAction> {
        let mut replication_actor = ReplicationActor {
            replication_streams: Vec::new(),
        };

        let (actions_tx, actions_rx) = mpsc::channel();

        thread::spawn(move || {
            for action in actions_rx {
                match action {
                    ReplicationAction::BroadcastCommand { bytes } => {
                        replication_actor.broadcast_command(&bytes);
                    }
                    ReplicationAction::SyncReplica {
                        stream: mut replication_stream,
                    } => {
                        let (history_tx, history_rx) = mpsc::channel();

                        storage_tx
                            .send(StorageAction::DumpHistory { history_tx })
                            .unwrap();

                        if let Ok(history) = history_rx.recv() {
                            if history.is_empty() {
                                replication_stream
                                    .write_all(&Vec::from(Command::Connection(
                                        ConnectionCommand::Ping(Ping { message: None }),
                                    )))
                                    .unwrap();
                            } else {
                                for cmd in history {
                                    replication_stream
                                        .write_all(&Vec::from(Command::Storage(cmd)))
                                        .unwrap();
                                }
                            }

                            log_tx
                                .send(log::info!("replicado historial de comandos en replica"))
                                .unwrap();
                        }

                        replication_actor
                            .replication_streams
                            .push(replication_stream);
                    }
                }
            }
        });

        actions_tx
    }

    fn broadcast_command(&mut self, bytes: &[u8]) {
        for stream in &mut self.replication_streams {
            stream.write_all(bytes).unwrap();
        }
    }
}
