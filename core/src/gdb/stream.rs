use std::{
    io::{ErrorKind, Read, Write},
    net::{Shutdown, ToSocketAddrs},
};

use anyhow::{Context, Result, bail};
use mio::net::TcpStream;

use crate::hex_char_to_byte;

#[derive(Default)]
pub struct GdbStream {
    stream: Option<TcpStream>,
}

impl GdbStream {
    pub fn new() -> Self {
        GdbStream { stream: None }
    }

    pub fn connect<A: ToSocketAddrs>(&mut self, address: A) -> Result<()> {
        let addr = address.to_socket_addrs()?.next().context("No socket address found")?;

        let stream = TcpStream::connect(addr).context("Failed to open TCP connection")?;
        stream.set_nodelay(true)?;
        self.stream = Some(stream);
        self.send_ack().context("Failed to send initial ACK")?;
        self.receive_ack().context("Failed to receive initial ACK")?;
        Ok(())
    }

    pub fn disconnect(&mut self) -> Result<()> {
        if let Some(stream) = self.stream.take() {
            stream.shutdown(Shutdown::Both)?;
        }
        Ok(())
    }

    pub fn is_connected(&self) -> bool {
        self.stream.is_some()
    }

    pub fn send_ack(&mut self) -> Result<()> {
        let Some(ref mut stream) = self.stream else {
            bail!("Not connected to GDB server");
        };
        log::debug!("Sending ACK to GDB server");
        stream.write_all(b"+")?;
        Ok(())
    }

    pub fn receive_ack(&mut self) -> Result<()> {
        let Some(ref mut stream) = self.stream else {
            bail!("Not connected to GDB server");
        };
        let mut buf = [0; 1];
        loop {
            let Err(e) = stream.read_exact(&mut buf) else {
                break;
            };
            let kind = e.kind();
            match kind {
                ErrorKind::WouldBlock => {
                    continue;
                }
                _ => {
                    bail!("Failed to read ACK from GDB server: {kind}");
                }
            }
        }
        if buf[0] != b'+' {
            bail!("Failed to receive ACK from GDB server, got: {}", buf[0] as char);
        }
        log::debug!("Received ACK from GDB server");
        Ok(())
    }

    pub fn receive_ok(&mut self) -> Result<()> {
        let response = self.receive_packet()?;
        if response != "OK" {
            bail!("Expected 'OK' response, got: {}", response);
        }
        log::debug!("Received OK from GDB server");
        Ok(())
    }

    pub fn send_packet(&mut self, packet: &str) -> Result<()> {
        let Some(ref mut stream) = self.stream else {
            bail!("Not connected to GDB server");
        };

        log::debug!("Sending packet: {packet}");

        let checksum = packet.as_bytes().iter().fold(0u8, |acc, &b| acc.wrapping_add(b));
        let packet_with_checksum = format!("${packet}#{checksum:02x}");
        stream.write_all(packet_with_checksum.as_bytes()).context("Failed to send packet")?;

        Ok(())
    }

    pub fn receive_packet(&mut self) -> Result<String> {
        let Some(ref mut stream) = self.stream else {
            bail!("Not connected to GDB server");
        };

        let mut buf = [0; 128];
        let mut vec = Vec::new();
        loop {
            let bytes_read = loop {
                match stream.read(&mut buf) {
                    Ok(n) => break n,
                    Err(e) => match e.kind() {
                        ErrorKind::WouldBlock => continue,
                        _ => {
                            bail!("Failed to read from GDB server: {e}");
                        }
                    },
                }
            };
            // let bytes_read = stream.read(&mut buf).context("Failed to read from GDB server")?;
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

        let response =
            String::from_utf8(packet.to_vec()).context("Failed to parse GDB response")?;
        log::debug!("Received packet: {response}");
        Ok(response)
    }
}
