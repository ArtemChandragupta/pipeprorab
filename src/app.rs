use eframe::egui::{self, Ui};
use egui_snarl::{
    InPin, NodeId, OutPin, Snarl,
    ui::{PinInfo, SnarlStyle, SnarlViewer},
};

use crate::model::PipeNode;

// Слайдер и поле для значения в виде-графе
fn ui_val(ui: &mut Ui, label: &str, val: &mut f64, suf: &str) {
    ui.horizontal(|ui| {
        ui.label(label);
        ui.add(egui::DragValue::new(val).suffix(suf));
    });
}

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
                ui_val(ui, "Длина:", length, " м");
                ui_val(ui, "Диаметр:", diameter, " м");
                ui_val(ui, "Шероховатость:", roughness, " м");
            }
            PipeNode::Fitting { diameter, zeta } => {
                ui_val(ui, "Диаметр:", diameter, " м");
                ui_val(ui, "Сопротивление:", zeta, " ξ");
            }
            PipeNode::Pump { points } => {
                ui.label("Рабочие точки:");
                for (i, (q, h)) in points.iter_mut().enumerate() {
                    ui.horizontal(|ui| {
                        ui.label(format!("{}: Q:", i + 1));
                        ui.add(egui::DragValue::new(q).suffix(" м³/ч"));
                        ui.label("H:");
                        ui.add(egui::DragValue::new(h).suffix(" м"));
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

pub struct HydroApp {
    snarl: Snarl<PipeNode>,
    style: SnarlStyle,
}

impl Default for HydroApp {
    fn default() -> Self {
        Self {
            snarl: Snarl::new(),
            style: SnarlStyle::default(),
        }
    }
}

impl eframe::App for HydroApp {
    fn ui(&mut self, ui: &mut egui::Ui, frame: &mut eframe::Frame) {
        egui::Panel::top("top_bar").show(ui, |ui| {
            ui.heading("Hello World!");
        });

        egui::CentralPanel::default().show(ui, |ui| {
            let id = ui.make_persistent_id("snarl_editor");
            self.snarl.show(&mut PipeViewer, &self.style, id, ui);
        });
    }
}
