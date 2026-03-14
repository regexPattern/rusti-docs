use std::{
    cmp::Reverse,
    io::Write,
    net::{SocketAddr, TcpStream},
};

use chrono::{DateTime, Local};
use eframe::egui::{self, RichText, Vec2, ViewportCommand};
use redis_cmd::{
    Command,
    pub_sub::{PubSubCommand, Publish},
};
use uuid::Uuid;

use crate::{
    client,
    editor::{DocKind, DocMetadata},
    error::Error,
};

/// Menú principal para gestionar y crear documentos.
#[derive(Debug)]
pub struct Menu {
    db_addr: SocketAddr,
    saved_docs: Vec<DocMetadata>,
    new_doc_basename: String,
}

impl Menu {
    /// Crea una nueva instancia del menú y carga la lista de documentos guardados.
    /// /// Crea una nueva instancia del menú conectada a una base de datos Redis en la dirección dada.
    ///
    /// También inicializa la lista de documentos cargados desde la base de datos.
    /// Si ocurre un error al leer los documentos, devuelve un `Error`.
    pub fn new(db_addr: SocketAddr) -> Result<Self, Error> {
        let mut menu = Self {
            db_addr,
            saved_docs: Vec::new(),
            new_doc_basename: String::new(),
        };

        menu.update_saved_documents_list()?;

        Ok(menu)
    }

    fn update_saved_documents_list(&mut self) -> Result<(), Error> {
        print!(
            "{}",
            log::debug!("actualizando listado de documentos existentes")
        );

        let docs_ids = client::read_resp_map(self.db_addr, "docs_ids")?;
        let docs_ts = client::read_resp_map(self.db_addr, "docs_ts")?;

        let mut saved_docs = Vec::with_capacity(docs_ids.len());

        for (doc_id, doc_basename) in &docs_ids {
            if let Some(doc_ts) = docs_ts.get(doc_id) {
                let basename = doc_basename.to_string();
                let ts = DateTime::parse_from_rfc3339(&doc_ts.to_string()).unwrap();

                saved_docs.push(DocMetadata {
                    id: doc_id.to_string(),
                    basename: basename.clone(),
                    kind: DocKind::from_basename(&basename)
                        .ok_or(Error::UnsupportedDocType(basename.clone()))?,
                    last_edited: ts,
                });

                print!(
                    "{}",
                    log::info!(
                        "encontrado documento {doc_id} {doc_basename} editado en {}",
                        ts.format("%d-%m-%Y %H:%M:%S")
                    )
                );
            }
        }

        saved_docs.sort_by_key(|d| Reverse(d.last_edited));
        self.saved_docs = saved_docs;

        Ok(())
    }

    fn create_new_document(&mut self, kind: DocKind) -> Result<DocMetadata, Error> {
        let doc_id = Uuid::new_v4().to_string();
        let doc_basename = self.new_doc_basename.clone();

        let doc_metadata = format!("{doc_id}@{doc_basename}");

        let cmd = Command::PubSub(PubSubCommand::Publish(Publish {
            channel: "docs_syncer".into(),
            message: doc_metadata.into(),
        }));

        let mut stream = TcpStream::connect(self.db_addr).map_err(Error::OpenConn)?;

        stream
            .write_all(&Vec::from(cmd))
            .map_err(Error::SendCommand)?;

        print!(
            "{}",
            log::debug!(
                "notificado al microservicio de la creación de documento {doc_id} {doc_basename}"
            )
        );

        Ok(DocMetadata {
            id: doc_id,
            basename: std::mem::take(&mut self.new_doc_basename),
            kind,
            last_edited: Local::now().into(),
        })
    }

    pub fn ui(&mut self, ui: &mut egui::Ui) -> Option<DocMetadata> {
        let mut selected_doc = None;

        ui.ctx()
            .send_viewport_cmd(ViewportCommand::Title("Editor de Documentos".to_string()));

        ui.horizontal(|ui| {
            ui.vertical(|ui| {
                ui.set_width(300.0);

                ui.heading(RichText::new("Abrir Documento"));
                ui.add_space(8.0);

                if ui.button("Refrescar 🔃").clicked() {
                    self.update_saved_documents_list().unwrap();
                }

                ui.add_space(8.0);

                ui.vertical(|ui| {
                    let now = Local::now();

                    ui.vertical(|ui| {
                        ui.spacing_mut().item_spacing = Vec2 { x: 0.0, y: 8.0 };

                        if self.saved_docs.is_empty() {
                            ui.label("Ningún Documento Encontrado");
                        } else {
                            for doc in &self.saved_docs {
                                ui.horizontal(|ui| {
                                    if ui.link(&doc.basename).clicked() {
                                        selected_doc = Some(doc.clone());
                                    }

                                    ui.add_space(8.0);

                                    let time_since_last_edit =
                                        now.signed_duration_since(doc.last_edited).num_minutes();

                                    ui.small(format!(
                                        "(editado hace {} minutos)",
                                        time_since_last_edit
                                    ));
                                });
                            }
                        }
                    })
                });

                ui.add_space(8.0);
            });

            ui.add_space(16.0);

            ui.vertical(|ui| {
                ui.heading(RichText::new("Crear Nuevo Documento"));
                ui.add_space(16.0);
                ui.vertical(|ui| {
                    ui.spacing_mut().item_spacing = Vec2 { x: 8.0, y: 8.0 };

                    ui.label("Nombre del Documento");

                    ui.horizontal(|ui| {
                        ui.text_edit_singleline(&mut self.new_doc_basename);

                        if let Some(kind) = DocKind::from_basename(&self.new_doc_basename)
                            && ui.button("Crear Documento").clicked() {
                                match self.create_new_document(kind) {
                                    Ok(doc_metadata) => {
                                        self.new_doc_basename.clear();
                                        self.saved_docs.insert(0, doc_metadata);
                                    }
                                    Err(err) => {
                                        print!("{}", log::error!("error creando documento: {err}"))
                                    }
                                }
                            }
                    });

                    ui.small(
                        r#"Extensiones Soportadas:
 • Documento de Texto: '.txt'
 • Planilla de Cálculo: '.xsl'"#,
                    );
                })
            });
        });

        selected_doc
    }
}
