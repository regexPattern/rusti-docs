use std::{ops::Range, sync::mpsc::Sender};

use eframe::egui::{self, Color32, Frame, Key, ScrollArea, TextEdit, Vec2, Widget};

use crate::editor::EditorAction;

/// Editor de texto que permite editar el contenido de un String mutable.
#[derive(Debug)]
pub struct TextEditor<'c, 'p, 'r, 'a, 'f> {
    pub content: &'c mut String,
    pub prompt: &'p mut String,
    pub selected_range: &'r mut Option<Range<usize>>,
    pub actions_tx: &'a Sender<EditorAction>,
    pub full_gen_on: &'f mut bool,
    pub waiting_llm_response: bool,
    pub rango_valido: &'f mut bool,
}

impl Widget for TextEditor<'_, '_, '_, '_, '_> {
    fn ui(self, ui: &mut egui::Ui) -> egui::Response {
        ui.horizontal(|ui| {
            ScrollArea::vertical()
                .id_salt("editor_scroll")
                .min_scrolled_height(512.0)
                .max_height(512.0)
                .show(ui, |ui| {
                    let text_edit = TextEdit::multiline(self.content)
                        .min_size(Vec2::new(450.0, 512.0))
                        .desired_width(450.0)
                        .show(ui);

                    if ui.input(|i| i.modifiers.command && i.key_pressed(Key::Enter)) {
                        if let Some(cursor_range) = text_edit.cursor_range {
                            let selected_range = cursor_range.as_sorted_char_range();

                            print!(
                                "{}",
                                log::debug!(
                                    "seleccionado rango de contexto de byte {} a byte {}",
                                    selected_range.start,
                                    selected_range.end
                                )
                            );
                            print!("{}", log::debug!("activado modo generación parcial"));

                            *self.selected_range = Some(selected_range);
                            *self.full_gen_on = false;
                        }
                    }
                });

            Frame::new()
                .stroke((0.5, Color32::DARK_GRAY))
                .inner_margin(8.0)
                .show(ui, |ui| {
                    ui.set_width(256.0);

                    ui.vertical(|ui| {
                        ui.heading("Asistente con IA");
                        ui.small(
                            "(seleccione texto y presione ctrl+enter para generación parcial)",
                        );

                        ScrollArea::vertical()
                            .id_salt("prompt_scroll")
                            .min_scrolled_height(200.0)
                            .max_height(200.0)
                            .show(ui, |ui| {
                                ui.add_sized([256.0, 200.0], TextEdit::multiline(self.prompt));
                            });

                        Frame::new()
                            .inner_margin(Vec2::new(0.0, 8.0))
                            .show(ui, |ui| {
                                ui.label("Generación");
                                ui.horizontal(|ui| {
                                    ui.label("Parcial");
                                    toggle_gen_kind(ui, self.full_gen_on);
                                    ui.label("Completa");
                                });

                                ui.horizontal(|ui| {
                                    ui.add_enabled_ui(
                                        !self.waiting_llm_response
                                            && (*self.full_gen_on || *self.rango_valido),
                                        |ui| {
                                            if ui.button("Generar").clicked() {
                                                let _ =
                                                    self.actions_tx.send(EditorAction::PromptLLM);
                                            }
                                        },
                                    );

                                    ui.add_space(2.0);

                                    if self.waiting_llm_response {
                                        ui.spinner();
                                    }
                                });
                            });

                        if !*self.full_gen_on {
                            if let Some(selected_range) = self.selected_range {
                                if let Some(range) =
                                    char_range_to_byte_range(self.content, selected_range)
                                {
                                    *self.rango_valido = true;
                                    if !*self.full_gen_on {
                                        ui.label("Contenido a Reemplazar");

                                        ScrollArea::vertical().id_salt("replace_scroll").show(
                                            ui,
                                            |ui| {
                                                Frame::new().fill(Color32::BLACK).show(ui, |ui| {
                                                    if Range::is_empty(&range) {
                                                        ui.label(format!(
                                                            "(Caracter {})",
                                                            range.start
                                                        ));
                                                    } else {
                                                        ui.label(&self.content[range]);
                                                    }
                                                });
                                            },
                                        );
                                    }
                                } else {
                                    *self.rango_valido = false;
                                    *self.selected_range = None;
                                }
                            } else {
                                *self.rango_valido = false;
                                ui.label("No hay texto seleccionado.");
                            }
                        }
                    });
                });
        })
        .response
    }
}

fn toggle_gen_kind(ui: &mut egui::Ui, full_gen_on: &mut bool) -> egui::Response {
    let desired_size = ui.spacing().interact_size.y * egui::vec2(2.0, 1.0);
    let (rect, mut response) = ui.allocate_exact_size(desired_size, egui::Sense::click());
    if response.clicked() {
        *full_gen_on = !*full_gen_on;
        if *full_gen_on {
            print!("{}", log::debug!("activado modo generación completa"));
        } else {
            print!("{}", log::debug!("activado modo generación parcial"));
        }
        response.mark_changed();
    }
    response.widget_info(|| {
        egui::WidgetInfo::selected(
            egui::WidgetType::Checkbox,
            ui.is_enabled(),
            *full_gen_on,
            "",
        )
    });

    if ui.is_rect_visible(rect) {
        let how_on = ui.ctx().animate_bool_responsive(response.id, *full_gen_on);
        let visuals = ui.style().interact_selectable(&response, *full_gen_on);
        let rect = rect.expand(visuals.expansion);
        let radius = 0.5 * rect.height();
        ui.painter().rect(
            rect,
            radius,
            visuals.bg_fill,
            visuals.bg_stroke,
            egui::StrokeKind::Inside,
        );
        let circle_x = egui::lerp((rect.left() + radius)..=(rect.right() - radius), how_on);
        let center = egui::pos2(circle_x, rect.center().y);
        ui.painter()
            .circle(center, 0.75 * radius, visuals.bg_fill, visuals.fg_stroke);
    }

    response
}

pub fn char_range_to_byte_range(
    text: &str,
    range: &Range<usize>,
) -> Option<std::ops::Range<usize>> {
    if text.is_empty() && range.start == 0 && range.end == 0 {
        return Some(0..0);
    }
    if range.start == text.chars().count() && range.end == text.chars().count() {
        let len = text.len();
        return Some(len..len);
    }
    let start = text.char_indices().nth(range.start).map(|(i, _)| i)?;
    let end = text
        .char_indices()
        .nth(range.end)
        .map(|(i, _)| i)
        .unwrap_or(text.len());
    Some(start..end)
}
