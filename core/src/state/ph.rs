use std::{collections::BTreeSet, fmt::Display};

use anyhow::Result;
use bytemuck::{Pod, Zeroable};

use crate::{
    gdb::client::GdbClient,
    types::{
        fx32::{Fx16, Fx32, Vec3p},
        pod::{Bool, Pad, Ptr},
    },
};

#[derive(Default)]
pub struct State {
    pub player_pos: Vec3p,
    pub actor_manager: PhActorManager,
}

#[derive(Default)]
pub struct RequestedData {
    pub player: bool,
    pub actor_manager: bool,
    pub actor_indices: BTreeSet<u16>,
}

impl super::State for State {
    type RequestedData = RequestedData;

    fn new() -> Self {
        Default::default()
    }

    fn update(&mut self, gdb: &mut GdbClient, requested_data: &RequestedData) -> Result<()> {
        if requested_data.player {
            self.player_pos.read(gdb, 0x027e0f94)?;
        }
        if requested_data.actor_manager {
            self.actor_manager.update(gdb, requested_data)?;
        }
        Ok(())
    }
}

#[derive(Default)]
pub struct PhActorManager {
    pub address: u32,
    pub max_actor_count: u16,
    pub actor_table_address: u32,
    pub actor_addresses: Vec<u32>,
    pub actors: Vec<PhActorInstance>,
}

impl PhActorManager {
    pub fn update(&mut self, gdb: &mut GdbClient, requested_data: &RequestedData) -> Result<()> {
        self.address = gdb.read_u32(0x027e0fe4)?;
        if self.address == 0 {
            return Ok(());
        }
        self.max_actor_count = gdb.read_u16(self.address)?;
        self.actor_table_address = gdb.read_u32(self.address + 0x10)?;

        self.actor_addresses.resize(self.max_actor_count as usize, 0);
        self.actors.resize_with(self.max_actor_count as usize, Default::default);
        gdb.read_slice(
            self.actor_table_address,
            bytemuck::cast_slice_mut(&mut self.actor_addresses[0..self.max_actor_count as usize]),
        )?;

        for &index in requested_data.actor_indices.iter() {
            if index >= self.max_actor_count {
                continue;
            }
            let address = self.actor_addresses[index as usize];
            if address == 0 {
                continue;
            }
            self.actors[index as usize].update(gdb, address)?;
        }

        Ok(())
    }
}

#[derive(Default, Clone)]
pub struct PhActorInstance {
    pub base: PhActor,
}

impl PhActorInstance {
    pub fn update(&mut self, gdb: &mut GdbClient, address: u32) -> Result<()> {
        let buf = bytemuck::bytes_of_mut(&mut self.base);
        gdb.read_slice(address, buf)?;

        Ok(())
    }
}

#[repr(C)]
#[derive(Default, Clone, Copy, Pod, Zeroable)]
pub struct PhActor {
    pub vtable: u32,
    pub actor_type: ActorType, // mType
    pub actor_ref: ActorRef,   // mRef
    pub unk_010: u8,
    pub unk_011: u8,
    pub unk_012: i16,
    pub unk_014: Vec3p,
    pub unk_020: ActorUnkStruct020,
    pub unk_034: ActorRef,
    pub unk_03c: i32,
    pub unk_040: ActorRef,
    pub pos: Vec3p,
    pub prev_pos: Vec3p,
    pub vel: Vec3p,
    pub gravity: Fx32,
    pub max_fall: Fx32,
    pub unk_074: i32,
    pub angle: u16,
    pub unk_07a: u16,
    pub hitbox: Cylinder,
    pub unk_08c: Cylinder,
    pub unk_09c: ActorUnkStruct09c,
    pub unk_0a4: ActorUnkStruct0a4,
    pub unk_0b8: Vec3p,
    pub unk_0c4: Pad<0x18>,
    pub unk_0dc: u16,
    pub unk_0de: u16,
    pub unk_0e0: u16,
    pub unk_0e2: u16,
    pub unk_0e4: i16,
    pub unk_0e6: Pad<0x1a>,
    pub unk_100: Ptr<()>,
    pub unk_104: i16,
    pub unk_106: i8,
    pub unk_107: i8,
    pub unk_108: i8,
    pub unk_109: i8,
    pub unk_10a: Pad<0x6>,
    pub touching_wall: Bool,
    pub touching_floor: Bool,
    pub unk_112: Bool,
    pub unk_113: Bool,
    pub unk_114: i8,
    pub unk_115: i8,
    pub unk_116: i8,
    pub unk_117: i8,
    pub alive: Bool,
    pub unk_119: i8,
    pub visible: Bool,
    pub unk_11b: Bool,
    pub unk_11c: i8,
    pub unk_11d: Bool,
    pub unk_11e: Fx16,
    pub unk_120: i16,
    pub unk_122: i16,
    pub unk_124: u8,
    pub unk_125: u8,
    pub unk_126: u16,
    pub unk_128: Bool,
    pub unk_129: Bool,
    pub unk_12a: i8,
    pub unk_12b: i8,
    pub unk_12c: i32,
    pub state: i32,
    pub unk_134: i32,
    pub active_frames: i32,
    pub unk_13c: i32,
    pub unk_140: i32,
    pub unk_144: i32,
    pub wall_touch: Vec3p,
    pub inactive: u32,
}

#[repr(C)]
#[derive(Default, Clone, Copy, Pod, Zeroable)]
pub struct ActorType(pub u32);

impl Display for ActorType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let slice = self.0.to_be_bytes();
        let str = str::from_utf8(&slice).unwrap_or("ERR!");
        write!(f, "{str}")
    }
}

#[repr(C)]
#[derive(Default, Clone, Copy, Pod, Zeroable)]
pub struct ActorRef {
    pub id: i32,
    pub index: i32,
}

impl Display for ActorRef {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "id: {}, index: {}", self.id, self.index)
    }
}

#[repr(C)]
#[derive(Default, Clone, Copy, Pod, Zeroable)]
pub struct ActorUnkStruct020 {
    pub unk_00: [u16; 4],
    pub unk_08: [u8; 2],
    pub unk_0a: [u8; 2],
    pub unk_0c: i8,
    pub unk_0d: u8,
    pub unk_0e: u8,
    pub unk_0f: i8,
    pub unk_10: i32,
}

#[repr(C)]
#[derive(Default, Clone, Copy, Pod, Zeroable)]
pub struct Cylinder {
    pub pos: Vec3p,
    pub size: Fx32,
}

impl Display for Cylinder {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if self.size.0 < 0 {
            write!(f, "(none)")
        } else {
            write!(f, "{} @ {}", self.pos, self.size)
        }
    }
}

#[repr(C)]
#[derive(Default, Clone, Copy, Pod, Zeroable)]
pub struct ActorUnkStruct09c {
    pub unk_0: u16,
    pub unk_2: i8,
    pub unk_3: u8,
    pub unk_4: Ptr<()>,
}

#[repr(C)]
#[derive(Default, Clone, Copy, Pod, Zeroable)]
pub struct ActorUnkStruct0a4 {
    pub unk_00: Bool,
    pub unk_01: Bool,
    pub unk_02: Bool,
    pub unk_03: Bool,
    pub unk_04: Cylinder,
}
