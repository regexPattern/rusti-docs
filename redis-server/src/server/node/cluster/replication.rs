use std::{
    io::{BufRead, BufReader, Write},
    net::{SocketAddr, TcpStream},
    sync::mpsc::{self, Sender},
    thread,
};

use log::Log;
use redis_cmd::{
    Command,
    cluster::Replicate,
    server::{ServerCommand, Sync},
};
use redis_resp::{SimpleError, SimpleString};

use crate::server::node::storage::StorageAction;

use super::{ClusterActor, ClusterNode, NodeId, flags};

impl ClusterActor {
    pub fn replicate(&mut self, cmd: Replicate, log_tx: &Sender<Log>) -> Vec<u8> {
        let mut new_master_id = [0u8; 20];
        hex::decode_to_slice(&cmd.node_id.to_string(), &mut new_master_id).unwrap();

        if new_master_id == self.myself.id {
            let err_msg = "nodo no puede ser réplica de si mismo";
            return SimpleError::from(err_msg).into();
        } else if let Some(curr_master_id) = self.myself.master_id {
            if curr_master_id == new_master_id {
                let err_msg = format!("nodo ya es réplica de nodo {}", cmd.node_id);
                return SimpleError::from(err_msg).into();
            }
        }

        match self.set_master(new_master_id, log_tx) {
            Ok(None) => {
                let err_msg = format!("nodo {} no conocido", hex::encode(new_master_id));
                SimpleError::from(err_msg).into()
            }
            Ok(_) => SimpleString::from("OK").into(),
            Err(_) => todo!(),
        }
    }

    pub fn set_master(
        &mut self,
        master_id: NodeId,
        log_tx: &Sender<Log>,
    ) -> Result<Option<NodeId>, ()> {
        let master = match self.cluster_view.get(&master_id) {
            Some(new_master) => new_master,
            None => return Ok(None),
        };

        log_tx
            .send(log::info!("cerrado replication link con master anterior"))
            .unwrap();

        self.connect_replication_link(master, log_tx.clone())
            .unwrap();

        self.myself.master_id = Some(master.id);
        self.myself.flags.0 &= !flags::FLAG_MASTER;
        self.myself.flags.0 |= flags::FLAG_SLAVE;
        self.myself.slots = master.slots;

        Ok(Some(master.id))
    }

    fn connect_replication_link(
        &self,
        master: &ClusterNode,
        log_tx: Sender<Log>,
    ) -> Result<(), ()> {
        let master_id = hex::encode(master.id);
        let storage_tx = self.storage_tx.clone();

        let mut replication_link =
            TcpStream::connect(SocketAddr::new(master.ip.into(), master.port)).unwrap();

        thread::spawn(move || {
            let cmd = Vec::from(Command::Server(ServerCommand::Sync(Sync)));

            replication_link.write_all(&cmd).unwrap();

            log_tx
                .send(log::debug!("mandado conmando SYNC a nodo {master_id}"))
                .unwrap();

            let mut sync_done = false;

            loop {
                let mut reader = BufReader::new(&mut replication_link);

                match reader.fill_buf() {
                    Ok(bytes) if !bytes.is_empty() => {
                        let cmd = Command::try_from(bytes).unwrap();

                        if let Command::Storage(cmd) = cmd {
                            let (reply_tx, reply_rx) = mpsc::channel();

                            storage_tx
                                .send(StorageAction::ClientCommand { cmd, reply_tx })
                                .unwrap();

                            let _ = reply_rx.recv();
                        }

                        if !sync_done {
                            log_tx
                                .send(log::info!("replicado historial de comandos de master"))
                                .unwrap();

                            sync_done = true;
                        }
                    }
                    Ok(_) => {
                        let _ = log_tx
                            .send(log::warn!("replication link con nodo {master_id} cerrado"));
                        return;
                    }
                    Err(err) => {
                        let _ = log_tx.send(log::error!(
                            "error en replication link con nodo {master_id}: {err}"
                        ));
                        return;
                    }
                }
            }
        });

        Ok(())
    }
}
