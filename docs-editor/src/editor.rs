mod doc;
mod error;
mod patch;
mod spreadsheet;
mod text;

use std::{
    io::{BufRead, BufReader, Write},
    net::{SocketAddr, TcpStream},
    sync::mpsc::{self, Receiver, Sender},
    thread,
};

pub use doc::{DocKind, DocMetadata};
use eframe::egui::{self};
use redis_cmd::{
    Command,
    pub_sub::{PubSubCommand, Publish, Subscribe},
};
use redis_resp::{BulkString, RespDataType};

use crate::editor::{doc::DocContent, spreadsheet::SpreadSheetEditor, text::TextEditor};

/// Editor de documentos de texto y hojas de cálculo.
#[derive(Debug)]
pub struct Editor {
    db_addr: SocketAddr,
    client_id: String,
    md: DocMetadata,
    editing_content: DocContent,
    peers: Vec<String>,
    actions_rx: Receiver<EditorAction>,
}

#[derive(Debug)]
enum EditorAction {
    FetchContent { kind: String, content: String },
    UpdateConnectedClients { peers: Vec<String> },
    PatchContent { kind: String, content: String },
}

impl Editor {
    /// Crea un nuevo editor para el documento especificado.
    /// Inicializa la suscripción y obtiene el contenido inicial.
    pub fn new(db_addr: SocketAddr, md: DocMetadata) -> Result<Self, crate::Error> {
        print!("{}", log::info!("accediendo documento {}", md.id));

        let client_id = std::process::id().to_string();
        let (actions_tx, actions_rx) = mpsc::channel();

        Self::subscribe_to_document(db_addr, client_id.clone(), &md, actions_tx)?;
        Self::fetch_document(db_addr, client_id.clone(), &md)?;

        let content = match md.kind {
            DocKind::Text => DocContent::Text("".to_string()),
            DocKind::SpreadSheet => DocContent::SpreadSheet(Default::default()),
        };

        Ok(Self {
            db_addr,
            client_id,
            md,
            editing_content: content,
            peers: Vec::new(),
            actions_rx,
        })
    }

    fn subscribe_to_document(
        db_addr: SocketAddr,
        client_id: String,
        md: &DocMetadata,
        actions_tx: Sender<EditorAction>,
    ) -> Result<(), crate::Error> {
        let cmd = Command::PubSub(PubSubCommand::Subscribe(Subscribe {
            channels: vec![BulkString::from(&md.id)],
        }));

        let mut stream = TcpStream::connect(db_addr).map_err(crate::Error::OpenConn)?;
        stream
            .write_all(&Vec::from(cmd))
            .map_err(crate::Error::SendCommand)?;

        let doc_id = md.id.clone();

        thread::spawn(move || {
            let mut buffer = BufReader::new(&mut stream);

            loop {
                match buffer.fill_buf() {
                    Ok(bytes) if !bytes.is_empty() => {
                        if Self::handle_document_listener_message(&client_id, bytes, &actions_tx)
                            .is_err()
                        {
                            return;
                        }

                        let length = bytes.len();
                        buffer.consume(length);
                    }
                    Ok(_) => {
                        print!(
                            "{}",
                            log::warn!("desconectado stream de documento: {doc_id}")
                        );
                        return;
                    }
                    Err(_) => {
                        print!(
                            "{}",
                            log::error!("error leyendo stream de documentos: {doc_id}")
                        );
                        return;
                    }
                }
            }
        });

        Ok(())
    }

    fn handle_document_listener_message(
        client_id: &str,
        bytes: &[u8],
        actions_tx: &Sender<EditorAction>,
    ) -> Result<(), crate::Error> {
        let mut payload = match RespDataType::try_from(bytes).unwrap() {
            RespDataType::Array(payload) => payload.into_iter().filter_map(|e| {
                if let RespDataType::BulkString(e) = e {
                    Some(e)
                } else {
                    None
                }
            }),
            _ => unreachable!(),
        };

        let doc_id = payload.nth(1).unwrap();

        if let Some(payload) = payload.next() {
            Self::handle_document_action(client_id, payload.to_string(), actions_tx)?;
        } else {
            print!("{}", log::info!("suscrito a channel de documento {doc_id}"));
        }

        Ok(())
    }

    fn handle_document_action(
        client_id: &str,
        payload: String,
        actions_tx: &Sender<EditorAction>,
    ) -> Result<(), crate::Error> {
        match payload.split_once('@').ok_or(crate::Error::MissingData)? {
            ("FETCH_ACK", payload) => {
                let mut payload = payload.splitn(3, '@').map(String::from);
                let recepient_client_id = payload.next().unwrap();

                if recepient_client_id == client_id {
                    let kind = payload.next().unwrap();
                    let content = payload.next().unwrap();
                    let _ = actions_tx.send(EditorAction::FetchContent { kind, content });
                }
            }
            ("CLIENTS", payload) => {
                let _ = actions_tx.send(EditorAction::UpdateConnectedClients {
                    peers: payload.split(",").map(String::from).collect(),
                });
            }
            ("PATCH", payload) => {
                let mut payload = payload.splitn(3, '@').map(String::from);
                let sender_id = payload.next().ok_or(crate::Error::MissingData)?;

                if sender_id != client_id {
                    let kind = payload.next().ok_or(crate::Error::MissingData)?;
                    let content = payload.next().ok_or(crate::Error::MissingData)?;
                    let _ = actions_tx.send(EditorAction::PatchContent { kind, content });
                }
            }
            _ => {}
        };

        Ok(())
    }

    fn fetch_document(
        db_addr: SocketAddr,
        client_id: String,
        md: &DocMetadata,
    ) -> Result<(), crate::Error> {
        let msg = format!("FETCH_REQ@{}@{}@{}", client_id, md.kind, md.basename);

        let cmd = Command::PubSub(PubSubCommand::Publish(Publish {
            channel: BulkString::from(&md.id),
            message: msg.into(),
        }));

        let mut stream = TcpStream::connect(db_addr).map_err(crate::Error::OpenConn)?;

        stream
            .write_all(&Vec::from(cmd))
            .map_err(crate::Error::SendCommand)?;

        print!(
            "{}",
            log::debug!(
                "enviando fetch inicial de documento {} {}",
                md.id,
                md.basename
            )
        );

        Ok(())
    }

    fn save_document(&mut self) -> Result<(), crate::Error> {
        let msg = format!(
            "PATCH@{}@{}@{}",
            self.client_id, self.md.kind, self.editing_content
        );

        let mut stream = TcpStream::connect(self.db_addr).map_err(crate::Error::OpenConn)?;

        let cmd = Command::PubSub(PubSubCommand::Publish(Publish {
            channel: BulkString::from(&self.md.id),
            message: BulkString::from(msg),
        }));

        stream
            .write_all(&Vec::from(cmd))
            .map_err(crate::Error::SendCommand)?;

        print!(
            "{}",
            log::info!("enviando update de documento {}", self.md.id)
        );

        Ok(())
    }

    fn fetch_content(&mut self, doc_kind: String, content: String) {
        self.editing_content = match doc_kind.as_str() {
            "TEXT" => DocContent::Text(content),
            "SPREADSHEET" => {
                let mut cells: [[String; 10]; 10] = Default::default();

                for (i, value) in content.split(',').enumerate() {
                    let row = i / 10;
                    let col = i % 10;
                    cells[row][col] = value.to_string();
                }

                DocContent::SpreadSheet(cells)
            }
            _ => unreachable!(),
        };
    }

    fn patch_content(&mut self, _kind: String, content: String) {
        match &mut self.editing_content {
            DocContent::Text(old) => {
                let patches = patch::diff_lines(old, &content);
                *old = patch::apply_text_patches(old, &patches);
            }
            DocContent::SpreadSheet(old) => {
                let mut cells: [[String; 10]; 10] = Default::default();

                for (i, value) in content.split(',').enumerate() {
                    let row = i / 10;
                    let col = i % 10;
                    cells[row][col] = value.to_string();
                }

                let patches = patch::diff_cells(old, &cells);
                *old = patch::apply_cell_patches(old, &patches);
            }
        }
    }

    fn leave(&self) -> Result<(), crate::Error> {
        let cmd = Command::PubSub(PubSubCommand::Publish(Publish {
            channel: BulkString::from(&self.md.id),
            message: format!("LEAVE@{}", self.client_id).into(),
        }));

        let mut stream = TcpStream::connect(self.db_addr).map_err(crate::Error::OpenConn)?;

        stream
            .write_all(&Vec::from(cmd))
            .map_err(crate::Error::SendCommand)?;

        Ok(())
    }

    /// Renderiza la interfaz de usuario del editor y gestiona las acciones.
    /// Devuelve true si el usuario permanece en el editor.
    pub fn ui(&mut self, ui: &mut egui::Ui) -> bool {
        if let Ok(action) = self.actions_rx.try_recv() {
            match action {
                EditorAction::FetchContent {
                    kind: doc_kind,
                    content: doc_content,
                } => {
                    self.fetch_content(doc_kind, doc_content);
                }
                EditorAction::UpdateConnectedClients { mut peers } => {
                    peers.sort();
                    self.peers = peers;
                }
                EditorAction::PatchContent { kind, content } => self.patch_content(kind, content),
            }
        }

        let mut stay_in_editor = true;

        ui.vertical(|ui| {
            if ui.link("⬅️ Volver").clicked() {
                self.leave().unwrap();
                stay_in_editor = false;
            };

            ui.add_space(8.0);

            ui.heading(&self.md.basename);
        });

        ui.add_space(8.0);

        match &mut self.editing_content {
            DocContent::Text(content) => {
                ui.add(TextEditor { content });
            }
            DocContent::SpreadSheet(cells) => {
                (SpreadSheetEditor(cells)).render(ui);
            }
        };

        ui.vertical(|ui| {
            for client_id in &self.peers {
                ui.horizontal(|ui| {
                    ui.label(client_id);

                    if client_id == &self.client_id {
                        ui.add_space(8.0);

                        ui.small("(yo)");
                    }
                });
            }
        });

        if ui.button("Guardar ✅").clicked() {
            self.save_document().unwrap();
        }

        stay_in_editor
    }
}
