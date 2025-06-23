use std::{collections::BTreeMap, net::SocketAddr};

use eframe::{
    CreationContext,
    egui::{self, FontFamily, FontId, TextStyle},
};

use crate::{editor::Editor, menu::Menu};

/// Información de acceso a la base de datos para los documentos.
#[derive(Clone, Debug)]
pub struct DbAccessInfo {
    pub saved_docs_ids_key: String,
    pub saved_docs_ts_key: String,
    pub addr: SocketAddr,
}

/// Aplicación principal del editor de documentos.
#[derive(Debug)]
pub struct DocsEditorApp {
    db_addr: SocketAddr,
    screen: Screen,
}

#[allow(clippy::large_enum_variant)]
/// Representa la pantalla actual: menú o editor.
#[derive(Debug)]
pub enum Screen {
    Menu(Menu),
    Editor(Editor),
}

impl DocsEditorApp {
    /// Crea una nueva instancia de la aplicación DocsEditorApp.
    /// Inicializa el menú
    pub fn new(db_addr: SocketAddr, cc: &CreationContext<'_>) -> Result<Self, crate::error::Error> {
        configure_text_styles(&cc.egui_ctx);

        Ok(Self {
            db_addr,
            screen: Screen::Menu(Menu::new(db_addr)?),
        })
    }
}

impl eframe::App for DocsEditorApp {
    fn update(&mut self, ctx: &eframe::egui::Context, _: &mut eframe::Frame) {
        egui::CentralPanel::default().show(ctx, |ui| {
            match &mut self.screen {
                Screen::Menu(menu) => {
                    if let Some(doc_metadata) = menu.ui(ui) {
                        match Editor::new(self.db_addr, doc_metadata) {
                            Ok(editor) => self.screen = Screen::Editor(editor),
                            Err(err) => {
                                print!("{}", log::error!("{err}"));
                                ctx.send_viewport_cmd(egui::ViewportCommand::Close);
                            }
                        }
                    }
                }
                Screen::Editor(editor) => {
                    if !editor.ui(ui) {
                        match Menu::new(self.db_addr) {
                            Ok(menu) => self.screen = Screen::Menu(menu),
                            Err(err) => {
                                print!("{}", log::error!("{err}"));
                                ctx.send_viewport_cmd(egui::ViewportCommand::Close);
                            }
                        };
                    }
                }
            };
        });
    }
}

fn configure_text_styles(ctx: &egui::Context) {
    use FontFamily::{Monospace, Proportional};

    let text_styles: BTreeMap<TextStyle, FontId> = [
        (TextStyle::Heading, FontId::new(25.0, Proportional)),
        (TextStyle::Body, FontId::new(16.0, Proportional)),
        (TextStyle::Monospace, FontId::new(14.0, Monospace)),
        (TextStyle::Button, FontId::new(14.0, Proportional)),
        (TextStyle::Small, FontId::new(12.0, Proportional)),
    ]
    .into();

    ctx.all_styles_mut(move |style| style.text_styles = text_styles.clone());
}
