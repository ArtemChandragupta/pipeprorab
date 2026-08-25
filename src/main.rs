mod app;
mod docs;
mod model;

use app::HydroApp;

fn main() -> eframe::Result<()> {
    eframe::run_native(
        "Трубопровод",
        eframe::NativeOptions::default(),
        Box::new(|cc| {
            egui_extras::install_image_loaders(&cc.egui_ctx);
            Ok(Box::<HydroApp>::default())
        }),
    )
}
