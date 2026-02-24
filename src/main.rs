mod app;
mod editor;
mod model;
mod persistence;

fn load_icon() -> eframe::egui::IconData {
    let img = image::load_from_memory(include_bytes!("../assets/icon.png"))
        .expect("Failed to load icon")
        .into_rgba8();
    let (w, h) = img.dimensions();
    eframe::egui::IconData {
        rgba: img.into_raw(),
        width: w,
        height: h,
    }
}

fn main() -> eframe::Result {
    let options = eframe::NativeOptions {
        viewport: eframe::egui::ViewportBuilder::default()
            .with_inner_size([1280.0, 720.0])
            .with_title("Сметана — Строительная смета")
            .with_icon(std::sync::Arc::new(load_icon())),
        ..Default::default()
    };

    eframe::run_native(
        "Сметана",
        options,
        Box::new(|cc| Ok(Box::new(app::App::new(cc)))),
    )
}
