mod doc;
mod patch;
mod spreadsheet;
mod text;

use std::{
    io::{BufRead, BufReader, Write},
    net::{SocketAddr, TcpStream},
    ops::Range,
    sync::mpsc::{self, Receiver, Sender},
    thread,
};

pub use doc::{DocKind, DocMetadata};
use eframe::egui::{self, ViewportCommand};
use redis_cmd::{
    Command,
    pub_sub::{PubSubCommand, Publish, Subscribe},
};
use redis_resp::{BulkString, RespDataType};

use crate::{
    Error,
    editor::{doc::DocContent, spreadsheet::SpreadSheetEditor, text::TextEditor},
};

/// Editor de documentos de texto y hojas de cálculo.
#[derive(Debug)]
pub struct Editor {
    db_addr: SocketAddr,
    client_id: String,
    doc_md: DocMetadata,
    doc_content: DocContent,
    connected_peers: Vec<String>,
    actions_tx: Sender<EditorAction>,
    actions_rx: Receiver<EditorAction>,
    prompt: String,
    selected_range: Option<Range<usize>>,
    full_gen_mode: bool,
    waiting_llm_response: bool,
    rango_valido: bool,
}

#[derive(Debug)]
enum EditorAction {
    FetchContent { kind: String, content: String },
    UpdateConnectedClients { peers: Vec<String> },
    PatchContent { kind: String, content: String },
    PromptLLM,
    ApplyGeneratedContent { content: String },
}

impl Editor {
    /// Crea un nuevo editor para el documento especificado.
    /// Inicializa la suscripción y obtiene el contenido inicial.
    pub fn new(db_addr: SocketAddr, doc_md: DocMetadata) -> Result<Self, Error> {
        print!("{}", log::info!("accediendo documento {}", doc_md.id));

        let client_id = std::process::id().to_string();
        let (actions_tx, actions_rx) = mpsc::channel();

        Self::subscribe_to_document(db_addr, client_id.clone(), &doc_md, actions_tx.clone())?;
        Self::fetch_document(db_addr, client_id.clone(), &doc_md)?;

        let doc_content = match doc_md.kind {
            DocKind::Text => DocContent::Text("".to_string()),
            DocKind::SpreadSheet => DocContent::SpreadSheet(Default::default()),
        };

        Ok(Self {
            db_addr,
            client_id,
            doc_md,
            doc_content,
            connected_peers: Vec::new(),
            actions_tx,
            actions_rx,
            prompt: String::new(),
            selected_range: None,
            full_gen_mode: false,
            waiting_llm_response: false,
            rango_valido: false,
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
                        match Self::handle_document_listener_message(&client_id, bytes, &actions_tx)
                        {
                            Ok(stay_in_loop) => {
                                if !stay_in_loop {
                                    log::info!(
                                        "cerrando stream de suscripción a documento {doc_id}"
                                    );
                                    return;
                                }
                            }
                            Err(err) => {
                                println!(
                                    "{}",
                                    log::error!(
                                        "error manejando mensaje en channel de documento: {err}"
                                    )
                                )
                            }
                        };

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
    ) -> Result<bool, Error> {
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

        let doc_id = payload.nth(1).ok_or(Error::MissingData)?;

        if let Some(payload) = payload.next() {
            Self::handle_document_action(client_id, payload.to_string(), actions_tx)
        } else {
            print!("{}", log::info!("suscrito a channel de documento {doc_id}"));
            Ok(true)
        }
    }

    fn handle_document_action(
        my_client_id: &str,
        payload: String,
        actions_tx: &Sender<EditorAction>,
    ) -> Result<bool, Error> {
        match payload.split_once('@').ok_or(crate::Error::MissingData)? {
            // FETCH_ACK@<CLIENT_ID>@<DOC_KIND>@<DOC_CONTENT>
            ("FETCH_ACK", payload) => {
                let mut payload = payload.splitn(3, '@').map(String::from);
                let recepient_client_id = payload.next().ok_or(Error::MissingData)?;
                if recepient_client_id == my_client_id {
                    let kind = payload.next().ok_or(Error::MissingData)?;
                    let content = payload.next().ok_or(Error::MissingData)?;
                    let _ = actions_tx.send(EditorAction::FetchContent { kind, content });
                }
            }
            // CLIENTS@<CLIENT_ID_1>,<CLIENT_ID_2>,<CLIENT_ID_3>,...
            ("CLIENTS", payload) => {
                let _ = actions_tx.send(EditorAction::UpdateConnectedClients {
                    peers: payload.split(",").map(String::from).collect(),
                });
            }
            // PATCH@<CLIENT_ID>@<DOC_KIND>@<DOC_CONTENT>
            ("PATCH", payload) => {
                let mut payload = payload.splitn(3, '@').map(String::from);
                let sender_id = payload.next().ok_or(Error::MissingData)?;

                if sender_id != my_client_id {
                    let kind = payload.next().ok_or(Error::MissingData)?;
                    let content = payload.next().ok_or(Error::MissingData)?;
                    let _ = actions_tx.send(EditorAction::PatchContent { kind, content });
                }
            }
            // PARTIALGEN_ACK@<CLIENT_ID>@<GENERATED_CONTENT>
            // FULLGEN_ACK@<CLIENT_ID>@<GENERATED_CONTENT>
            ("PARTIALGEN_ACK", payload) | ("FULLGEN_ACK", payload) => {
                let (recepient_client_id, gen_content) =
                    payload.split_once('@').ok_or(Error::MissingData)?;
                if recepient_client_id == my_client_id {
                    let _ = actions_tx.send(EditorAction::ApplyGeneratedContent {
                        content: gen_content.to_string(),
                    });
                }
            }
            // LEAVE@<CLIENT_ID>
            ("LEAVE", sender_id) => {
                if sender_id == my_client_id {
                    return Ok(false);
                }
            }
            _ => {}
        };

        Ok(true)
    }

    fn fetch_document(
        db_addr: SocketAddr,
        client_id: String,
        md: &DocMetadata,
    ) -> Result<(), crate::Error> {
        // FETCH_REQ@<CLIENT_ID>@<DOC_KIND>@<DOC_BASENAME>
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
        // PATCH@<CLIENT_ID>@<DOC_KIND>@<DOC_CONTENT>
        let msg = format!(
            "PATCH@{}@{}@{}",
            self.client_id, self.doc_md.kind, self.doc_content
        );

        let mut stream = TcpStream::connect(self.db_addr).map_err(crate::Error::OpenConn)?;

        let cmd = Command::PubSub(PubSubCommand::Publish(Publish {
            channel: BulkString::from(&self.doc_md.id),
            message: BulkString::from(msg),
        }));

        stream
            .write_all(&Vec::from(cmd))
            .map_err(crate::Error::SendCommand)?;

        print!(
            "{}",
            log::info!("enviando update de documento {}", self.doc_md.id)
        );

        Ok(())
    }

    fn fetch_content(&mut self, doc_kind: String, content: String) {
        self.doc_content = match doc_kind.as_str() {
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
        match &mut self.doc_content {
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
        // LEAVE@<CLIENT_ID>
        let msg = format!("LEAVE@{}", self.client_id);

        let cmd = Command::PubSub(PubSubCommand::Publish(Publish {
            channel: BulkString::from(&self.doc_md.id),
            message: msg.into(),
        }));

        let mut stream = TcpStream::connect(self.db_addr).map_err(crate::Error::OpenConn)?;

        stream
            .write_all(&Vec::from(cmd))
            .map_err(crate::Error::SendCommand)?;

        Ok(())
    }

    fn prompt_llm(&mut self) -> Result<(), Error> {
        let doc_content = match &self.doc_content {
            DocContent::Text(content) => content,
            DocContent::SpreadSheet(_) => unreachable!(),
        };

        let ctx_content = if let Some(selected_range) = &self.selected_range {
            let selected_range =
                text::char_range_to_byte_range(doc_content, selected_range).ok_or(
                    Error::InvalidContentIndexRange(selected_range.clone(), doc_content.len()),
                )?;
            &doc_content[selected_range]
        } else {
            ""
        };

        let msg = if self.full_gen_mode {
            // FULLGEN_REQ@<CLIENT_ID>@<DOC_ID>@<PROMPT>
            format!(
                "FULLGEN_REQ@{}@{}@{}",
                self.client_id, self.doc_md.id, self.prompt
            )
        } else {
            // PARTIALGEN_REQ@<CLIENT_ID>@<DOC_ID>@<PROMPT>@<CTX_CONTENT>
            format!(
                "PARTIALGEN_REQ@{}@{}@{}@{}",
                self.client_id, self.doc_md.id, self.prompt, ctx_content
            )
        };

        let cmd = Command::PubSub(PubSubCommand::Publish(Publish {
            channel: "docs_gpt".into(),
            message: msg.into(),
        }));

        let mut stream = TcpStream::connect(self.db_addr).map_err(Error::OpenConn)?;

        stream
            .write_all(&Vec::from(cmd))
            .map_err(Error::SendCommand)?;

        self.waiting_llm_response = true;

        Ok(())
    }

    fn apply_generated_content(&mut self, gen_content: String) -> Result<(), Error> {
        if let Some(selected_range) = &self.selected_range {
            print!(
                "{}",
                log::info!(
                    "reemplazado rango de bytes de {} a {} con {} bytes generados",
                    selected_range.start,
                    selected_range.end,
                    gen_content.len()
                )
            );

            if let DocContent::Text(doc_content) = &mut self.doc_content {
                let selected_range =
                    text::char_range_to_byte_range(doc_content, selected_range).ok_or(
                        Error::InvalidContentIndexRange(selected_range.clone(), doc_content.len()),
                    )?;
                doc_content.replace_range(selected_range, &gen_content);
            }
        } else {
            print!(
                "{}",
                log::info!(
                    "reemplazado contenido completo con {} bytes generados",
                    gen_content.len()
                )
            );

            if let DocContent::Text(doc_content) = &mut self.doc_content {
                *doc_content = gen_content;
            }
        }

        self.selected_range = None;

        Ok(())
    }

    pub fn ui(&mut self, ui: &mut egui::Ui) -> bool {
        if let Ok(action) = self.actions_rx.try_recv() {
            match action {
                EditorAction::FetchContent {
                    kind: doc_kind,
                    content: doc_content,
                } => {
                    self.fetch_content(doc_kind, doc_content);
                    ui.ctx()
                        .send_viewport_cmd(ViewportCommand::Title(self.doc_md.basename.clone()));
                }
                EditorAction::UpdateConnectedClients { mut peers } => {
                    peers.sort();
                    self.connected_peers = peers;
                }
                EditorAction::PatchContent { kind, content } => self.patch_content(kind, content),
                EditorAction::PromptLLM => {
                    if let Err(err) = self.prompt_llm() {
                        print!("{}", log::error!("error generando contenido: {err}"));
                    }
                }
                EditorAction::ApplyGeneratedContent { content } => {
                    self.waiting_llm_response = false;
                    if let Err(err) = self.apply_generated_content(content) {
                        print!(
                            "{}",
                            log::error!("error aplicando contenido generado: {err}")
                        );
                    } else {
                        self.prompt.clear();
                    }
                }
            }
        }

        let mut stay_in_editor = true;

        if ui.link("< Volver").clicked() {
            let _ = self.leave();
            stay_in_editor = false;
        };

        ui.add_space(8.0);

        match &mut self.doc_content {
            DocContent::Text(content) => {
                ui.horizontal(|ui| {
                    ui.add(TextEditor {
                        content,
                        prompt: &mut self.prompt,
                        selected_range: &mut self.selected_range,
                        actions_tx: &self.actions_tx,
                        full_gen_on: &mut self.full_gen_mode,
                        waiting_llm_response: self.waiting_llm_response,
                        rango_valido: &mut self.rango_valido,
                    });
                });
            }
            DocContent::SpreadSheet(cells) => {
                (SpreadSheetEditor(cells)).render(ui);
            }
        };

        ui.vertical(|ui| {
            ui.add_space(4.0);
            ui.label("Cliente Conectados");
            for client_id in &self.connected_peers {
                ui.horizontal(|ui| {
                    ui.label(client_id);
                    if client_id == &self.client_id {
                        ui.add_space(4.0);
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
