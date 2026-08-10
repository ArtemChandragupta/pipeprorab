use eframe::egui::{self, Ui};
use egui_extras::{Column, TableBuilder};
use egui_plot::{Legend, Line, Plot, PlotPoints, Points};
use egui_snarl::{
    InPin, NodeId, OutPin, Snarl,
    ui::{PinInfo, SnarlStyle, SnarlViewer},
};

use crate::model::{
    CalculationResult, Component, ElementKind, G_GRAV, PipeNode, RHO, calculate_pipeline,
};

// Слайдеры и поля для значения в виде-графе
fn ui_val(ui: &mut Ui, label: &str, val: &mut f64) {
    ui.horizontal(|ui| {
        ui.label(label);
        ui.add(
            egui::DragValue::new(val)
                .range(0.0..=f32::INFINITY)
                .speed(0.01),
        );
    });
}

fn ui_val_mm(ui: &mut egui::Ui, label: &str, val_m: &mut f64) {
    let mut val_mm = *val_m * 1000.0;
    ui.horizontal(|ui| {
        ui.label(label);
        if ui
            .add(
                egui::DragValue::new(&mut val_mm)
                    .speed(0.01)
                    .range(0.0..=f64::INFINITY),
            )
            .changed()
        {
            *val_m = val_mm / 1000.0;
        }
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
                ui_val_mm(ui, "Диаметр (мм):", diameter);
                ui_val_mm(ui, "Шероховатость (мм):", roughness);
            }
            PipeNode::Fitting { diameter, zeta } => {
                ui_val_mm(ui, "Диаметр (мм):", diameter);
                ui_val(ui, "Сопротивление (ξ):", zeta);
            }
            PipeNode::Pump { points } => {
                ui.label("Рабочие точки:");
                for (i, (q, h)) in points.iter_mut().enumerate() {
                    ui.horizontal(|ui| {
                        ui.label(format!("{}: Q (м³/с):", i + 1));
                        ui.add(egui::DragValue::new(q).speed(0.00001).max_decimals(6));
                        ui.label("H (м):");
                        ui.add(egui::DragValue::new(h).speed(0.01));
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
                    diameter: 0.1,
                    roughness: 0.0001,
                },
            );
            ui.close();
        }
        if ui.button("Местное сопротивление").clicked() {
            snarl.insert_node(
                pos,
                PipeNode::Fitting {
                    diameter: 0.1,
                    zeta: 1.0,
                },
            );
            ui.close();
        }
        if ui.button("Насос").clicked() {
            snarl.insert_node(
                pos,
                PipeNode::Pump {
                    points: [(0.01, 20.0), (0.02, 15.0), (0.03, 5.0)],
                },
            );
            ui.close();
        }
    }
}

// Отрисовка таблицы и вспомогательная функция
fn flatten_pipeline<'a>(comp: &'a Component, depth: usize, out: &mut Vec<(usize, &'a Component)>) {
    out.push((depth, comp));
    if let ElementKind::Series(elems) | ElementKind::Parallel(elems) = &comp.kind {
        for sub in elems {
            flatten_pipeline(sub, depth + 1, out);
        }
    }
}

fn draw_results_table(ui: &mut egui::Ui, pipeline: &Component) {
    let mut flat_tree = Vec::new();
    flatten_pipeline(pipeline, 0, &mut flat_tree);

    TableBuilder::new(ui)
        .striped(true)
        .resizable(true)
        .cell_layout(egui::Layout::left_to_right(egui::Align::Center))
        .column(Column::initial(220.0).at_least(100.0)) // Название
        .column(Column::initial(100.0).at_least(60.0)) // Тип
        .column(Column::initial(100.0).at_least(60.0)) // Расход
        .column(Column::initial(100.0).at_least(60.0)) // P вх
        .column(Column::initial(100.0).at_least(60.0)) // P вых
        .column(Column::remainder().at_least(80.0)) // dP
        .header(24.0, |mut header| {
            header.col(|ui| {
                ui.strong("Элемент");
            });
            header.col(|ui| {
                ui.strong("Тип");
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
                        ui.label(comp.type_name());
                    });
                    row.col(|ui| {
                        ui.label(format!("{:.4}", comp.state.q));
                    });
                    row.col(|ui| {
                        ui.label(format!("{:.1}", comp.state.p_in / 1000.0));
                    });
                    row.col(|ui| {
                        ui.label(format!("{:.1}", comp.state.p_out / 1000.0));
                    });
                    row.col(|ui| {
                        ui.label(format!(
                            "{:.1}",
                            (comp.state.p_in - comp.state.p_out) / 1000.0
                        ));
                    });
                });
            }
        });
}

// Характеристики сети и насоса
fn interpolate_quadratic(p0: [f64; 2], p1: [f64; 2], p2: [f64; 2], q: f64) -> f64 {
    let (q0, h0) = (p0[0], p0[1]);
    let (q1, h1) = (p1[0], p1[1]);
    let (q2, h2) = (p2[0], p2[1]);

    let l0 = (q - q1) * (q - q2) / ((q0 - q1) * (q0 - q2));
    let l1 = (q - q0) * (q - q2) / ((q1 - q0) * (q1 - q2));
    let l2 = (q - q0) * (q - q1) / ((q2 - q0) * (q2 - q1));

    h0 * l0 + h1 * l1 + h2 * l2
}

pub fn draw_hq_plot(ui: &mut egui::Ui, res: &CalculationResult, snarl: &Snarl<PipeNode>) {
    ui.heading("Характеристики системы (Q-H)");

    let plot = Plot::new("hq_plot")
        .height(300.0)
        .x_axis_label("Расход Q (м³/с)")
        .y_axis_label("Напор H (м)")
        .legend(Legend::default())
        .include_x(0.0)
        .allow_zoom(true)
        .include_y(0.0);

    plot.show(ui, |plot_ui| {
        let op_q = res.q_op;
        let op_h = res.h_op;

        let q_max = (op_q * 1.5).max(0.01);
        let steps = 100;

        // 1. Построение параболы сети
        let mut net_points = Vec::with_capacity(steps + 1);
        for i in 0..=steps {
            let q = q_max * (i as f64) / (steps as f64);
            let h = res.h_static + res.k_total * q * q / (RHO * G_GRAV);
            net_points.push([q, h]);
        }

        plot_ui.line(
            Line::new("Кривая сети", PlotPoints::from(net_points)).color(egui::Color32::BLUE),
        );

        // 2. Построение характеристики насоса
        for node in snarl.nodes() {
            if let PipeNode::Pump { points } = node {
                let mut pts: Vec<[f64; 2]> = points.iter().map(|&(q, h)| [q, h]).collect();

                pts.sort_by(|a, b| a[0].partial_cmp(&b[0]).unwrap());

                if pts.len() >= 3 {
                    // Строим плавную параболу по первым трем точкам
                    let p0 = pts[0];
                    let p1 = pts[1];
                    let p2 = pts[2];

                    let min_q = 0.0;
                    let max_q = pts.last().unwrap()[0] * 1.15;

                    let mut dense_pump_pts = Vec::with_capacity(steps + 1);
                    for i in 0..=steps {
                        let q = min_q + (max_q - min_q) * (i as f64) / (steps as f64);
                        let h = interpolate_quadratic(p0, p1, p2, q);
                        dense_pump_pts.push([q, h]);
                    }

                    // Линия характеристики насоса
                    plot_ui.line(
                        Line::new("Кривая насоса", PlotPoints::from(dense_pump_pts))
                            .color(egui::Color32::RED),
                    );

                    // Опорные точки насоса
                    plot_ui.points(
                        Points::new("Точки насоса", PlotPoints::from(pts))
                            .radius(3.5)
                            .color(egui::Color32::RED),
                    );
                } else {
                    plot_ui.line(
                        Line::new("Насос (ломаная)", PlotPoints::from(pts))
                            .color(egui::Color32::RED),
                    );
                }
            }
        }

        // 3. Рабочая точка
        plot_ui.points(
            Points::new("Рабочая точка", PlotPoints::from(vec![[op_q, op_h]]))
                .radius(5.0)
                .color(egui::Color32::GREEN),
        );
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
}

impl Default for HydroApp {
    fn default() -> Self {
        Self {
            snarl: Snarl::new(),
            style: SnarlStyle::default(),
            filename: "NewPipeline.json".to_owned(),
            calc_state: CalculationState::Idle,
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
                    match calculate_pipeline(&self.snarl) {
                        Ok(res) => self.calc_state = CalculationState::Success(res),
                        Err(err) => self.calc_state = CalculationState::Error(err),
                    }
                }

                egui::widgets::global_theme_preference_buttons(ui);
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
            let id = ui.make_persistent_id("snarl_editor");
            self.snarl.show(&mut PipeViewer, &self.style, id, ui);
        });
    }
}
