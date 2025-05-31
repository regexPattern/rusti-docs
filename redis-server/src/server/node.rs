mod cluster;
mod error;
mod pub_sub;
mod storage;

use std::io::{BufReader, Write};
use std::path::PathBuf;
use std::{
    io::BufRead,
    net::TcpStream,
    sync::mpsc::{self, Sender},
};

use cluster::{ClusterActor, ClusterEnvelope};
pub use error::InternalError;
use log::LogMsg;
use pub_sub::{PubSubBroker, PubSubEnvelope};
use redis_cmd::Command;
use redis_resp::SimpleError;
use storage::{StorageActor, StorageEnvelope};

use crate::config::ClusterConfig;

#[derive(Debug)]
pub struct Node {
    broker_tx: Sender<PubSubEnvelope>,
    storage_tx: Sender<StorageEnvelope>,
    cluster_tx: Option<Sender<ClusterEnvelope>>,
    logger_tx: Sender<LogMsg>,
    //VER GOSSIP.RS PARA VER QUE ES NODEINFO
    //mi info xd
    // pub info: NodeInfo,

    //cluster view
    // pub conocidos: HashMap<String, NodeInfo>,

    //pueden estar fallando
    // pub sospechosos: Vec<SuspectNode>
}

impl Node {
    pub fn start(
        append_file_path: PathBuf,
        logger_tx: Sender<LogMsg>,
    ) -> Result<Self, InternalError> {
        let broker_tx = PubSubBroker::start(logger_tx.clone());
        let storage_tx = StorageActor::start(append_file_path, logger_tx.clone())?;

        Ok(Self {
            broker_tx,
            storage_tx,
            cluster_tx: None,
            logger_tx,
        })
    }

    pub fn enable_cluster_mode(&mut self, config: ClusterConfig) -> Result<(), ()> {
        let cluster_tx = ClusterActor::start(config, self.logger_tx.clone()).unwrap();
        self.cluster_tx = Some(cluster_tx);
        Ok(())
    }

    pub fn handle_client(&self, mut client: TcpStream) -> Result<(), InternalError> {
        let mut reader = BufReader::new(&mut client);

        let bytes = match reader.fill_buf() {
            Ok(bytes) if !bytes.is_empty() => bytes,
            Ok(_) => {
                self.logger_tx.send(log::debug!("cliente desconectado"))?;
                return Ok(());
            }
            Err(err) => {
                return Err(InternalError::StreamRead(err));
            }
        };

        self.logger_tx
            .send(log::debug!("recibidos {} bytes del cliente", bytes.len()))?;

        let cmd = Command::try_from(bytes);

        match cmd {
            Ok(cmd) => {
                self.logger_tx
                    .send(log::info!("procesando comando {cmd}"))?;
                self.execute_command(client, cmd)?;
                Ok(())
            }
            Err(err) => {
                self.logger_tx
                    .send(log::info!("comando recibido es inválido {err}"))?;

                client
                    .write_all(&Vec::from(SimpleError::from(err)))
                    .map_err(InternalError::StreamWrite)?;

                Ok(())
            }
        }
    }

    fn execute_command(&self, mut client: TcpStream, cmd: Command) -> Result<(), InternalError> {
        let (reply_tx, reply_rx) = mpsc::channel();

        match cmd {
            Command::Storage(cmd) => {
                self.storage_tx.send(StorageEnvelope { cmd, reply_tx })?;

                if let Ok(reply) = reply_rx.recv() {
                    client
                        .write_all(&reply)
                        .map_err(InternalError::StreamWrite)?;
                }
            }
            Command::PubSub(cmd) => {
                self.broker_tx.send(PubSubEnvelope { client, cmd })?;
            }
            Command::Cluster(cmd) => {
                if let Some(cluster_tx) = &self.cluster_tx {
                    cluster_tx.send(ClusterEnvelope { cmd, reply_tx })?;
                    if let Ok(reply) = reply_rx.recv() {
                        client
                            .write_all(&reply)
                            .map_err(InternalError::StreamWrite)?;
                    }
                } else {
                    self.logger_tx
                        .send(log::warn!(
                            "recibido comando de cluster sin tener modo cluster activado"
                        ))
                        .unwrap();
                }
            }
        };

        Ok(())
    }

    // pub fn handle_incomming_gossip(&self, client_conn: TcpStream) -> Result<(), InternalError> {

    //     //READ GOSSIP MSG IMPLEMENTAR READ WRITE FROM UNA VEZ DEFINIDO EL ENVELOPE Y Q TIENE

    //     // self.update_cluster_view(client_conn, cmd)?;
    // }

    // fn update_cluster_view(
    //     &self,
    //     mut gossip_envelope: ,

    // ) -> Result<(), > {

    //     // UPDATE CLUSTER VIEW CON NODOS CONOCIDOS
    //     //ESTO VNDRIA A SER LA CLUSTER VIEW

    //     // UPDATE CLUSTER VIEW CON NODOS SOSPECHOSOS
    //     //nodo que podrian estar fallanndo

    // }

    // pub fn hacer_gossip(
    //     yo: &NodeInfo,
    //     conocidos: &HashMap<String, NodeInfo>,
    //     sospechosos: Vec<SuspectNode>,
    // ) -> Result<(), Box<dyn std::error::Error>> {

    //     // Elegir un nodo al azar distinto a vos y que esté vivo
    //     //node <node_info?

    //     // Armar mensaje leyendo mi clustyer info
    //     // let msg = GossipMessage {
    //     //     from: yo.clone(),
    //     //     known_nodes: conocidos.values().cloned().collect(),
    //     //     suspect_nodes: sospechosos,
    //     //     timestamp: Utc::now().timestamp() as u64,
    //     // };

    //     //SERIALIZAR GOSSIP MESSAGE
    //     //let gossip_env_ser = goosip_en.to_bytes()

    //     // MANDAMOS EL GOSSIP
    //     // let addr = format!("{}:{}", nodo.ip, nodo.gossip_port);
    //     // println!("Conectando a nodo {} en {}", nodo.id, addr);

    //     // let mut stream = TcpStream::connect(addr)?;
    //     // stream.write_all(&gossip_env_ser)?;
    //     // println!("Mensaje gossip enviado a {}", nodo.id);

    //     Ok(())
    // }
}
