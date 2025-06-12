use std::{
    collections::hash_map::Entry,
    io::prelude::*,
    net::{SocketAddr, TcpStream},
    sync::mpsc::Sender,
    time::SystemTime,
};

use log::Log;

use super::{ClusterActor, ClusterNode, fail::FailureReport, flags, message::*};

impl ClusterActor {
    pub fn ping(&mut self, node: &GossipNode) {
        let msg = Message {
            header: MessageHeader::from(&self.myself),
            payload: MessagePayload::Gossip(GossipPayload {
                kind: GossipKind::Ping,
                nodes: self.select_random_nodes(),
            }),
        };

        let stream = self.cluster_streams.entry(node.id).or_insert_with(|| {
            TcpStream::connect(SocketAddr::new(node.ip.into(), node.cluster_port)).unwrap()
        });

        let node = self.cluster_view.get_mut(&node.id).unwrap();
        node.ping_sent = SystemTime::now();

        let _ = stream.write_all(&Vec::from(&msg));
    }

    pub fn pong(&mut self, header: &MessageHeader) {
        let msg = Message {
            header: MessageHeader::from(&self.myself),
            payload: MessagePayload::Gossip(GossipPayload {
                kind: GossipKind::Pong,
                nodes: self.select_random_nodes(),
            }),
        };

        let stream = self.cluster_streams.entry(header.id).or_insert_with(|| {
            TcpStream::connect(SocketAddr::new(header.ip.into(), header.cluster_port)).unwrap()
        });

        let _ = stream.write_all(&Vec::from(&msg));
    }

    pub fn handle_gossip_message(
        &mut self,
        header: &MessageHeader,
        payload: GossipPayload,
        log_tx: &Sender<Log>,
    ) {
        for gossip_node in payload.nodes {
            if gossip_node.id == self.myself.id {
                continue;
            }

            if let Entry::Vacant(entry) = self.cluster_view.entry(gossip_node.id) {
                log_tx
                    .send(log::info!(
                        "conocido al nodo {} mediante gossip",
                        hex::encode(gossip_node.id),
                    ))
                    .unwrap();

                entry.insert(ClusterNode::from(&gossip_node));
            }

            let sender_is_master = self
                .cluster_view
                .get(&header.id)
                .and_then(|n| Some(n.flags.contains(flags::FLAG_MASTER)))
                .is_some();

            if gossip_node.flags.contains(flags::FLAG_PFAIL) && sender_is_master {
                let reports = self.failure_reports.entry(gossip_node.id).or_default();
                reports.insert(
                    header.id,
                    FailureReport {
                        reporter_id: header.id,
                        time: SystemTime::now(),
                    },
                );
            }
        }

        if payload.kind == GossipKind::Ping {
            self.pong(header);
        } else if payload.kind == GossipKind::Pong {
            let node = self.cluster_view.get_mut(&header.id).unwrap();
            node.pong_received = SystemTime::now();
        }
    }
}
