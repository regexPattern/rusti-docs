mod clients;
mod error;
mod persistence;

use std::{
    collections::{HashMap, HashSet},
    io::{BufReader, prelude::*},
    net::{Shutdown, SocketAddr, TcpStream},
    sync::mpsc::{self, Sender},
    thread::{self, JoinHandle},
};

use chrono::Local;
use redis_cmd::{
    Command,
    pub_sub::{PubSubCommand, Subscribe},
    storage::{Get, HGetAll, HKeys, HSet, StorageCommand},
};
use redis_resp::{BulkString, RespDataType};

use crate::error::Error;

/// Sincronizador de documentos: gestiona clientes conectados y persistencia.
#[derive(Debug)]
pub struct DocsSyncer {
    db_addr: SocketAddr,
    docs_stream: TcpStream,
    connected_clients: HashMap<String, HashSet<String>>,
}

/// Acciones que puede realizar el DocsSyncer sobre los documentos y clientes.
#[derive(Debug)]
pub enum DocsSyncerAction {
    CreateNewDocument {
        doc_id: BulkString,
        doc_basename: BulkString,
    },
    SubscribeToDocuments {
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
    Quit,
}

impl DocsSyncer {
    pub fn new(db_addr: SocketAddr) -> Result<Self, Error> {
        Ok(Self {
            db_addr,
            docs_stream: TcpStream::connect(db_addr).map_err(Error::OpenConn)?,
            connected_clients: HashMap::new(),
        })
    }

    /// Inicia la ejecución del sincronizador y los threads de gestión.
    /// Devuelve los handles de los threads lanzados.
    pub fn start(mut self) -> Result<JoinHandle<Result<(), Error>>, Error> {
        let db_addr = self.db_addr;
        let docs_stream = self.docs_stream.try_clone().map_err(Error::OpenConn)?;
        let (actions_tx, actions_rx) = mpsc::channel();

        let handle = thread::spawn(move || {
            for action in actions_rx {
                match action {
                    DocsSyncerAction::CreateNewDocument {
                        doc_id,
                        doc_basename,
                    } => {
                        self.create_new_document(doc_id.clone(), doc_basename)?;
                        self.subscribe_to_document(vec![doc_id])?;
                    }
                    DocsSyncerAction::SubscribeToDocuments { docs_ids } => {
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
                    } => self.persist_document(doc_id, doc_kind, doc_content)?,
                    DocsSyncerAction::Quit => {
                        return Ok(());
                    }
                }
            }
            Ok(())
        });

        Self::subscribe_to_saved_documents(db_addr, actions_tx.clone())?;

        Self::start_documents_creator(db_addr, actions_tx.clone())?;
        Self::start_documents_watcher(docs_stream, actions_tx.clone());
        Self::start_connected_clients_publisher(actions_tx);

        Ok(handle)
    }

    fn cluster_command(
        mut slot_addr: SocketAddr,
        cmd: Vec<u8>,
    ) -> Result<(SocketAddr, RespDataType), Error> {
        loop {
            let mut stream = TcpStream::connect(slot_addr).map_err(Error::OpenConn)?;
            stream.write_all(&cmd).map_err(Error::SendCommand)?;
            stream.shutdown(Shutdown::Write).map_err(Error::OpenConn)?;

            let mut buffer = Vec::new();
            stream.read_to_end(&mut buffer).map_err(Error::ReadReply)?;

            let reply =
                RespDataType::try_from(buffer.as_slice()).map_err(|_| Error::ReplyRespType)?;

            if let RespDataType::SimpleError(err) = reply {
                if err.0.contains("MOVED") {
                    let port = err.0.split(":").last().ok_or(Error::MissingData)?;
                    let port = port.parse().unwrap();
                    print!("{}", log::debug!("redirigiendo a nodo en puerto {port}"));
                    slot_addr.set_port(port);
                    continue;
                } else {
                    return Err(Error::RedisClient(err.0));
                }
            }

            print!("{}", log::debug!("comando enviado a nodo en {slot_addr:?}"));

            return Ok((slot_addr, reply));
        }
    }

    fn create_new_document(
        &mut self,
        doc_id: BulkString,
        doc_basename: BulkString,
    ) -> Result<(), Error> {
        let log_msg = log::info!("creado documento {doc_id} {doc_basename}");

        let cmd = Vec::from(Command::Storage(StorageCommand::HSet(HSet {
            key: "docs_ids".into(),
            field_value_pairs: vec![doc_id.clone(), doc_basename],
        })));

        let mut slot_addr = self.db_addr;
        let (new_slot, _) = Self::cluster_command(slot_addr, cmd)?;
        slot_addr = new_slot;

        let cmd = Vec::from(Command::Storage(StorageCommand::HSet(HSet {
            key: "docs_ts".into(),
            field_value_pairs: vec![doc_id, Local::now().to_rfc3339().into()],
        })));

        let _ = Self::cluster_command(slot_addr, cmd)?;

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
            .map_err(Error::SendCommand)?;

        print!("{}", log::info!("{log_msg}"));

        Ok(())
    }

    fn get_text_content(&self, doc_id: &str) -> Result<BulkString, Error> {
        let cmd = Vec::from(Command::Storage(StorageCommand::Get(Get {
            key: doc_id.into(),
        })));

        let (_, reply) = Self::cluster_command(self.db_addr, cmd)?;
        match reply {
            RespDataType::BulkString(content) => Ok(content),
            RespDataType::Null => Ok(BulkString::from("")),
            _ => unreachable!(),
        }
    }

    fn get_spreadsheet_content(&self, doc_id: &str) -> Result<BulkString, Error> {
        let cmd = Vec::from(Command::Storage(StorageCommand::HGetAll(HGetAll {
            key: doc_id.into(),
        })));

        let (_, reply) = Self::cluster_command(self.db_addr, cmd)?;
        let cells_mat: [[String; 10]; 10] = match reply {
            RespDataType::Map(content) => {
                let mut cells: [[String; 10]; 10] = Default::default();

                for (i, v) in content {
                    let (i, v) = match (i, v) {
                        (RespDataType::BulkString(i), RespDataType::BulkString(v)) => {
                            (i.to_string(), v.to_string())
                        }
                        _ => unreachable!(),
                    };

                    let (fil, col) = i.split_once(',').ok_or(Error::MissingData)?;
                    let row: usize = fil.parse().unwrap();
                    let col: usize = col.parse().unwrap();

                    cells[row][col] = v;
                }

                cells
            }
            RespDataType::Null => Default::default(),
            _ => unreachable!(),
        };

        let cells: Vec<_> = cells_mat.iter().flat_map(|row| row.iter()).collect();

        Ok(cells
            .iter()
            .map(|v| v.to_string())
            .collect::<Vec<_>>()
            .join(",")
            .into())
    }

    fn subscribe_to_saved_documents(
        slot_addr: SocketAddr,
        actions_tx: Sender<DocsSyncerAction>,
    ) -> Result<(), Error> {
        let cmd = Vec::from(Command::Storage(StorageCommand::HKeys(HKeys {
            key: "docs_ids".into(),
        })));

        let (_, reply) = Self::cluster_command(slot_addr, cmd)?;
        let saved_docs_ids = match reply {
            RespDataType::Array(array) => array
                .into_iter()
                .filter_map(|i| {
                    if let RespDataType::BulkString(id) = i {
                        Some(id)
                    } else {
                        None
                    }
                })
                .collect(),
            RespDataType::Null => Vec::new(),
            _ => unreachable!(),
        };

        if !saved_docs_ids.is_empty() {
            let _ = actions_tx.send(DocsSyncerAction::SubscribeToDocuments {
                docs_ids: saved_docs_ids,
            });
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

        let mut docs_creator_stream = TcpStream::connect(db_addr).map_err(Error::OpenConn)?;

        docs_creator_stream
            .write_all(&Vec::from(cmd))
            .map_err(Error::SendCommand)?;

        Ok(thread::spawn(move || {
            let mut buffer = BufReader::new(&mut docs_creator_stream);

            loop {
                match buffer.fill_buf() {
                    Ok(bytes) if !bytes.is_empty() => {
                        Self::handle_documents_creator_message(bytes, &actions_tx)?;
                        let length = bytes.len();
                        buffer.consume(length);
                    }
                    Ok(_) => {
                        print!(
                            "{}",
                            log::warn!("desconectado stream de creación de documentos")
                        );
                        let _ = actions_tx.send(DocsSyncerAction::Quit);
                        return Ok(());
                    }
                    Err(err) => {
                        print!(
                            "{}",
                            log::error!("error leyendo stream de creación de documentos: {err}")
                        );
                        let _ = actions_tx.send(DocsSyncerAction::Quit);
                        return Ok(());
                    }
                }
            }
        }))
    }

    fn handle_documents_creator_message(
        bytes: &[u8],
        actions_tx: &Sender<DocsSyncerAction>,
    ) -> Result<(), Error> {
        let reply = RespDataType::try_from(bytes).map_err(|_| Error::ReplyRespType)?;

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

            let _ = actions_tx.send(DocsSyncerAction::CreateNewDocument {
                doc_id: doc_metadata.next().ok_or(Error::MissingData)?,
                doc_basename: doc_metadata.next().ok_or(Error::MissingData)?,
            });
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
                        Self::handle_document_channels_message(bytes, &actions_tx)?;
                        let length = bytes.len();
                        buffer.consume(length);
                    }
                    Ok(_) => {
                        print!(
                            "{}",
                            log::warn!("desconectado stream de creación de documentos")
                        );
                        return Ok(());
                    }
                    Err(err) => {
                        print!(
                            "{}",
                            log::error!("error leyendo stream de creación de documentos: {err}")
                        );
                        return Ok(());
                    }
                }
            }
        })
    }

    fn handle_document_channels_message(
        bytes: &[u8],
        actions_tx: &Sender<DocsSyncerAction>,
    ) -> Result<(), Error> {
        let reply = RespDataType::try_from(bytes).map_err(|_| Error::ReplyRespType)?;

        let mut payload = match reply {
            RespDataType::Array(payload) => payload.into_iter().filter_map(|e| {
                if let RespDataType::BulkString(e) = e {
                    Some(e)
                } else {
                    None
                }
            }),
            _ => unreachable!(),
        };

        let doc_id = payload.nth(1).ok_or(Error::MissingData)?;

        if let Some(payload) = payload.next() {
            Self::handle_document_actions(doc_id.to_string(), payload.to_string(), actions_tx)?;
        }

        Ok(())
    }

    fn handle_document_actions(
        doc_id: String,
        payload: String,
        actions_tx: &Sender<DocsSyncerAction>,
    ) -> Result<(), Error> {
        match payload.split_once('@').ok_or(Error::MissingData)? {
            // FETCH_REQ@<CLIENT_ID>@<DOC_KIND>@<DOC_BASENAME>
            ("FETCH_REQ", payload) => {
                let mut payload = payload.splitn(3, '@').map(String::from);
                let _ = actions_tx.send(DocsSyncerAction::ConnectClient {
                    client_id: payload.next().ok_or(Error::MissingData)?,
                    doc_id,
                    doc_kind: payload.next().ok_or(Error::MissingData)?,
                    doc_basename: payload.next().ok_or(Error::MissingData)?,
                });
            }
            // PATCH@<CLIENT_ID>@<DOC_KIND>@<DOC_CONTENT>
            ("PATCH", payload) => {
                let mut payload = payload.splitn(3, '@').map(String::from);
                let _ = actions_tx.send(DocsSyncerAction::PersistDocument {
                    doc_id,
                    doc_kind: payload.nth(1).ok_or(Error::MissingData)?,
                    doc_content: payload.next().ok_or(Error::MissingData)?,
                });
            }
            // LEAVE@<CLIENT_ID>
            ("LEAVE", client_id) => {
                let _ = actions_tx.send(DocsSyncerAction::DisconnectClient {
                    client_id: client_id.to_string(),
                    doc_id,
                });
            }
            _ => {}
        };

        Ok(())
    }
}
