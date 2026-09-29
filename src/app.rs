use eframe::egui::{self, Ui};
use egui_extras::{Column, TableBuilder};
use egui_snarl::{
    InPin, NodeId, OutPin, Snarl,
    ui::{PinInfo, SnarlStyle, SnarlViewer},
};
use std::collections::HashMap;

use crate::export::export_to_excel;
use crate::model::{
    CalculationResult, Component, PartType, PipeNode, PipeNodeKind, calculate_pipeline,
    flatten_pipeline,
};
use crate::ui::docs::*;
use crate::ui::plots::*;
use crate::ui::units::*;

// Поле ввода
fn ui_unit_input(
    ui: &mut egui::Ui,
    id_salt: impl std::hash::Hash + std::fmt::Debug,
    label: &str,
    val_si: &mut f64,
    units: &[(f64, &str)],
    default_idx: usize,
) {
    let id = ui.make_persistent_id(id_salt);
    let selected_idx = ui.data_mut(|d| d.get_temp::<usize>(id).unwrap_or(default_idx));
    let mut new_idx = selected_idx;

    let (factor, unit_name) = units[new_idx];

    ui.horizontal(|ui| {
        ui.label(label);

        let mut display_val = *val_si / factor;
        if ui
            .add(
                egui::DragValue::new(&mut display_val)
                    .speed(0.01)
                    .max_decimals(4),
            )
            .changed()
        {
            *val_si = display_val * factor; // Возвращаем в СИ
        }

        if units.len() > 1 {
            egui::ComboBox::from_id_salt(id.with("combo"))
                .selected_text(unit_name)
                .width(0.0)
                .show_ui(ui, |ui| {
                    for (i, &(_, name)) in units.iter().enumerate() {
                        ui.selectable_value(&mut new_idx, i, name);
                    }
                });
        } else if !unit_name.is_empty() {
            ui.label(unit_name);
        }
    });

    if new_idx != selected_idx {
        ui.data_mut(|d| d.insert_temp(id, new_idx));
    }
}

// Поле вывода
fn ui_unit_output(
    ui: &mut egui::Ui,
    id_salt: impl std::hash::Hash + std::fmt::Debug,
    label: &str,
    val_si: f64,
    units: &[(f64, &str)],
    default_idx: usize,
    decimals: usize,
) {
    let id = ui.make_persistent_id(id_salt);
    let selected_idx = ui.data_mut(|d| d.get_temp::<usize>(id).unwrap_or(default_idx));
    let mut new_idx = selected_idx;

    let (factor, unit_name) = units[new_idx];

    ui.horizontal(|ui| {
        ui.label(label);
        let text = format!("{:.*}", decimals, val_si / factor);
        ui.label(text);

        if units.len() > 1 {
            egui::ComboBox::from_id_salt(id.with("combo"))
                .selected_text(unit_name)
                .width(0.0)
                .show_ui(ui, |ui| {
                    for (i, &(_, name)) in units.iter().enumerate() {
                        ui.selectable_value(&mut new_idx, i, name);
                    }
                });
        } else if !unit_name.is_empty() {
            ui.label(unit_name);
        }
    });

    if new_idx != selected_idx {
        ui.data_mut(|d| d.insert_temp(id, new_idx));
    }
}

// Поле с трубопроводом
struct NodeDrawResult {
    q: f64,
    p_in: f64,
    p_out: f64,
}
struct PipeViewer {
    results: HashMap<String, NodeDrawResult>,
}
impl SnarlViewer<PipeNode> for PipeViewer {
    fn connect(&mut self, from: &OutPin, to: &InPin, snarl: &mut Snarl<PipeNode>) {
        snarl.connect(from.id, to.id);
    }

    fn disconnect(&mut self, from: &OutPin, to: &InPin, snarl: &mut Snarl<PipeNode>) {
        snarl.disconnect(from.id, to.id);
    }

    fn title(&mut self, node: &PipeNode) -> String {
        node.name.to_owned()
    }

    // Видимые тела блоков
    fn has_body(&mut self, _: &PipeNode) -> bool {
        true
    }
    fn show_body(
        &mut self,
        node: NodeId,
        _: &[InPin],
        _: &[OutPin],
        ui: &mut Ui,
        snarl: &mut Snarl<PipeNode>,
    ) {
        ui.set_max_width(180.0);

        ui.vertical(|ui| {
            ui.horizontal(|ui| {
                ui.label("Имя:");
                ui.text_edit_singleline(&mut snarl[node].name);
            });
            ui.separator();

            use PartType::*;
            use PipeNodeKind::*;

            match &mut snarl[node].kind {
                Type(Pipe { l, d, r }) => {
                    ui_unit_input(ui, (node, "l"), "Длина:", l, UNITS_LENGTH, 0);
                    ui_unit_input(ui, (node, "d"), "Диаметр:", d, UNITS_LENGTH, 2);
                    ui_unit_input(ui, (node, "r"), "Шероховатость:", r, UNITS_LENGTH, 2);
                }
                Type(Fitting { d, z }) => {
                    ui_unit_input(ui, (node, "d"), "Диаметр:", d, UNITS_LENGTH, 2);
                    ui_unit_input(ui, (node, "z"), "Сопротивление:", z, UNITS_NONE, 0);
                }
                Type(ValveKv { kv }) => {
                    ui_unit_input(ui, (node, "kv"), "Kv/Cv:", kv, UNITS_KV, 0);
                }
                Type(Orifice { d1, d0 }) => {
                    ui_unit_input(ui, (node, "d1"), "Диаметр наружный:", d1, UNITS_LENGTH, 2);
                    ui_unit_input(ui, (node, "d0"), "Диаметр внутренний:", d0, UNITS_LENGTH, 2);
                }
                Type(Elbow { d, angle, r_d }) => {
                    ui_unit_input(ui, (node, "d"), "Диаметр:", d, UNITS_LENGTH, 2);
                    ui_unit_input(ui, (node, "angle"), "Угол (градус):", angle, UNITS_NONE, 0);
                    ui_unit_input(
                        ui,
                        (node, "r_d"),
                        "Радиус поворота / диаметр:",
                        r_d,
                        UNITS_NONE,
                        0,
                    );
                }
                Type(SuddenExpansion { d1, d2 }) => {
                    ui_unit_input(ui, (node, "d1"), "Диаметр начальный:", d1, UNITS_LENGTH, 2);
                    ui_unit_input(ui, (node, "d2"), "Диаметр конечный:", d2, UNITS_LENGTH, 2);
                }
                Type(SmoothExpansion { d1, d2, angle }) => {
                    ui_unit_input(ui, (node, "d1"), "Диаметр начальный:", d1, UNITS_LENGTH, 2);
                    ui_unit_input(ui, (node, "d2"), "Диаметр конечный:", d2, UNITS_LENGTH, 2);
                    ui_unit_input(
                        ui,
                        (node, "angle"),
                        "Угол (градус):",
                        angle,
                        UNITS_LENGTH,
                        0,
                    );
                }
                Type(HeightDrop { dh }) => {
                    ui_unit_input(ui, (node, "dh"), "Δh:", dh, UNITS_LENGTH, 0);
                }
                Type(PressureDrop { dp }) => {
                    ui_unit_input(ui, (node, "dp"), "ΔP:", dp, UNITS_PRESSURE, 1);
                }
                PipeNodeKind::Pump { points } => {
                    ui.label("Рабочие точки:");
                    for (i, (q, h)) in points.iter_mut().enumerate() {
                        ui.group(|ui| {
                            ui_unit_input(
                                ui,
                                (node, "q", i),
                                format!("Q {}:", i + 1).as_str(),
                                q,
                                UNITS_FLOW,
                                0,
                            );
                            ui_unit_input(
                                ui,
                                (node, "h", i),
                                format!("H {}:", i + 1).as_str(),
                                h,
                                UNITS_LENGTH,
                                0,
                            );
                        });
                    }
                }
            }

            if let Some(res) = self.results.get(&snarl[node].name) {
                ui.separator();
                ui.group(|ui| {
                    ui.label(egui::RichText::new("Результаты расчёта").strong());

                    ui_unit_output(ui, (node, "res_q"), "Q:", res.q, UNITS_FLOW, 0, 4);
                    ui_unit_output(
                        ui,
                        (node, "res_pin"),
                        "P вх:",
                        res.p_in,
                        UNITS_PRESSURE,
                        1,
                        1,
                    );
                    ui_unit_output(
                        ui,
                        (node, "res_pout"),
                        "P вых:",
                        res.p_out,
                        UNITS_PRESSURE,
                        1,
                        1,
                    );
                });
            }
        });
    }

    // 1 входной пин для каждого блока кроме насоса
    fn inputs(&mut self, node: &PipeNode) -> usize {
        match &node.kind {
            PipeNodeKind::Pump { .. } => 0,
            _ => 1,
        }
    }
    #[allow(refining_impl_trait)]
    fn show_input(
        &mut self,
        _pin: &InPin,
        _ui: &mut egui::Ui,
        _snarl: &mut Snarl<PipeNode>,
    ) -> PinInfo {
        PinInfo::square().with_fill(egui::Color32::RED)
    }

    // 1 выходной пин для каждого блока
    fn outputs(&mut self, _node: &PipeNode) -> usize {
        1
    }
    #[allow(refining_impl_trait)]
    fn show_output(
        &mut self,
        _pin: &OutPin,
        _ui: &mut egui::Ui,
        _snarl: &mut Snarl<PipeNode>,
    ) -> PinInfo {
        PinInfo::triangle().with_fill(egui::Color32::GREEN)
    }

    // Действия на элементе - убрать и копировать
    fn has_node_menu(&mut self, _: &PipeNode) -> bool {
        true
    }
    fn show_node_menu(
        &mut self,
        node: NodeId,
        _: &[InPin],
        _: &[OutPin],
        ui: &mut Ui,
        snarl: &mut Snarl<PipeNode>,
    ) {
        if ui.button("Копировать элемент").clicked() {
            let mut cloned_node = snarl[node].clone();
            cloned_node.name = format!("{} (копия)", cloned_node.name);

            let old_pos = if let Some(node_info) = snarl.get_node_info(node) {
                node_info.pos
            } else {
                egui::Pos2::ZERO
            };
            let new_pos = old_pos + egui::vec2(50.0, 50.0);

            snarl.insert_node(new_pos, cloned_node);

            ui.close();
        }
        if ui.button("Убрать элемент").clicked() {
            snarl.remove_node(node);
            ui.close();
        }
    }

    // Действия на поле - добавить элемент каждого типа. Потом добавлю приближение к видимой области
    fn has_graph_menu(&mut self, _: egui::Pos2, _: &mut Snarl<PipeNode>) -> bool {
        true
    }
    fn show_graph_menu(&mut self, pos: egui::Pos2, ui: &mut Ui, snarl: &mut Snarl<PipeNode>) {
        ui.label("Добавить элемент");
        let idx = snarl.node_ids().count() + 1;

        use PartType::*;
        use PipeNodeKind::*;

        let mut add_btn = |ui: &mut egui::Ui, label: &str, kind: PipeNodeKind| {
            let response = ui.button(label);

            response.clone().on_hover_ui(|ui| show_tooltip(ui, label));

            if response.clicked() {
                let name = if label == "Насос" {
                    label.to_owned()
                } else {
                    format!("{label} {idx}")
                };

                snarl.insert_node(pos, PipeNode { name, kind });
                ui.close();
            }
        };

        add_btn(
            ui,
            "Насос",
            Pump {
                points: [(0.01, 20.0), (0.02, 15.0), (0.03, 5.0)],
            },
        );
        add_btn(
            ui,
            "Труба",
            Type(Pipe {
                l: 1.0,
                d: 0.1,
                r: 0.0001,
            }),
        );
        ui.menu_button("Арматура и фитинги", |ui| {
            add_btn(
                ui,
                "Местное сопротивление",
                Type(Fitting { d: 0.1, z: 1.0 }),
            );
            add_btn(ui, "Клапан типа \"Рей\"", Type(Fitting { d: 0.1, z: 3.4 }));
            add_btn(ui, "Клапан штампованный", Type(Fitting { d: 0.1, z: 7.8 }));
            add_btn(ui, "Задвижка клинкетная", Type(Fitting { d: 0.1, z: 0.2 }));
            add_btn(
                ui,
                "Задвижка с рычажным затвором",
                Type(Fitting { d: 0.1, z: 0.75 }),
            );
            add_btn(ui, "Клапан по Kv", Type(ValveKv { kv: 10.0 }));
            add_btn(ui, "Диафрагма", Type(Orifice { d1: 0.1, d0: 0.01 }));
            add_btn(
                ui,
                "Поворотное колено",
                Type(Elbow {
                    d: 0.1,
                    angle: 90.0,
                    r_d: 10.0,
                }),
            );
        });
        ui.menu_button("Изменение сечения", |ui| {
            add_btn(ui, "Резкое", Type(SuddenExpansion { d1: 0.1, d2: 0.2 }));
            add_btn(
                ui,
                "Гладкое",
                Type(SmoothExpansion {
                    d1: 0.1,
                    d2: 0.2,
                    angle: 15.0,
                }),
            );
        });
        ui.menu_button("Условия среды", |ui| {
            add_btn(ui, "Перепад высоты", Type(HeightDrop { dh: 1.0 }));
            add_btn(ui, "Падение давления", Type(PressureDrop { dp: 1000.0 }));
        });
    }
}

// Отрисовка таблицы
fn draw_results_table(ui: &mut egui::Ui, pipeline: &Component) {
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
