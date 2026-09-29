use crate::model::{PartType, PipeNode, PipeNodeKind};
use crate::ui::{docs::*, units::*};
use eframe::egui::{self, Ui};
use egui_snarl::{
    InPin, NodeId, OutPin, Snarl,
    ui::{PinInfo, SnarlViewer},
};
use std::collections::HashMap;

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

pub struct NodeDrawResult {
    pub q: f64,
    pub p_in: f64,
    pub p_out: f64,
}
pub struct PipeViewer {
    pub results: HashMap<String, NodeDrawResult>,
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

    // Действия на поле - добавить элемент каждого типа
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
