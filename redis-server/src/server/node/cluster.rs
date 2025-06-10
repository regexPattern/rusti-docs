mod client;
mod flags;
pub mod message;
mod node;

use std::{
    collections::{HashMap, hash_map::Entry},
    io::{BufRead, BufReader, Write},
    net::{SocketAddr, TcpListener, TcpStream},
    sync::mpsc::{self, Receiver, Sender},
    thread,
    time::{Duration, SystemTime, UNIX_EPOCH},
};

use log::Log;
use node::ClusterNode;
use rand::{
    Rng,
    seq::{IndexedRandom, IteratorRandom},
};
use redis_cmd::cluster::ClusterCommand;

use redis_resp::{BulkString};

use flags::{FLAG_MYSELF, Flags};
use message::{Message, gossip::GossipNode, PublishData};

use crate::{
    config::ClusterConfig,
    server::node::{
        cluster::{
            flags::{FLAG_FAIL, FLAG_MASTER, FLAG_PFAIL, FLAG_SLAVE},
            message::{
                MessageData, MessageHeader,
                fail::FailData,
                failover::{FailOverData, FailOverKind},
                gossip::{GossipData, GossipKind},
            },
        },
        storage::StorageAction,
    },
};

const CLUSTER_SLOTS: usize = 16384;
const CLUSTER_GOSSIP_SHARE_FACTOR: f64 = 0.25;

type NodeId = [u8; 20];

#[derive(Debug)]
pub struct ClusterState {
    myself: ClusterNode,
    config_epoch: u64,
    current_epoch: u64,
    cluster_view: HashMap<NodeId, ClusterNode>,
    cluster_streams: HashMap<NodeId, TcpStream>,
    timeout_millis: u64,
    failure_reports: HashMap<NodeId, HashMap<NodeId, SystemTime>>,
    replication_stream: Option<TcpStream>,
    storage_tx: Sender<StorageAction>,
    pub_sub_tx: Sender<PublishData>,
}

#[derive(Debug)]
pub enum ClusterAction {
    ClientCommand {
        cmd: ClusterCommand,
        client_tx: Sender<Vec<u8>>,
    },
    ClusterMessage {
        msg: Message,
    },
    Ping,
    CheckFailures,
    ConfirmFailure {
        id: NodeId,
    },
    BroadCastPublish{
        channel: BulkString,
        message: BulkString,
    }
}

impl ClusterState {
    pub fn start(
        config: ClusterConfig,
        storage_tx: Sender<StorageAction>,
        pub_sub_tx: Sender<PublishData>,
        log_tx: Sender<Log>,
    ) -> Sender<ClusterAction> {
        let mut id = [0_u8; 20];
        rand::rng().fill(&mut id);

        let state = ClusterState {
            myself: ClusterNode {
                id,
                ip: config.bind,
                port: config.port,
                cluster_port: config.cluster_port,
                flags: Flags(FLAG_MYSELF | FLAG_MASTER),
                ping_sent: UNIX_EPOCH,
                pong_received: UNIX_EPOCH,
                config_epoch: 0,
                master_id: None,
                slots: [0; CLUSTER_SLOTS / 8],
            },
            config_epoch: 0,
            current_epoch: 0,
            cluster_view: HashMap::new(),
            cluster_streams: HashMap::new(),
            timeout_millis: 10000,
            failure_reports: HashMap::new(),
            replication_stream: None,
            storage_tx,
            pub_sub_tx,
        };

        let (actions_tx, actions_rx) = mpsc::channel();

        log_tx
            .send(log::info!(
                "iniciando nodo {}",
                hex::encode(state.myself.id)
            ))
            .unwrap();

        let addr = SocketAddr::new(state.myself.ip.into(), state.myself.cluster_port);
        let listener = TcpListener::bind(addr).unwrap();

        log_tx
            .send(log::info!("servidor escuchando cluster en {addr:?}"))
            .unwrap();

        {
            let actions_tx = actions_tx.clone();
            thread::spawn(move || Self::listen_cluster(listener, actions_tx));
        }

        {
            let actions_tx = actions_tx.clone();
            thread::spawn(move || {
                let interval = Duration::from_millis((state.timeout_millis / 2) / 3);
                loop {
                    thread::sleep(interval);
                    actions_tx.send(ClusterAction::Ping).unwrap();
                }
            });
        }

        {
            let actions_tx = actions_tx.clone();
            thread::spawn(move || {
                let interval = Duration::from_millis(state.timeout_millis / 2);
                loop {
                    thread::sleep(interval);
                    actions_tx.send(ClusterAction::CheckFailures).unwrap();
                }
            });
        }

        {
            let actions_tx = actions_tx.clone();
            let log_tx = log_tx.clone();
            thread::spawn(move || state.handle_actions(actions_tx, actions_rx, log_tx));
        }

        actions_tx
    }

    fn listen_cluster(listener: TcpListener, actions_tx: Sender<ClusterAction>) {
        for stream in listener.incoming() {
            let mut stream = stream.unwrap();
            let actions_tx = actions_tx.clone();

            thread::spawn(move || {
                let mut reader = BufReader::new(&mut stream);

                loop {
                    match reader.fill_buf() {
                        Ok(bytes) if !bytes.is_empty() => {
                            let msg = Message::from(bytes);

                            let length = bytes.len();
                            reader.consume(length);

                            let mut drop_stream = false;
                            if let MessageData::Gossip(GossipData { kind, nodes: _ }) = &msg.data {
                                drop_stream = *kind == GossipKind::Meet;
                            }

                            actions_tx
                                .send(ClusterAction::ClusterMessage { msg })
                                .unwrap();

                            if drop_stream {
                                return;
                            }
                        }
                        Ok(_) => return,
                        Err(_) => todo!(),
                    }
                }
            });
        }
    }

    fn handle_actions(
        mut self,
        actions_tx: Sender<ClusterAction>,
        actions_rx: Receiver<ClusterAction>,
        log_tx: Sender<Log>,
    ) {
        for action in actions_rx {
            match action {
                ClusterAction::ClientCommand { cmd, client_tx } => {
                    self.process_command(cmd, client_tx, &log_tx)
                }
                ClusterAction::ClusterMessage { msg } => {
                    self.handle_cluster_message(msg, &log_tx);
                }
                ClusterAction::Ping => {
                    let random_nodes = self.select_random_nodes();
                    if let Some(node) = random_nodes.choose(&mut rand::rng()) {
                        self.ping(node);
                    }
                }
                ClusterAction::CheckFailures => {
                    self.check_failures(&actions_tx, &log_tx);
                }
                ClusterAction::BroadCastPublish { channel, message } => {
                    self.broadcast_pub_sub(channel, message, &actions_tx, &log_tx);
                }
                ClusterAction::ConfirmFailure { id } => self.fail(id),

            }
        }
    }

    fn broadcast_pub_sub(
        &mut self,
        channel: BulkString,
        message: BulkString,
        _actions_tx: &Sender<ClusterAction>,
        log_tx: &Sender<Log>,
    ) {
        use crate::server::node::cluster::message::{
            Message, MessageHeader, MessageData, publish::PublishData,
        };

        let msg = Message {
            header: MessageHeader::from(&self.myself),
            data: MessageData::Publish(PublishData { channel, message }),
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


    fn handle_cluster_message(&mut self, msg: Message, log_tx: &Sender<Log>) {
        log_tx
            .send(log::debug!(
                "recibido {} de nodo {}",
                msg.data,
                hex::encode(msg.header.id)
            ))
            .unwrap();

        match self.cluster_view.entry(msg.header.id) {
            Entry::Occupied(mut entry) => {
                let known_node = entry.get_mut();
                known_node.ip = msg.header.ip;
                known_node.port = msg.header.port;
                known_node.cluster_port = msg.header.cluster_port;
                known_node.flags = msg.header.flags;
                known_node.config_epoch = msg.header.config_epoch;
                known_node.slots = msg.header.slots;
                known_node.master_id = msg.header.master_id;
            }
            Entry::Vacant(entry) => {
                entry.insert(ClusterNode::from(&msg.header));
            }
        };

        match msg.data {
            MessageData::Gossip(data) => self.handle_gossip_message(&msg.header, data, log_tx),
            MessageData::Fail(data) => self.handle_fail_message(data),
            MessageData::Publish(data) => {
                self.handle_publish(data);
            }
            MessageData::FailOver(_) => self.handle_fail_over(&msg.header),
            MessageData::Update(_data) => todo!(),
        }
    }

    fn handle_publish(&mut self, data: PublishData) {
        self.pub_sub_tx.send(data);
    }

    fn handle_gossip_message(
        &mut self,
        header: &MessageHeader,
        data: GossipData,
        log_tx: &Sender<Log>,
    ) {
        for gossip_node in data.nodes {
            if gossip_node.id == self.myself.id {
                continue;
            }

            match self.cluster_view.entry(gossip_node.id) {
                Entry::Occupied(mut entry) => {
                    let known_node = entry.get_mut();
                    // TODO realmente deberiamos actualizar solo con los epochs
                    if known_node.flags != gossip_node.flags {
                        known_node.ip = gossip_node.ip;
                        known_node.port = gossip_node.port;
                        known_node.cluster_port = gossip_node.cluster_port;

                        log_tx
                            .send(log::debug!(
                                "actualizada informacion de nodo {}",
                                hex::encode(gossip_node.id),
                            ))
                            .unwrap();
                    }
                }
                Entry::Vacant(entry) => {
                    log_tx
                        .send(log::info!(
                            "conocido al nodo {} mediante gossip",
                            hex::encode(gossip_node.id),
                        ))
                        .unwrap();
                    entry.insert(ClusterNode::from(&gossip_node));
                }
            };

            let sender_is_master = self
                .cluster_view
                .get(&header.id)
                .and_then(|n| Some(n.flags.contains(FLAG_MASTER)))
                .is_some();

            if gossip_node.flags.contains(FLAG_PFAIL) && sender_is_master {
                let reports = self.failure_reports.entry(gossip_node.id).or_default();
                reports.insert(header.id, SystemTime::now());
            }
        }

        if data.kind == GossipKind::Ping {
            self.pong(header);
        } else if data.kind == GossipKind::Pong {
            let node = self.cluster_view.get_mut(&header.id).unwrap();
            node.pong_received = SystemTime::now();
        }
    }

    fn ping(&mut self, node: &GossipNode) {
        let msg = Message {
            header: MessageHeader::from(&self.myself),
            data: MessageData::Gossip(GossipData {
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

    fn pong(&mut self, header: &MessageHeader) {
        let msg = Message {
            header: MessageHeader::from(&self.myself),
            data: MessageData::Gossip(GossipData {
                kind: GossipKind::Pong,
                nodes: self.select_random_nodes(),
            }),
        };

        let stream = self.cluster_streams.entry(header.id).or_insert_with(|| {
            TcpStream::connect(SocketAddr::new(header.ip.into(), header.cluster_port)).unwrap()
        });

        let _ = stream.write_all(&Vec::from(&msg));
    }

    fn fail(&mut self, id: NodeId) {
        let msg = Message {
            header: MessageHeader::from(&self.myself),
            data: MessageData::Fail(FailData { id }),
        };

        for stream in self.cluster_streams.values_mut() {
            let _ = stream.write_all(&Vec::from(&msg));
        }
    }

    fn handle_fail_message(&mut self, data: FailData) {
        let node = self.cluster_view.get_mut(&data.id).unwrap();
        node.flags.0 |= FLAG_FAIL;
        node.flags.0 &= !FLAG_PFAIL;

        if Some(node.id) == self.myself.master_id {
            self.start_election();
        }
    }

    fn select_random_nodes(&self) -> Vec<GossipNode> {
        let n = ((self.cluster_view.len() as f64) * CLUSTER_GOSSIP_SHARE_FACTOR).ceil() as usize;
        let n = n.max(1);

        self.cluster_view
            .values()
            .map(GossipNode::from)
            .choose_multiple(&mut rand::rng(), n)
    }

    fn check_failures(&mut self, actions_tx: &Sender<ClusterAction>, log_tx: &Sender<Log>) {
        let now = SystemTime::now();

        for node in self.cluster_view.values_mut() {
            if now.duration_since(node.pong_received).unwrap()
                > Duration::from_millis(self.timeout_millis)
                && !node.flags.contains(FLAG_FAIL)
            {
                node.flags.0 |= FLAG_PFAIL;

                log_tx
                    .send(log::warn!(
                        "nodo {} marcado como PFAIL",
                        hex::encode(node.id)
                    ))
                    .unwrap();
            }
        }

        let n_known_masters = self
            .cluster_view
            .values()
            .filter(|n| n.flags.contains(FLAG_MASTER))
            .count();

        for (id, reports) in &mut self.failure_reports {
            if self.cluster_view.get(id).unwrap().flags.contains(FLAG_FAIL) {
                continue;
            };

            reports.retain(|_, t| {
                now.duration_since(*t).unwrap() < Duration::from_millis(self.timeout_millis * 2)
            });

            let lo_tengo_pfail = if self
                .cluster_view
                .get(id)
                .unwrap()
                .flags
                .contains(FLAG_PFAIL)
            {
                1
            } else {
                0
            };

            let soy_master = if self.myself.flags.contains(FLAG_MASTER) {
                1
            } else {
                0
            };

            if (reports.len() + lo_tengo_pfail) as f64
                >= (((n_known_masters + soy_master) / 2 + 1) as f64).floor()
            {
                actions_tx
                    .send(ClusterAction::ConfirmFailure { id: *id })
                    .unwrap();

                let node = self.cluster_view.get_mut(id).unwrap();
                node.flags.0 |= FLAG_FAIL;
                node.flags.0 &= !FLAG_PFAIL;

                reports.clear();
            }
        }
    }

    fn start_election(&mut self) {
        // todo suponemos por ahora que solo las replicas hacen esto

        let master_id = self.myself.master_id.unwrap();
        let master = self.cluster_view.get(&master_id).unwrap();

        if !master.flags.contains(FLAG_FAIL) {
            return;
        }

        self.current_epoch += 1;

        let msg = Message {
            header: MessageHeader::from(&self.myself),
            data: MessageData::FailOver(FailOverData {
                kind: FailOverKind::AuthRequest,
            }),
        };

        let bytes = Vec::from(&msg);

        for stream in self.cluster_streams.values_mut() {
            let _ = stream.write_all(&bytes);
        }
    }

    fn handle_fail_over(&mut self, header: &MessageHeader) {
        // el nodo que me mando esto va a ser una replica.

        if !self.myself.flags.contains(FLAG_MASTER) {
            self.myself.flags.0 &= !FLAG_SLAVE;
            self.myself.flags.0 |= FLAG_MASTER;

            return;
        }

        let stream = self.cluster_streams.get_mut(&header.id).unwrap();

        let msg = Message {
            header: MessageHeader::from(&self.myself),
            data: MessageData::FailOver(FailOverData {
                kind: FailOverKind::AuthAck,
            }),
        };

        stream.write_all(&Vec::from(&msg)).unwrap();
    }
}
