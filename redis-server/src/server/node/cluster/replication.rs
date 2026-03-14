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

use super::{ClusterActor, ClusterNode, NodeId, flags};
use crate::server::node::{cluster::InternalError, storage::StorageAction};

impl ClusterActor {
    /// Configura el nodo actual como réplica de un nuevo master.
    /// Realiza validaciones y conecta el enlace de replicación.
    /// Devuelve un error si el master no es válido o no se puede conectar.
    pub fn replicate(&mut self, cmd: Replicate, log_tx: &Sender<Log>) -> Vec<u8> {
        let mut new_master_id = [0u8; 20];
        hex::decode_to_slice(cmd.node_id.to_string(), &mut new_master_id).unwrap();

        if new_master_id == self.myself.id {
            let err_msg = "nodo no puede ser réplica de si mismo";
            return SimpleError::from(err_msg).into();
        } else if let Some(curr_master_id) = self.myself.master_id
            && curr_master_id == new_master_id
        {
            let err_msg = format!("nodo ya es réplica de nodo {}", cmd.node_id);
            return SimpleError::from(err_msg).into();
        }

        match self.set_master(new_master_id, log_tx) {
            Ok(None) => {
                let err_msg = format!("nodo {} no conocido", hex::encode(new_master_id));
                SimpleError::from(err_msg).into()
            }
            Ok(_) => SimpleString::from("OK").into(),
            Err(err) => SimpleError::from(format!("{err}")).into(),
        }
    }

    /// Cambia el master del nodo y actualiza los flags y slots.
    /// Devuelve el id del nuevo master si la operación es exitosa.
    pub fn set_master(
        &mut self,
        master_id: NodeId,
        log_tx: &Sender<Log>,
    ) -> Result<Option<NodeId>, InternalError> {
        let master = match self.cluster_view.get(&master_id) {
            Some(new_master) => new_master,
            None => return Ok(None),
        };

        self.connect_replication_link(master, log_tx.clone())?;

        self.myself.master_id = Some(master.id);
        self.myself.flags.0 &= !flags::FLAG_MASTER;
        self.myself.flags.0 |= flags::FLAG_SLAVE;
        self.myself.slots = master.slots;

        let _ = log_tx.send(log::info!(
            "configurado como replica de nodo {} con slots {}-{}",
            hex::encode(master.id),
            master.slots.0,
            master.slots.1
        ));

        Ok(Some(master.id))
    }

    fn connect_replication_link(
        &self,
        master: &ClusterNode,
        log_tx: Sender<Log>,
    ) -> Result<(), InternalError> {
        let master_id = hex::encode(master.id);
        let storage_tx = self.storage_tx.clone();

        let mut replication_link =
            TcpStream::connect(SocketAddr::new(master.ip.into(), master.port))
                .map_err(InternalError::ReplicationLinkConnect)?;

        thread::spawn(move || {
            let cmd = Vec::from(Command::Server(ServerCommand::Sync(Sync)));

            if let Err(err) = replication_link.write_all(&cmd) {
                let _ = log_tx.send(log::error!("error enviando SYNC a nodo {master_id}: {err}"));
                return;
            }

            let _ = log_tx.send(log::debug!("mandado conmando SYNC a nodo {master_id}"));

            let mut sync_done = false;

            loop {
                let mut reader = BufReader::new(&mut replication_link);

                match reader.fill_buf() {
                    Ok(bytes) if !bytes.is_empty() => {
                        let cmd = Command::try_from(bytes).unwrap();

                        if let Command::Storage(cmd) = cmd {
                            let (reply_tx, reply_rx) = mpsc::channel();
                            let _ = storage_tx.send(StorageAction::ClientCommand { cmd, reply_tx });
                            let _ = reply_rx.recv();
                        }

                        if !sync_done {
                            let _ = log_tx
                                .send(log::info!("recibido historial de comandos desde master"));
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
