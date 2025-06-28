use std::{collections::BTreeMap, net::SocketAddr, sync::Arc};

use eframe::{
    CreationContext,
    egui::{
        self, FontData, FontDefinitions,
        FontFamily::{self, Monospace, Proportional},
        FontId, TextStyle,
    },
};

use crate::{editor::Editor, menu::Menu};

/// Información de acceso a la base de datos para los documentos.
///
/// Contiene las claves y la dirección del servidor Redis.
#[derive(Clone, Debug)]
pub struct DbAccessInfo {
    /// Clave para los IDs de documentos guardados.
    pub saved_docs_ids_key: String,
    /// Clave para los timestamps de documentos guardados.
    pub saved_docs_ts_key: String,
    /// Dirección del servidor Redis.
    pub addr: SocketAddr,
}

/// Aplicación principal del editor de documentos.
///
/// Gestiona el estado de la UI y la conexión a la base de datos.
#[derive(Debug)]
pub struct DocsEditorApp {
    db_addr: SocketAddr,
    screen: Screen,
}

#[allow(clippy::large_enum_variant)]
/// Representa la pantalla actual: menú o editor.
#[derive(Debug)]
pub enum Screen {
    /// Pantalla de menú principal.
    Menu(Menu),
    /// Pantalla del editor de documentos.
    Editor(Editor),
}

impl DocsEditorApp {
    ///Crea una nueva instancia de la aplicación DocsEditorApp
    ///
    /// Inicializa el menú principal y configura estilos y fuentes.
    ///
    /// # Argumentos
    /// * `db_addr` - Dirección del servidor Redis.
    /// * `cc` - Contexto de creación de la app (de eframe).
    ///
    /// # Errors
    /// Devuelve un error si falla la inicialización del menú.
    pub fn new(db_addr: SocketAddr, cc: &CreationContext<'_>) -> Result<Self, crate::error::Error> {
        configure_text_styles(&cc.egui_ctx);
        configure_fonts(&cc.egui_ctx);

        Ok(Self {
            db_addr,
            screen: Screen::Menu(Menu::new(db_addr)?),
        })
    }
}

impl eframe::App for DocsEditorApp {
    /// Actualiza la UI en cada frame.
    ///
    /// Cambia entre el menú y el editor según la interacción del usuario.
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

/// Configura los estilos de texto personalizados para la UI.
///
/// Ajusta tamaños y familias de fuente para los distintos estilos.
fn configure_text_styles(ctx: &egui::Context) {
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

/// Configura las fuentes personalizadas para la UI.
fn configure_fonts(ctx: &egui::Context) {
    let mut fonts = FontDefinitions::default();

    fonts.font_data.insert(
        "Inter".to_owned(),
        Arc::new(FontData::from_static(include_bytes!(
            "../assets/InterVariable.ttf"
        ))),
    );

    fonts
        .families
        .get_mut(&FontFamily::Proportional)
        .unwrap()
        .insert(0, "Inter".to_owned());

    ctx.set_fonts(fonts);
}
