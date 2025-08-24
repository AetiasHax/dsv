use std::{borrow::Cow, collections::BTreeSet};

use anyhow::Result;
use dsv_core::{gdb::client::GdbClient, state::State};
use eframe::egui::{self};

use crate::{
    client::{Client, Command},
    config::Config,
    util::read::{TypeInstance, TypeInstanceOptions},
    views::{read_object, read_pointer_object},
};

const ACTOR_MANAGER_ADDRESS: u32 = 0x027e0ce4;

pub struct View {
    client: Client,
    windows: Windows,
}

struct Windows {
    actor_manager: ActorManagerWindow,
    actors: ActorsWindow,
    actor_list: BTreeSet<ActorWindow>,
    basic_windows: Vec<BasicWindow>,
}

impl View {
    pub fn new(gdb_client: GdbClient) -> Self {
        View { client: Client::new(gdb_client), windows: Windows::default() }
    }
}

impl Default for Windows {
    fn default() -> Self {
        Self {
            actor_manager: ActorManagerWindow::default(),
            actors: ActorsWindow::default(),
            actor_list: BTreeSet::new(),
            basic_windows: vec![
                // BasicWindow {
                //     open: false,
                //     title: "Item manager",
                //     type_name: "ItemManager",
                //     address: ITEM_MANAGER_ADDRESS,
                //     pointer: true,
                // }
            ],
        }
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
                    ui.toggle_value(&mut self.windows.actors.open, "Actors");
                    for window in &mut self.windows.basic_windows {
                        ui.toggle_value(&mut window.open, window.title);
                    }
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
        self.windows.actors.render(ctx, types, &mut state, &mut self.windows.actor_list);

        let mut remove_actor = None;
        for actor in &self.windows.actor_list {
            if !actor.render(ctx, types, &mut state, st_config) {
                remove_actor = Some(actor.clone());
            }
        }
        if let Some(actor) = remove_actor {
            self.windows.actor_list.remove(&actor);
        }

        for window in &mut self.windows.basic_windows {
            window.render(ctx, types, &mut state);
        }

        Ok(())
    }

    fn exit(&mut self) -> Result<()> {
        if !self.client.is_running() {
            return Ok(());
        }
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

                instance.into_data_widget(ui, types).render_compound(ui, types, state);
            });
        });
        self.open = open;
    }
}

fn get_actor_table(
    types: &type_crawler::Types,
    state: &mut State,
    actor_manager: TypeInstance<'_>,
) -> Result<Vec<u32>, String> {
    let Some(actor_table) = actor_manager.read_int_field::<u32>(types, "mActorTable") else {
        return Err("ActorManager does not have mActorTable field".into());
    };
    let Some(actor_table_end) = actor_manager.read_int_field::<u32>(types, "mActorTableEnd") else {
        return Err("ActorManager does not have mActorTableEnd field".into());
    };
    let max_actors = (actor_table_end - actor_table) / 4;
    state.request(actor_table, max_actors as usize * 4);
    let Some(actors_data) = state.get_data(actor_table) else {
        return Err("Actors data not found".into());
    };
    let actors_data: Vec<u32> = actors_data
        .chunks_exact(4)
        .map(|chunk| u32::from_le_bytes(chunk.try_into().unwrap_or([0; 4])))
        .collect();
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
            let actor_manager =
                match read_pointer_object(types, state, "ActorManager", ACTOR_MANAGER_ADDRESS) {
                    Ok(data) => data,
                    Err(err) => {
                        ui.label(err);
                        return;
                    }
                };

            let actors_table = match get_actor_table(types, state, actor_manager) {
                Ok(data) => data,
                Err(err) => {
                    ui.label(err);
                    return;
                }
            };

            let Some(actor_type) = types.get("Actor") else {
                ui.label("Actor struct not found");
                return;
            };

            egui::ScrollArea::vertical().show(ui, |ui| {
                for (index, &actor_ptr) in actors_table.iter().enumerate() {
                    if actor_ptr == 0 {
                        continue;
                    }
                    state.request(actor_ptr, actor_type.size(types));
                    let Some(actor_data) = state.get_data(actor_ptr) else {
                        ui.label(format!("Failed to read actor at {actor_ptr:#x}"));
                        continue;
                    };
                    let actor = TypeInstance::new(TypeInstanceOptions {
                        ty: actor_type,
                        address: actor_ptr,
                        bit_field_range: None,
                        data: actor_data.to_vec().into(),
                    });

                    let actor_type_id = match get_actor_type_id(types, state, &actor) {
                        Ok(id) => id,
                        Err(err) => {
                            ui.label(err);
                            continue;
                        }
                    };
                    let actor_type_bytes = actor_type_id.to_be_bytes();
                    let Ok(actor_type_id) = str::from_utf8(&actor_type_bytes) else {
                        ui.label("Invalid actor type ID".to_string());
                        continue;
                    };

                    let Some(actor_ref) = actor.read_field(types, "mRef") else {
                        ui.label("Actor does not have mRef field".to_string());
                        continue;
                    };
                    let Some(actor_id) = actor_ref.read_int_field::<i32>(types, "id") else {
                        ui.label(format!("Actor ref does not have id field {:#?}", actor_ref.ty()));
                        continue;
                    };

                    let actor_ref = ActorWindow { id: actor_id, index: index as i32 };
                    let mut checked = actor_list.contains(&actor_ref);
                    if ui
                        .toggle_value(&mut checked, format!("{actor_id}: {actor_type_id}"))
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

fn get_actor_type_id(
    types: &type_crawler::Types,
    state: &mut State,
    actor: &TypeInstance<'_>,
) -> Result<u32, String> {
    let Some(actor_type_type) = types.get("ActorType") else {
        return Err("ActorType struct not found".into());
    };

    let Some(actor_type_ptr) = actor.read_int_field::<u32>(types, "mType") else {
        return Err("Actor does not have mType field".into());
    };
    state.request(actor_type_ptr, actor_type_type.size(types));
    let Some(actor_type_data) = state.get_data(actor_type_ptr) else {
        return Err(format!("Failed to read actor type at {actor_type_ptr:#x}"));
    };
    let actor_type = TypeInstance::new(TypeInstanceOptions {
        ty: actor_type_type,
        address: actor_type_ptr,
        bit_field_range: None,
        data: actor_type_data.to_vec().into(),
    });
    let Some(actor_type_id) = actor_type.read_int_field::<u32>(types, "mActorId") else {
        return Err("ActorType does not have mActorId field".into());
    };
    Ok(actor_type_id)
}

#[derive(PartialEq, Eq, PartialOrd, Ord, Clone)]
struct ActorWindow {
    id: i32,
    index: i32,
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

        let Ok(actor_manager) =
            read_pointer_object(types, state, "ActorManager", ACTOR_MANAGER_ADDRESS)
        else {
            return true;
        };
        let Ok(actor_table) = get_actor_table(types, state, actor_manager) else {
            return true;
        };

        let actor_ptr = actor_table.get(self.index as usize).copied().unwrap_or(0);
        if actor_ptr == 0 {
            return false;
        }
        let Some(actor_type) = types.get("Actor") else {
            return false;
        };
        state.request(actor_ptr, actor_type.size(types));
        let Some(actor_data) = state.get_data(actor_ptr) else {
            // Actor data not received yet
            return true;
        };

        let actor = TypeInstance::new(TypeInstanceOptions {
            ty: actor_type,
            address: actor_ptr,
            bit_field_range: None,
            data: actor_data.to_vec().into(),
        });

        let actor_type_id = match get_actor_type_id(types, state, &actor) {
            Ok(id) => id,
            Err(_) => {
                return false;
            }
        };
        let actor_type_bytes = actor_type_id.to_be_bytes();
        let Ok(actor_type_id) = str::from_utf8(&actor_type_bytes) else {
            return false;
        };

        let actor_type_name =
            actor_types.get(actor_type_id).and_then(|v| v.as_str()).unwrap_or("Actor");

        let mut open = true;
        egui::Window::new(format!("{actor_type_name} ({actor_type_id})"))
            .id(egui::Id::new(actor_ptr))
            .open(&mut open)
            .resizable(true)
            .show(ctx, |ui| {
                egui::ScrollArea::vertical().show(ui, |ui| {
                    let Some(actor_type) = types.get(actor_type_name) else {
                        ui.label(format!("Actor type '{actor_type_name}' not found"));
                        return;
                    };
                    state.request(actor_ptr, actor_type.size(types));
                    let Some(actor_data) = state.get_data(actor_ptr) else {
                        ui.label(format!("Failed to read actor at {actor_ptr:#x}"));
                        return;
                    };
                    let actor = TypeInstance::new(TypeInstanceOptions {
                        ty: actor_type,
                        address: actor_ptr,
                        bit_field_range: None,
                        data: Cow::Owned(actor_data.to_vec()),
                    });
                    actor.into_data_widget(ui, types).render_compound(ui, types, state);
                });
            });
        open
    }
}

#[derive(Default)]
struct BasicWindow {
    open: bool,
    title: &'static str,
    type_name: &'static str,
    address: u32,
    pointer: bool,
}

impl BasicWindow {
    fn render(&mut self, ctx: &egui::Context, types: &type_crawler::Types, state: &mut State) {
        let mut open = self.open;
        egui::Window::new(self.title).open(&mut open).resizable(true).show(ctx, |ui| {
            egui::ScrollArea::vertical().show(ui, |ui| {
                let object = if self.pointer {
                    read_pointer_object(types, state, self.type_name, self.address)
                } else {
                    read_object(types, state, self.type_name, self.address)
                };

                let instance = match object {
                    Ok(instance) => instance,
                    Err(err) => {
                        ui.label(err);
                        return;
                    }
                };
                instance.into_data_widget(ui, types).render_compound(ui, types, state);
            });
        });
        self.open = open;
    }
}
