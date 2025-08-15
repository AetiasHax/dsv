use anyhow::Result;
use eframe::egui;

use crate::config::Config;

pub mod ph;

pub trait View {
    fn render_side_panel(
        &mut self,
        ctx: &egui::Context,
        ui: &mut egui::Ui,
        types: &type_crawler::Types,
        config: &mut Config,
    ) -> Result<()>;

    fn render_central_panel(
        &mut self,
        ctx: &egui::Context,
        ui: &mut egui::Ui,
        types: &type_crawler::Types,
        config: &mut Config,
    ) -> Result<()>;

    fn exit(&mut self) -> Result<()>;
}
