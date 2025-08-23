use anyhow::Result;
use dzv_core::{gdb::client::GdbClient, state::State};
use eframe::egui::{self};

use crate::{
    client::{Client, Command},
    config::Config,
    views::read_pointer_object,
};

const ACTOR_MANAGER_ADDRESS: u32 = 0x027e0ce4;

pub struct View {
    client: Client,
    windows: Windows,
}

struct Windows {
    actor_manager: ActorManagerWindow,
}

impl View {
    pub fn new(gdb_client: GdbClient) -> Self {
        View { client: Client::new(gdb_client), windows: Windows::default() }
    }
}

impl Default for Windows {
    fn default() -> Self {
        Self { actor_manager: ActorManagerWindow::default() }
    }
}

impl super::View for View {
    fn render_side_panel(
        &mut self,
        _ctx: &egui::Context,
        ui: &mut egui::Ui,
        _types: &type_crawler::Types,
        _config: &mut Config,
    ) -> Result<()> {
        egui::ScrollArea::vertical().max_width(100.0).show(ui, |ui| {
            ui.with_layout(
                egui::Layout::top_down(egui::Align::LEFT).with_cross_justify(true),
                |ui| {
                    ui.toggle_value(&mut self.windows.actor_manager.open, "Actor manager");
                },
            );
        });
        Ok(())
    }

    fn render_central_panel(
        &mut self,
        ctx: &egui::Context,
        _ui: &mut egui::Ui,
        types: &type_crawler::Types,
        config: &mut Config,
    ) -> Result<()> {
        let mut state = self.client.state.lock().unwrap();

        let st_config = config.games.entry("st").or_insert_with(|| toml::Table::new().into());
        let st_config = st_config
            .as_table_mut()
            .ok_or_else(|| anyhow::anyhow!("Failed to get 'st' config as a table"))?;

        self.windows.actor_manager.render(ctx, types, &mut state);

        Ok(())
    }

    fn exit(&mut self) -> Result<()> {
        self.client.send_command(Command::Disconnect)?;
        self.client.join_update_thread();
        Ok(())
    }
}

#[derive(Default)]
struct ActorManagerWindow {
    open: bool,
}

impl ActorManagerWindow {
    fn render(&mut self, ctx: &egui::Context, types: &type_crawler::Types, state: &mut State) {
        let mut open = self.open;
        egui::Window::new("Actor manager").open(&mut open).resizable(true).show(ctx, |ui| {
            egui::ScrollArea::vertical().show(ui, |ui| {
                let instance = match read_pointer_object(
                    types,
                    state,
                    "ActorManager",
                    ACTOR_MANAGER_ADDRESS,
                ) {
                    Ok(data) => data,
                    Err(err) => {
                        ui.label(err);
                        return;
                    }
                };

                instance.as_data_widget(ui, types).render_compound(ui, types, state);
            });
        });
        self.open = open;
    }
}
