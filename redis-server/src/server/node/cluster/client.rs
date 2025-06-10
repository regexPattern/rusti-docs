use std::{
    io::{BufRead, BufReader, Write},
    net::{Ipv4Addr, Shutdown, SocketAddr, TcpStream},
    sync::mpsc::{self, Sender},
    thread,
};

use log::Log;
use redis_cmd::{
    Command,
    cluster::{ClusterCommand, Meet, Replicate},
    management::{ManagementCommand, Sync},
};
use redis_resp::{BulkString, SimpleString};

use crate::server::node::{
    cluster::flags::{FLAG_MASTER, FLAG_SLAVE},
    storage::StorageAction,
};

use super::{
    ClusterState,
    message::{
        Message, MessageData, MessageHeader,
        gossip::{GossipData, GossipKind},
    },
};

impl ClusterState {
    pub fn process_command(
        &mut self,
        cmd: ClusterCommand,
        client_tx: Sender<Vec<u8>>,
        log_tx: &Sender<Log>,
    ) {
        let reply = match cmd {
            ClusterCommand::FailOver(_cmd) => todo!(),
            ClusterCommand::Info(_cmd) => todo!(),
            ClusterCommand::Meet(cmd) => self.meet(cmd, log_tx),
            ClusterCommand::Nodes(_) => self.nodes(),
            ClusterCommand::Shards(_cmd) => todo!(),
            ClusterCommand::AddSlots(_cmd) => todo!(),
            ClusterCommand::DelSlots(_cmd) => todo!(),
            ClusterCommand::GetKeysInSlot(_cmd) => todo!(),
            ClusterCommand::SetSlot(_cmd) => todo!(),
            ClusterCommand::Replicate(cmd) => self.replicate(cmd, log_tx),
        };

        client_tx.send(reply).unwrap();
    }

    fn meet(&mut self, cmd: Meet, log_tx: &Sender<Log>) -> Vec<u8> {
        let ip: Ipv4Addr = cmd.ip.to_string().parse().unwrap();

        let port = if let Some(cluster_port) = cmd.cluster_port {
            cluster_port.parse().unwrap()
        } else {
            cmd.port.parse::<u16>().unwrap() + 10000
        };

        let addr = SocketAddr::new(ip.into(), port);
        let mut stream = TcpStream::connect(addr).unwrap();

        log_tx
            .send(log::info!("establecido handshake con el nodo {addr:?}",))
            .unwrap();

        let msg = Message {
            header: MessageHeader::from(&self.myself),
            data: MessageData::Gossip(GossipData {
                kind: GossipKind::Meet,
                nodes: self.select_random_nodes(),
            }),
        };

        stream.write_all(&Vec::from(&msg)).unwrap();

        SimpleString::from("OK").into()
    }

    fn nodes(&self) -> Vec<u8> {
        let mut nodes = Vec::with_capacity(self.cluster_view.len());

        for node in self.cluster_view.values() {
            nodes.push(format!("{node:?}"));
        }

        BulkString::from(nodes.join("\n")).into()
    }

    fn replicate(&mut self, cmd: Replicate, log_tx: &Sender<Log>) -> Vec<u8> {
        if let Some(stream) = self.replication_stream.take() {
            stream.shutdown(Shutdown::Both).unwrap();
        }

        let node_id = cmd.node_id.to_string();

        let mut master_id = [0u8; 20];
        hex::decode_to_slice(&node_id, &mut master_id).unwrap();

        self.myself.master_id = Some(master_id);

        self.myself.flags.0 &= !FLAG_MASTER;
        self.myself.flags.0 |= FLAG_SLAVE;

        log_tx
            .send(log::info!("configurado nodo como replica de {node_id}"))
            .unwrap();

        // TODO lo que vamos a hacer es que nuestra implementacion no va a soportar que se pueda
        // asignar un nodo como master si el nodo en cuestion no conoce al master todavia.

        let master = self.cluster_view.get(&master_id).unwrap();
        let mut stream =
            TcpStream::connect(SocketAddr::new(master.ip.into(), master.port)).unwrap();

        let storage_tx = self.storage_tx.clone();

        let log_tx = log_tx.clone();

        thread::spawn(move || {
            let bytes = Vec::from(Command::Management(ManagementCommand::Sync(Sync)));
            stream.write_all(&bytes).unwrap();

            let mut full_sync_done = false;

            loop {
                let mut reader = BufReader::new(&mut stream);
                match reader.fill_buf() {
                    Ok(bytes) if !bytes.is_empty() => {
                        let cmd = Command::try_from(bytes).unwrap();

                        if let Command::Storage(cmd) = cmd {
                            let (reply_tx, reply_rx) = mpsc::channel();

                            storage_tx
                                .send(StorageAction::ClientCommand {
                                    cmd,
                                    client_tx: reply_tx,
                                })
                                .unwrap();

                            reply_rx.recv().unwrap();

                            if !full_sync_done {
                                log_tx
                                    .send(log::info!(
                                        "replicado información completa de nodo {node_id}",
                                    ))
                                    .unwrap();

                                full_sync_done = true;
                            }
                        }
                    }
                    Ok(_) => {
                        log_tx
                            .send(log::warn!("replicatoin link con nodo {node_id} cerrado"))
                            .unwrap();
                        return;
                    }
                    Err(_) => todo!(),
                }
            }
        });

        SimpleString::from("OK").into()
    }
}
