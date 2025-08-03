use std::{
    io::{Read, Write},
    net::{Shutdown, TcpStream, ToSocketAddrs},
    time::Duration,
};

use anyhow::{Context, Result, bail};

use crate::hex_char_to_byte;

pub struct GdbStream {
    stream: Option<TcpStream>,
}

impl GdbStream {
    pub fn new() -> Self {
        GdbStream { stream: None }
    }

    pub fn connect<A: ToSocketAddrs>(&mut self, address: A) -> Result<()> {
        let addr = address.to_socket_addrs()?.next().context("No socket address found")?;

        let stream = TcpStream::connect_timeout(&addr, Duration::from_secs(10))?;
        stream.set_read_timeout(Some(Duration::from_secs(1)))?;
        stream.set_write_timeout(Some(Duration::from_secs(1)))?;

        self.stream = Some(stream);
        self.send_ack()?;
        self.receive_ack()?;
        Ok(())
    }

    pub fn disconnect(&mut self) -> Result<()> {
        if let Some(stream) = self.stream.take() {
            stream.shutdown(Shutdown::Both)?;
        }
        Ok(())
    }

    fn send_ack(&mut self) -> Result<()> {
        let Some(ref mut stream) = self.stream else {
            bail!("Not connected to GDB server");
        };
        stream.write_all(b"+")?;
        Ok(())
    }

    fn receive_ack(&mut self) -> Result<()> {
        let Some(ref mut stream) = self.stream else {
            bail!("Not connected to GDB server");
        };
        let mut buf = [0; 1];
        stream.read_exact(&mut buf)?;
        if buf[0] != b'+' {
            bail!("Failed to receive ACK from GDB server, got: {}", buf[0] as char);
        }
        Ok(())
    }

    pub fn send_packet(&mut self, packet: &str) -> Result<()> {
        let Some(ref mut stream) = self.stream else {
            bail!("Not connected to GDB server");
        };

        let checksum = packet.as_bytes().iter().fold(0u8, |acc, &b| acc.wrapping_add(b));
        let packet_with_checksum = format!("${packet}#{checksum:02x}");
        stream.write_all(packet_with_checksum.as_bytes()).context("Failed to send packet")?;
        self.receive_ack()?;

        Ok(())
    }

    pub fn receive_packet(&mut self) -> Result<String> {
        let Some(ref mut stream) = self.stream else {
            bail!("Not connected to GDB server");
        };

        let mut buf = [0; 128];
        let mut vec = Vec::new();
        loop {
            let bytes_read = stream.read(&mut buf).context("Failed to read from GDB server")?;
            if bytes_read == 0 {
                bail!("Connection closed by GDB server");
            }
            vec.extend_from_slice(&buf[..bytes_read]);
            if vec[0] != b'$' {
                self.disconnect()?;
                bail!("Response did not start with '$', got: {}", String::from_utf8_lossy(&vec));
            }
            let len = vec.len();
            if vec[len - 3] == b'#'
                && vec[len - 2].is_ascii_hexdigit()
                && vec[len - 1].is_ascii_hexdigit()
            {
                break;
            }
            if bytes_read == buf.len() {
                continue;
            } else {
                self.disconnect()?;
                bail!("Response did not end with checksum, got: {}", String::from_utf8_lossy(&vec));
            }
        }

        let len = vec.len();
        let packet = &vec[1..len - 3];
        let expected_checksum = packet.iter().fold(0u8, |acc, &b| acc.wrapping_add(b));
        let actual_checksum =
            hex_char_to_byte(vec[len - 2] as char) << 4 | hex_char_to_byte(vec[len - 1] as char);
        if expected_checksum != actual_checksum {
            self.disconnect()?;
            bail!("Checksum mismatch: expected {expected_checksum:02x}, got {actual_checksum:02x}");
        }

        self.send_ack()?;
        String::from_utf8(packet.to_vec()).context("Failed to parse GDB response")
    }
}
