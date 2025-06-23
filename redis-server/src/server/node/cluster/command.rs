use std::{
    io::Write,
    net::{Ipv4Addr, SocketAddr, TcpStream},
    sync::mpsc::Sender,
};

use log::Log;
use redis_cmd::cluster::{AddSlots, ClusterCommand, Meet};
use redis_resp::{BulkString, Integer, SimpleError, SimpleString};

use super::{
    ClusterActor,
    message::{
        Message, MessageHeader, MessagePayload,
        payload::{GossipKind, GossipPayload},
    },
};

impl ClusterActor {
    /// Maneja un comando de clúster recibido y envía la respuesta al cliente.
    /// Hace el Dispatch según el tipo de comando: nodos, slots, replicación, etc.
    /// Si ocurre un error al responder, lo registra en el log.
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
            ClusterCommand::MyId(_) => self.my_id(),
        };

        if client_tx.send(reply).is_err() {
            let _ = log_tx.send(log::error!("no se pudo responder al cliente"));
        }
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
        let mut nodes: Vec<_> = self.cluster_view.values().collect();
        let mut output = Vec::with_capacity(nodes.len() + 1);

        nodes.push(&self.myself);

        nodes.sort_by_key(|n| n.port);

        for node in nodes {
            output.push(format!("{node}"));
        }

        BulkString::from(output.join("\n")).into()
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

    fn my_id(&self) -> Vec<u8> {
        BulkString::from(hex::encode(self.myself.id)).into()
    }

    fn key_slot(key: &BulkString) -> Vec<u8> {
        let slot = Self::get_key_slot(key);
        Integer::from(slot as i64).into()
    }
}
