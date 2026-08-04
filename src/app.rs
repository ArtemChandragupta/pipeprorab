use eframe::egui::{self, Ui};
use egui_snarl::{
    InPin, NodeId, OutPin, Snarl,
    ui::{PinInfo, SnarlStyle, SnarlViewer},
};
use std::fmt::Write;

use crate::model::{
    Component, ElementKind, PipeNode, Pump, build_model, calc_flow_pressure, get_static_pressure,
    solve_operating_point, update_k,
};

const G_GRAV: f64 = 9.81;
const RHO: f64 = 1000.0;
const NU: f64 = 1e-6;

// Слайдер и поле для значения в виде-графе
fn ui_val(ui: &mut Ui, label: &str, val: &mut f64) {
    ui.horizontal(|ui| {
        ui.label(label);
        ui.add(egui::DragValue::new(val));
    });
}

// Поле с трубопроводом
struct PipeViewer;
impl SnarlViewer<PipeNode> for PipeViewer {
    fn connect(&mut self, from: &OutPin, to: &InPin, snarl: &mut Snarl<PipeNode>) {
        snarl.connect(from.id, to.id);
    }

    fn disconnect(&mut self, from: &OutPin, to: &InPin, snarl: &mut Snarl<PipeNode>) {
        snarl.disconnect(from.id, to.id);
    }

    fn title(&mut self, node: &PipeNode) -> String {
        match node {
            PipeNode::Pump { .. } => "Насос".to_owned(),
            PipeNode::Pipe { .. } => "Труба".to_owned(),
            PipeNode::Fitting { .. } => "Местное сопротивление".to_owned(),
        }
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
        ui.vertical(|ui| match &mut snarl[node] {
            PipeNode::Pipe {
                length,
                diameter,
                roughness,
            } => {
                ui_val(ui, "Длина (м):", length);
                ui_val(ui, "Диаметр (м):", diameter);
                ui_val(ui, "Шероховатость (м):", roughness);
            }
            PipeNode::Fitting { diameter, zeta } => {
                ui_val(ui, "Диаметр (м):", diameter);
                ui_val(ui, "Сопротивление (ξ):", zeta);
            }
            PipeNode::Pump { points } => {
                ui.label("Рабочие точки:");
                for (i, (q, h)) in points.iter_mut().enumerate() {
                    ui.horizontal(|ui| {
                        ui.label(format!("{}: Q (м³/ч):", i + 1));
                        ui.add(egui::DragValue::new(q));
                        ui.label("H (м):");
                        ui.add(egui::DragValue::new(h));
                    });
                }
            }
        });
    }

    // 1 входной пин для каждого блока кроме насоса
    fn inputs(&mut self, node: &PipeNode) -> usize {
        match node {
            PipeNode::Pump { .. } => 0,
            PipeNode::Pipe { .. } | PipeNode::Fitting { .. } => 1,
        }
    }
    #[allow(refining_impl_trait)]
    fn show_input(
        &mut self,
        _pin: &InPin,
        _ui: &mut egui::Ui,
        _snarl: &mut Snarl<PipeNode>,
    ) -> PinInfo {
        PinInfo::square().with_fill(egui::Color32::from_rgb(200, 50, 50))
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
        PinInfo::triangle().with_fill(egui::Color32::from_rgb(50, 200, 50))
    }

    // Действия на элементе - пока только убрать
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
        if ui.button("Труба").clicked() {
            snarl.insert_node(
                pos,
                PipeNode::Pipe {
                    length: 1.0,
                    diameter: 1.0,
                    roughness: 0.1,
                },
            );
            ui.close();
        }
        if ui.button("Местное сопротивление").clicked() {
            snarl.insert_node(
                pos,
                PipeNode::Fitting {
                    diameter: 1.0,
                    zeta: 1.0,
                },
            );
            ui.close();
        }
        if ui.button("Насос").clicked() {
            snarl.insert_node(
                pos,
                PipeNode::Pump {
                    points: [(1.0, 1.0), (2.0, 0.5), (3.0, 0.3)],
                },
            );
            ui.close();
        }
    }
}

fn format_element_tree(comp: &Component, depth: usize, out: &mut String) {
    let name_col = format!("{}{}", "  ".repeat(depth), comp.name);
    let name_str = if name_col.chars().count() > 30 {
        format!("{}...", name_col.chars().take(27).collect::<String>())
    } else {
        name_col
    };
    let st = &comp.state;

    let _ = writeln!(
        out,
        "| {name_str:<30} | {:<10} | {:>12.2} | {:>10.1} | {:>11.1} | {:>8.1} |",
        comp.type_name(),
        st.q * 1000.0,
        st.p_in / 1000.0,
        st.p_out / 1000.0,
        (st.p_in - st.p_out) / 1000.0
    );

    if let ElementKind::Series(elems) | ElementKind::Parallel(elems) = &comp.kind {
        for sub in elems {
            format_element_tree(sub, depth + 1, out);
        }
    }
}

pub struct HydroApp {
    snarl: Snarl<PipeNode>,
    style: SnarlStyle,
    filename: String,
    calc_result: String,
}

impl Default for HydroApp {
    fn default() -> Self {
        Self {
            snarl: Snarl::new(),
            style: SnarlStyle::default(),
            filename: "NewPipeline.json".to_owned(),
            calc_result: String::new(),
        }
    }
}

impl eframe::App for HydroApp {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        egui::Panel::top("top_bar").show(ui, |ui| {
            ui.horizontal(|ui| {
                ui.heading("Файл:");
                ui.add(egui::TextEdit::singleline(&mut self.filename).desired_width(150.0));

                if ui.button("Сохранить JSON").clicked()
                    && let Ok(s) = serde_json::to_string_pretty(&self.snarl)
                {
                    let _ = std::fs::write(&self.filename, s);
                }
                if ui.button("Загрузить JSON").clicked()
                    && let Ok(s) = std::fs::read_to_string(&self.filename)
                    && let Ok(snarl) = serde_json::from_str(&s)
                {
                    self.snarl = snarl;
                }

                ui.separator();

                if ui.button("▶ Рассчитать").clicked() {
                    let mut res = String::new();
                    match build_model(&self.snarl) {
                        Ok((pts, mut pipeline)) => match Pump::from_points(&pts) {
                            Ok(pump) => {
                                let (mut q_op, mut h_op, mut k_total, mut converged) = (0.1, 0.0, 1.0, false);
                                let h_static = get_static_pressure(&pipeline) / (RHO * G_GRAV);

                                for _ in 0..100 {
                                    k_total = update_k(&mut pipeline, q_op);
                                    if let Ok((q_new, h_new)) = solve_operating_point(&pump, k_total, h_static) {
                                        if (q_new - q_op).abs() < 1e-6 { q_op = q_new; h_op = h_new; converged = true; break; }
                                        q_op = (q_op + q_new) / 2.0;
                                    } else { break; }
                                }

                                if converged {
                                    let start_p = RHO * G_GRAV * h_op;
                                    calc_flow_pressure(&mut pipeline, q_op, start_p);
                                    let _ = writeln!(res, "=== ХАРАКТЕРИСТИКИ СЕТИ ===\nОбщее сопротивление : {k_total:.4e} Па·с²/м⁶\nСтатический напор   : {h_static:.2} м\nРабочая точка       : Q = {:.2} л/с, H = {h_op:.2} м\nДавление на входе   : {:.1} кПа\n\n=== СТРУКТУРА ===\n| {:<30} | {:<10} | {:>12} | {:>10} | {:>11} | {:>8} |\n|{}|{}|{}|{}|{}|{}|",
                                        q_op * 1000.0, start_p / 1000.0, "Элемент", "Тип", "Расход (л/с)", "P вх (кПа)", "P вых (кПа)", "dP (кПа)", "-".repeat(32), "-".repeat(12), "-".repeat(14), "-".repeat(12), "-".repeat(13), "-".repeat(10));
                                    format_element_tree(&pipeline, 0, &mut res);
                                } else { res.push_str("Ошибка: Расчет не сошелся.\n"); }
                            },
                            Err(e) => { let _ = writeln!(res, "Ошибка насоса: {e}"); }
                        },
                        Err(e) => { let _ = writeln!(res, "Ошибка модели: {e}"); }
                    }
                    self.calc_result = res;
                }
            });
        });

        if !self.calc_result.is_empty() {
            egui::Panel::bottom("calc_panel")
                .resizable(true)
                .show(ui, |ui| {
                    ui.horizontal(|ui| {
                        ui.heading("Результаты");
                        if ui.button("Закрыть").clicked() {
                            self.calc_result.clear();
                        }
                    });
                    ui.separator();
                    egui::ScrollArea::vertical().show(ui, |ui| {
                        ui.label(egui::RichText::new(&self.calc_result).monospace())
                    });
                });
        }

        egui::CentralPanel::default().show(ui, |ui| {
            let id = ui.make_persistent_id("snarl_editor");
            self.snarl.show(&mut PipeViewer, &self.style, id, ui);
        });
    }
}
