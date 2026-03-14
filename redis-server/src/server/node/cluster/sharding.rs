use std::{net::SocketAddr, sync::mpsc::Sender};

use crc16::{State, XMODEM};
use log::Log;
use redis_resp::{BulkString, SimpleError};

use super::{CLUSTER_SLOTS, ClusterActor, flags};

impl ClusterActor {
    /// Calcula el slot de hash para una clave usando CRC16 y el total de slots del clúster.
    /// Permite determinar a qué nodo debe dirigirse una operación para esa clave.
    pub fn get_key_slot(key: &BulkString) -> u16 {
        let key: &str = key.into();
        let hash = State::<XMODEM>::calculate(key.as_bytes());
        hash % CLUSTER_SLOTS
    }

    /// Redirige la operación al nodo que posee el slot correspondiente a la clave.
    /// Si el slot no está en el nodo actual, envía un error MOVED o UNSETSLOT al cliente.
    pub fn redirect_to_holding_node(
        &self,
        key: &BulkString,
        redir_tx: Sender<Option<Vec<u8>>>,
        log_tx: &Sender<Log>,
    ) {
        let slot = Self::get_key_slot(key);

        if !(self.myself.slots.0..=self.myself.slots.1).contains(&slot) {
            let redir_addr = if let Some(redir_addr) = self.get_redirect_address(slot) {
                redir_addr
            } else {
                let _ = log_tx.send(log::warn!("no se conocen nodos con slot {slot} asignado"));
                let reply = SimpleError::from(format!(
                    "-UNSETSLOT slot {slot} no se ha configurado en cluster"
                ));
                redir_tx.send(Some(Vec::from(reply))).unwrap();
                return;
            };

            let reply = SimpleError::from(format!("-MOVED {slot} {redir_addr:?}"));
            let _ = redir_tx.send(Some(Vec::from(reply)));
            return;
        }

        let _ = redir_tx.send(None);
    }

    fn get_redirect_address(&self, slot: u16) -> Option<SocketAddr> {
        for node_id in self.cluster_streams.keys() {
            if let Some(node) = self.cluster_view.get(node_id)
                && node.flags.contains(flags::FLAG_MASTER)
                && !node.flags.contains(flags::FLAG_FAIL)
                && (node.slots.0..=node.slots.1).contains(&slot)
            {
                return Some(SocketAddr::new(node.ip.into(), node.port));
            }
        }
        None
    }
}
