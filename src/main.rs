mod app;

use app::App;

fn main() -> eframe::Result<()> {
    eframe::run_native(
        "Трубопровод",
        eframe::NativeOptions::default(),
        Box::new(|_| Ok(Box::<App>::default())),
    )
}
