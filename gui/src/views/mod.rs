use anyhow::Result;
use eframe::egui;

pub mod ph;

pub trait View {
    fn render(&mut self, ctx: &egui::Context) -> egui::Response;

    fn exit(&mut self) -> Result<()>;
}
