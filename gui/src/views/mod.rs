use std::borrow::Cow;

use anyhow::Result;
use dzv_core::state::State;
use eframe::egui;

use crate::{
    config::Config,
    util::read::{TypeInstance, TypeInstanceOptions},
};

pub mod ph;
pub mod st;

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

fn read_object<'a>(
    types: &'a type_crawler::Types,
    state: &mut State,
    type_name: &str,
    address: u32,
) -> Result<TypeInstance<'a>, String> {
    let Some(ty) = types.get(type_name) else {
        return Err(format!("{} struct not found", type_name));
    };

    state.request(address, ty.size(types));
    let Some(game_data) = state.get_data(address).map(|d| d.to_vec()) else {
        return Err(format!("{} data not found", type_name));
    };

    let instance = TypeInstance::new(TypeInstanceOptions {
        ty,
        address,
        bit_field_range: None,
        data: Cow::Owned(game_data),
    });
    Ok(instance)
}

fn read_pointer_object<'a>(
    types: &'a type_crawler::Types,
    state: &mut State,
    type_name: &str,
    address: u32,
) -> Result<TypeInstance<'a>, String> {
    state.request(address, 4);
    let Some(data) = state.get_data(address) else {
        return Err(format!("{} pointer data not found", type_name));
    };
    let ptr = u32::from_le_bytes(data.try_into().unwrap_or([0; 4]));

    read_object(types, state, type_name, ptr)
}
