use std::{
    io::Write,
    net::TcpStream,
    sync::mpsc::{self, Sender},
    thread,
};

use log::Log;
use redis_cmd::Command;

use crate::server::node::storage::StorageAction;

#[derive(Debug)]
pub struct ReplicationActor {
    replication_streams: Vec<TcpStream>,
}

#[derive(Debug)]
pub enum ReplicationAction {
    BroadcastCommand { bytes: Vec<u8> },
    SyncReplica { client: TcpStream },
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
                    ReplicationAction::SyncReplica { mut client } => {
                        let (history_tx, history_rx) = mpsc::channel();

                        storage_tx
                            .send(StorageAction::DumpHistory { history_tx })
                            .unwrap();

                        if let Ok(history) = history_rx.recv() {
                            // TODO hacer un buffer de todos los comandos. el tema seria que el
                            // storage actor va a tener que saber procesar un bulk de comandos,
                            // pero al menos mejoramos la eficiencia del stream.

                            for cmd in history {
                                client.write_all(&Vec::from(Command::Storage(cmd))).unwrap();
                            }

                            log_tx
                                .send(log::cinfo!("restaurado historial de comandos en replica"))
                                .unwrap();
                        }

                        replication_actor.replication_streams.push(client);
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
