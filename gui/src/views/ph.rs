use std::collections::BTreeSet;

use anyhow::Result;
use dzv_core::{gdb::client::GdbClient, state::State};
use eframe::egui::{self};

use crate::{
    client::{Client, Command},
    config::Config,
    ui::{self, type_decl::AsDataWidget},
    util::read::{ReadField, ReadIntValue, TypeInstance},
};

const PLAYER_POS_ADDRESS: u32 = 0x027e0f94;
const ACTOR_MANAGER_ADDRESS: u32 = 0x027e0fe4;

pub struct View {
    client: Client,
    windows: Windows,
}

#[derive(Default)]
struct Windows {
    player: PlayerWindow,
    actor_manager: ActorManagerWindow,
    actors: ActorsWindow,
    actor_list: BTreeSet<ActorWindow>,
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
        _config: &mut Config,
    ) -> Result<()> {
        egui::ScrollArea::vertical().max_width(100.0).show(ui, |ui| {
            ui.with_layout(
                egui::Layout::top_down(egui::Align::LEFT).with_cross_justify(true),
                |ui| {
                    ui.toggle_value(&mut self.windows.player.open, "Player");
                    ui.toggle_value(&mut self.windows.actor_manager.open, "Actor manager");
                    ui.toggle_value(&mut self.windows.actors.open, "Actors");
                },
            );
        });
        Ok(())
    }

    fn render_central_panel(
        &mut self,
        ctx: &egui::Context,
        ui: &mut egui::Ui,
        types: &type_crawler::Types,
        config: &mut Config,
    ) -> Result<()> {
        let mut state = self.client.state.lock().unwrap();

        let ph_config = config.games.entry("ph").or_insert_with(|| toml::Table::new().into());
        let ph_config = ph_config
            .as_table_mut()
            .ok_or_else(|| anyhow::anyhow!("Failed to get 'ph' config as a table"))?;

        self.windows.player.render(ctx, types, &mut state);
        self.windows.actor_manager.render(ctx, types, &mut state);
        self.windows.actors.render(ctx, types, &mut state, &mut self.windows.actor_list);

        let mut remove_actor = None;
        for actor in &self.windows.actor_list {
            if !actor.render(ctx, types, &mut state, ph_config) {
                remove_actor = Some(actor.clone());
            }
        }
        if let Some(actor) = remove_actor {
            self.windows.actor_list.remove(&actor);
        }

        Ok(())
    }

    fn exit(&mut self) -> Result<()> {
        self.client.send_command(Command::Disconnect)?;
        self.client.join_update_thread();
        Ok(())
    }
}

#[derive(Default)]
struct PlayerWindow {
    open: bool,
}

impl PlayerWindow {
    fn render(&mut self, ctx: &egui::Context, types: &type_crawler::Types, state: &mut State) {
        let mut open = self.open;
        egui::Window::new("Player").open(&mut open).resizable(false).show(ctx, |ui| {
            egui::ScrollArea::vertical().show(ui, |ui| {
                let Some(vec3p_type) = types.get("Vec3p") else {
                    ui.label("Vec3p struct not found");
                    return;
                };

                state.request(PLAYER_POS_ADDRESS, vec3p_type.size(types));

                let Some(player_data) = state.get_data(PLAYER_POS_ADDRESS).map(|d| d.to_vec())
                else {
                    ui.label("Player data not found");
                    return;
                };

                let instance = TypeInstance::new(PLAYER_POS_ADDRESS, &player_data);
                vec3p_type.as_data_widget(ui, types, instance).render_compound(ui, types, state);
            });
        });
        self.open = open;
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
                let (actor_manager_type, instance) = match get_actor_manager_data(types, state) {
                    Ok(data) => data,
                    Err(err) => {
                        ui.label(err);
                        return;
                    }
                };

                actor_manager_type
                    .as_data_widget(ui, types, instance)
                    .render_compound(ui, types, state);
            });
        });
        self.open = open;
    }
}

fn get_actor_manager_data<'a>(
    types: &'a type_crawler::Types,
    state: &mut State,
) -> Result<(&'a type_crawler::StructDecl, TypeInstance<'a>), String> {
    state.request(ACTOR_MANAGER_ADDRESS, 0x4);
    let actor_manager_data = state.get_data(ACTOR_MANAGER_ADDRESS).unwrap_or(&[0; 0x4]);
    let actor_manager_ptr = u32::from_le_bytes(actor_manager_data.try_into().unwrap_or([0; 4]));
    if actor_manager_ptr == 0 {
        return Err("Actor manager not initialized".into());
    }

    let Some(actor_manager_type) = types.get("ActorManager") else {
        return Err("ActorManager struct not found".into());
    };
    let type_crawler::TypeDecl::Struct(actor_manager_struct) = actor_manager_type else {
        return Err("ActorManager is not a struct".into());
    };

    state.request(actor_manager_ptr, actor_manager_type.size(types));
    let Some(actor_manager_data) = state.get_data(actor_manager_ptr).map(|d| d.to_vec()) else {
        return Err("ActorManager data not found".into());
    };
    let instance = TypeInstance::new(actor_manager_ptr, actor_manager_data);
    Ok((actor_manager_struct, instance))
}

fn get_actor_table(
    types: &type_crawler::Types,
    state: &mut State,
    actor_manager_struct: &type_crawler::StructDecl,
    instance: TypeInstance<'_>,
) -> Result<Vec<u32>, String> {
    let Some(max_actors_field) = actor_manager_struct.get_field("mMaxActors") else {
        return Err("ActorManager does not have mMaxActors field".into());
    };
    let (max_actor_count_type, max_actor_count) = max_actors_field.read_field(types, &instance);
    let Some(max_actor_count) = max_actor_count_type.read_int_value(types, &max_actor_count) else {
        return Err("mMaxActors is not an integer".into());
    };

    let Some(actor_table_field) = actor_manager_struct.get_field("mActorTable") else {
        return Err("ActorManager does not have mActorTable field".into());
    };
    let (_, actor_table) = actor_table_field.read_field(types, &instance);

    let actor_table_address = u32::from_le_bytes(actor_table.get(4).try_into().unwrap_or([0; 4]));
    state.request(actor_table_address, max_actor_count as usize * 4);
    let Some(actors_data) = state.get_data(actor_table_address) else {
        return Err("Actors data not found".into());
    };
    let actors_data: Vec<u32> = bytemuck::cast_slice(actors_data).to_vec();
    Ok(actors_data)
}

#[derive(Default)]
struct ActorsWindow {
    open: bool,
}

impl ActorsWindow {
    fn render(
        &mut self,
        ctx: &egui::Context,
        types: &type_crawler::Types,
        state: &mut State,
        actor_list: &mut BTreeSet<ActorWindow>,
    ) {
        let mut open = self.open;
        egui::Window::new("Actors").open(&mut open).resizable(true).show(ctx, |ui| {
            let (actor_manager_struct, instance) = match get_actor_manager_data(types, state) {
                Ok(data) => data,
                Err(err) => {
                    ui.label(err);
                    return;
                }
            };

            let actors_table = match get_actor_table(types, state, actor_manager_struct, instance) {
                Ok(data) => data,
                Err(err) => {
                    ui.label(err);
                    return;
                }
            };

            let Some(type_crawler::TypeDecl::Struct(actor_type)) = types.get("Actor") else {
                ui.label("Actor struct not found");
                return;
            };
            let Some(actor_type_id_field) = actor_type.get_field("mType") else {
                ui.label("Actor does not have mType field");
                return;
            };
            let Some(actor_ref_field) = actor_type.get_field("mRef") else {
                ui.label("Actor does not have mRef field");
                return;
            };
            let Some(actor_ref_struct) = actor_ref_field.kind().as_struct(types) else {
                ui.label("ActorRef field is not a struct");
                return;
            };
            let Some(actor_ref_id_field) = actor_ref_struct.get_field("id") else {
                ui.label("ActorRef does not have id field");
                return;
            };

            egui::ScrollArea::vertical().show(ui, |ui| {
                for (index, &actor_ptr) in actors_table.iter().enumerate() {
                    if actor_ptr == 0 {
                        continue;
                    }
                    state.request(actor_ptr, actor_type.size());
                    let Some(actor_data) = state.get_data(actor_ptr) else {
                        ui.label(format!("Failed to read actor at {actor_ptr:#x}"));
                        continue;
                    };
                    let actor_instance = TypeInstance::new(actor_ptr, actor_data);
                    let (_, actor_type_id) = actor_type_id_field.read_field(types, &actor_instance);

                    let mut actor_type_id_bytes = [0u8; 4];
                    actor_type_id_bytes.copy_from_slice(actor_type_id.get(4));
                    actor_type_id_bytes.reverse();
                    let Ok(actor_type_id) = str::from_utf8(&actor_type_id_bytes) else {
                        ui.label(format!("Invalid actor type ID at {actor_ptr:#x}"));
                        continue;
                    };

                    let (_, actor_ref) = actor_ref_field.read_field(types, &actor_instance);
                    let (actor_ref_id_type, actor_ref_id) =
                        actor_ref_id_field.read_field(types, &actor_ref);
                    let Some(actor_ref_id) = actor_ref_id_type.read_int_value(types, &actor_ref_id)
                    else {
                        ui.label(format!("Invalid actor ref ID at {actor_ptr:#x}"));
                        continue;
                    };

                    let actor_ref = ActorWindow { id: actor_ref_id as u32, index: index as u32 };
                    let mut checked = actor_list.contains(&actor_ref);
                    if ui
                        .toggle_value(&mut checked, format!("{}: {}", actor_ref_id, actor_type_id))
                        .clicked()
                    {
                        if checked {
                            actor_list.insert(actor_ref);
                        } else {
                            actor_list.remove(&actor_ref);
                        }
                    }
                }
            });
        });
        self.open = open;
    }
}

#[derive(PartialEq, Eq, PartialOrd, Ord, Clone)]
struct ActorWindow {
    id: u32,
    index: u32,
}

impl ActorWindow {
    fn render(
        &self,
        ctx: &egui::Context,
        types: &type_crawler::Types,
        state: &mut State,
        config: &mut toml::Table,
    ) -> bool {
        let actor_types = config.entry("actors").or_insert_with(|| toml::Table::new().into());

        let Ok((actor_manager_struct, instance)) = get_actor_manager_data(types, state) else {
            return true;
        };
        let Ok(actors_table) = get_actor_table(types, state, actor_manager_struct, instance) else {
            return true;
        };

        let actor_ptr = actors_table.get(self.index as usize).copied().unwrap_or(0);
        if actor_ptr == 0 {
            return false;
        }

        // let actor_type_name =
        //     actor_types.get(actor_type_id).and_then(|v| v.as_str()).unwrap_or("Actor");
        // let Some(actor_type) = types.get(actor_type_name) else {
        //     ui.label(format!("Actor type '{actor_type_name}' not found"));
        //     continue;
        // };

        let mut open = true;
        egui::Window::new("Actor")
            .id(egui::Id::new(actor_ptr))
            .open(&mut open)
            .resizable(true)
            .show(ctx, |ui| {});
        open
    }
}
