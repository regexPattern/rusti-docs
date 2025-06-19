mod command;
mod fail;
mod failover;
mod flags;
mod gossip;
pub mod message;
mod publish;

use std::{
    collections::{HashMap, hash_map::Entry},
    fmt,
    io::{BufRead, BufReader},
    net::{Ipv4Addr, SocketAddr, TcpListener, TcpStream},
    sync::mpsc::{self, Receiver, Sender},
    thread,
    time::{Duration, SystemTime, UNIX_EPOCH},
};

use log::Log;
use rand::{
    Rng,
    seq::{IndexedRandom, IteratorRandom},
};
use redis_cmd::cluster::ClusterCommand;
use redis_resp::BulkString;

use fail::FailureReport;
use flags::Flags;
use message::{
    Message, MessageHeader, MessagePayload,
    payload::{GossipKind, GossipNode, GossipPayload, PublishPayload},
};

use crate::{config::ClusterConfig, server::node::storage::StorageAction};

const CLUSTER_GOSSIP_SHARE_FACTOR: f64 = 0.25;

type NodeId = [u8; 20];

// ¿Qué valor poner en el header?
// Para mensajes de gossip y actualización de nodos:
// header.config_epoch = self.myself.config_epoch;
//Para mensajes de failover:
//header.config_epoch = self.current_epoch;

#[derive(Debug)]
pub struct ClusterActor {
    myself: ClusterNode,
    config_epoch: u64,
    current_epoch: u64,
    last_vote_epoch: u64,
    cluster_view: HashMap<NodeId, ClusterNode>,
    cluster_streams: HashMap<NodeId, TcpStream>,
    timeout_millis: u64,
    failure_reports: HashMap<NodeId, HashMap<NodeId, FailureReport>>,
    replication_stream: Option<TcpStream>,
    storage_tx: Sender<StorageAction>,
    pub_sub_tx: Sender<PublishPayload>,
    votes_received: u64,
    failover_epoch: Option<u64>,
    failover_start: Option<SystemTime>,
    failover_in_progress: bool,
    failover_timeout_millis: u64,
}

#[derive(Debug)]
pub enum ClusterAction {
    ClientCmd {
        cmd: ClusterCommand,
        client_tx: Sender<Vec<u8>>,
    },
    ClusterMsg(Message),
    TickPing,
    CheckFailures,
    ConfirmFailure {
        id: NodeId,
    },
    BroadcastPublish {
        channel: BulkString,
        message: BulkString,
    },
}

impl ClusterActor {
    pub fn start(
        config: ClusterConfig,
        storage_tx: Sender<StorageAction>,
        pub_sub_tx: Sender<PublishPayload>,
        log_tx: Sender<Log>,
    ) -> Sender<ClusterAction> {
        let mut id = [0_u8; 20];
        rand::rng().fill(&mut id);

        let cluster_actor = ClusterActor {
            myself: ClusterNode {
                id,
                ip: config.bind,
                port: config.port,
                cluster_port: config.cluster_port,
                flags: Flags(flags::FLAG_MYSELF | flags::FLAG_MASTER),
                ping_sent: UNIX_EPOCH,
                pong_received: UNIX_EPOCH,
                config_epoch: 0,
                slots: (0, 0),
                master_id: None,
            },
            config_epoch: 0,
            current_epoch: 0,
            last_vote_epoch: 0,
            cluster_view: HashMap::new(),
            cluster_streams: HashMap::new(),
            timeout_millis: 10000,
            failure_reports: HashMap::new(),
            replication_stream: None,
            storage_tx,
            pub_sub_tx,
            votes_received: 0,
            failover_epoch: None,
            failover_start: None,
            failover_in_progress: false,
            failover_timeout_millis: 5000,
        };

        let (actions_tx, actions_rx) = mpsc::channel();

        log_tx
            .send(log::info!(
                "iniciando nodo {}",
                hex::encode(cluster_actor.myself.id)
            ))
            .unwrap();

        let addr = SocketAddr::new(
            cluster_actor.myself.ip.into(),
            cluster_actor.myself.cluster_port,
        );

        Self::start_cluster_server(addr, actions_tx.clone(), &log_tx);
        Self::start_ping_tick_loop(Duration::from_millis(1000), actions_tx.clone());

        Self::start_failure_check_loop(
            Duration::from_millis(cluster_actor.timeout_millis / 2),
            actions_tx.clone(),
        );

        cluster_actor.start_handling_actions(actions_tx.clone(), actions_rx, log_tx);

        actions_tx
    }

    fn start_cluster_server(
        addr: SocketAddr,
        actions_tx: Sender<ClusterAction>,
        log_tx: &Sender<Log>,
    ) {
        let listener = TcpListener::bind(addr).unwrap();
        thread::spawn(move || Self::listen_cluster(listener, actions_tx));

        log_tx
            .send(log::info!("servidor escuchando cluster en {addr:?}"))
            .unwrap();
    }

    fn start_ping_tick_loop(interval: Duration, actions_tx: Sender<ClusterAction>) {
        thread::spawn(move || {
            loop {
                thread::sleep(interval);
                actions_tx.send(ClusterAction::TickPing).unwrap();
            }
        });
    }

    fn start_failure_check_loop(interval: Duration, actions_tx: Sender<ClusterAction>) {
        thread::spawn(move || {
            loop {
                thread::sleep(interval);
                actions_tx.send(ClusterAction::CheckFailures).unwrap();
            }
        });
    }

    fn start_handling_actions(
        self,
        actions_tx: Sender<ClusterAction>,
        actions_rx: Receiver<ClusterAction>,
        log_tx: Sender<Log>,
    ) {
        thread::spawn(move || self.handle_actions(actions_tx, actions_rx, log_tx));
    }

    fn listen_cluster(listener: TcpListener, actions_tx: Sender<ClusterAction>) {
        for stream in listener.incoming() {
            let mut stream = stream.unwrap();
            let actions_tx = actions_tx.clone();

            thread::spawn(move || {
                let mut reader = BufReader::new(&mut stream);

                loop {
                    match reader.fill_buf() {
                        Ok(bytes) if bytes.len() >= std::mem::size_of::<usize>() => {
                            let (msg, consumed) = Message::from(bytes);
                            reader.consume(consumed);

                            let mut drop_stream = false;
                            if let MessagePayload::Gossip(GossipPayload { kind, nodes: _ }) =
                                &msg.payload
                            {
                                drop_stream = *kind == GossipKind::Meet;
                            }

                            actions_tx.send(ClusterAction::ClusterMsg(msg)).unwrap();

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
                ClusterAction::ClientCmd { cmd, client_tx } => {
                    self.handle_cmd(cmd, client_tx, &log_tx)
                }
                ClusterAction::ClusterMsg(msg) => {
                    self.handle_incoming_message(msg, &log_tx);
                }
                ClusterAction::TickPing => {
                    let now = SystemTime::now();
                    let prioritized_nodes: Vec<_> = self
                        .cluster_view
                        .values()
                        .filter(|n| {
                            now.duration_since(n.pong_received).unwrap()
                                > Duration::from_millis(self.timeout_millis)
                        })
                        .map(GossipNode::from)
                        .collect();

                    for node in &prioritized_nodes {
                        self.ping(node);
                    }

                    let random_nodes = self.select_random_nodes();
                    if let Some(node) = random_nodes.choose(&mut rand::rng()) {
                        self.ping(node);
                    }
                }
                ClusterAction::CheckFailures => {
                    self.check_failures(&actions_tx, &log_tx);

                    if self.failover_in_progress {
                        if let (Some(start), Some(epoch)) =
                            (self.failover_start, self.failover_epoch)
                        {
                            if SystemTime::now().duration_since(start).unwrap()
                                > Duration::from_millis(self.failover_timeout_millis)
                            {
                                if !self.myself.flags.contains(flags::FLAG_MASTER) {
                                    //si aun no soy master...
                                    log_tx
                                        .send(log::info!(
                                            "Reiniciando failover: current_epoch pasa de {} a {}",
                                            self.current_epoch,
                                            self.current_epoch + 1
                                        ))
                                        .unwrap();
                                    self.current_epoch += 1;
                                    self.votes_received = 0;
                                    self.request_failover(&log_tx);
                                } else {
                                    //apago la eleccion
                                    self.failover_in_progress = false;
                                    self.failover_epoch = None;
                                    self.failover_start = None;
                                }
                            }
                        }
                    }
                }
                ClusterAction::BroadcastPublish { channel, message } => {
                    self.broadcast_publish(channel, message, &actions_tx, &log_tx);
                }
                ClusterAction::ConfirmFailure { id } => self.confirm_failure(id, &log_tx),
            }
        }
    }

    fn handle_incoming_message(&mut self, msg: Message, log_tx: &Sender<Log>) {
        log_tx
            .send(log::gossip!(
                "recibido {} de nodo {}",
                msg.payload,
                hex::encode(msg.header.id)
            ))
            .unwrap();

        self.add_message_sender_to_cluster_view(&msg.header, log_tx);

        if msg.header.config_epoch > self.current_epoch {
            log_tx
                .send(log::info!(
                    "actualizando current epoch de {} a {}",
                    self.current_epoch,
                    msg.header.config_epoch,
                ))
                .unwrap();

            self.current_epoch = msg.header.config_epoch;
        }

        //si el header anuncia MISMOS SLOTS
        //puedo preguntar por la conifg epoch pero sin peros

        match msg.payload {
            MessagePayload::Gossip(payload) => {
                self.handle_gossip_message(&msg.header, payload, log_tx)
            }
            MessagePayload::Fail(payload) => self.handle_fail_message(payload, log_tx),
            MessagePayload::Publish(payload) => {
                self.handle_publish_msg(payload);
            }
            MessagePayload::FailOver(payload) => {
                self.handle_failover_message(payload, &msg.header, log_tx)
            }
            MessagePayload::Update(_payload) => todo!(),
        }
    }

    fn handle_config_epoch_collision(&mut self, header: &MessageHeader, log_tx: &Sender<Log>) {
        // Prerequisitos: ambos son master y tienen el mismo config_epoch
        if header.config_epoch != self.myself.config_epoch {
            return;
        }
        if !header.flags.contains(flags::FLAG_MASTER) || !self.myself.flags.contains(flags::FLAG_MASTER) {
            return;
        }
        // No actuar si el otro nodo tiene un NodeId menor o igual
        if header.id <= self.myself.id {
            log_tx
                .send(log::info!(
                    "ignorado configEpoch collision con nodo {} xq mi id es {} y el del nodo que me hablo {}
                    mi epoch queda en {}",
                    hex::encode(header.id),
                    hex::encode(self.myself.id),
                    hex::encode(header.id),
                    self.myself.config_epoch,
                ))
                .unwrap();
            return;
        }
        self.current_epoch += 1;
        self.myself.config_epoch = self.current_epoch;
        // self.save_config();

        log_tx
            .send(log::warn!(
                "WARNING: configEpoch collision con nodo {}. configEpoch actualizado a {}
                xq mi id es {} y el del nodo que me hablo {}",
                hex::encode(header.id),
                self.myself.config_epoch,
                hex::encode(self.myself.id),
                hex::encode(header.id)
            ))
            .unwrap();
    }

    fn add_message_sender_to_cluster_view(&mut self, header: &MessageHeader, log_tx: &Sender<Log>) {
        
        self.handle_config_epoch_collision(header, log_tx);

        // log_tx
        //         .send(log::info!(
        //             "mi config epoch es de: {}",
        //             self.myself.config_epoch,
        //         ))
        //         .unwrap();

        match self.cluster_view.entry(header.id) {
            Entry::Occupied(mut entry) => {
                // TODO realmente solo deberiamos hacer esto si el otro tiene mayor config epoch.
                let known_node = entry.get_mut();

                if known_node.flags.contains(flags::FLAG_PFAIL)
                    && !header.flags.contains(flags::FLAG_PFAIL)
                    && !header.flags.contains(flags::FLAG_FAIL)
                {
                    log_tx
                        .send(log::info!(
                            "removido PFAIL de nodo {}",
                            hex::encode(known_node.id)
                        ))
                        .unwrap();
                }

                known_node.flags = header.flags;

                known_node.config_epoch = header.config_epoch;
                known_node.slots = header.slots;
                known_node.master_id = header.master_id;
            }
            Entry::Vacant(entry) => {
                entry.insert(ClusterNode {
                    id: header.id,
                    ip: header.ip,
                    port: header.port,
                    cluster_port: header.cluster_port,
                    flags: header.flags,
                    ping_sent: UNIX_EPOCH,
                    pong_received: UNIX_EPOCH,
                    config_epoch: 0,
                    slots: header.slots,
                    master_id: header.master_id,
                });
            }
        };

        self.check_for_same_slots(header, log_tx);

    }

    fn check_for_same_slots(&mut self, header: &MessageHeader, log_tx: &Sender<Log>) {
        // Solo si yo soy master y el otro nodo también es master
        if self.myself.flags.contains(flags::FLAG_MASTER) && header.flags.contains(flags::FLAG_MASTER) {
            // Chequea si los slots coinciden y no son (0, 0)
            if self.myself.slots == header.slots && self.myself.slots != (0, 0) {
                if self.myself.config_epoch < header.config_epoch
                    // Yo debo hacerme slave del otro si mi config epoch es menor (o mi id es lexicografica// menor)
                    || (self.myself.config_epoch == header.config_epoch && self.myself.id < header.id)
                    //la verificion del id creo es inecesaria xq ya etsa hanlde_collision pero x las dudas
                {
                    // Yo debo hacerme slave del otro
                    log_tx.send(log::warn!(
                        "slots Conflict: me hago SLAVE de {} (config_epoch {}), xq mi config epoch es {}",
                        hex::encode(header.id),
                        header.config_epoch,
                        self.myself.config_epoch
                    )).unwrap();

                    self.myself.flags.0 &= !flags::FLAG_MASTER;
                    self.myself.flags.0 |= flags::FLAG_SLAVE;
                    self.myself.master_id = Some(header.id);
                    
                    //apago la eleecion
                    self.failover_in_progress = false;
                    self.failover_epoch = None;
                    self.failover_start = None;

                } else {
                    // El otro debería hacerse slave mío
                    //mandar update??
                    log_tx.send(log::warn!(
                        "Slots conflict: nodo {} (config epoch: {}) debería hacerse SLAVE mío xq mi config_epoch es {}",
                        hex::encode(header.id),
                        header.config_epoch,
                        self.myself.config_epoch,
                    )).unwrap();
                }
            }
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
}

#[derive(PartialEq)]
pub struct ClusterNode {
    pub id: NodeId,
    pub ip: Ipv4Addr,
    pub port: u16,
    pub cluster_port: u16,
    pub flags: Flags,
    pub ping_sent: SystemTime,
    pub pong_received: SystemTime,
    pub config_epoch: u64,
    pub slots: (u16, u16),
    pub master_id: Option<NodeId>,
}

impl From<&ClusterConfig> for ClusterNode {
    fn from(config: &ClusterConfig) -> Self {
        let mut id = [0_u8; 20];
        rand::rng().fill(&mut id);

        Self {
            id,
            ip: config.bind,
            port: config.port,
            cluster_port: config.cluster_port,
            flags: Flags(flags::FLAG_MASTER),
            ping_sent: UNIX_EPOCH,
            pong_received: UNIX_EPOCH,
            config_epoch: 0,
            slots: (0, 0),
            master_id: None,
        }
    }
}

impl fmt::Display for ClusterNode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let Self {
            id,
            ip,
            port,
            cluster_port,
            flags,
            ping_sent,
            pong_received,
            config_epoch,
            slots,
            master_id,
        } = self;

        let id = hex::encode(id);
        let addr = format!("{}:{}@{}", ip.to_string(), port, cluster_port);
        let master = master_id
            .map(|id| hex::encode(id))
            .unwrap_or("-".to_string());

        let ping_sent = ping_sent.duration_since(UNIX_EPOCH).unwrap().as_millis();

        let pong_received = pong_received
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_millis();

        let slots = if self.master_id.is_some() {
            "".to_string()
        } else {
            format!("{}-{}", slots.0, slots.1)
        };

        write!(
            f,
            "{id} {addr} {flags} {master} {ping_sent} {pong_received} {config_epoch} {slots}"
        )
    }
}

impl fmt::Debug for ClusterNode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{self}")
    }
}

#[cfg(test)]
mod tests {
    use std::net::Ipv4Addr;

    use super::*;

    pub fn dummy_empty_cluster_actor() -> ClusterActor {
        let cluster_config = ClusterConfig::default(Ipv4Addr::new(127, 0, 0, 1), 6379);

        let (storage_tx, _) = mpsc::channel();
        let (pub_sub_tx, _) = mpsc::channel();
        let myself = ClusterNode::from(&cluster_config);

        ClusterActor {
            myself,
            config_epoch: 0,
            current_epoch: 0,
            last_vote_epoch: 0,
            cluster_view: HashMap::new(),
            cluster_streams: HashMap::new(),
            timeout_millis: cluster_config.node_timeout as u64,
            failure_reports: HashMap::new(),
            replication_stream: None,
            storage_tx,
            pub_sub_tx,
            votes_received: 0,
            failover_epoch: None,
            failover_start: None,
            failover_in_progress: false,
            failover_timeout_millis: 5000,
        }
    }

    #[test]
    fn sender_de_un_msg_se_agrega_a_la_cluster_view() {
        let mut cluster_actor = dummy_empty_cluster_actor();

        assert!(cluster_actor.cluster_view.is_empty());

        let mut id = [0_u8; 20];
        hex::decode_to_slice("c7e10a92b34f5d861e05c9f7a8d3b2640e1f5a7d", &mut id).unwrap();

        let header = MessageHeader {
            id,
            ip: Ipv4Addr::new(127, 0, 0, 1),
            port: 7000,
            cluster_port: 17000,
            flags: Flags(flags::FLAG_MYSELF | flags::FLAG_MASTER),
            config_epoch: 0,
            slots: (0, 0),
            master_id: None,
        };

        let (log_tx, _log_rx) = mpsc::channel();

        cluster_actor.add_message_sender_to_cluster_view(&header, &log_tx);

        assert_eq!(
            cluster_actor.cluster_view.get(&header.id).unwrap(),
            &ClusterNode::from(&header)
        );
    }

    #[test]
    fn sender_de_un_msg_que_ya_estaba_en_cluster_view_se_actualiza() {
        let mut cluster_actor = dummy_empty_cluster_actor();

        let mut id = [0_u8; 20];
        hex::decode_to_slice("c7e10a92b34f5d861e05c9f7a8d3b2640e1f5a7d", &mut id).unwrap();

        let mut header = MessageHeader {
            id,
            ip: Ipv4Addr::new(127, 0, 0, 1),
            port: 7000,
            cluster_port: 17000,
            flags: Flags(flags::FLAG_MYSELF | flags::FLAG_MASTER),
            config_epoch: 0,
            slots: (0, 0),
            master_id: None,
        };

        let (log_tx, _log_rx) = mpsc::channel();

        cluster_actor.add_message_sender_to_cluster_view(&header, &log_tx);

        let mut master_id = [0_u8; 20];
        hex::decode_to_slice("73f0b8a1e9c4d5270618fa3c5bde6792401f8a7c", &mut master_id).unwrap();

        header.flags.0 &= !flags::FLAG_MASTER;
        header.flags.0 |= flags::FLAG_SLAVE;
        header.master_id = Some(master_id);

        let (log_tx, _log_rx) = mpsc::channel();

        cluster_actor.add_message_sender_to_cluster_view(&header, &log_tx);

        assert_eq!(
            cluster_actor.cluster_view.get(&header.id).unwrap(),
            &ClusterNode::from(&header)
        );
    }

    //LOGICA PARA NO VOTAR MAS DE UNA VEZ POR EPOCH !
    // let request_epoch = header.config_epoch;

    // if request_epoch > self.last_vote_epoch {
    //     self.last_vote_epoch = request_epoch;

    //     }
}
