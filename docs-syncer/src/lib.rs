mod clients;
mod error;
mod persistence;

use std::{
    collections::{HashMap, HashSet},
    io::{BufReader, prelude::*},
    net::{Ipv4Addr, Shutdown, SocketAddr, TcpStream},
    sync::mpsc::{self, Sender},
    thread::{self, JoinHandle},
};

use chrono::Local;
use log;
use redis_cmd::{
    Command,
    pub_sub::{PubSubCommand, Subscribe},
    storage::{Get, HGetAll, HKeys},
};
use redis_resp::{BulkString, RespDataType};

use redis_cmd::storage::{HSet, StorageCommand};

use crate::error::Error;

#[derive(Clone, Debug)]
pub struct DbAccessInfo {
    pub saved_docs_ids_key: String,
    pub saved_docs_ts_key: String,
    pub addr: SocketAddr,
}

#[derive(Debug)]
pub struct DocsSyncer {
    db_addr: SocketAddr,
    docs_stream: TcpStream,
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
        client_id: String,
        doc_id: String,
    },
    PublishConnectedClients,
    PersistDocument {
        doc_id: String,
        doc_kind: String,
        doc_content: String,
    },
}

impl DocsSyncer {
    pub fn new() -> Result<Self, Error> {
        let db_addr = SocketAddr::new(Ipv4Addr::new(127, 0, 0, 1).into(), 7000);

        Ok(Self {
            db_addr,
            docs_stream: TcpStream::connect(db_addr).map_err(Error::ConnectionError)?,
            connected_clients: HashMap::new(),
        })
    }

    pub fn start(mut self) -> Result<Vec<JoinHandle<Result<(), Error>>>, Error> {
        let db_addr = self.db_addr;

        let docs_stream = self
            .docs_stream
            .try_clone()
            .map_err(Error::ConnectionError)?;

        let (actions_tx, actions_rx) = mpsc::channel();

        let mut handles = Vec::new();

        handles.push(thread::spawn(move || {
            for action in actions_rx {
                match action {
                    DocsSyncerAction::CreateNewDocument {
                        doc_id,
                        doc_basename,
                    } => {
                        self.create_new_document(doc_id.clone(), doc_basename)?;
                        self.subscribe_to_document(vec![doc_id])?;
                    }
                    DocsSyncerAction::SubscribeToDocument { docs_ids } => {
                        self.subscribe_to_document(docs_ids)?;
                    }
                    DocsSyncerAction::ConnectClient {
                        client_id,
                        doc_id,
                        doc_kind,
                        doc_basename,
                    } => {
                        self.connect_client(client_id, doc_id, doc_kind, doc_basename)?;
                    }
                    DocsSyncerAction::DisconnectClient { client_id, doc_id } => {
                        self.disconnect_client(client_id, doc_id)
                    }
                    DocsSyncerAction::PublishConnectedClients => {
                        self.publish_connected_clients()?
                    }
                    DocsSyncerAction::PersistDocument {
                        doc_id,
                        doc_kind,
                        doc_content,
                    } => self.persist_doc(doc_id, doc_kind, doc_content),
                }
            }
            Ok(())
        }));

        handles.push(Self::start_documents_creator(db_addr, actions_tx.clone())?);

        handles.push(Self::start_documents_watcher(
            docs_stream,
            actions_tx.clone(),
        ));

        handles.push(Self::start_connected_clients_publisher(actions_tx.clone()));

        Self::subscribe_to_saved_documents(db_addr, actions_tx)?;

        Ok(handles)
    }

    fn create_new_document(
        &mut self,
        doc_id: BulkString,
        doc_basename: BulkString,
    ) -> Result<(), Error> {
        let log_msg = log::info!("creado documento {doc_id} {doc_basename}");

        let mut stream = TcpStream::connect(self.db_addr).map_err(Error::ConnectionError)?;

        let cmd = Command::Storage(StorageCommand::HSet(HSet {
            key: "docs_ids".into(),
            field_value_pairs: vec![doc_id.clone(), doc_basename],
        }));

        stream
            .write_all(&Vec::from(cmd))
            .map_err(Error::WriteError)?;

        let mut stream = TcpStream::connect(self.db_addr).map_err(Error::ConnectionError)?;

        let cmd = Command::Storage(StorageCommand::HSet(HSet {
            key: "docs_ts".into(),
            field_value_pairs: vec![doc_id, Local::now().to_rfc3339().into()],
        }));

        stream
            .write_all(&Vec::from(cmd))
            .map_err(Error::WriteError)?;

        print!("{log_msg}");

        Ok(())
    }

    fn subscribe_to_document(&mut self, docs_ids: Vec<BulkString>) -> Result<(), Error> {
        let mut log_msg = String::from("suscrito a documentos");

        for doc_id in &docs_ids {
            log_msg.push_str(&format!(" {doc_id}"));
        }

        let cmd = Command::PubSub(PubSubCommand::Subscribe(Subscribe { channels: docs_ids }));

        self.docs_stream
            .write_all(&Vec::from(cmd))
            .map_err(Error::WriteError)?;

        print!("{}", log::info!("{log_msg}"));

        Ok(())
    }

    fn get_text_content(&self, doc_id: &str) -> BulkString {
        let mut slot_addr = self.db_addr;
        let cmd = Vec::from(Command::Storage(StorageCommand::Get(Get {
            key: doc_id.into(),
        })));

        loop {
            let mut stream = TcpStream::connect(slot_addr).unwrap();
            stream.write_all(&cmd).unwrap();

            stream
                .shutdown(Shutdown::Write)
                .map_err(Error::ConnectionError)
                .unwrap();

            let mut buffer = Vec::new();
            stream
                .read_to_end(&mut buffer)
                .map_err(Error::ReadError)
                .unwrap();

            let reply = RespDataType::try_from(buffer.as_slice())
                .map_err(|_| Error::InvalidRespReply)
                .unwrap();

            match reply {
                RespDataType::BulkString(content) => {
                    return content;
                }
                RespDataType::Null => {
                    return BulkString::from("");
                }
                RespDataType::SimpleError(err) => {
                    let mut err = err.0.splitn(3, " ");
                    let redir_slot = err.nth(1).unwrap().parse().unwrap();
                    let redir_addr = err.next().unwrap().parse().unwrap();
                    slot_addr = SocketAddr::new(redir_slot, redir_addr);

                    continue;
                }
                _ => unreachable!(),
            }
        }
    }

    fn get_spreadsheet_content(&self, doc_id: &str) -> BulkString {
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

    fn subscribe_to_saved_documents(
        mut slot_addr: SocketAddr,
        actions_tx: Sender<DocsSyncerAction>,
    ) -> Result<(), Error> {
        let cmd = Vec::from(Command::Storage(StorageCommand::HKeys(HKeys {
            key: "docs_ids".into(),
        })));

        let saved_docs_ids = loop {
            let mut stream = TcpStream::connect(slot_addr).map_err(Error::ConnectionError)?;
            stream.write_all(&cmd).map_err(Error::WriteError)?;

            stream
                .shutdown(Shutdown::Write)
                .map_err(Error::ConnectionError)?;

            let mut buffer = Vec::new();
            stream.read_to_end(&mut buffer).map_err(Error::WriteError)?;

            let reply =
                RespDataType::try_from(buffer.as_slice()).map_err(|_| Error::InvalidRespReply)?;

            match reply {
                RespDataType::Array(array) => {
                    break array
                        .into_iter()
                        .filter_map(|i| {
                            if let RespDataType::BulkString(id) = i {
                                Some(id)
                            } else {
                                None
                            }
                        })
                        .collect();
                }
                RespDataType::Null => break Vec::new(),
                RespDataType::SimpleError(err) => {
                    let mut err = err.0.splitn(3, " ");
                    slot_addr = err.nth(2).unwrap().parse().unwrap();
                    continue;
                }
                _ => unreachable!(),
            }
        };

        if !saved_docs_ids.is_empty() {
            actions_tx.send(DocsSyncerAction::SubscribeToDocument {
                docs_ids: saved_docs_ids,
            })?;
        } else {
            print!("{}", log::info!("no hay documentos existentes"));
        }

        Ok(())
    }

    fn start_documents_creator(
        db_addr: SocketAddr,
        actions_tx: Sender<DocsSyncerAction>,
    ) -> Result<JoinHandle<Result<(), Error>>, Error> {
        let cmd = Command::PubSub(PubSubCommand::Subscribe(Subscribe {
            channels: vec!["docs_syncer".into()],
        }));

        let mut docs_creator_stream =
            TcpStream::connect(db_addr).map_err(Error::ConnectionError)?;

        docs_creator_stream
            .write_all(&Vec::from(cmd))
            .map_err(Error::WriteError)?;

        Ok(thread::spawn(move || {
            let mut buffer = BufReader::new(&mut docs_creator_stream);

            loop {
                match buffer.fill_buf() {
                    Ok(bytes) if !bytes.is_empty() => {
                        Self::handle_documents_creator_message(bytes, &actions_tx)?;
                        let length = bytes.len();
                        buffer.consume(length);
                    }
                    Ok(_) => todo!(),
                    Err(_) => todo!(),
                }
            }
        }))
    }

    fn handle_documents_creator_message(
        bytes: &[u8],
        actions_tx: &Sender<DocsSyncerAction>,
    ) -> Result<(), Error> {
        let reply = RespDataType::try_from(bytes).map_err(|_| Error::InvalidRespReply)?;

        let mut msg = match reply {
            RespDataType::Array(payload) => payload.into_iter().filter_map(|e| {
                if let RespDataType::BulkString(e) = e {
                    Some(e)
                } else {
                    None
                }
            }),
            _ => unreachable!(),
        };

        if let Some(doc_metadata) = msg.nth(2) {
            let doc_metadata = doc_metadata.to_string();
            let doc_metadata = doc_metadata.split('@');
            let mut doc_metadata = doc_metadata.map(BulkString::from);

            actions_tx.send(DocsSyncerAction::CreateNewDocument {
                doc_id: doc_metadata.next().unwrap(),
                doc_basename: doc_metadata.next().unwrap(),
            })?;
        } else {
            print!(
                "{}",
                log::info!("iniciando canal de escucha de editores clientes")
            );
        }

        Ok(())
    }

    fn start_documents_watcher(
        mut docs_stream: TcpStream,
        actions_tx: Sender<DocsSyncerAction>,
    ) -> JoinHandle<Result<(), Error>> {
        thread::spawn(move || {
            let mut buffer = BufReader::new(&mut docs_stream);

            loop {
                match buffer.fill_buf() {
                    Ok(bytes) if !bytes.is_empty() => {
                        Self::handle_documents_channels_message(bytes, &actions_tx)?;
                        let length = bytes.len();
                        buffer.consume(length);
                    }
                    Ok(_) => todo!(),
                    Err(_) => todo!(),
                }
            }
        })
    }

    fn handle_documents_channels_message(
        bytes: &[u8],
        actions_tx: &Sender<DocsSyncerAction>,
    ) -> Result<(), Error> {
        let reply = RespDataType::try_from(bytes).map_err(|_| Error::InvalidRespReply)?;

        dbg!(&reply);

        let mut payload = match reply {
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

        Ok(())
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
                        client_id: client_id.to_string(),
                        doc_id,
                    })
                    .unwrap();
            }
            _ => {}
        }
    }
}
