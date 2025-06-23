use std::{
    fmt,
    io::prelude::*,
    sync::mpsc::Sender,
    time::{Duration, SystemTime, UNIX_EPOCH},
};

use chrono::{DateTime, Local};
use log::Log;

use super::{
    ClusterAction, ClusterActor, NodeId,
    flags::{self, Flags},
    message::{FailPayload, Message, MessageHeader, MessagePayload},
};

#[derive(Copy, Clone, PartialEq, Hash)]
/// Reporte de fallo de un nodo en el clúster, con id del reportante y timestamp.
pub struct FailureReport {
    pub reporter_id: NodeId,
    pub time: SystemTime,
}

impl Eq for FailureReport {}

impl fmt::Display for FailureReport {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let time: DateTime<Local> = self.time.into();

        f.debug_struct("FailureReport")
            .field("reporter_id", &hex::encode(self.reporter_id))
            .field("time", &time.format("%d-%m-%Y %H:%M:%S").to_string())
            .finish()
    }
}

impl fmt::Debug for FailureReport {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{self}")
    }
}

impl ClusterActor {
    /// Marca un nodo como FAIL en la vista del clúster y solicita failover si es el master.
    /// Actualiza los flags del nodo y registra el evento en el log.
    pub fn handle_fail_message(&mut self, payload: FailPayload, log_tx: &Sender<Log>) {
        if let Some(node) = self.cluster_view.get_mut(&payload.id) {
            node.flags.0 |= flags::FLAG_FAIL;
            node.flags.0 &= !flags::FLAG_PFAIL;

            let _ = log_tx.send(log::info!(
                "nodo {} marcado como FAIL",
                hex::encode(node.id)
            ));

            if Some(node.id) == self.myself.master_id {
                self.request_failover(log_tx);
            }
        }
    }

    /// Revisa el estado de los nodos para detectar fallos potenciales y confirmados.
    /// Llama a los métodos internos para marcar PFAIL/FAIL y enviar acciones si hay quorum.
    pub fn check_failures(&mut self, actions_tx: &Sender<ClusterAction>, log_tx: &Sender<Log>) {
        let now = SystemTime::now();

        self.check_potential_failures(now, log_tx);
        self.check_confirmed_failures(now, actions_tx, log_tx);
    }

    fn check_potential_failures(&mut self, now: SystemTime, log_tx: &Sender<Log>) {
        for node in self.cluster_view.values_mut() {
            if node.ping_sent != UNIX_EPOCH
                && now.duration_since(node.pong_received).unwrap()
                    > Duration::from_millis(self.timeout_millis)
            {
                if !node.flags.contains(flags::FLAG_PFAIL) && !node.flags.contains(flags::FLAG_FAIL)
                {
                    node.flags.0 |= flags::FLAG_PFAIL;
                    let _ = log_tx.send(log::warn!(
                        "nodo {} marcado como PFAIL",
                        hex::encode(node.id)
                    ));
                }
            } else {
                node.flags.0 &= !flags::FLAG_PFAIL;
            }
        }
    }
    /// Revisa los nodos marcados como PFAIL y confirma fallos si hay quorum.
    /// Envía acciones de confirmación de fallo y actualiza los flags de los nodos.
    /// Si el nodo afectado es el master, solicita failover.
    pub fn check_confirmed_failures(
        &mut self,
        now: SystemTime,
        actions_tx: &Sender<ClusterAction>,
        log_tx: &Sender<Log>,
    ) {
        self.filter_expired_failure_reports(now);

        let n_known_other_masters = self
            .cluster_view
            .values()
            .filter(|n| {
                n.flags.contains(flags::FLAG_MASTER)
                    && (!n.flags.contains(flags::FLAG_PFAIL) && !n.flags.contains(flags::FLAG_FAIL))
            })
            .count();

        let affected_node_ids: Vec<_> = self
            .cluster_view
            .values()
            .filter(|n| n.flags.contains(flags::FLAG_PFAIL) && !n.flags.contains(flags::FLAG_FAIL))
            .map(|n| n.id)
            .collect();

        for node_id in affected_node_ids {
            if let Some(reports) = self.failure_reports.get(&node_id) {
                if Self::failure_has_quorum(self.myself.flags, reports.len(), n_known_other_masters)
                {
                    actions_tx
                        .send(ClusterAction::ConfirmFailure { id: node_id })
                        .unwrap();

                    let _ =
                        log_tx.send(log::info!("enviado FAIL de nodo {}", hex::encode(node_id)));

                    if let Some(node) = self.cluster_view.get_mut(&node_id) {
                        node.flags.0 &= !flags::FLAG_PFAIL;
                        node.flags.0 |= flags::FLAG_FAIL;
                    }

                    if let Some(master_id) = self.myself.master_id {
                        if master_id == node_id {
                            let _ = log_tx.send(log::info!(
                                "Mi master {} ha fallado, solicitando failover",
                                hex::encode(master_id)
                            ));
                            self.request_failover(log_tx);
                        }
                    }
                }
            }
        }
    }

    fn failure_has_quorum(
        myself_flags: Flags,
        n_known_masters_reports: usize,
        n_known_alive_masters: usize,
    ) -> bool {
        n_known_masters_reports + (myself_flags.contains(flags::FLAG_MASTER) as usize)
            >= (n_known_alive_masters / 2 + 1)
    }

    fn filter_expired_failure_reports(&mut self, now: SystemTime) {
        self.failure_reports.retain(|_, reports| {
            reports.retain(|_, ts| {
                now.duration_since(ts.time).unwrap()
                    < Duration::from_millis(self.timeout_millis * 4)
            });

            !reports.is_empty()
        });
    }

    /// Confirma el fallo de un nodo y notifica a los demás nodos del clúster.
    /// Envía un mensaje FAIL a todos los nodos excepto al fallido y registra en el log.
    pub fn confirm_failure(&mut self, fail_id: NodeId, log_tx: &Sender<Log>) {
        let msg = Message {
            header: MessageHeader::from(&self.myself),
            payload: MessagePayload::Fail(FailPayload { id: fail_id }),
        };

        for (id, stream) in &mut self.cluster_streams {
            if *id != fail_id && stream.write_all(&Vec::from(&msg)).is_err() {
                let _ = log_tx.send(log::error!(
                    "error escribiendo a stream de nodo {}",
                    hex::encode(id)
                ));
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::server::node::cluster::flags::Flags;

    #[test]
    fn failure_tiene_quorum_si_hay_reports_de_mayoria_de_masters_y_soy_replica() {
        let n_known_masters_reports = 1;
        let n_known_alive_masters = 2;

        assert!(!ClusterActor::failure_has_quorum(
            Flags(flags::FLAG_SLAVE),
            n_known_masters_reports,
            n_known_alive_masters
        ));

        let n_known_masters_reports = 2;
        let n_known_alive_masters = 3;

        assert!(ClusterActor::failure_has_quorum(
            Flags(flags::FLAG_SLAVE),
            n_known_masters_reports,
            n_known_alive_masters
        ));
    }

    #[test]
    fn failure_tiene_quorum_si_hay_reports_de_mayoria_de_masters_y_soy_master() {
        let n_known_masters_reports = 0;
        let n_known_alive_masters = 1;

        assert!(ClusterActor::failure_has_quorum(
            Flags(flags::FLAG_MASTER),
            n_known_masters_reports,
            n_known_alive_masters
        ));

        let n_known_masters_reports = 1;
        let n_known_alive_masters = 3;

        assert!(ClusterActor::failure_has_quorum(
            Flags(flags::FLAG_MASTER),
            n_known_masters_reports,
            n_known_alive_masters
        ));

        let n_known_masters_reports = 0;
        let n_known_alive_masters = 2;

        assert!(!ClusterActor::failure_has_quorum(
            Flags(flags::FLAG_MASTER),
            n_known_masters_reports,
            n_known_alive_masters
        ));
    }

    #[test]
    fn failure_tiene_quorum_si_soy_el_unico_master() {
        let n_known_masters_reports = 0;
        let n_known_alive_masters = 0;

        assert!(ClusterActor::failure_has_quorum(
            Flags(flags::FLAG_MASTER),
            n_known_masters_reports,
            n_known_alive_masters
        ));
    }
}
