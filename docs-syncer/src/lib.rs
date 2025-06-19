use std::{
    collections::{HashMap, HashSet},
    io::{BufReader, prelude::*},
    net::{Ipv4Addr, Shutdown, SocketAddr, TcpStream},
    sync::mpsc::{self, Sender},
    thread::{self, JoinHandle},
    time::Duration,
};

use chrono::Local;
use log;
use redis_cmd::{
    Command,
    pub_sub::{PubSubCommand, Publish, Subscribe},
    storage::{Get, HGetAll, HKeys},
};
use redis_resp::{BulkString, RespDataType};

use redis_cmd::storage::{HSet, Set, StorageCommand};

#[derive(Clone, Debug)]
pub struct DbAccessInfo {
    pub saved_docs_ids_key: String,
    pub saved_docs_ts_key: String,
    pub addr: SocketAddr,
}

#[derive(Debug)]
pub struct DocsSyncer {
    db_addr: SocketAddr,
    doc_listener_stream: TcpStream,
    connected_clients: HashMap<String, HashSet<String>>,
}

#[derive(Debug)]
pub enum DocsSyncerAction {
    CreateNewDocument {
        doc_id: BulkString,
        doc_basename: BulkString,
    },
    SubscribeToDocument {
        docs_ids: Vec<BulkString>,
    },
    ConnectClient {
        client_id: String,
        doc_id: String,
        doc_kind: String,
        doc_basename: String,
    },
    DisconnectClient {
        doc_id: String,
        client_id: String,
    },
    PublishConnectedClients,
    PersistDocument {
        doc_id: String,
        doc_kind: String,
        doc_content: String,
    },
}

impl DocsSyncer {
    pub fn new() -> Self {
        let db_addr = SocketAddr::new(Ipv4Addr::new(127, 0, 0, 1).into(), 7000);

        Self {
            db_addr,
            doc_listener_stream: TcpStream::connect(db_addr).unwrap(),
            connected_clients: HashMap::new(),
        }
    }

    pub fn start(mut self) {
        let db_addr = self.db_addr;
        let docs_listener_stream = self.doc_listener_stream.try_clone().unwrap();

        let (actions_tx, actions_rx) = mpsc::channel();

        let handle = thread::spawn(move || {
            for action in actions_rx {
                match action {
                    DocsSyncerAction::CreateNewDocument {
                        doc_id,
                        doc_basename,
                    } => {
                        self.create_new_doc(doc_id.clone(), doc_basename);
                        self.subscribe_to_doc(vec![doc_id]);
                    }
                    DocsSyncerAction::SubscribeToDocument { docs_ids } => {
                        self.subscribe_to_doc(docs_ids)
                    }
                    DocsSyncerAction::ConnectClient {
                        client_id,
                        doc_id,
                        doc_kind,
                        doc_basename,
                    } => {
                        self.connect_client(client_id, doc_id, doc_kind, doc_basename);
                    }
                    DocsSyncerAction::DisconnectClient { doc_id, client_id } => {
                        self.disconnect_client(doc_id, client_id)
                    }
                    DocsSyncerAction::PublishConnectedClients => self.publish_connected_clients(),
                    DocsSyncerAction::PersistDocument {
                        doc_id,
                        doc_kind,
                        doc_content,
                    } => self.persist_doc(doc_id, doc_kind, doc_content),
                }
            }
        });

        Self::start_docs_creator(db_addr, actions_tx.clone()).unwrap();
        Self::start_docs_listener(docs_listener_stream, actions_tx.clone());
        Self::start_connected_clients_publisher(actions_tx.clone());

        Self::subscribe_to_saved_docs(db_addr, actions_tx).unwrap();

        handle.join().unwrap();
    }

    fn create_new_doc(&mut self, doc_id: BulkString, doc_basename: BulkString) {
        let log_msg = log::info!("creado documento {doc_id} {doc_basename}");

        let mut stream = TcpStream::connect(self.db_addr).unwrap();

        let cmd = Command::Storage(StorageCommand::HSet(HSet {
            key: "docs_ids".into(),
            field_value_pairs: vec![doc_id.clone(), doc_basename],
        }));

        stream.write_all(&Vec::from(cmd)).unwrap();

        let mut stream = TcpStream::connect(self.db_addr).unwrap();

        let cmd = Command::Storage(StorageCommand::HSet(HSet {
            key: "docs_ts".into(),
            field_value_pairs: vec![doc_id, Local::now().to_rfc3339().into()],
        }));

        stream.write_all(&Vec::from(cmd)).unwrap();

        print!("{log_msg}");
    }

    fn subscribe_to_doc(&mut self, docs_ids: Vec<BulkString>) {
        let mut log_msg = String::from("suscrito a documentos");

        for doc_id in &docs_ids {
            log_msg.push_str(&format!(" {doc_id}"));
        }

        let cmd = Command::PubSub(PubSubCommand::Subscribe(Subscribe { channels: docs_ids }));

        self.doc_listener_stream.write_all(&Vec::from(cmd)).unwrap();

        print!("{}", log::info!("{log_msg}"));
    }

    fn connect_client(
        &mut self,
        client_id: String,
        doc_id: String,
        doc_kind: String,
        doc_basename: String,
    ) {
        if self
            .connected_clients
            .entry(doc_id.clone())
            .or_default()
            .insert(client_id.clone())
        {
            print!(
                "{}",
                log::debug!(
                    "registrando al cliente {client_id} en documento {doc_id} {doc_basename}",
                )
            );
        }

        print!(
            "{}",
            log::info!("cliente {client_id} accediendo al documento {doc_id}")
        );

        let content = match doc_kind.as_str() {
            "TEXT" => self.get_text_doc(&doc_id),
            "SPREADSHEET" => self.get_spreadsheet_doc(&doc_id),
            _ => todo!(),
        };

        let msg = format!("FETCH_ACK@{client_id}@{doc_kind}@{content}");

        let log_msg = log::debug!(
            "enviado al cliente {client_id} información de documento {doc_id} {doc_basename}"
        );

        let mut stream = TcpStream::connect(self.db_addr).unwrap();

        let cmd = Command::PubSub(PubSubCommand::Publish(Publish {
            channel: doc_id.into(),
            message: msg.into(),
        }));

        stream.write_all(&Vec::from(cmd)).unwrap();

        print!("{log_msg}");
    }

    fn disconnect_client(&mut self, doc_id: String, client_id: String) {
        if let Some(doc_clients) = self.connected_clients.get_mut(&doc_id) {
            doc_clients.remove(&client_id);
        }
    }

    fn get_text_doc(&self, doc_id: &str) -> BulkString {
        let mut stream = TcpStream::connect(self.db_addr).unwrap();

        let cmd = Command::Storage(StorageCommand::Get(Get { key: doc_id.into() }));

        stream.write_all(&Vec::from(cmd)).unwrap();

        stream.shutdown(Shutdown::Write).unwrap();

        let mut buffer = Vec::new();
        stream.read_to_end(&mut buffer).unwrap();

        match RespDataType::try_from(buffer.as_slice()).unwrap() {
            RespDataType::BulkString(content) => content,
            RespDataType::Null => BulkString::from(""),
            _ => todo!(),
        }
    }

    fn get_spreadsheet_doc(&self, doc_id: &str) -> BulkString {
        let mut stream = TcpStream::connect(self.db_addr).unwrap();

        let cmd = Command::Storage(StorageCommand::HGetAll(HGetAll { key: doc_id.into() }));

        stream.write_all(&Vec::from(cmd)).unwrap();

        stream.shutdown(Shutdown::Write).unwrap();

        let mut buffer = Vec::new();
        stream.read_to_end(&mut buffer).unwrap();

        let content = match RespDataType::try_from(buffer.as_slice()).unwrap() {
            RespDataType::Map(content) => {
                let mut cells: [[String; 10]; 10] = Default::default();

                for (i, v) in content {
                    let (i, v) = match (i, v) {
                        (RespDataType::BulkString(i), RespDataType::BulkString(v)) => {
                            (i.to_string(), v.to_string())
                        }
                        _ => todo!(),
                    };

                    let (fil, col) = i.split_once(',').unwrap();
                    let row: usize = fil.parse().unwrap();
                    let col: usize = col.parse().unwrap();

                    cells[row][col] = v.to_string();
                }

                cells
            }
            RespDataType::Null => Default::default(),
            _ => todo!(),
        };

        let values: Vec<_> = content.iter().flat_map(|row| row.iter()).collect();

        values
            .iter()
            .map(|v| v.to_string())
            .collect::<Vec<_>>()
            .join(",")
            .into()
    }

    fn subscribe_to_saved_docs(
        db_addr: SocketAddr,
        actions_tx: Sender<DocsSyncerAction>,
    ) -> Result<(), ()> {
        let cmd = Command::Storage(StorageCommand::HKeys(HKeys {
            key: "docs_ids".into(),
        }));

        let mut stream = TcpStream::connect(db_addr).unwrap();
        stream.write_all(&Vec::from(cmd)).unwrap();

        stream.shutdown(Shutdown::Write).unwrap();

        let mut buffer = Vec::new();
        stream.read_to_end(&mut buffer).unwrap();

        let docs_ids: Vec<_> = match RespDataType::try_from(buffer.as_slice()).unwrap() {
            RespDataType::Array(docs_ids) => docs_ids
                .into_iter()
                .filter_map(|i| {
                    if let RespDataType::BulkString(id) = i {
                        Some(id)
                    } else {
                        None
                    }
                })
                .collect(),
            _ => [].into(),
        };

        actions_tx
            .send(DocsSyncerAction::SubscribeToDocument { docs_ids })
            .unwrap();

        Ok(())
    }

    fn start_docs_creator(
        db_addr: SocketAddr,
        actions_tx: Sender<DocsSyncerAction>,
    ) -> Result<JoinHandle<()>, ()> {
        let cmd = Command::PubSub(PubSubCommand::Subscribe(Subscribe {
            channels: vec!["docs_syncer".into()],
        }));

        let mut stream = TcpStream::connect(db_addr).unwrap();
        stream.write_all(&Vec::from(cmd)).unwrap();

        Ok(thread::spawn(move || {
            let mut buffer = BufReader::new(&mut stream);

            loop {
                match buffer.fill_buf() {
                    Ok(bytes) if !bytes.is_empty() => {
                        Self::handle_docs_creator_msg(bytes, &actions_tx);
                        let length = bytes.len();
                        buffer.consume(length);
                    }
                    Ok(_) => todo!(),
                    Err(_) => todo!(),
                }
            }
        }))
    }

    fn handle_docs_creator_msg(bytes: &[u8], actions_tx: &Sender<DocsSyncerAction>) {
        let mut payload = match RespDataType::try_from(bytes).unwrap() {
            RespDataType::Array(payload) => payload.into_iter().filter_map(|e| {
                if let RespDataType::BulkString(e) = e {
                    Some(e)
                } else {
                    None
                }
            }),
            _ => todo!(), // deberia regresar un error porque un mensaje de pub/sub siempre deberia
                          // ser un array
        };

        if let Some(doc_metadata) = payload.nth(2) {
            let doc_metadata = doc_metadata.to_string();
            let doc_metadata = doc_metadata.split('@');
            let mut doc_metadata = doc_metadata.map(BulkString::from);

            actions_tx
                .send(DocsSyncerAction::CreateNewDocument {
                    doc_id: doc_metadata.next().unwrap(),
                    doc_basename: doc_metadata.next().unwrap(),
                })
                .unwrap();
        } else {
            print!(
                "{}",
                log::info!("iniciando canal de escucha de editores clientes")
            );
        }
    }

    fn start_docs_listener(
        mut docs_listener_stream: TcpStream,
        actions_tx: Sender<DocsSyncerAction>,
    ) {
        thread::spawn(move || {
            let mut buffer = BufReader::new(&mut docs_listener_stream);

            loop {
                match buffer.fill_buf() {
                    Ok(bytes) if !bytes.is_empty() => {
                        Self::handle_docs_listener_msg(bytes, &actions_tx);
                        let length = bytes.len();
                        buffer.consume(length);
                    }
                    Ok(_) => todo!(),
                    Err(_) => todo!(),
                }
            }
        });
    }

    fn start_connected_clients_publisher(actions_tx: Sender<DocsSyncerAction>) {
        thread::spawn(move || {
            loop {
                thread::sleep(Duration::from_millis(1000));
                actions_tx
                    .send(DocsSyncerAction::PublishConnectedClients)
                    .unwrap();
            }
        });
    }

    fn publish_connected_clients(&mut self) {
        for (doc_id, clients) in &self.connected_clients {
            if !clients.is_empty() {
                let clients_ids: Vec<_> = clients.iter().map(|c| c.as_str()).collect();

                let msg = format!("CLIENTS@{}", clients_ids.join(","));

                let cmd = Command::PubSub(PubSubCommand::Publish(Publish {
                    channel: doc_id.into(),
                    message: msg.into(),
                }));

                let mut stream = TcpStream::connect(self.db_addr).unwrap();

                stream.write_all(&Vec::from(cmd)).unwrap();
            }
        }
    }

    fn handle_docs_listener_msg(bytes: &[u8], actions_tx: &Sender<DocsSyncerAction>) {
        let mut payload = match RespDataType::try_from(bytes).unwrap() {
            RespDataType::Array(payload) => payload.into_iter().filter_map(|e| {
                if let RespDataType::BulkString(e) = e {
                    Some(e)
                } else {
                    None
                }
            }),
            _ => todo!(),
        };

        let doc_id = payload.nth(1).unwrap();

        if let Some(payload) = payload.next() {
            Self::handle_doc_actions(doc_id.to_string(), payload.to_string(), actions_tx);
        }
    }

    fn handle_doc_actions(doc_id: String, payload: String, actions_tx: &Sender<DocsSyncerAction>) {
        match payload.split_once('@').unwrap() {
            ("FETCH_REQ", payload) => {
                let mut payload = payload.splitn(3, '@').map(String::from);
                actions_tx
                    .send(DocsSyncerAction::ConnectClient {
                        client_id: payload.next().unwrap(),
                        doc_id,
                        doc_kind: payload.next().unwrap(),
                        doc_basename: payload.next().unwrap(),
                    })
                    .unwrap()
            }
            ("PATCH", payload) => {
                let mut payload = payload.splitn(3, '@').map(String::from);
                actions_tx
                    .send(DocsSyncerAction::PersistDocument {
                        doc_id,
                        doc_kind: payload.nth(1).unwrap(),
                        doc_content: payload.next().unwrap(),
                    })
                    .unwrap();
            }
            ("LEAVE", client_id) => {
                actions_tx
                    .send(DocsSyncerAction::DisconnectClient {
                        doc_id,
                        client_id: client_id.to_string(),
                    })
                    .unwrap();
            }
            _ => {}
        }
    }

    fn persist_doc(&self, doc_id: String, doc_kind: String, doc_content: String) {
        let persist_cmd = match doc_kind.as_str() {
            "TEXT" => self.persist_text_doc_cmd(&doc_id, &doc_content),
            "SPREADSHEET" => self.persist_spreadsheet_doc_cmd(&doc_id, &doc_content),
            _ => todo!(),
        };

        let mut stream = TcpStream::connect(self.db_addr).unwrap();

        stream
            .write_all(&Vec::from(Command::Storage(persist_cmd)))
            .unwrap();

        let mut stream = TcpStream::connect(self.db_addr).unwrap();

        let cmd = Command::Storage(StorageCommand::HSet(HSet {
            key: "docs_ts".into(),
            field_value_pairs: vec![BulkString::from(&doc_id), Local::now().to_rfc3339().into()],
        }));

        stream.write_all(&Vec::from(cmd)).unwrap();

        print!("{}", log::info!("persistido documento {}", doc_id));
    }

    fn persist_text_doc_cmd(&self, doc_id: &str, doc_content: &str) -> StorageCommand {
        StorageCommand::Set(Set {
            key: BulkString::from(doc_id),
            value: BulkString::from(doc_content),
        })
    }

    fn persist_spreadsheet_doc_cmd(&self, doc_id: &str, doc_content: &str) -> StorageCommand {
        let mut field_value_pairs = Vec::new();

        for (i, value) in doc_content.split(',').enumerate() {
            let row = i / 10;
            let col = i % 10;

            field_value_pairs.push(BulkString::from(format!("{row},{col}")));
            field_value_pairs.push(BulkString::from(value));
        }

        StorageCommand::HSet(HSet {
            key: BulkString::from(doc_id),
            field_value_pairs,
        })
    }
}
