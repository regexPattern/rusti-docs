use std::{
    io::Write,
    net::{Ipv4Addr, SocketAddr, TcpStream},
    sync::{Arc, RwLock, mpsc::Sender},
};

use log::LogMsg;
use redis_cmd::cluster::{ClusterCommand, Meet};
use redis_resp::{BulkString, SimpleString};

use super::{
    packet::{GossipData, GossipDataKind, Packet, PacketData}, ClusterActor, ClusterEnvelope, ClusterStreams, ClusterView, NodeId, Peer
};

#[derive(Copy, Clone, Debug)]
pub struct Myself {
    pub id: NodeId,
    pub ip: Ipv4Addr,
    pub port: u16,
    pub cluster_port: u16,
}

impl Myself {
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
            }) => self.meet(ip, port, cluster_port, logger_tx),
            ClusterCommand::Nodes(_) => {
                let cluster_view = cluster_view.read().unwrap();
                self.nodes(&cluster_view)
            }
            ClusterCommand::Shards(_shards) => todo!(),
        };

        envel.reply_tx.send(reply).unwrap();
    }

    fn meet(
        &self,
        ip: BulkString,
        port: BulkString,
        cluster_port: Option<BulkString>,
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

        let packet = Packet {
            header: self.into(),
            data: PacketData::Gossip(GossipData {
                kind: GossipDataKind::Meet,
                cluster_view: vec![self.into()],
            }),
        };

        stream.write_all(&Vec::from(packet)).unwrap();

        SimpleString::from("OK").into()
    }

    pub fn ping(
        &self,
        peer: &Peer,
        cluster_view: &ClusterView,
        cluster_streams: &mut ClusterStreams,
    ) {
        let stream = cluster_streams.entry(peer.id).or_insert_with(|| {
            TcpStream::connect(SocketAddr::new(peer.ip.into(), peer.cluster_port)).unwrap()
        });

        let mut cluster_view: Vec<_> = cluster_view.values().copied().collect();
        cluster_view.push(self.into());

        let random_peer: Vec<_> = ClusterActor::pick_random_peers(&cluster_view)
            .copied()
            .collect();

        let packet = Packet {
            header: self.into(),
            data: PacketData::Gossip(GossipData {
                kind: GossipDataKind::Ping,
                cluster_view: random_peer,
            }),
        };

        stream.write_all(&Vec::from(packet)).unwrap();
    }

    fn nodes(&self, cluster_view: &ClusterView) -> Vec<u8> {
        let mut output = vec![format!("{}", Peer::from(self))];

        for peer in cluster_view.values() {
            output.push(format!("{peer}"));
        }

        BulkString::from(output.join("\n")).into()
    }
}

// TODO realmente tenemos que eliminar esto y obtener guardarnos la información del header que nos
// manda el nodo, no incluirnos a nosotros mismos en la cluster view que compartimos.

impl From<&Myself> for Peer {
    fn from(myself: &Myself) -> Self {
        Self {
            id: myself.id,
            ip: myself.ip,
            port: myself.port,
            cluster_port: myself.cluster_port,
        }
    }
}
