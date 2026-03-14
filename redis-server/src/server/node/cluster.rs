mod command;
mod error;
mod fail;
mod failover;
mod flags;
mod gossip;
pub mod message;
mod pub_sub;
mod replication;
mod sharding;

use std::{
    collections::{HashMap, hash_map::Entry},
    fmt,
    io::{BufRead, BufReader},
    net::{Ipv4Addr, SocketAddr, TcpListener, TcpStream},
    sync::mpsc::{self, Receiver, Sender},
    thread,
    time::{Duration, SystemTime, UNIX_EPOCH},
};

pub use error::InternalError;
use fail::FailureReport;
use flags::Flags;
use log::Log;
use message::{
    Message, MessageHeader, MessagePayload,
    payload::{GossipKind, GossipNode, GossipPayload, PublishPayload},
};
use rand::{
    Rng,
    seq::{IndexedRandom, IteratorRandom},
};
use redis_cmd::cluster::ClusterCommand;
use redis_resp::BulkString;

use crate::{config::ClusterConfig, server::node::storage::StorageAction};

const CLUSTER_GOSSIP_SHARE_FACTOR: f64 = 0.25;
const CLUSTER_SLOTS: u16 = 16384;

type NodeId = [u8; 20];

#[derive(Debug)]
pub struct ClusterActor {
    myself: ClusterNode,
    current_epoch: u64,
    last_vote_epoch: u64,
    cluster_view: HashMap<NodeId, ClusterNode>,
    cluster_streams: HashMap<NodeId, TcpStream>,
    timeout_millis: u64,
    failure_reports: HashMap<NodeId, HashMap<NodeId, FailureReport>>,
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
    ClientCommand {
        cmd: ClusterCommand,
        reply_tx: Sender<Vec<u8>>,
    },
    ClusterMessage(Message),
    TickPing,
    CheckFailures,
    ConfirmFailure {
        id: NodeId,
    },
    BroadcastPublish {
        channel: BulkString,
        message: BulkString,
    },
    RedirectToHoldingNode {
        key: BulkString,
        redir_tx: Sender<Option<Vec<u8>>>,
    },
}

impl ClusterActor {
    /// Inicia el ciclo principal del actor de clúster y lanza los threads de gestión.
    /// Configura el servidor, el loop de ping y el loop de chequeo de fallos.
    /// Devuelve un canal para enviar acciones al actor.
    pub fn start(
        config: ClusterConfig,
        storage_tx: Sender<StorageAction>,
        pub_sub_tx: Sender<PublishPayload>,
        log_tx: Sender<Log>,
    ) -> Result<Sender<ClusterAction>, InternalError> {
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
            current_epoch: 0,
            last_vote_epoch: 0,
            cluster_view: HashMap::new(),
            cluster_streams: HashMap::new(),
            timeout_millis: 10000,
            failure_reports: HashMap::new(),
            storage_tx,
            pub_sub_tx,
            votes_received: 0,
            failover_epoch: None,
            failover_start: None,
            failover_in_progress: false,
            failover_timeout_millis: 5000,
        };

        let (actions_tx, actions_rx) = mpsc::channel();

        log_tx.send(log::info!(
            "iniciando nodo {}",
            hex::encode(cluster_actor.myself.id)
        ))?;

        let addr = SocketAddr::new(
            cluster_actor.myself.ip.into(),
            cluster_actor.myself.cluster_port,
        );

        Self::start_cluster_server(addr, actions_tx.clone(), &log_tx)?;
        Self::start_ping_tick_loop(Duration::from_millis(1000), actions_tx.clone());

        Self::start_failure_check_loop(
            Duration::from_millis(cluster_actor.timeout_millis / 2),
            actions_tx.clone(),
        );

        cluster_actor.start_handling_actions(actions_tx.clone(), actions_rx, log_tx);

        Ok(actions_tx)
    }

    fn start_cluster_server(
        addr: SocketAddr,
        actions_tx: Sender<ClusterAction>,
        log_tx: &Sender<Log>,
    ) -> Result<(), InternalError> {
        let cluster_listener = TcpListener::bind(addr).map_err(InternalError::ClusterPortBind)?;
        let log_tx_clone = log_tx.clone();
        thread::spawn(move || Self::listen_cluster(cluster_listener, actions_tx, &log_tx_clone));
        log_tx.send(log::info!("servidor escuchando cluster en {addr:?}"))?;
        Ok(())
    }

    fn start_ping_tick_loop(interval: Duration, actions_tx: Sender<ClusterAction>) {
        thread::spawn(move || {
            loop {
                thread::sleep(interval);
                let _ = actions_tx.send(ClusterAction::TickPing);
            }
        });
    }

    fn start_failure_check_loop(interval: Duration, actions_tx: Sender<ClusterAction>) {
        thread::spawn(move || {
            loop {
                thread::sleep(interval);
                let _ = actions_tx.send(ClusterAction::CheckFailures);
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

    fn listen_cluster(
        listener: TcpListener,
        actions_tx: Sender<ClusterAction>,
        log_tx: &Sender<Log>,
    ) {
        for stream in listener.incoming() {
            let mut stream = stream.unwrap();

            let log_tx = log_tx.clone();
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
                            let _ = actions_tx.send(ClusterAction::ClusterMessage(msg));
                            if drop_stream {
                                return;
                            }
                        }
                        Ok(_) => {
                            let _ = log_tx.send(log::warn!(
                                "nodo conocido ha cerrado su canal de comunicación"
                            ));
                            return;
                        }
                        Err(err) => {
                            let _ = log_tx.send(log::warn!(
                                "error en canal de comunicación con cluster: {err}"
                            ));
                            return;
                        }
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
                ClusterAction::ClientCommand {
                    cmd,
                    reply_tx: client_tx,
                } => self.handle_cmd(cmd, client_tx, &log_tx),
                ClusterAction::ClusterMessage(msg) => {
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

                    if self.failover_in_progress
                        && let (Some(start), Some(_epoch)) =
                            (self.failover_start, self.failover_epoch)
                            && SystemTime::now().duration_since(start).unwrap()
                                > Duration::from_millis(self.failover_timeout_millis)
                            {
                                let should_stop_failover =
                                    if self.myself.flags.contains(flags::FLAG_MASTER) {
                                        true
                                    } else if let Some(master_id) = self.myself.master_id {
                                        if let Some(master) = self.cluster_view.get(&master_id) {
                                            !master.flags.contains(flags::FLAG_FAIL)
                                        } else {
                                            false
                                        }
                                    } else {
                                        false
                                    };

                                if should_stop_failover {
                                    let _ = log_tx.send(log::debug!(
                                        "finalizando failover - situación resuelta"
                                    ));

                                    self.failover_in_progress = false;
                                    self.failover_epoch = None;
                                    self.failover_start = None;
                                } else {
                                    let _ = log_tx.send(log::info!(
                                        "reiniciando failover: current epoch pasa de {} a {}",
                                        self.current_epoch,
                                        self.current_epoch + 1
                                    ));

                                    self.current_epoch += 1;
                                    self.votes_received = 0;
                                    self.request_failover(&log_tx);
                                }
                            }
                }
                ClusterAction::BroadcastPublish { channel, message } => {
                    self.broadcast_publish(channel, message, &actions_tx, &log_tx);
                }
                ClusterAction::ConfirmFailure { id } => self.confirm_failure(id, &log_tx),
                ClusterAction::RedirectToHoldingNode { key, redir_tx } => {
                    self.redirect_to_holding_node(&key, redir_tx, &log_tx)
                }
            }
        }
    }

    fn handle_incoming_message(&mut self, msg: Message, log_tx: &Sender<Log>) {
        let _ = log_tx.send(log::gossip!(
            "recibido {} de nodo {}",
            msg.payload,
            hex::encode(msg.header.id)
        ));

        self.add_message_sender_to_cluster_view(&msg.header, log_tx);

        if msg.header.config_epoch > self.current_epoch {
            let _ = log_tx.send(log::info!(
                "actualizando current epoch de {} a {}",
                self.current_epoch,
                msg.header.config_epoch,
            ));

            self.current_epoch = msg.header.config_epoch;
        }

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
        }
    }

    fn handle_config_epoch_collision(&mut self, header: &MessageHeader, log_tx: &Sender<Log>) {
        if header.config_epoch != self.myself.config_epoch {
            return;
        }
        if !header.flags.contains(flags::FLAG_MASTER)
            || !self.myself.flags.contains(flags::FLAG_MASTER)
        {
            return;
        }
        if header.id >= self.myself.id {
            let _ = log_tx
                .send(log::info!(
                    "ignorado colision de config epoch con nodo {} porque mi id es {} y el del nodo que me hablo {}",
                    hex::encode(header.id),
                    hex::encode(self.myself.id),
                    hex::encode(header.id),
                ));

            let _ = log_tx.send(log::info!(
                "config epoch queda en {}",
                self.myself.config_epoch
            ));

            return;
        }
        self.current_epoch += 1;
        self.myself.config_epoch = self.current_epoch;

        let _ = log_tx.send(log::warn!(
                "colision de config epoch con nodo {} -> config epoch actualizado a {} porque mi id es {} y el del nodo que me hablo {}",
            hex::encode(header.id),
            self.myself.config_epoch,
            hex::encode(self.myself.id),
            hex::encode(header.id)
        ));
    }

    fn add_message_sender_to_cluster_view(&mut self, header: &MessageHeader, log_tx: &Sender<Log>) {
        self.handle_config_epoch_collision(header, log_tx);

        match self.cluster_view.entry(header.id) {
            Entry::Occupied(mut entry) => {
                let known_node = entry.get_mut();

                if known_node.flags.contains(flags::FLAG_PFAIL)
                    && !header.flags.contains(flags::FLAG_PFAIL)
                    && !header.flags.contains(flags::FLAG_FAIL)
                {
                    let _ = log_tx.send(log::info!(
                        "removido PFAIL de nodo {}",
                        hex::encode(known_node.id)
                    ));
                }

                known_node.flags = header.flags;

                known_node.config_epoch = header.config_epoch;
                known_node.slots = header.slots;
                known_node.master_id = header.master_id;

                // Si este nodo es replica y el mensaje viene de su master, actualizar mis slots
                if let Some(my_master_id) = self.myself.master_id
                    && my_master_id == header.id && header.flags.contains(flags::FLAG_MASTER) {
                        let old_slots = self.myself.slots;
                        self.myself.slots = header.slots;

                        if old_slots != header.slots {
                            let _ = log_tx.send(log::info!(
                                "actualizados mis slots de {}-{} a {}-{} recibidos del master {}",
                                old_slots.0,
                                old_slots.1,
                                header.slots.0,
                                header.slots.1,
                                hex::encode(header.id)
                            ));
                        }
                    }
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

                // Si este nodo es replica y el nuevo nodo es su master, actualizar mis slots
                if let Some(my_master_id) = self.myself.master_id
                    && my_master_id == header.id && header.flags.contains(flags::FLAG_MASTER) {
                        let old_slots = self.myself.slots;
                        self.myself.slots = header.slots;

                        if old_slots != header.slots {
                            let _ = log_tx.send(log::info!(
                                "actualizados mis slots de {}-{} a {}-{} al conocer a mi master {}",
                                old_slots.0,
                                old_slots.1,
                                header.slots.0,
                                header.slots.1,
                                hex::encode(header.id)
                            ));
                        }
                    }
            }
        };

        self.check_for_same_slots(header, log_tx);
    }

    fn check_for_same_slots(&mut self, header: &MessageHeader, log_tx: &Sender<Log>) {
        if self.myself.flags.contains(flags::FLAG_MASTER)
            && header.flags.contains(flags::FLAG_MASTER)
            && self.myself.slots == header.slots
            && self.myself.slots != (0, 0)
        {
            if self.myself.config_epoch < header.config_epoch
                || (self.myself.config_epoch == header.config_epoch && self.myself.id < header.id)
            {
                let _ = log_tx.send(log::warn!(
                    "conflicto de slots -> me hago replica de nodo {} con config epoch {} porque mi config epoch es {}",
                    hex::encode(header.id),
                    header.config_epoch,
                    self.myself.config_epoch
                ));

                let _ = log_tx.send(log::info!("¡Perdido elección!"));

                self.set_master(header.id, log_tx).unwrap();

                self.failover_in_progress = false;
                self.failover_epoch = None;
                self.failover_start = None;
            } else {
                let _ = log_tx.send(log::warn!(
                    "conflicto de slots -> nodo {} con config epoch {} debería hacerse slave mío porque mi config epoch es {}",
                    hex::encode(header.id),
                    header.config_epoch,
                    self.myself.config_epoch,
                ));

                let _ = log_tx.send(log::info!("¡Ganado elección!"));
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
        let addr = format!("{}:{}@{}", ip, port, cluster_port);
        let master = master_id.map(hex::encode).unwrap_or("-".to_string());

        let ping_sent = ping_sent.duration_since(UNIX_EPOCH).unwrap().as_millis();

        let pong_received = pong_received
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_millis();

        let slots = if self.master_id.is_some() {
            if slots != &(0, 0) {
                format!("\x1b[38;2;107;114;128m{}-{}\x1b[0m", slots.0, slots.1)
            } else {
                "".to_string()
            }
        } else if slots != &(0, 0) {
            if flags.contains(flags::FLAG_FAIL) {
                format!(
                    "\x1b[38;2;107;114;128m\x1b[9m{}-{}\x1b[29m\x1b[0m",
                    slots.0, slots.1
                )
            } else {
                format!("\x1b[92m{}-{}\x1b[0m", slots.0, slots.1)
            }
        } else {
            "".to_string()
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
        let cluster_config = ClusterConfig::default(Ipv4Addr::new(0, 0, 0, 0), 6379);

        let (storage_tx, _) = mpsc::channel();
        let (pub_sub_tx, _) = mpsc::channel();
        let myself = ClusterNode::from(&cluster_config);

        ClusterActor {
            myself,
            current_epoch: 0,
            last_vote_epoch: 0,
            cluster_view: HashMap::new(),
            cluster_streams: HashMap::new(),
            timeout_millis: cluster_config.node_timeout as u64,
            failure_reports: HashMap::new(),
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
            ip: Ipv4Addr::new(0, 0, 0, 0),
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
            ip: Ipv4Addr::new(0, 0, 0, 0),
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
}
