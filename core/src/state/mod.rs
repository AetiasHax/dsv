use anyhow::Result;

use crate::gdb::client::GdbClient;

pub mod ph;

pub trait State {
    type RequestedData: Default;

    fn new() -> Self;

    fn update(&mut self, gdb: &mut GdbClient, requested_data: &Self::RequestedData) -> Result<()>;
}
