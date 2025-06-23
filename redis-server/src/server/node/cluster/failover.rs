use std::{io::prelude::*, sync::mpsc::Sender, time::SystemTime};

use log::Log;

use super::{
    ClusterActor, flags,
    message::{
        Message, MessageHeader, MessagePayload,
        payload::{FailOverKind, FailOverPayload},
    },
};

impl ClusterActor {
    /// Inicia el proceso de failover si el master ha fallado.
    /// Actualiza epoch, inicia votación y solicita votos a otros nodos masters.
    pub fn request_failover(&mut self, log_tx: &Sender<Log>) {
        let master_id = match self.myself.master_id {
            Some(master_id) => master_id,
            None => return,
        };

        let master = match self.cluster_view.get(&master_id) {
            Some(master) => master,
            None => {
                let _ = log_tx.send(log::info!("nodo no tienen a su master en su cluster view"));
                return;
            }
        };

        if !master.flags.contains(flags::FLAG_FAIL) {
            return;
        }

        if let Some(existing_master) = self.cluster_view.values().find(|n| {
            n.flags.contains(flags::FLAG_MASTER)
                && !n.flags.contains(flags::FLAG_FAIL)
                && n.slots == master.slots
        }) {
            let _ = log_tx.send(log::info!(
                "ya existe master {} para slots {}-{}, haciendome replica",
                hex::encode(existing_master.id),
                existing_master.slots.0,
                existing_master.slots.1,
            ));

            if let Err(err) = self.set_master(existing_master.id, log_tx) {
                let _ = log_tx.send(log::error!("error configurando master: {err}"));
            }
            return;
        }

        self.failover_epoch = Some(self.current_epoch);
        self.failover_start = Some(SystemTime::now());
        self.failover_in_progress = true;

        let prev_current_epoch = self.current_epoch;

        self.current_epoch += 1;
        self.votes_received = 1;

        let _ = log_tx.send(log::info!(
            "aumentando current epoch de {} a {}",
            prev_current_epoch,
            self.current_epoch
        ));

        let _ = log_tx.send(log::info!("iniciando elección"));

        let mut header = MessageHeader::from(&self.myself);
        header.config_epoch = self.current_epoch;

        let msg = Message {
            header,
            payload: MessagePayload::FailOver(FailOverPayload {
                kind: FailOverKind::AuthRequest,
            }),
        };

        let bytes = Vec::from(&msg);

        for (id, node) in &self.cluster_view {
            if node.flags.contains(flags::FLAG_MASTER) && !node.flags.contains(flags::FLAG_FAIL) {
                if let Some(stream) = self.cluster_streams.get_mut(id) {
                    if let Err(err) = stream.write_all(&bytes) {
                        let _ = log_tx.send(log::error!("{err}"));
                        continue;
                    }
                    let _ = log_tx.send(log::info!("pidiendo voto a nodo {}", hex::encode(id)));
                }
            }
        }
    }

    /// Maneja mensajes de failover: solicitudes de voto y confirmaciones.
    /// Llama a los métodos internos para votar o intentar promoción a master.
    pub fn handle_failover_message(
        &mut self,
        payload: FailOverPayload,
        header: &MessageHeader,
        log_tx: &Sender<Log>,
    ) {
        match payload.kind {
            FailOverKind::AuthRequest => self.cast_vote_for_replica(header, log_tx),
            FailOverKind::AuthAck => self.try_to_promote_myself_to_master(header, log_tx),
        };
    }

    fn cast_vote_for_replica(&mut self, replica_header: &MessageHeader, log_tx: &Sender<Log>) {
        let request_epoch = replica_header.config_epoch;
        if request_epoch > self.last_vote_epoch && request_epoch >= self.current_epoch {
            let _ = log_tx.send(log::info!(
                "votando por epoch {} (último epoch votado: {}) para replica {}",
                request_epoch,
                self.last_vote_epoch,
                hex::encode(replica_header.id)
            ));

            self.last_vote_epoch = request_epoch;

            if request_epoch > self.current_epoch {
                self.current_epoch = request_epoch;
            }

            let replica = match self.cluster_view.get(&replica_header.id) {
                Some(replica) => replica,
                None => {
                    let _ = log_tx.send(log::warn!(
                        "replica {} no encontrada en la vista del cluster",
                        hex::encode(replica_header.id)
                    ));
                    return;
                }
            };

            if let Some(failing_master_id) = replica.master_id {
                if let Some(failing_master) = self.cluster_view.get(&failing_master_id) {
                    if !failing_master.flags.contains(flags::FLAG_FAIL) {
                        let _ = log_tx.send(log::warn!(
                            "master de la replica {} no está fallando, no se emite voto",
                            hex::encode(replica.id)
                        ));
                        return;
                    }
                } else {
                    let _ = log_tx.send(log::warn!(
                        "master {} de la replica {} no encontrado en la vista del cluster",
                        hex::encode(failing_master_id),
                        hex::encode(replica.id)
                    ));
                    return;
                }
            }

            let stream = self.cluster_streams.get_mut(&replica.id).unwrap();

            let mut header = MessageHeader::from(&self.myself);
            header.config_epoch = self.current_epoch;

            let msg = Message {
                header,
                payload: MessagePayload::FailOver(FailOverPayload {
                    kind: FailOverKind::AuthAck,
                }),
            };

            stream.write_all(&Vec::from(&msg)).unwrap();

            let _ = log_tx.send(log::info!(
                "emitido voto para replica {} con config_epoch {}",
                hex::encode(replica.id),
                header.config_epoch,
            ));
        } else {
            let _ = log_tx.send(log::info!(
                "no voto por epoch {}: ya voté en epoch {}",
                request_epoch,
                self.last_vote_epoch
            ));
        }
    }

    fn try_to_promote_myself_to_master(&mut self, header: &MessageHeader, log_tx: &Sender<Log>) {
        self.votes_received += 1;

        let n_masters = self
            .cluster_view
            .values()
            .filter(|n| n.flags.contains(flags::FLAG_MASTER) && !n.flags.contains(flags::FLAG_FAIL))
            .count();

        let quorum = (n_masters / 2) + 1;

        let _ = log_tx.send(log::info!(
            "recibido voto de {} (total: {}/{})",
            hex::encode(header.id),
            self.votes_received,
            quorum
        ));

        if self.votes_received >= quorum as u64 && !self.myself.flags.contains(flags::FLAG_MASTER) {
            if let Some(master) = self.cluster_view.values().find(|n| {
                n.flags.contains(flags::FLAG_MASTER)
                    && !n.flags.contains(flags::FLAG_FAIL)
                    && n.slots == self.myself.slots
            }) {
                let _ = log_tx.send(log::debug!(
                    "ya existe un master {} con los slots del {} al {}",
                    hex::encode(master.id),
                    master.slots.0,
                    master.slots.1,
                ));

                let _ = log_tx.send(log::info!(
                    "haciendome replica de nodo {}",
                    hex::encode(master.id)
                ));

                self.failover_in_progress = false;
                self.set_master(master.id, log_tx).unwrap();

                return;
            }

            if let Some(competing_master) = self.cluster_view.values().find(|n| {
                n.flags.contains(flags::FLAG_MASTER)
                    && !n.flags.contains(flags::FLAG_FAIL)
                    && n.config_epoch >= self.current_epoch
                    && n.slots == self.myself.slots
                    && (n.config_epoch > self.current_epoch || n.id < self.myself.id)
            }) {
                let _ = log_tx.send(log::info!(
                    "perdiendo elección contra master {} (epoch: {}, mi epoch: {})",
                    hex::encode(competing_master.id),
                    competing_master.config_epoch,
                    self.current_epoch
                ));

                self.failover_in_progress = false;
                self.set_master(competing_master.id, log_tx).unwrap();
                return;
            }

            self.myself.flags.0 &= !flags::FLAG_SLAVE;
            self.myself.flags.0 |= flags::FLAG_MASTER;
            self.myself.master_id = None;
            self.myself.config_epoch = self.current_epoch;

            let _ = log_tx.send(log::debug!(
                "ascendido a master con slots {}-{} en epoch {}",
                self.myself.slots.0,
                self.myself.slots.1,
                self.current_epoch
            ));
        }
    }
}
