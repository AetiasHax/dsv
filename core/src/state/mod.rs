use std::collections::BTreeMap;

use anyhow::Result;

use crate::gdb::client::GdbClient;

#[derive(Default)]
pub struct State {
    data_objects: BTreeMap<u32, Vec<u8>>,
    requests: BTreeMap<u32, u32>,
    writes: Vec<(u32, Vec<u8>)>,
}

impl State {
    pub fn update(&mut self, gdb: &mut GdbClient) -> Result<()> {
        for (address, data) in self.writes.drain(..) {
            gdb.write_slice(address, &data)?;
        }

        for (&address, &length) in self.requests.iter() {
            let buffer = self.data_objects.entry(address).or_default();
            buffer.resize(length as usize, 0);
            gdb.read_slice(address, buffer)?;
        }

        Ok(())
    }

    pub fn request(&mut self, address: u32, length: usize) {
        self.requests.insert(address, length as u32);
    }

    pub fn request_write(&mut self, address: u32, data: Vec<u8>) {
        self.writes.push((address, data));
    }

    pub fn get_data(&self, address: u32) -> Option<&[u8]> {
        self.data_objects.get(&address).map(|v| v.as_slice())
    }
}
