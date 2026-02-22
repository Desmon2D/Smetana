mod app;
mod model;
mod editor;
mod panels;
mod export;
mod persistence;
mod history;

fn main() -> eframe::Result {
    let options = eframe::NativeOptions {
        viewport: eframe::egui::ViewportBuilder::default()
            .with_inner_size([1280.0, 720.0])
            .with_title("Сметана — Строительная смета"),
        ..Default::default()
    };

    eframe::run_native(
        "Сметана",
        options,
        Box::new(|cc| Ok(Box::new(app::App::new(cc)))),
    )
}
