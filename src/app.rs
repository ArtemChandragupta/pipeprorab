use eframe::egui;
use egui_snarl::{Snarl, ui::SnarlStyle};
use std::collections::HashMap;

use crate::export::export_to_excel;
use crate::model::{CalculationResult, PipeNode, calculate_pipeline, flatten_pipeline};
use crate::ui::{docs::*, graph::*, plots::*, table::*};

// Состояние приложения - граф, имя файла и результат(ошибка)
#[derive(Default)]
enum CalculationState {
    #[default]
    Idle,
    Success(CalculationResult),
    Error(String),
}

pub struct HydroApp {
    snarl: Snarl<PipeNode>,
    style: SnarlStyle,
    filename: String,
    calc_state: CalculationState,
    doc_widget: DocWidget,
    show_overwrite_dialog: bool,
    load_error_message: Option<String>,
}

impl Default for HydroApp {
    fn default() -> Self {
        Self {
            snarl: Snarl::new(),
            style: SnarlStyle::default(),
            filename: "NewPipeline".to_owned(),
            calc_state: CalculationState::Idle,
            doc_widget: DocWidget::default(),
            show_overwrite_dialog: false,
            load_error_message: None,
        }
    }
}

impl eframe::App for HydroApp {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        if self.show_overwrite_dialog {
            let full_path = format!("{}.json", self.filename);
            egui::Window::new("Внимание")
                .collapsible(false)
                .resizable(false)
                .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
                .show(ui.ctx(), |ui| {
                    ui.label(format!("Файл «{}» уже существует.", full_path));
                    ui.label("Перезаписать существующий файл или переименовать текущий?");

                    ui.add_space(8.0);

                    ui.horizontal(|ui| {
                        if ui.button("Перезаписать").clicked() {
                            if let Ok(s) = serde_json::to_string_pretty(&self.snarl) {
                                let _ = std::fs::write(&full_path, s);
                            }
                            self.show_overwrite_dialog = false;
                        }

                        if ui.button("Переименовать").clicked() {
                            self.show_overwrite_dialog = false;
                        }
                    });
                });
        }

        if self.load_error_message.is_some() {
            let mut close_error = false;
            egui::Window::new("Ошибка загрузки")
                .collapsible(false)
                .resizable(false)
                .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
                .show(ui.ctx(), |ui| {
                    ui.label(self.load_error_message.as_ref().unwrap());

                    ui.add_space(8.0);

                    ui.with_layout(egui::Layout::top_down(egui::Align::Center), |ui| {
                        if ui.button("ОК").clicked() {
                            close_error = true;
                        }
                    });
                });

            if close_error {
                self.load_error_message = None;
            }
        }

        egui::Panel::top("top_bar").show(ui, |ui| {
            ui.horizontal(|ui| {
                if !self.doc_widget.is_open {
                    ui.heading("Файл:");
                    ui.add(egui::TextEdit::singleline(&mut self.filename).desired_width(150.0));

                    let json_path = format!("{}.json", self.filename);
                    let xlsx_path = format!("{}.xlsx", self.filename);

                    ui.menu_button("📄", |ui| {
                        if ui.button("Загрузить JSON").clicked() {
                            match std::fs::read_to_string(&json_path) {
                                Ok(s) => match serde_json::from_str(&s) {
                                    Ok(snarl) => {
                                        self.snarl = snarl;
                                        self.load_error_message = None;
                                    }
                                    Err(err) => {
                                        self.load_error_message = Some(format!(
                                            "Файл повреждён или имеет неверный формат:\n{}",
                                            err
                                        ));
                                    }
                                },
                                Err(err) => {
                                    if err.kind() == std::io::ErrorKind::NotFound {
                                        self.load_error_message = Some(format!(
                                            "Файл «{}» не найден в корневой папке.",
                                            json_path
                                        ));
                                    } else {
                                        self.load_error_message =
                                            Some(format!("Ошибка чтения файла:\n{}", err));
                                    }
                                }
                            }
                        }

                        if ui.button("Сохранить JSON").clicked() {
                            if std::path::Path::new(&json_path).exists() {
                                self.show_overwrite_dialog = true;
                            } else {
                                if let Ok(s) = serde_json::to_string_pretty(&self.snarl) {
                                    let _ = std::fs::write(&json_path, s);
                                }
                            }
                        }

                        if let CalculationState::Success(res) = &self.calc_state
                            && ui.button("Экспорт в Excel").clicked()
                            && let Err(err) = export_to_excel(res, &xlsx_path)
                        {
                            eprintln!("Ошибка экспорта в Excel: {err}");
                        }
                    });

                    ui.separator();

                    if ui.button("▶ Рассчитать").clicked() {
                        match calculate_pipeline(&self.snarl) {
                            Ok(res) => self.calc_state = CalculationState::Success(res),
                            Err(err) => self.calc_state = CalculationState::Error(err),
                        }
                    }
                }

                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if ui.button("Справка").clicked() {
                        self.doc_widget.is_open = !self.doc_widget.is_open;
                    }

                    if ui.button("Сменить тему").clicked() {
                        if ui.visuals().dark_mode {
                            ui.ctx().set_visuals(egui::Visuals::light());
                        } else {
                            ui.ctx().set_visuals(egui::Visuals::dark());
                        }
                    }
                });
            });
        });

        if !matches!(self.calc_state, CalculationState::Idle) {
            egui::Panel::bottom("calc_panel")
                .resizable(true)
                .show(ui, |ui| {
                    ui.horizontal(|ui| {
                        ui.heading("Результаты");
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            if ui.button("Закрыть").clicked() {
                                self.calc_state = CalculationState::Idle;
                            }
                        });
                    });
                    ui.separator();

                    match &self.calc_state {
                        CalculationState::Idle => {}
                        CalculationState::Error(err_msg) => {
                            ui.colored_label(egui::Color32::RED, err_msg);
                        }
                        CalculationState::Success(res) => {
                            ui.horizontal(|ui| {
                                ui.label(format!(
                                    "Общее сопротивление: {:.4e} Па·с²/м⁶",
                                    res.k_total
                                ));
                                ui.separator();
                                ui.label(format!("Статический напор: {:.2} м", res.h_static));
                                ui.separator();
                                ui.label(format!(
                                    "Рабочая точка: Q: {:.4} м³/с, H: {:.2} м",
                                    res.q_op, res.h_op
                                ));
                                ui.separator();
                                ui.label(format!("Входное давление: {:.1} кПа", res.p_in / 1000.0));
                            });

                            ui.separator();

                            ui.columns(2, |columns| {
                                columns[0].vertical(|ui| {
                                    draw_hq_plot(ui, res, &self.snarl);
                                });

                                columns[1].vertical(|ui| {
                                    ui.label(egui::RichText::new("Состояние элементов"));
                                    egui::ScrollArea::both().show(ui, |ui| {
                                        draw_results_table(ui, &res.pipeline);
                                    });
                                });
                            });
                        }
                    }
                });
        }

        egui::CentralPanel::default().show(ui, |ui| {
            if self.doc_widget.is_open {
                self.doc_widget.render_docs(ui);
            } else {
                let id = ui.make_persistent_id("snarl_editor");

                let mut results_map = HashMap::new();
                if let CalculationState::Success(ref res) = self.calc_state {
                    let mut flat_tree = Vec::new();
                    flatten_pipeline(&res.pipeline, 0, &mut flat_tree);
                    for (_, comp) in flat_tree {
                        results_map.insert(
                            comp.name.clone(),
                            NodeDrawResult {
                                q: comp.state.q,
                                p_in: comp.state.p_in,
                                p_out: comp.state.p_out,
                            },
                        );
                    }
                }

                let mut viewer = PipeViewer {
                    results: results_map,
                };
                self.snarl.show(&mut viewer, &self.style, id, ui);
            }
        });
    }
}
