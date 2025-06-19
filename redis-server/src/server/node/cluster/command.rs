use std::{
    io::{BufRead, BufReader, Write},
    net::{Ipv4Addr, Shutdown, SocketAddr, TcpStream},
    sync::mpsc::{self, Sender},
    thread,
};

use log::Log;
use redis_cmd::{
    Command,
    cluster::{AddSlots, ClusterCommand, Meet, Replicate},
    server::{ServerCommand, Sync},
};
use redis_resp::{BulkString, Integer, SimpleError, SimpleString};

use crate::server::node::storage::StorageAction;

use super::ClusterNode;

use super::{
    ClusterActor, flags,
    message::{
        Message, MessageHeader, MessagePayload,
        payload::{GossipKind, GossipPayload},
    },
};

impl ClusterActor {
    pub fn handle_cmd(
        &mut self,
        cmd: ClusterCommand,
        client_tx: Sender<Vec<u8>>,
        log_tx: &Sender<Log>,
    ) {
        let reply = match cmd {
            ClusterCommand::Meet(meet) => self.meet(meet, log_tx),
            ClusterCommand::Nodes(_) => self.nodes(),
            ClusterCommand::AddSlots(add_slots) => self.add_slots(add_slots, log_tx),
            ClusterCommand::AddSlotsRange(add_slots_range) => {
                self.add_slots_range(add_slots_range.start, add_slots_range.end, log_tx)
            }
            ClusterCommand::Replicate(replicate) => self.replicate(replicate, log_tx),
            ClusterCommand::KeySlot(key_slot) => Self::key_slot(&key_slot.key),
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

        if self
            .cluster_streams
            .values()
            .filter(|s| s.peer_addr().unwrap() == addr)
            .count()
            > 0
        {
            log_tx
                .send(log::warn!("ya existe conexión el nodo en {addr:?}"))
                .unwrap();
        }

        let mut stream = match TcpStream::connect(addr) {
            Ok(stream) => stream,
            Err(err) => {
                log_tx.send(log::error!("{err}")).unwrap();
                return SimpleError::from(format!("error contactando con nodo en {addr:?}")).into();
            }
        };

        let msg = Message {
            header: MessageHeader::from(&self.myself),
            payload: MessagePayload::Gossip(GossipPayload {
                kind: GossipKind::Meet,
                nodes: self.select_random_nodes(),
            }),
        };

        if let Err(err) = stream.write_all(&Vec::from(&msg)) {
            log_tx.send(log::error!("{err}")).unwrap();
            SimpleError::from(format!("error enviando MEET al nodo en {addr:?}")).into()
        } else {
            SimpleString::from("OK").into()
        }
    }

    fn nodes(&self) -> Vec<u8> {
        let mut nodes = Vec::with_capacity(self.cluster_view.len());

        nodes.push(format!("{}", self.myself));

        for node in self.cluster_view.values() {
            nodes.push(format!("{node}"));
        }

        BulkString::from(nodes.join("\n")).into()
    }

    fn add_slots(&mut self, cmd: AddSlots, log_tx: &Sender<Log>) -> Vec<u8> {
        let mut slots = vec![cmd.slot];
        slots.extend(cmd.slots);

        let start: u16 = slots[0].to_string().parse().unwrap();
        let end: u16 = slots[slots.len() - 1].to_string().parse().unwrap();

        self.myself.slots = (start, end);

        log_tx
            .send(log::info!("asignados {} slots", slots.len()))
            .unwrap();

        SimpleString::from("OK").into()
    }

    fn add_slots_range(
        &mut self,
        start: BulkString,
        end: BulkString,
        log_tx: &Sender<Log>,
    ) -> Vec<u8> {
        let start: u16 = start.to_string().parse().unwrap();
        let end: u16 = end.to_string().parse().unwrap();

        if start > end {
            return SimpleError::from("rango de slots inválido").into();
        }

        self.myself.slots = (start, end);

        log_tx
            .send(log::info!("asignados slots del {start} al {end}"))
            .unwrap();

        SimpleString::from("OK").into()
    }

    // fn replicate(&mut self, cmd: Replicate, log_tx: &Sender<Log>) -> Vec<u8> {
    //     if let Some(stream) = self.replication_link.take() {
    //         log_tx
    //             .send(log::info!("cerrado replication stream con master anterior"))
    //             .unwrap();
    //         stream.shutdown(Shutdown::Both).unwrap();
    //     }
    //
    //     let mut master_id = [0u8; 20];
    //     hex::decode_to_slice(&cmd.node_id.to_string(), &mut master_id).unwrap();
    //
    //     let master = match self.cluster_view.get(&master_id) {
    //         Some(master) => master,
    //         None => {
    //             return SimpleError::from(format!("nodo {} no conocido", hex::encode(master_id)))
    //                 .into();
    //         }
    //     };
    //
    //     // TODO hacer que esto retorne un error si ya conozco al nodo pero no puedo contactarme con
    //     // el.
    //     self.connect_replication_stream(master, self.storage_tx.clone(), log_tx.clone());
    //
    //     self.myself.master_id = Some(master_id);
    //     self.myself.flags.0 &= !flags::FLAG_MASTER;
    //     self.myself.flags.0 |= flags::FLAG_SLAVE;
    //     self.myself.slots = master.slots;
    //
    //     log_tx
    //         .send(log::info!(
    //             "configurado nodo como replica de {}",
    //             hex::encode(master_id)
    //         ))
    //         .unwrap();
    //
    //     SimpleString::from("OK").into()
    // }

    fn connect_replication_stream(
        &self,
        master: &ClusterNode,
        storage_tx: Sender<StorageAction>,
        log_tx: Sender<Log>,
    ) {
        let master_id = hex::encode(master.id);

        let mut stream =
            TcpStream::connect(SocketAddr::new(master.ip.into(), master.port)).unwrap();

        thread::spawn(move || {
            let bytes = Vec::from(Command::Server(ServerCommand::Sync(Sync)));
            stream.write_all(&bytes).unwrap();

            let mut is_first_sync = false;

            loop {
                let mut reader = BufReader::new(&mut stream);
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

                        if !is_first_sync {
                            log_tx
                                .send(log::info!("replicado historial de comandos de master"))
                                .unwrap();

                            is_first_sync = true;
                        }
                    }
                    Ok(_) => {
                        log_tx
                            .send(log::warn!(
                                "replication stream con nodo {master_id} cerrado"
                            ))
                            .unwrap();
                        return;
                    }
                    Err(_) => todo!(),
                }
            }
        });
    }

    fn key_slot(key: &BulkString) -> Vec<u8> {
        let slot = Self::get_key_slot(key);
        Integer::from(slot as i64).into()
    }
}
