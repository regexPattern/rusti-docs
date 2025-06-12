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
    pub fn handle_fail_msg(&mut self, payload: FailPayload, log_tx: &Sender<Log>) {
        if let Some(node) = self.cluster_view.get_mut(&payload.id) {
            node.flags.0 |= flags::FLAG_FAIL;
            node.flags.0 &= !flags::FLAG_PFAIL;

            log_tx
                .send(log::info!(
                    "nodo {} marcado como FAIL",
                    hex::encode(node.id)
                ))
                .unwrap();

            if Some(node.id) == self.myself.master_id {
                self.request_failover();
            }
        }
    }

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

                    log_tx
                        .send(log::warn!(
                            "nodo {} marcado como PFAIL",
                            hex::encode(node.id)
                        ))
                        .unwrap();
                }
            } else {
                node.flags.0 &= !flags::FLAG_PFAIL;
            }
        }
    }

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

        for node in self
            .cluster_view
            .values_mut()
            .filter(|n| n.flags.contains(flags::FLAG_PFAIL) && !n.flags.contains(flags::FLAG_FAIL))
        {
            if let Some(reports) = self.failure_reports.get(&node.id) {
                if Self::failure_has_quorum(self.myself.flags, reports.len(), n_known_other_masters)
                {
                    actions_tx
                        .send(ClusterAction::ConfirmFailure { id: node.id })
                        .unwrap();

                    log_tx
                        .send(log::info!("enviado FAIL de nodo {}", hex::encode(node.id)))
                        .unwrap();
                }

                node.flags.0 &= !flags::FLAG_PFAIL;
                node.flags.0 |= flags::FLAG_FAIL;
            }
        }
    }

    fn failure_has_quorum(
        myself_flags: Flags,
        n_other_masters_reports: usize,
        n_other_alive_masters: usize,
    ) -> bool {
        n_other_masters_reports + (myself_flags.contains(flags::FLAG_MASTER) as usize)
            >= (n_other_alive_masters / 2 + 1)
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

    pub fn confirm_failure(&mut self, fail_id: NodeId, log_tx: &Sender<Log>) {
        let msg = Message {
            header: MessageHeader::from(&self.myself),
            payload: MessagePayload::Fail(FailPayload { id: fail_id }),
        };

        for (id, stream) in &mut self.cluster_streams {
            if *id != fail_id && stream.write_all(&Vec::from(&msg)).is_err() {
                log_tx
                    .send(log::error!(
                        "error escribiendo a stream de nodo {}",
                        hex::encode(id)
                    ))
                    .unwrap();
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use std::{
        collections::{HashMap, HashSet},
        net::Ipv4Addr,
        sync::mpsc,
    };

    use rand::Rng;

    use crate::{
        config::ClusterConfig,
        server::node::cluster::{ClusterNode, flags::Flags, tests::dummy_empty_cluster_actor},
    };

    use super::*;

    fn dummy_node_id() -> NodeId {
        let mut id = [0_u8; 20];
        rand::rng().fill(&mut id);
        id
    }

    #[test]
    fn se_marcan_como_pfail_los_nodos_con_pong_received_vencido() {
        let mut cluster_actor = dummy_empty_cluster_actor();

        let now = SystemTime::now();

        let mut insert_node = |offset: Duration| {
            let id = dummy_node_id();
            let mut node =
                ClusterNode::from(&ClusterConfig::default(Ipv4Addr::new(127, 0, 0, 1), 6379));
            node.flags = Flags(flags::FLAG_MASTER);
            node.pong_received = now - offset;
            cluster_actor.cluster_view.insert(id, node);
        };

        // dentro de rango
        insert_node(Duration::from_millis(cluster_actor.timeout_millis / 3));
        insert_node(Duration::from_millis(cluster_actor.timeout_millis / 4));
        insert_node(Duration::from_millis(cluster_actor.timeout_millis / 5));

        // fuera de rango
        insert_node(Duration::from_millis(cluster_actor.timeout_millis * 3));
        insert_node(Duration::from_millis(cluster_actor.timeout_millis * 4));
        insert_node(Duration::from_millis(cluster_actor.timeout_millis * 5));

        assert_eq!(
            cluster_actor
                .cluster_view
                .values()
                .filter(|n| n.flags.contains(flags::FLAG_PFAIL))
                .count(),
            0
        );

        let (dummy_actions_tx, _dummy_ations_rx) = mpsc::channel();
        let (dummy_log_tx, _dummy_log_rx) = mpsc::channel();

        cluster_actor.check_failures(&dummy_actions_tx, &dummy_log_tx);

        assert_eq!(
            cluster_actor
                .cluster_view
                .values()
                .filter(|n| n.flags.contains(flags::FLAG_PFAIL))
                .count(),
            3
        );
    }

    #[test]
    fn se_filtran_los_failure_reports_expirados() {
        let mut cluster_actor = dummy_empty_cluster_actor();

        let node_id = dummy_node_id();

        let now = SystemTime::now();

        let report_at = |offset: Duration| {
            let reporter_id = dummy_node_id();
            (
                reporter_id,
                FailureReport {
                    reporter_id,
                    time: now - offset,
                },
            )
        };

        let reports = [
            // no expirados
            report_at(Duration::from_millis(cluster_actor.timeout_millis / 3)),
            report_at(Duration::from_millis(cluster_actor.timeout_millis / 4)),
            report_at(Duration::from_millis(cluster_actor.timeout_millis / 5)),
            // expirados
            report_at(Duration::from_millis(cluster_actor.timeout_millis * 3)),
            report_at(Duration::from_millis(cluster_actor.timeout_millis * 4)),
            report_at(Duration::from_millis(cluster_actor.timeout_millis * 5)),
        ];

        cluster_actor
            .failure_reports
            .insert(node_id, HashMap::from(reports));

        cluster_actor.filter_expired_failure_reports(now);

        let node_failure_reports: HashSet<_> = cluster_actor
            .failure_reports
            .get(&node_id)
            .unwrap()
            .values()
            .collect();

        assert_eq!(node_failure_reports.len(), 3);
        assert!(node_failure_reports.contains(&reports[0].1));
        assert!(node_failure_reports.contains(&reports[1].1));
        assert!(node_failure_reports.contains(&reports[2].1));
    }

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
