use std::{io::prelude::*, sync::mpsc::Sender};

use log::Log;

use crate::server::node::cluster::NodeId;

use super::{
    ClusterActor, flags,
    message::{
        Message, MessageHeader, MessagePayload,
        payload::{FailOverKind, FailOverPayload},
    },
};

impl ClusterActor {
    pub fn request_failover(&mut self) {
        let master_id = match self.myself.master_id {
            Some(master_id) => master_id,
            None => return,
        };

        let master = self.cluster_view.get(&master_id).unwrap();

        if !master.flags.contains(flags::FLAG_FAIL) {
            return;
        }

        self.current_epoch += 1;

        let msg = Message {
            header: MessageHeader::from(&self.myself),
            payload: MessagePayload::FailOver(FailOverPayload {
                kind: FailOverKind::AuthRequest,
            }),
        };

        let bytes = Vec::from(&msg);

        for stream in self.cluster_streams.values_mut() {
            let _ = stream.write_all(&bytes);
        }
    }

    pub fn handle_failover_message(
        &mut self,
        payload: FailOverPayload,
        header: &MessageHeader,
        log_tx: &Sender<Log>,
    ) {
        match payload.kind {
            FailOverKind::AuthRequest => self.cast_vote_for_replica(header.id, log_tx),
            FailOverKind::AuthAck => self.promote_myself_to_master(log_tx),
        };
    }

    fn cast_vote_for_replica(&mut self, replica_id: NodeId, log_tx: &Sender<Log>) {
        // TODO aqui tendriamos que verificar que no se haya emitido voto anterior en esta epoch.

        let stream = self.cluster_streams.get_mut(&replica_id).unwrap();

        let msg = Message {
            header: MessageHeader::from(&self.myself),
            payload: MessagePayload::FailOver(FailOverPayload {
                kind: FailOverKind::AuthAck,
            }),
        };

        stream.write_all(&Vec::from(&msg)).unwrap();

        log_tx
            .send(log::info!(
                "emitido voto para replica {}",
                hex::encode(replica_id)
            ))
            .unwrap();
    }

    fn promote_myself_to_master(&mut self, log_tx: &Sender<Log>) {
        self.myself.flags.0 &= !flags::FLAG_SLAVE;
        self.myself.flags.0 |= flags::FLAG_MASTER;

        log_tx.send(log::info!("promovido a master")).unwrap();
    }
}
