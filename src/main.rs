mod app;
mod model;

use app::HydroApp;

fn main() -> eframe::Result<()> {
    eframe::run_native(
        "Трубопровод",
        eframe::NativeOptions::default(),
        Box::new(|_| Ok(Box::<HydroApp>::default())),
    )
}
