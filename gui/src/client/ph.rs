use std::{
    net::ToSocketAddrs,
    sync::{Arc, Mutex, mpsc::Sender},
    thread::JoinHandle,
    time::{Duration, Instant},
};

use anyhow::{Context, Result, bail};
use dzv_core::gdb::client::GdbClient;

pub struct PhClient {
    connected: Arc<Mutex<bool>>,
    tx: Option<Sender<PhCommand>>,
    pub state: Arc<Mutex<PhState>>,
}

#[derive(PartialEq, Eq)]
pub enum PhCommand {
    Disconnect,
    StopExecution,
    ContinueExecution,
}

impl PhClient {
    const FRAME_TIME: Duration = Duration::from_nanos(16_666_667);

    pub fn new() -> Self {
        PhClient {
            connected: Arc::new(Mutex::new(false)),
            tx: None,
            state: Arc::new(Mutex::new(PhState::default())),
        }
    }

    pub fn connect<A: ToSocketAddrs>(&mut self, address: A) -> Result<JoinHandle<()>> {
        if self.is_connected() {
            bail!("Already connected to GDB server");
        }

        let addr = address
            .to_socket_addrs()
            .context("Failed to resolve address")?
            .next()
            .context("No socket address found")?;

        let (tx, rx) = std::sync::mpsc::channel();
        self.tx = Some(tx);

        let connected = self.connected.clone();
        let state = self.state.clone();
        let thread = std::thread::spawn(move || {
            let mut gdb_client = GdbClient::new();
            if let Err(e) = gdb_client.connect(addr) {
                log::error!("Failed to connect to GDB server: {e}");
                return;
            }
            log::info!("Connected to GDB server");
            *connected.lock().unwrap() = true;

            // Continue execution in case "Break on startup" is enabled
            gdb_client.continue_execution().unwrap_or_else(|e| {
                log::error!("Failed to continue execution: {e}");
            });

            let mut next_time = Instant::now();
            while gdb_client.is_connected() {
                if let Ok(cmd) = rx.try_recv() {
                    state.lock().unwrap().handle_command(cmd, &mut gdb_client).unwrap_or_else(
                        |e| {
                            log::error!("Failed to handle command: {e}");
                        },
                    );
                    continue;
                }

                state.lock().unwrap().update(&mut gdb_client).unwrap_or_else(|e| {
                    log::error!("Failed to update state: {e}");
                });
                let time = Instant::now();
                next_time += Duration::from_nanos(
                    (time - next_time).as_nanos().next_multiple_of(Self::FRAME_TIME.as_nanos())
                        as u64,
                );
                std::thread::sleep(next_time - time);
            }

            gdb_client.disconnect().unwrap_or_else(|e| {
                log::error!("Failed to disconnect from GDB server: {e}");
            });
            *connected.lock().unwrap() = false;
        });

        Ok(thread)
    }

    pub fn is_connected(&self) -> bool {
        *self.connected.lock().unwrap()
    }

    pub fn send_command(&self, cmd: PhCommand) -> Result<()> {
        if !self.is_connected() {
            bail!("Not connected to GDB server");
        }
        if let Some(ref tx) = self.tx {
            tx.send(cmd).context("Failed to send command")?;
        }
        Ok(())
    }
}

#[derive(Default)]
pub struct PhState {
    pub x: u32,
    pub y: u32,
    pub z: u32,
}

impl PhState {
    pub fn update(&mut self, gdb: &mut GdbClient) -> Result<()> {
        self.x = gdb.read_u32(0x027e0f94).unwrap_or(0);
        self.y = gdb.read_u32(0x027e0f98).unwrap_or(0);
        self.z = gdb.read_u32(0x027e0f9c).unwrap_or(0);
        Ok(())
    }

    pub fn handle_command(&mut self, cmd: PhCommand, gdb: &mut GdbClient) -> Result<()> {
        match cmd {
            PhCommand::StopExecution => gdb.stop_execution(),
            PhCommand::ContinueExecution => gdb.continue_execution(),
            PhCommand::Disconnect => gdb.disconnect(),
        }
    }
}
