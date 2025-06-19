use eframe::egui::{self, Widget};

#[derive(Debug)]
pub struct TextEditor<'c> {
    pub content: &'c mut String,
}

impl Widget for TextEditor<'_> {
    fn ui(self, ui: &mut egui::Ui) -> egui::Response {
        ui.text_edit_multiline(self.content)
    }
}
