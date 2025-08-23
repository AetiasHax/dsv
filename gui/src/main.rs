mod app;
mod client;
mod config;
mod tasks;
mod ui;
mod util;
mod views;

use eframe::egui;

use crate::app::DsvApp;

fn main() -> eframe::Result {
    env_logger::builder()
        .filter_level(log::LevelFilter::Info)
        .format_timestamp(None)
        .format_target(true)
        .init();
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default().with_inner_size([640.0, 480.0]),
        ..Default::default()
    };
    eframe::run_native(
        "dsv",
        options,
        Box::new(|cc| {
            cc.egui_ctx.set_visuals(egui::Visuals::dark());
            Ok(Box::<DsvApp>::default())
        }),
    )
}
