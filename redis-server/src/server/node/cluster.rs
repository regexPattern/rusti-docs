mod node;
mod packet;
mod persist;

use std::{
    collections::HashMap,
    io::{BufRead, BufReader},
    net::{SocketAddr, TcpListener, TcpStream},
    sync::{
        Arc, RwLock,
        mpsc::{self, Sender},
    },
    thread,
    time::Duration,
};

use log::LogMsg;
use node::{ClusterNode, GossipedNode};
use packet::{GossipDataKind, Packet, PacketData, PacketHeader};
use rand::seq::IndexedRandom;
use redis_cmd::cluster::ClusterCommand;

use crate::config::ClusterConfig;

const CLUSTER_SLOTS: usize = 16384;

/// Porción del cluster view que se selecciona para compartirse de manera aleatoria mediante
/// gossip. El valor debe estar entre cumple 0 < x <= 1.0.
const CLUSTER_VIEW_SHARE_FACTOR: f64 = 0.25;

/// Frecuencia con la que se envían heartbeats.
const CLUSTER_PING_FREQ: Duration = Duration::from_secs(3);

type NodeId = [u8; 20];
type ClusterView = HashMap<NodeId, Node>;
type ClusterStreams = HashMap<NodeId, TcpStream>;

#[derive(Debug)]
pub struct ClusterEnvelope {
    pub cmd: ClusterCommand,
    pub reply_tx: Sender<Vec<u8>>,
}

#[derive(Debug)]
pub struct ClusterActor {
    myself: ClusterNode,
    cluster_view: Arc<RwLock<ClusterView>>,
    cluster_streams: Arc<RwLock<ClusterStreams>>,
}

#[derive(Copy, Clone, Debug)]
enum Node {
    Complete(ClusterNode),
    Incomplete(GossipedNode),
}

impl ClusterActor {
    pub fn start(
        config: ClusterConfig,
        logger_tx: Sender<LogMsg>,
    ) -> Result<Sender<ClusterEnvelope>, ()> {
        let cluster_actor = Self::try_from(config).unwrap();

        logger_tx
            .send(log::info!(
                "iniciando nodo {}",
                hex::encode(cluster_actor.myself.id)
            ))
            .unwrap();

        let tx = Self::listen_client_commands(
            cluster_actor.myself,
            Arc::clone(&cluster_actor.cluster_view),
            logger_tx.clone(),
        );

        Self::listen_cluster_bus(
            cluster_actor.myself,
            Arc::clone(&cluster_actor.cluster_view),
            logger_tx.clone(),
        )
        .unwrap();

        Self::send_heartbeats(
            cluster_actor.myself,
            cluster_actor.cluster_view,
            cluster_actor.cluster_streams,
            logger_tx,
        );

        Ok(tx)
    }

    fn listen_client_commands(
        myself: ClusterNode,
        cluster_view: Arc<RwLock<ClusterView>>,
        logger_tx: Sender<LogMsg>,
    ) -> Sender<ClusterEnvelope> {
        let (tx, rx) = mpsc::channel();

        thread::spawn(move || {
            while let Ok(envel) = rx.recv() {
                myself.process(envel, &cluster_view, &logger_tx);
            }
        });

        tx
    }

    fn listen_cluster_bus(
        myself: ClusterNode,
        cluster_view: Arc<RwLock<ClusterView>>,
        logger_tx: Sender<LogMsg>,
    ) -> Result<(), ()> {
        let addr = SocketAddr::new(myself.ip.into(), myself.port);
        let listener = TcpListener::bind(addr).unwrap();

        logger_tx
            .send(log::info!("servidor escuchando cluster en {addr:?}"))
            .unwrap();

        thread::spawn(move || {
            for stream in listener.incoming() {
                let stream = stream.unwrap();

                let cluster_view = Arc::clone(&cluster_view);
                let logger_tx = logger_tx.clone();

                thread::spawn(move || {
                    Self::handle_incoming_packet(&myself, stream, cluster_view, &logger_tx);
                });
            }
        });

        Ok(())
    }

    fn send_heartbeats(
        myself: ClusterNode,
        cluster_view: Arc<RwLock<ClusterView>>,
        cluster_streams: Arc<RwLock<ClusterStreams>>,
        _logger_tx: Sender<LogMsg>,
    ) {
        thread::spawn(move || {
            loop {
                thread::sleep(CLUSTER_PING_FREQ);

                let cluster_view = cluster_view.read().unwrap();
                let nodes: Vec<_> = cluster_view.values().copied().collect();

                if let Some(node) = nodes.choose(&mut rand::rng()) {
                    let mut cluster_streams = cluster_streams.write().unwrap();
                    myself.ping(
                        &GossipedNode::from(node),
                        &cluster_view,
                        &mut cluster_streams,
                    );
                }
            }
        });
    }

    fn handle_incoming_packet(
        myself: &ClusterNode,
        mut stream: TcpStream,
        cluster_view: Arc<RwLock<ClusterView>>,
        logger_tx: &Sender<LogMsg>,
    ) {
        let mut reader = BufReader::new(&mut stream);
        loop {
            match reader.fill_buf() {
                Ok(bytes) if !bytes.is_empty() => {
                    let packet = Packet::try_from(bytes).unwrap();

                    logger_tx
                        .send(log::cluster_debug!(
                            "recibido {} del nodo {}",
                            packet.data,
                            hex::encode(packet.header.id)
                        ))
                        .unwrap();

                    match packet.data {
                        PacketData::Fail(_) => todo!(),
                        PacketData::Gossip(data) => {
                            let mut cluster_view = cluster_view.write().unwrap();

                            Self::handle_gossip_packet(
                                myself,
                                &mut cluster_view,
                                packet.header,
                                data.cluster_view,
                                logger_tx,
                            );

                            if data.kind == GossipDataKind::Meet {
                                return;
                            }
                        }
                        PacketData::Update(_) => todo!(),
                    };

                    let length = bytes.len();
                    reader.consume(length);
                }
                Ok(_) => continue,
                Err(_) => continue,
            }
        }
    }

    fn handle_gossip_packet(
        myself: &ClusterNode,
        cluster_view: &mut ClusterView,
        packet_header: PacketHeader,
        gossip_data: Vec<GossipedNode>,
        logger_tx: &Sender<LogMsg>,
    ) {
        cluster_view.insert(
            packet_header.id,
            Node::Complete(ClusterNode {
                id: packet_header.id,
                ip: packet_header.ip,
                port: packet_header.port,
                cluster_port: packet_header.cluster_port,
                config_epoch: 0,
            }),
        );

        for node in gossip_data {
            if node.id != myself.id
                && cluster_view
                    .insert(node.id, Node::Incomplete(node))
                    .is_none()
            {
                logger_tx
                    .send(log::cluster_debug!(
                        "conocido al nodo {}",
                        hex::encode(node.id)
                    ))
                    .unwrap();

                logger_tx
                    .send(log::cluster_debug!(
                        "cluster view actual abarca {} nodos",
                        cluster_view.len()
                    ))
                    .unwrap();
            }
        }
    }

    fn pick_random_nodes(nodes: &[Node]) -> impl Iterator<Item = &Node> {
        let n = ((nodes.len() as f64) * CLUSTER_VIEW_SHARE_FACTOR).ceil() as usize;
        let n = n.max(1);

        nodes.choose_multiple(&mut rand::rng(), n)
    }
}

#[cfg(test)]
mod tests {
    use rand::Rng;

    use super::*;

    #[test]
    fn se_seleccionan_cero_random_peers_cuando_cluster_view_esta_vacia() {
        let cluster_view: ClusterView = HashMap::new();

        let nodes: Vec<_> = cluster_view.values().cloned().collect();
        let random_peers = ClusterActor::pick_random_nodes(&nodes);

        assert_eq!(random_peers.count(), 0);
    }

    #[test]
    fn se_selecciona_una_porcion_del_cluster_view_de_manera_aleatoria() {
        let mut nodes = Vec::new();

        for i in 0..100 {
            let mut id = [0_u8; 20];
            rand::rng().fill(&mut id);

            nodes.push(Node::Complete(ClusterNode {
                id,
                ip: "127.0.0.1".parse().unwrap(),
                port: "6379".parse::<u16>().unwrap() + i,
                cluster_port: "16379".parse::<u16>().unwrap() + i,
                config_epoch: 0,
            }));
        }

        let random_nodes = ClusterActor::pick_random_nodes(&nodes);

        assert_eq!(
            random_nodes.count(),
            ((nodes.len() as f64) * CLUSTER_VIEW_SHARE_FACTOR) as usize
        );
    }
}
