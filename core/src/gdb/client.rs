use std::net::ToSocketAddrs;

use anyhow::{Result, bail};

use crate::{gdb::stream::GdbStream, hex_char_to_byte};

#[derive(Default)]
pub struct GdbClient {
    stream: GdbStream,
}

impl GdbClient {
    pub fn new() -> Self {
        GdbClient { stream: GdbStream::new() }
    }

    pub fn connect<A: ToSocketAddrs>(&mut self, address: A) -> Result<()> {
        self.stream.connect(address)
    }

    pub fn disconnect(&mut self) -> Result<()> {
        self.stream.disconnect()
    }

    pub fn is_connected(&self) -> bool {
        self.stream.is_connected()
    }

    fn handle_error(&self, response: &str) -> Result<()> {
        if response.starts_with("E") {
            bail!("Error from GDB server: {}", response);
        }
        Ok(())
    }

    pub fn read_bytes(&mut self, address: u32, length: usize) -> Result<Vec<u8>> {
        let packet = format!("m {address:x},{length:x}");
        self.stream.send_packet(&packet)?;
        self.stream.receive_ack()?;
        let response = self.stream.receive_packet()?;
        self.stream.send_ack()?;
        self.handle_error(&response)?;
        let mut data = Vec::with_capacity(length);
        for chunk in response.as_bytes().chunks(2) {
            let high = hex_char_to_byte(chunk[0] as char);
            let low = hex_char_to_byte(chunk[1] as char);
            data.push((high << 4) | low);
        }
        Ok(data)
    }

    pub fn read_slice(&mut self, address: u32, buf: &mut [u8]) -> Result<()> {
        let packet = format!("m {address:x},{:x}", buf.len());
        self.stream.send_packet(&packet)?;
        self.stream.receive_ack()?;
        let response = self.stream.receive_packet()?;
        self.stream.send_ack()?;
        self.handle_error(&response)?;
        if response.len() != buf.len() * 2 {
            bail!("Expected {} bytes, got {}", buf.len() * 2, response.len());
        }
        for (i, chunk) in response.as_bytes().chunks(2).enumerate() {
            let high = hex_char_to_byte(chunk[0] as char);
            let low = hex_char_to_byte(chunk[1] as char);
            buf[i] = (high << 4) | low;
        }
        Ok(())
    }

    pub fn read_u32(&mut self, address: u32) -> Result<u32> {
        let mut buf = [0; 4];
        self.read_slice(address, &mut buf)?;
        Ok(u32::from_le_bytes(buf))
    }

    pub fn continue_execution(&mut self) -> Result<()> {
        self.stream.send_packet("c")?;
        self.stream.receive_ack()?;
        Ok(())
    }

    pub fn stop_execution(&mut self) -> Result<()> {
        self.stream.send_packet("s")?;
        self.stream.receive_ack()?;
        let response = self.stream.receive_packet()?;
        self.handle_error(&response)?;
        self.stream.send_ack()?;
        Ok(())
    }
}
