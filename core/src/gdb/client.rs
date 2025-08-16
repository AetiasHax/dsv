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

    fn read_slice_part(&mut self, address: u32, buf: &mut [u8]) -> Result<()> {
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

    pub fn read_slice(&mut self, mut address: u32, buf: &mut [u8]) -> Result<()> {
        // Exclude $#(checksum) and divide by 2 for hex encoding
        let max_read_length = (self.stream.packet_size().unwrap_or(usize::MAX) - 4) / 2;
        let mut read_buf = buf;
        while !read_buf.is_empty() {
            let end = read_buf.len().min(max_read_length);
            self.read_slice_part(address, &mut read_buf[..end])?;
            address += end as u32;
            read_buf = &mut read_buf[end..];
        }
        Ok(())
    }

    pub fn read_u32(&mut self, address: u32) -> Result<u32> {
        let mut buf = [0; 4];
        self.read_slice(address, &mut buf)?;
        Ok(u32::from_le_bytes(buf))
    }

    pub fn read_u16(&mut self, address: u32) -> Result<u16> {
        let mut buf = [0; 2];
        self.read_slice(address, &mut buf)?;
        Ok(u16::from_le_bytes(buf))
    }

    pub fn write_slice(&mut self, address: u32, buf: &[u8]) -> Result<()> {
        let length = buf.len();
        let mut data = String::with_capacity(length * 2);
        for &byte in buf {
            data.push_str(&format!("{:02x}", byte));
        }
        self.stream.send_packet(&format!("M {address:x},{length:x}:{data}"))?;
        self.stream.receive_ack()?;
        let response = self.stream.receive_packet()?;
        self.handle_error(&response)?;
        self.stream.send_ack()?;
        Ok(())
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
