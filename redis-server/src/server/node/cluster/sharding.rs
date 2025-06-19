use std::{net::SocketAddr, sync::mpsc::Sender};

use crc16::{State, XMODEM};
use redis_resp::{BulkString, SimpleError};

use super::{CLUSTER_SLOTS, ClusterActor};

impl ClusterActor {
    pub fn get_key_slot(key: &BulkString) -> u16 {
        let key: &str = key.into();
        let hash = State::<XMODEM>::calculate(key.as_bytes());
        hash % CLUSTER_SLOTS
    }

    pub fn redirect_to_holding_node(&self, key: &BulkString, redir_tx: Sender<Option<Vec<u8>>>) {
        let slot = Self::get_key_slot(key);

        if !(self.myself.slots.0..=self.myself.slots.1).contains(&slot) {
            let redir_addr = self.get_redirect_address(slot).unwrap();
            let reply = SimpleError::from(format!("-MOVED {slot} {redir_addr:?}"));
            redir_tx.send(Some(Vec::from(reply))).unwrap();
            return;
        }

        redir_tx.send(None).unwrap();
    }

    fn get_redirect_address(&self, slot: u16) -> Option<SocketAddr> {
        for node_id in self.cluster_streams.keys() {
            if let Some(node) = self.cluster_view.get(node_id) {
                if (node.slots.0..=node.slots.1).contains(&slot) {
                    return Some(SocketAddr::new(node.ip.into(), node.port.into()));
                }
            }
        }
        None
    }
}
