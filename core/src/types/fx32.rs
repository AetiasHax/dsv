use std::fmt::Display;

use anyhow::Result;
use bytemuck::{Pod, Zeroable};

use crate::gdb::client::GdbClient;

#[repr(C)]
#[derive(Default, Clone, Copy, Pod, Zeroable)]
pub struct Fx16(pub i16);

impl Fx16 {
    pub fn to_f32(&self) -> f32 {
        self.0 as f32 / 4096.0
    }
}

impl Display for Fx16 {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:.5}", self.to_f32())
    }
}

#[repr(C)]
#[derive(Default, Clone, Copy, Pod, Zeroable)]
pub struct Fx32(pub i32);

impl Fx32 {
    pub fn to_f32(&self) -> f32 {
        self.0 as f32 / 4096.0
    }
}

impl Display for Fx32 {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:.5}", self.to_f32())
    }
}

#[repr(C)]
#[derive(Default, Clone, Copy, Pod, Zeroable)]
pub struct Vec3p {
    pub x: Fx32,
    pub y: Fx32,
    pub z: Fx32,
}

impl Vec3p {
    pub fn read(&mut self, gdb: &mut GdbClient, address: u32) -> Result<()> {
        let mut buf = [0u8; 12];
        gdb.read_slice(address, &mut buf)?;
        let x = i32::from_le_bytes([buf[0], buf[1], buf[2], buf[3]]);
        let y = i32::from_le_bytes([buf[4], buf[5], buf[6], buf[7]]);
        let z = i32::from_le_bytes([buf[8], buf[9], buf[10], buf[11]]);
        self.x = Fx32(x);
        self.y = Fx32(y);
        self.z = Fx32(z);
        Ok(())
    }
}

impl Display for Vec3p {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}, {}, {}", self.x, self.y, self.z)
    }
}
