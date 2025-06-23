use eframe::egui::{self, TextEdit, Widget};

/// Editor de texto que permite editar el contenido de un String mutable.
#[derive(Debug)]
pub struct TextEditor<'c> {
    pub content: &'c mut String,
}

impl Widget for TextEditor<'_> {
    fn ui(self, ui: &mut egui::Ui) -> egui::Response {
        ui.add_sized([512.0, 256.0], TextEdit::multiline(self.content))
    }
}
