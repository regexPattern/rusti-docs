use std::{
    io::Write,
    net::{Ipv4Addr, SocketAddr, TcpStream},
    sync::{Arc, RwLock, mpsc::Sender},
};

use log::LogMsg;
use redis_cmd::cluster::{ClusterCommand, Meet};
use redis_resp::{BulkString, SimpleString};

use super::{
    ClusterActor, ClusterEnvelope, ClusterStreams, ClusterView, Node, NodeId,
    packet::{GossipData, GossipDataKind, Packet, PacketData},
};

#[derive(Copy, Clone, Debug)]
pub struct ClusterNode {
    pub id: NodeId,
    pub ip: Ipv4Addr,
    pub port: u16,
    pub cluster_port: u16,
    pub config_epoch: u64,
}

#[derive(Copy, Clone, Debug)]
pub struct GossipedNode {
    pub id: NodeId,
    pub ip: Ipv4Addr,
    pub port: u16,
    pub cluster_port: u16,
}

impl From<&Node> for GossipedNode {
    fn from(node: &Node) -> Self {
        match node {
            Node::Complete(node) => Self {
                id: node.id,
                ip: node.ip,
                port: node.port,
                cluster_port: node.cluster_port,
            },
            Node::Incomplete(node) => Self {
                id: node.id,
                ip: node.ip,
                port: node.port,
                cluster_port: node.cluster_port,
            },
        }
    }
}

impl ClusterNode {
    pub fn process(
        &self,
        envel: ClusterEnvelope,
        cluster_view: &Arc<RwLock<ClusterView>>,
        logger_tx: &Sender<LogMsg>,
    ) {
        let reply = match envel.cmd {
            ClusterCommand::FailOver(_fail_over) => todo!(),
            ClusterCommand::Info(_info) => todo!(),
            ClusterCommand::Meet(Meet {
                ip,
                port,
                cluster_port,
            }) => {
                let cluster_view = cluster_view.read().unwrap();
                self.meet(ip, port, cluster_port, &cluster_view, logger_tx)
            }
            ClusterCommand::Nodes(_) => todo!(),
            ClusterCommand::Shards(_shards) => todo!(),
        };

        envel.reply_tx.send(reply).unwrap();
    }

    fn meet(
        &self,
        ip: BulkString,
        port: BulkString,
        cluster_port: Option<BulkString>,
        cluster_view: &ClusterView,
        logger_tx: &Sender<LogMsg>,
    ) -> Vec<u8> {
        let ip: Ipv4Addr = ip.to_string().parse().unwrap();
        let port = if let Some(port) = cluster_port {
            port.parse().unwrap()
        } else {
            port.parse::<u16>().unwrap() + 10000
        };

        let addr = SocketAddr::new(ip.into(), port);
        let mut stream = TcpStream::connect(addr).unwrap();

        logger_tx
            .send(log::debug!("establecido handshake con el nodo {:?}", addr))
            .unwrap();

        let nodes: Vec<_> = cluster_view.values().copied().collect();

        let random_nodes: Vec<_> = ClusterActor::pick_random_nodes(&nodes)
            .map(GossipedNode::from)
            .collect();

        let packet = Packet {
            header: self.into(),
            data: PacketData::Gossip(GossipData {
                kind: GossipDataKind::Meet,
                cluster_view: random_nodes,
            }),
        };

        stream.write_all(&Vec::from(packet)).unwrap();

        SimpleString::from("OK").into()
    }

    pub fn ping(
        &self,
        node: &GossipedNode,
        cluster_view: &ClusterView,
        cluster_streams: &mut ClusterStreams,
    ) {
        let stream = cluster_streams.entry(node.id).or_insert_with(|| {
            TcpStream::connect(SocketAddr::new(node.ip.into(), node.cluster_port)).unwrap()
        });

        let nodes: Vec<_> = cluster_view
            .values()
            .filter(|n| matches!(n, Node::Complete(_)))
            .copied()
            .collect();

        let random_nodes: Vec<_> = ClusterActor::pick_random_nodes(&nodes)
            .map(GossipedNode::from)
            .collect();

        let packet = Packet {
            header: self.into(),
            data: PacketData::Gossip(GossipData {
                kind: GossipDataKind::Ping,
                cluster_view: random_nodes,
            }),
        };

        stream.write_all(&Vec::from(packet)).unwrap();
    }
}
