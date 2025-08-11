use anyhow::Result;
use eframe::egui;

pub mod ph;

pub trait View {
    fn render_side_panel(
        &mut self,
        ctx: &egui::Context,
        ui: &mut egui::Ui,
        types: &type_crawler::Types,
    );

    fn render_central_panel(
        &mut self,
        ctx: &egui::Context,
        ui: &mut egui::Ui,
        types: &type_crawler::Types,
    );

    fn exit(&mut self) -> Result<()>;
}
