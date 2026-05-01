//! bike-fitter-1000 — GUI entry point.

#![warn(clippy::all)]

use eframe::egui;

fn main() -> eframe::Result<()> {
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info")).init();

    let native_options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([1280.0, 800.0])
            .with_min_inner_size([800.0, 600.0])
            .with_title("bike-fitter-1000"),
        ..Default::default()
    };

    eframe::run_native(
        "bike-fitter-1000",
        native_options,
        Box::new(|_cc| Ok(Box::new(App::default()))),
    )
}

#[derive(Default)]
struct App;

impl eframe::App for App {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        egui::CentralPanel::default().show(ctx, |ui| {
            ui.heading("bike-fitter-1000");
            ui.label("Scaffold up. Renderer and solver coming next.");
        });
    }
}
