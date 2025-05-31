mod message;

use std::{
    collections::HashMap,
    io::{BufRead, BufReader, Write},
    net::{Ipv4Addr, SocketAddr, TcpListener, TcpStream},
    sync::{
        Arc, RwLock,
        mpsc::{self, Sender},
    },
    thread,
    time::Duration,
};

use crate::config::ClusterConfig;
use log::LogMsg;
use rand::{Rng, seq::IndexedRandom};
use redis_cmd::cluster::{ClusterCommand, Meet};
use redis_resp::BulkString;

use self::message::*;

#[derive(Debug)]
pub struct ClusterEnvelope {
    pub cmd: ClusterCommand,
    pub reply_tx: Sender<Vec<u8>>,
}

#[derive(Clone, Debug)]
pub struct ClusterActor {
    node: Arc<ClusterNode>,
    cluster_view: Arc<RwLock<Vec<GossipData>>>,
    cluster_streams: Arc<RwLock<HashMap<String, TcpStream>>>,
}

#[derive(Debug)]
pub struct ClusterNode {
    id: String,
    ip: Ipv4Addr,
    port: u16,
    cluster_port: u16,
}

impl ClusterActor {
    pub fn start(
        config: ClusterConfig,
        logger_tx: Sender<LogMsg>,
    ) -> Result<Sender<ClusterEnvelope>, ()> {
        let (tx, rx) = mpsc::channel();

        let ClusterActor {
            node,
            cluster_view,
            cluster_streams,
        } = Self::restore_from_file_or_create_new(config).unwrap();

        let node = Arc::new(node);

        logger_tx
            .send(log::info!("iniciando nodo {}", node.id))
            .unwrap();

        let node_clone = Arc::clone(&node);
        let cluster_view_clone = Arc::clone(&cluster_view);
        let logger_tx_clone = logger_tx.clone();

        thread::spawn(move || {
            while let Ok(envel) = rx.recv() {
                node_clone.process(envel, &logger_tx_clone);
            }
        });

        let addr = SocketAddr::new(node.ip.into(), node.port);
        let listener = TcpListener::bind(addr).unwrap();

        logger_tx
            .send(log::info!("servidor escuchando cluster en {:?}", addr))
            .unwrap();

        let logger_tx_clone = logger_tx.clone();

        thread::spawn(move || {
            for stream in listener.incoming() {
                let stream = stream.unwrap();
                let cluster_view = Arc::clone(&cluster_view_clone);
                let logger_tx = logger_tx_clone.clone();

                thread::spawn(move || {
                    Self::handle_peer_comm(stream, cluster_view, &logger_tx);
                });
            }
        });

        let logger_tx_clone = logger_tx.clone();

        thread::spawn(move || {
            loop {
                thread::sleep(Duration::from_secs(3));

                let cluster_view = cluster_view.read().unwrap();

                if let Some(peer) = cluster_view.choose(&mut rand::rng()) {
                    let mut cluster_streams = cluster_streams.write().unwrap();
                    node.ping(peer, &mut cluster_streams, &logger_tx_clone);
                }
            }
        });

        Ok(tx)
    }

    fn restore_from_file_or_create_new(config: ClusterConfig) -> Result<Self, ()> {
        let mut id = [0_u8; 20];
        rand::rng().fill(&mut id);
        let id = hex::encode(id);

        let node = ClusterNode {
            id,
            ip: config.bind,
            port: config.port,
            cluster_port: config.port,
        };

        Ok(Self {
            node: Arc::new(node),
            cluster_view: Arc::new(RwLock::new(Vec::new())),
            cluster_streams: Arc::new(RwLock::new(HashMap::new())),
        })
    }

    fn handle_peer_comm(
        mut stream: TcpStream,
        cluster_view: Arc<RwLock<Vec<GossipData>>>,
        logger_tx: &Sender<LogMsg>,
    ) {
        let mut reader = BufReader::new(&mut stream);
        loop {
            match reader.fill_buf() {
                Ok(bytes) => {
                    if bytes.len() == 0 {
                        continue;
                    }

                    let msg_data = String::from_utf8_lossy(bytes);
                    let msg_data: Vec<_> = msg_data.split(';').collect();
                    let (kind, id, ip, port, cluster_port) = (
                        msg_data[0],
                        msg_data[1],
                        msg_data[2],
                        msg_data[3],
                        msg_data[4],
                    );

                    if kind == "MEET" {
                        logger_tx
                            .send(log::info!("recibido MEET de nodo {id}"))
                            .unwrap();
                    } else {
                        logger_tx
                            .send(log::info!("recibido PING de nodo {id}"))
                            .unwrap();
                    }

                    let mut cluster_view = cluster_view.write().unwrap();
                    cluster_view.push(GossipData {
                        id: id.parse().unwrap(),
                        ip: ip.parse().unwrap(),
                        port: port.parse().unwrap(),
                        cluster_port: cluster_port.parse().unwrap(),
                    });

                    let length = bytes.len();
                    reader.consume(length);
                }
                Err(_) => continue,
            }
        }
    }
}

impl ClusterNode {
    fn process(&self, envel: ClusterEnvelope, logger_tx: &Sender<LogMsg>) {
        let reply = match envel.cmd {
            ClusterCommand::Meet(Meet {
                ip,
                port,
                cluster_port,
            }) => self.meet(ip, port, cluster_port, logger_tx),
            ClusterCommand::FailOver(_fail_over) => todo!(),
            ClusterCommand::Info(_info) => todo!(),
            ClusterCommand::Nodes(_nodes) => todo!(),
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
            port.parse::<u16>().unwrap()
        } else {
            port.parse::<u16>().unwrap() + 10000
        };

        let addr = SocketAddr::new(ip.into(), port);
        let mut stream = TcpStream::connect(addr).unwrap();

        logger_tx
            .send(log::debug!("establecida handshake con el nodo {:?}", addr))
            .unwrap();

        let random_nodes = Vec::from([]);

        let msg = ClusterMsg {
            id: self.id.clone(),
            ip: self.ip,
            port: self.port,
            cluster_port: self.cluster_port,
            data: ClusterMsgData::Meet(random_nodes),
        };

        stream.write_all(&msg.into_bytes()).unwrap();

        Vec::new() // TODO respuesta del comando MEET
    }

    fn ping(
        &self,
        peer: &GossipData,
        cluster_streams: &mut HashMap<String, TcpStream>,
        logger_tx: &Sender<LogMsg>,
    ) {
        let stream = cluster_streams.entry(peer.id.clone()).or_insert(
            TcpStream::connect(SocketAddr::new(peer.ip, peer.cluster_port)).unwrap(),
        );

        let msg = ClusterMsg {
            id: self.id.clone(),
            ip: self.ip,
            port: self.port,
            cluster_port: self.cluster_port,
            data: ClusterMsgData::Ping(vec![]),
        };

        stream.write_all(&msg.into_bytes()).unwrap();

        logger_tx
            .send(log::info!("enviado PING a nodo {}", peer.id))
            .unwrap();
    }
}
