use eframe::egui::{self, Align, Layout};
use egui_extras::{Column, TableBuilder};

#[derive(Debug)]
pub struct SpreadSheetEditor<'c>(pub &'c mut [[String; 10]; 10]);

impl SpreadSheetEditor<'_> {
    pub fn render(self, ui: &mut egui::Ui) {
        TableBuilder::new(ui)
            .column(Column::exact(20.0))
            .columns(Column::exact(64.0), 10)
            .header(20.0, |mut header| {
                header.col(|ui| {
                    ui.label("");
                });

                for col in 0..10 {
                    let label = ((b'A' + col as u8) as char).to_string();
                    header.col(|ui| {
                        ui.centered_and_justified(|ui| {
                            ui.label(label);
                        });
                    });
                }
            })
            .body(|body| {
                body.rows(28.0, 10, |mut row| {
                    let row_index = row.index();

                    row.col(|ui| {
                        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                            ui.label(format!("{}", row_index + 1));
                        });
                    });

                    for col_index in 0..10 {
                        row.col(|ui| {
                            let in_mem_cell = &self.0[row_index][col_index];

                            if in_mem_cell.starts_with('=') {
                                if let Some(value) = self.eval_formula(in_mem_cell) {
                                    self.0[row_index][col_index] = value;
                                }
                            }

                            ui.text_edit_singleline(&mut self.0[row_index][col_index]);
                        });
                    }
                });
            });
    }

    fn eval_formula(&self, formula: &str) -> Option<String> {
        let expr = formula.trim_start_matches('=').replace(" ", "");

        let ops = ['+', '-', '*', '/'];

        for op in ops {
            if let Some(i) = expr.find(op) {
                let (left, right) = expr.split_at(i);
                let right = &right[1..];

                let (row_1, col_1) = parse_cell_ref(left)?;
                let (row_2, col_2) = parse_cell_ref(right)?;

                let value_1 = self.0[row_1][col_1].parse::<i64>().unwrap();
                let value_2 = self.0[row_2][col_2].parse::<i64>().unwrap();

                return Some(
                    match op {
                        '+' => value_1 + value_2,
                        '-' => value_1 - value_2,
                        '*' => value_1 * value_2,
                        '/' => value_1 / value_2,
                        _ => todo!(),
                    }
                    .to_string(),
                );
            }
        }

        None
    }
}

fn parse_cell_ref(cell: &str) -> Option<(usize, usize)> {
    let first = cell.chars().next()?.to_ascii_uppercase();
    if first < 'A' || first > 'Z' {
        return None;
    }
    let col = (first as u8 - b'A') as usize;

    let row_str = &cell[1..];
    let row_num = row_str.parse::<usize>().ok()?;
    if row_num == 0 {
        return None;
    }
    let row = row_num - 1;

    Some((row, col))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parseo_de_nombre_de_celdas_validos() {
        assert_eq!(parse_cell_ref("A1"), Some((0, 0)));
        assert_eq!(parse_cell_ref("a1"), Some((0, 0)));
        assert_eq!(parse_cell_ref("B2"), Some((1, 1)));
        assert_eq!(parse_cell_ref("C10"), Some((9, 2)));
        assert_eq!(parse_cell_ref("J5"), Some((4, 9)));
    }

    #[test]
    fn parseo_de_nombre_de_celdas_invalidos() {
        assert_eq!(parse_cell_ref(""), None);
        assert_eq!(parse_cell_ref("A"), None);

        assert_eq!(parse_cell_ref("1A"), None);
        assert_eq!(parse_cell_ref("5"), None);

        assert_eq!(parse_cell_ref("AX"), None);
        assert_eq!(parse_cell_ref("C-1"), None);

        assert_eq!(parse_cell_ref("AA10"), None);

        assert_eq!(parse_cell_ref("A0"), None);
    }
}
