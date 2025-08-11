use std::collections::BTreeMap;

use anyhow::Result;

use crate::gdb::client::GdbClient;

#[derive(Default)]
pub struct State {
    data_objects: BTreeMap<u32, Vec<u8>>,
}

#[derive(Default)]
pub struct DataRequests(BTreeMap<u32, u32>);

impl DataRequests {
    pub fn new() -> Self {
        Default::default()
    }

    pub fn request(&mut self, address: u32, length: usize) {
        self.0.insert(address, length as u32);
    }
}

impl State {
    pub fn update(&mut self, gdb: &mut GdbClient, requests: &DataRequests) -> Result<()> {
        for (&address, &length) in requests.0.iter() {
            let buffer = self.data_objects.entry(address).or_default();
            buffer.resize(length as usize, 0);
            gdb.read_slice(address, buffer)?;
        }

        Ok(())
    }

    pub fn get_data(&self, address: u32) -> Option<&[u8]> {
        self.data_objects.get(&address).map(|v| v.as_slice())
    }
}
