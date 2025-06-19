use std::{collections::BTreeMap, net::SocketAddr};

use eframe::{
    CreationContext,
    egui::{self, FontFamily, FontId, TextStyle},
};

use crate::{editor::Editor, menu::Menu};

#[derive(Clone, Debug)]
pub struct DbAccessInfo {
    pub saved_docs_ids_key: String,
    pub saved_docs_ts_key: String,
    pub addr: SocketAddr,
}

#[derive(Debug)]
pub struct DocsEditorApp {
    db_addr: SocketAddr,
    screen: Screen,
}

#[derive(Debug)]
pub enum Screen {
    Menu(Menu),
    Editor(Editor),
}

impl DocsEditorApp {
    pub fn new(db_addr: SocketAddr, cc: &CreationContext<'_>) -> Self {
        configure_text_styles(&cc.egui_ctx);

        Self {
            db_addr,
            screen: Screen::Menu(Menu::new(db_addr).unwrap()),
        }
    }
}

impl eframe::App for DocsEditorApp {
    fn update(&mut self, ctx: &eframe::egui::Context, _: &mut eframe::Frame) {
        egui::CentralPanel::default().show(ctx, |ui| {
            match &mut self.screen {
                Screen::Menu(menu) => match menu.ui(ui) {
                    Some(doc_metadata) => {
                        self.screen = Screen::Editor(Editor::new(self.db_addr, doc_metadata))
                    }
                    None => (),
                },
                Screen::Editor(editor) => {
                    if !editor.ui(ui) {
                        self.screen = Screen::Menu(Menu::new(self.db_addr).unwrap());
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
