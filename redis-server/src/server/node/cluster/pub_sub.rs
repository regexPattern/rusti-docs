use std::{io::prelude::*, sync::mpsc::Sender};

use log::Log;
use redis_resp::BulkString;

use super::{ClusterAction, ClusterActor, message::*};

impl ClusterActor {
    /// Maneja un mensaje PUBLISH recibido y lo reenvía al canal pub/sub local.
    /// Utiliza un canal interno para distribuir el mensaje a los suscriptores.
    pub fn handle_publish_msg(&mut self, payload: PublishPayload) {
        self.pub_sub_tx.send(payload).unwrap();
    }

    /// Broadcastea un mensaje PUBLISH a todos los nodos del clúster excepto a sí mismo.
    /// Envía el mensaje por la red y registra advertencias en caso de error.
    pub fn broadcast_publish(
        &mut self,
        channel: BulkString,
        message: BulkString,
        _actions_tx: &Sender<ClusterAction>,
        log_tx: &Sender<Log>,
    ) {
        let msg = Message {
            header: MessageHeader::from(&self.myself),
            payload: MessagePayload::Publish(PublishPayload { channel, message }),
        };
        let bytes = Vec::from(&msg);

        for (node_id, stream) in self.cluster_streams.iter_mut() {
            if *node_id == self.myself.id {
                continue;
            }
            if let Err(e) = stream.write_all(&bytes) {
                let _ = log_tx.send(log::warn!(
                    "Error enviando publish a nodo {}: {}",
                    hex::encode(node_id),
                    e
                ));
            }
        }
    }
}
