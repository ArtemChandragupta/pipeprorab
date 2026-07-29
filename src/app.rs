use eframe::egui;
use egui_snarl::{
    InPin, NodeId, OutPin, Snarl,
    ui::{PinInfo, SnarlStyle, SnarlViewer},
};

use crate::model::PipeNode;

struct PipeViewer;

impl SnarlViewer<PipeNode> for PipeViewer {
    fn title(&mut self, node: &PipeNode) -> String {
        match node {
            PipeNode::Pump { .. } => "Насос".to_owned(),
            PipeNode::Pipe { .. } => "Труба".to_owned(),
            PipeNode::Fitting { .. } => "Местное сопротивление".to_owned(),
        }
    }

    // Задаем ровно 1 входной пин для каждого блока кроме насоса
    fn inputs(&mut self, node: &PipeNode) -> usize {
        match node {
            PipeNode::Pump { .. } => 0,
            PipeNode::Pipe { .. } | PipeNode::Fitting { .. } => 1,
        }
    }

    // Задаем ровно 1 выходной пин для каждого блока
    fn outputs(&mut self, _node: &PipeNode) -> usize {
        1
    }

    // Отрисовка UI и настройка внешнего вида входного контакта
    #[allow(refining_impl_trait)]
    fn show_input(
        &mut self,
        _pin: &InPin,
        _ui: &mut egui::Ui,
        _snarl: &mut Snarl<PipeNode>,
    ) -> PinInfo {
        PinInfo::square().with_fill(egui::Color32::from_rgb(200, 50, 50))
    }

    // Отрисовка UI и настройка внешнего вида выходного контакта
    #[allow(refining_impl_trait)]
    fn show_output(
        &mut self,
        _pin: &OutPin,
        _ui: &mut egui::Ui,
        _snarl: &mut Snarl<PipeNode>,
    ) -> PinInfo {
        PinInfo::triangle().with_fill(egui::Color32::from_rgb(50, 200, 50))
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
