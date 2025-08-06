use anyhow::{Ok, Result};
use dzv_core::{
    gdb::client::GdbClient,
    state::ph::{self, PhActorInstance},
};
use eframe::egui::{self, Color32};

use crate::client::{Client, Command};

pub struct View {
    client: Client<ph::State>,
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
    fn render(&mut self, ctx: &egui::Context) -> egui::Response {
        egui::SidePanel::right("dzv_ph_side_panel")
            .frame(egui::Frame::new().inner_margin(4).fill(Color32::from_gray(20)))
            .show(ctx, |ui| {
                egui::ScrollArea::vertical().max_width(100.0).show(ui, |ui| {
                    ui.with_layout(
                        egui::Layout::top_down(egui::Align::LEFT).with_cross_justify(true),
                        |ui| {
                            ui.toggle_value(&mut self.windows.player.open, "Player");
                            ui.toggle_value(&mut self.windows.actors.open, "Actors");
                        },
                    );
                });
            });

        egui::CentralPanel::default()
            .show(ctx, |ui| {
                let state = self.client.state.lock().unwrap();
                let mut requested_data = self.client.requested_data.lock().unwrap();

                self.windows.player.render(ctx, &state, &mut requested_data);
                self.windows.actors.render(ctx, &state, &mut requested_data);

                ui.response()
            })
            .response
    }

    fn exit(&mut self) -> Result<()> {
        self.client.send_command(Command::Disconnect)?;
        self.client.join_update_thread();
        Ok(())
    }
}

trait Window {
    fn render(
        &mut self,
        ctx: &egui::Context,
        state: &ph::State,
        requested_data: &mut ph::RequestedData,
    );
}

#[derive(Default)]
struct PlayerWindow {
    open: bool,
}

impl Window for PlayerWindow {
    fn render(
        &mut self,
        ctx: &egui::Context,
        state: &ph::State,
        requested_data: &mut ph::RequestedData,
    ) {
        requested_data.player = self.open;
        egui::Window::new("Player").open(&mut self.open).resizable(false).show(ctx, |ui| {
            ui.label(format!("Position: {}", state.player_pos));
        });
    }
}

#[derive(Default)]
struct ActorsWindow {
    open: bool,
    show_unk_values: bool,
}

impl Window for ActorsWindow {
    fn render(
        &mut self,
        ctx: &egui::Context,
        state: &ph::State,
        requested_data: &mut ph::RequestedData,
    ) {
        requested_data.actor_manager = self.open;
        let mut open = self.open;
        egui::Window::new("Actors").open(&mut open).resizable(true).show(ctx, |ui| {
            ui.label(format!("Actor manager: {:#010x}", state.actor_manager.actor_table_address));
            ui.label(format!("Max actor count: {}", state.actor_manager.max_actor_count));
            ui.checkbox(&mut self.show_unk_values, "Show unknown values");
            egui::ScrollArea::vertical().show(ui, |ui| {
                ui.set_width(ui.available_width());
                for i in 0..state.actor_manager.max_actor_count as usize {
                    let address = state.actor_manager.actor_addresses[i];
                    if address == 0 {
                        requested_data.actor_indices.remove(&(i as u16));
                        continue;
                    }
                    let actor = &state.actor_manager.actors[i];
                    let response =
                        ui.collapsing(format!("#{i}"), |ui| self.render_actor_info(ui, actor));
                    let open = !response.fully_closed();
                    if open {
                        requested_data.actor_indices.insert(i as u16);
                    } else {
                        requested_data.actor_indices.remove(&(i as u16));
                    }
                }
            });
        });
        self.open = open;
    }
}

impl ActorsWindow {
    fn render_actor_info(&self, ui: &mut egui::Ui, actor: &PhActorInstance) {
        ui.label(format!("Type: {}", actor.base.actor_type));
        ui.label(format!("Position: {}", actor.base.pos));
        ui.label(format!("Velocity: {}", actor.base.vel));
        ui.label(format!("Gravity: {}", actor.base.gravity));
        ui.label(format!("Max fall: {}", actor.base.max_fall));
        ui.label(format!("Angle: {:#06x}", actor.base.angle));
        ui.label(format!("Hitbox: {}", actor.base.hitbox));
        ui.label(format!("Touching wall: {}", actor.base.touching_wall));
        ui.label(format!("Touching floor: {}", actor.base.touching_floor));
        ui.label(format!("Visible: {}", actor.base.visible));
        ui.label(format!("State: {:#x}", actor.base.state));
        ui.label(format!("Active frames: {}", actor.base.active_frames));
        ui.label(format!("Wall touch: {}", actor.base.wall_touch));
        ui.label(format!("Inactive: {}", actor.base.inactive));

        if self.show_unk_values {
            ui.label(format!("unk_010: {:#x}", actor.base.unk_010));
            ui.label(format!("unk_011: {:#x}", actor.base.unk_011));
            ui.label(format!("unk_012: {:#x}", actor.base.unk_012));
            ui.label(format!("unk_014: {}", actor.base.unk_014));
            ui.collapsing("unk_010", |ui| {
                ui.label(format!("unk_00: {:#x?}", actor.base.unk_020.unk_00));
                ui.label(format!("unk_08: {:#x?}", actor.base.unk_020.unk_08));
                ui.label(format!("unk_0a: {:#x?}", actor.base.unk_020.unk_0a));
                ui.label(format!("unk_0c: {:#x}", actor.base.unk_020.unk_0c));
                ui.label(format!("unk_0d: {:#x}", actor.base.unk_020.unk_0d));
                ui.label(format!("unk_0e: {:#x}", actor.base.unk_020.unk_0e));
                ui.label(format!("unk_0f: {:#x}", actor.base.unk_020.unk_0f));
                ui.label(format!("unk_10: {:#x}", actor.base.unk_020.unk_10));
            });
            ui.label(format!("unk_034: {}", actor.base.unk_034));
            ui.label(format!("unk_03c: {:#x}", actor.base.unk_03c));
            ui.label(format!("unk_040: {}", actor.base.unk_040));
            ui.label(format!("unk_074: {:#x}", actor.base.unk_074));
            ui.label(format!("unk_07a: {:#x}", actor.base.unk_07a));
            ui.label(format!("unk_08c: {}", actor.base.unk_08c));
            ui.collapsing("unk_09c", |ui| {
                ui.label(format!("unk_0: {:#x}", actor.base.unk_09c.unk_0));
                ui.label(format!("unk_2: {:#x}", actor.base.unk_09c.unk_2));
                ui.label(format!("unk_3: {:#x}", actor.base.unk_09c.unk_3));
                ui.label(format!("unk_4: {}", actor.base.unk_09c.unk_4));
            });
            ui.collapsing("unk_0a4", |ui| {
                ui.label(format!("unk_00: {}", actor.base.unk_0a4.unk_00));
                ui.label(format!("unk_01: {}", actor.base.unk_0a4.unk_01));
                ui.label(format!("unk_02: {}", actor.base.unk_0a4.unk_02));
                ui.label(format!("unk_03: {}", actor.base.unk_0a4.unk_03));
                ui.label(format!("unk_04: {}", actor.base.unk_0a4.unk_04));
            });
            ui.label(format!("unk_0b8: {}", actor.base.unk_0b8));
            ui.label(format!("unk_0c4: {}", actor.base.unk_0c4));
            ui.label(format!("unk_0dc: {:#x}", actor.base.unk_0dc));
            ui.label(format!("unk_0de: {:#x}", actor.base.unk_0de));
            ui.label(format!("unk_0e0: {:#x}", actor.base.unk_0e0));
            ui.label(format!("unk_0e2: {:#x}", actor.base.unk_0e2));
            ui.label(format!("unk_0e4: {:#x}", actor.base.unk_0e4));
            ui.label(format!("unk_0e6: {}", actor.base.unk_0e6));
            ui.label(format!("unk_106: {:#x}", actor.base.unk_106));
            ui.label(format!("unk_107: {:#x}", actor.base.unk_107));
            ui.label(format!("unk_108: {:#x}", actor.base.unk_108));
            ui.label(format!("unk_109: {:#x}", actor.base.unk_109));
            ui.label(format!("unk_10a: {}", actor.base.unk_10a));
            ui.label(format!("unk_112: {}", actor.base.unk_112));
            ui.label(format!("unk_113: {}", actor.base.unk_113));
            ui.label(format!("unk_114: {:#x}", actor.base.unk_114));
            ui.label(format!("unk_115: {:#x}", actor.base.unk_115));
            ui.label(format!("unk_116: {:#x}", actor.base.unk_116));
            ui.label(format!("unk_117: {:#x}", actor.base.unk_117));
            ui.label(format!("unk_119: {:#x}", actor.base.unk_119));
            ui.label(format!("unk_11b: {}", actor.base.unk_11b));
            ui.label(format!("unk_11c: {:#x}", actor.base.unk_11c));
            ui.label(format!("unk_11d: {}", actor.base.unk_11d));
            ui.label(format!("unk_11e: {}", actor.base.unk_11e));
            ui.label(format!("unk_120: {:#x}", actor.base.unk_120));
            ui.label(format!("unk_122: {:#x}", actor.base.unk_122));
            ui.label(format!("unk_124: {:#x}", actor.base.unk_124));
            ui.label(format!("unk_125: {:#x}", actor.base.unk_125));
            ui.label(format!("unk_126: {:#x}", actor.base.unk_126));
            ui.label(format!("unk_128: {}", actor.base.unk_128));
            ui.label(format!("unk_129: {}", actor.base.unk_129));
            ui.label(format!("unk_12a: {:#x}", actor.base.unk_12a));
            ui.label(format!("unk_12b: {:#x}", actor.base.unk_12b));
            ui.label(format!("unk_12c: {:#x}", actor.base.unk_12c));
            ui.label(format!("unk_134: {:#x}", actor.base.unk_134));
            ui.label(format!("unk_13c: {:#x}", actor.base.unk_13c));
            ui.label(format!("unk_140: {:#x}", actor.base.unk_140));
            ui.label(format!("unk_144: {:#x}", actor.base.unk_144));
        }
    }
}
