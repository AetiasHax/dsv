use anyhow::{Ok, Result};
use dzv_core::{gdb::client::GdbClient, state::State};
use eframe::egui::{self};

use crate::{
    client::{Client, Command},
    ui::type_decl::{AsDataWidget, TypeInstance},
};

pub struct View {
    client: Client,
    windows: Windows,
}

#[derive(Default)]
struct Windows {
    player: PlayerWindow,
    actors: ActorsWindow,
}

impl View {
    pub fn new(gdb_client: GdbClient) -> Self {
        View { client: Client::new(gdb_client), windows: Windows::default() }
    }
}

impl super::View for View {
    fn render_side_panel(
        &mut self,
        _ctx: &egui::Context,
        ui: &mut egui::Ui,
        _types: &type_crawler::Types,
    ) {
        egui::ScrollArea::vertical().max_width(100.0).show(ui, |ui| {
            ui.with_layout(
                egui::Layout::top_down(egui::Align::LEFT).with_cross_justify(true),
                |ui| {
                    ui.toggle_value(&mut self.windows.player.open, "Player");
                    ui.toggle_value(&mut self.windows.actors.open, "Actors");
                },
            );
        });
    }

    fn render_central_panel(
        &mut self,
        ctx: &egui::Context,
        _ui: &mut egui::Ui,
        types: &type_crawler::Types,
    ) {
        let mut state = self.client.state.lock().unwrap();

        self.windows.player.render(ctx, types, &mut state);
        self.windows.actors.render(ctx, types, &mut state);
    }

    fn exit(&mut self) -> Result<()> {
        self.client.send_command(Command::Disconnect)?;
        self.client.join_update_thread();
        Ok(())
    }
}

trait Window {
    fn render(&mut self, ctx: &egui::Context, types: &type_crawler::Types, state: &mut State);
}

#[derive(Default)]
struct PlayerWindow {
    open: bool,
}

impl Window for PlayerWindow {
    fn render(&mut self, ctx: &egui::Context, types: &type_crawler::Types, state: &mut State) {
        let mut open = self.open;
        egui::Window::new("Player").open(&mut open).resizable(false).show(ctx, |ui| {
            egui::ScrollArea::vertical().show(ui, |ui| {
                let Some(vec3p_type) = types.get("Vec3p") else {
                    ui.label("Vec3p struct not found");
                    return;
                };

                state.request(0x027e0f94, vec3p_type.size(types));

                let Some(player_data) = state.get_data(0x027e0f94).map(|d| d.to_vec()) else {
                    ui.label("Player data not found");
                    return;
                };

                let instance = TypeInstance::new(&player_data);
                vec3p_type.as_data_widget(ui, types, instance).render_compound(ui, types, state);
            });
        });
        self.open = open;
    }
}

#[derive(Default)]
struct ActorsWindow {
    open: bool,
}

impl Window for ActorsWindow {
    fn render(&mut self, ctx: &egui::Context, types: &type_crawler::Types, state: &mut State) {
        let mut open = self.open;
        egui::Window::new("Actors").open(&mut open).resizable(true).show(ctx, |ui| {
            egui::ScrollArea::vertical().show(ui, |ui| {
                state.request(0x027e0fe4, 0x4);
                let actor_manager_data = state.get_data(0x027e0fe4).unwrap_or(&[0; 0x4]);
                let actor_manager_ptr =
                    u32::from_le_bytes(actor_manager_data.try_into().unwrap_or([0; 4]));
                if actor_manager_ptr == 0 {
                    ui.label("Actor manager not initialized");
                    return;
                }

                let Some(actor_manager_type) = types.get("ActorManager") else {
                    ui.label("ActorManager struct not found");
                    return;
                };
                state.request(actor_manager_ptr, actor_manager_type.size(types));
                let Some(actor_manager_data) =
                    state.get_data(actor_manager_ptr).map(|d| d.to_vec())
                else {
                    ui.label("ActorManager data not found");
                    return;
                };

                let instance = TypeInstance::new(&actor_manager_data);
                actor_manager_type
                    .as_data_widget(ui, types, instance)
                    .render_compound(ui, types, state);
            });
        });
        self.open = open;
    }
}
