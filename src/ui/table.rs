use crate::model::{Component, flatten_pipeline};
use egui_extras::{Column, TableBuilder};

pub fn draw_results_table(ui: &mut egui::Ui, pipeline: &Component) {
    let mut flat_tree = Vec::new();
    flatten_pipeline(pipeline, 0, &mut flat_tree);

    TableBuilder::new(ui)
        .striped(true)
        .resizable(true)
        .cell_layout(egui::Layout::left_to_right(egui::Align::Center))
        .columns(Column::auto(), 5)
        .header(24.0, |mut header| {
            header.col(|ui| {
                ui.strong("Элемент");
            });
            header.col(|ui| {
                ui.strong("Расход (м³/с)");
            });
            header.col(|ui| {
                ui.strong("P вх (кПа)");
            });
            header.col(|ui| {
                ui.strong("P вых (кПа)");
            });
            header.col(|ui| {
                ui.strong("dP (кПа)");
            });
        })
        .body(|mut body| {
            for (depth, comp) in flat_tree {
                body.row(22.0, |mut row| {
                    row.col(|ui| {
                        let indent = "   ".repeat(depth);
                        ui.label(format!("{indent}{}", comp.name));
                    });
                    row.col(|ui| {
                        ui.label(format!("{:.4}", comp.state.q));
                    });
                    row.col(|ui| {
                        ui.label(format!("{:.4}", comp.state.p_in / 1000.0));
                    });
                    row.col(|ui| {
                        ui.label(format!("{:.4}", comp.state.p_out / 1000.0));
                    });
                    row.col(|ui| {
                        ui.label(format!(
                            "{:.4}",
                            (comp.state.p_in - comp.state.p_out) / 1000.0
                        ));
                    });
                });
            }
        });
}
