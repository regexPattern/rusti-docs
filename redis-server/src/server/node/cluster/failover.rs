use std::time::SystemTime;
use std::{io::prelude::*, sync::mpsc::Sender};

use log::Log;

use super::{
    ClusterActor, flags,
    message::{
        Message, MessageHeader, MessagePayload,
        payload::{FailOverKind, FailOverPayload},
    },
};

impl ClusterActor {
    pub fn request_failover(&mut self, log_tx: &Sender<Log>) {
        //Solo replicas puede hacer failover
        //esto de aajo no hace falta..
        let master_id = match self.myself.master_id {
            Some(master_id) => master_id,
            None => return,
        };
        let master = self.cluster_view.get(&master_id).unwrap();

        if !master.flags.contains(flags::FLAG_FAIL) {
            return;
        }

        self.failover_epoch = Some(self.current_epoch);
        self.failover_start = Some(SystemTime::now());
        self.failover_in_progress = true;

        let prev_current_epoch = self.current_epoch;
        self.current_epoch += 1;

        //start election
        //Me voto a mi mismo
        self.votes_received = 1;

        log_tx
            .send(log::info!(
                "aumentando current epoch de {} a {}",
                prev_current_epoch,
                self.current_epoch
            ))
            .unwrap();

        log_tx.send(log::info!("iniciando elección")).unwrap();

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
            // Si el nodo es master y no está fallando, le envío el mensaje
            if node.flags.contains(flags::FLAG_MASTER) && !node.flags.contains(flags::FLAG_FAIL) {
                if let Some(stream) = self.cluster_streams.get_mut(id) {
                    let _ = stream.write_all(&bytes);
                    log_tx
                        .send(log::info!("pidiendo voto a nodo {}", hex::encode(id)))
                        .unwrap();
                }
            }
        }
    }

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
        // TODO aqui tendriamos que verificar que no se haya emitido voto anterior en esta epoch.
        //solo le envie el mensaje a los masters asi q nunca se ejcuta esto en una replica

        let request_epoch = replica_header.config_epoch;
        if request_epoch > self.last_vote_epoch {
            log_tx
                .send(log::info!(
                    "Votando por epoch {} (último epoch votado: {}) para replica {}",
                    request_epoch,
                    self.last_vote_epoch,
                    hex::encode(replica_header.id)
                ))
                .unwrap();

            self.last_vote_epoch = request_epoch;

            let replica = match self.cluster_view.get(&replica_header.id) {
                Some(replica) => replica,
                None => {
                    log_tx
                        .send(log::warn!(
                            "Replica {} no encontrada en la vista del cluster",
                            hex::encode(replica_header.id)
                        ))
                        .unwrap();
                    return;
                }
            };

            //si el master de la replica no esta fallando no le doy mi voto
            if let Some(failing_master_id) = replica.master_id {
                if let Some(failing_master) = self.cluster_view.get(&failing_master_id) {
                    if !failing_master.flags.contains(flags::FLAG_FAIL) {
                        log_tx
                            .send(log::warn!(
                                "El master de la replica {} no está fallando, no se emite voto",
                                hex::encode(replica.id)
                            ))
                            .unwrap();
                        return;
                    }
                } else {
                    log_tx
                        .send(log::warn!(
                            "Master {} de la replica {} no encontrado en la vista del cluster",
                            hex::encode(failing_master_id),
                            hex::encode(replica.id)
                        ))
                        .unwrap();
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

            log_tx
                .send(log::info!(
                    "emitido voto para replica {} con config_epoch {}",
                    hex::encode(replica.id),
                    header.config_epoch,
                ))
                .unwrap();
        } else {
            log_tx
                .send(log::info!(
                    "No voto por epoch {}: ya voté en epoch {}",
                    request_epoch,
                    self.last_vote_epoch
                ))
                .unwrap();
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

        log_tx
            .send(log::info!(
                "Recibido voto de {} (total: {}/{})",
                hex::encode(header.id),
                self.votes_received,
                quorum
            ))
            .unwrap();

        // Si alcanzamos el quórum y aún no somos master, promovemos
        if self.votes_received >= quorum as u64 && !self.myself.flags.contains(flags::FLAG_MASTER) {
            //ESTA FALLANDO UNA VEZ CADA TANTO POREQUE OCURRE QUE EL QUE SE PROMOVIO AUN NO LLEGO A
            //POROPAGAR POR GOSSIP SU ESTADO ENONCES EL OTRO PREGUNTA SI TIENE EN SU CLUSTER VIEW UN
            //MASTER CON MISMOS SLOSTS Y DICE NOOO XQ AUNQ NO LO LLEGO LA FINO DEL OTRO ENTRONCES
            //SE PROMUEVE Y U YA HABIA OTRO Y NO SE HABIA ENTERADO
            //creo q estamos teniendo un spliot brain.
            //aumentar la vleocidad del gossip para que llegeu ela ifno mucho antes podria servir????

            if let Some(master) = self.cluster_view.values().find(|n| {
                n.flags.contains(flags::FLAG_MASTER)
                    && !n.flags.contains(flags::FLAG_FAIL)
                    && n.slots == self.myself.slots
            }) {
                log_tx
                    .send(log::warn!(
                        "No se puede promover a MASTER: ya existe un MASTER {} con los mismos slots {:?}",
                        hex::encode(master.id),
                        master.slots
                    ))
                    .unwrap();
                self.failover_in_progress = false;

                self.myself.master_id = Some(master.id);

                return;
            }

            log_tx
                .send(log::debug!("EPOCH DE PROMOTION {}", self.current_epoch))
                .unwrap();

            self.myself.flags.0 &= !flags::FLAG_SLAVE;
            self.myself.flags.0 |= flags::FLAG_MASTER;

            //ojo con esto q este bien
            self.myself.master_id = None;

            self.myself.config_epoch = self.current_epoch;

            //podriamos apagar la variables de elccion aca en vez de en el loop del cluster
            // self.failover_in_progress = false;
            // self.failover_epoch = None;
            // self.failover_start = None;

            log_tx
                .send(log::info!(
                    "¡Quórum alcanzado! Nodo promovido a MASTER: config_epoch actualizado a {}",
                    self.myself.config_epoch
                ))
                .unwrap();
        }
    }

    fn can_vote_for_replica(&self) -> bool {
        // si el master de la replica tiene fail, y no he votado en este epoch
        true
    }

    fn failover_has_quorum(n_known_masters_votes: usize, n_known_alive_masters: usize) -> bool {
        n_known_masters_votes >= (n_known_alive_masters / 2 + 1)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn failover_tiene_quorum_si_se_reciben_votos_de_mayoria_de_masters() {
        let n_known_masters_votes = 3;
        let n_known_alive_masters = 5;

        assert!(ClusterActor::failover_has_quorum(
            n_known_masters_votes,
            n_known_alive_masters
        ));
    }

    #[test]
    fn failover_no_tiene_quorum_si_no_se_reciben_votos_de_mayoria_de_masters() {
        let n_known_masters_votes = 2;
        let n_known_alive_masters = 5;

        assert!(!ClusterActor::failover_has_quorum(
            n_known_masters_votes,
            n_known_alive_masters
        ));
    }

    #[test]
    fn failover_no_tiene_quorum_en_caso_de_empate() {
        let n_known_masters_votes = 2;
        let n_known_alive_masters = 4;

        assert!(!ClusterActor::failover_has_quorum(
            n_known_masters_votes,
            n_known_alive_masters
        ));
    }
}
