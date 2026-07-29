use eframe::egui;

pub struct HydroApp {}

impl Default for HydroApp {
    fn default() -> Self {
        Self {}
    }
}

impl eframe::App for HydroApp {
    fn ui(&mut self, ui: &mut egui::Ui, frame: &mut eframe::Frame) {
        egui::CentralPanel::default().show(ui, |ui| {
            ui.heading("Hello World!");
        });
    }
}
