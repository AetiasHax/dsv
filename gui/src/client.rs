use std::{
    sync::{Arc, Mutex, mpsc::Sender},
    thread::JoinHandle,
    time::{Duration, Instant},
};

use anyhow::{Context, Result, bail};
use dzv_core::{gdb::client::GdbClient, state::State};

pub struct Client<S>
where
    S: State,
{
    running: Arc<Mutex<bool>>,
    tx: Sender<Command>,
    pub state: Arc<Mutex<S>>,
    pub requested_data: Arc<Mutex<S::RequestedData>>,
    update_thread: Option<JoinHandle<()>>,
}

#[derive(PartialEq, Eq)]
pub enum Command {
    Disconnect,
}

impl<S> Client<S>
where
    S: State + Send + 'static,
    S::RequestedData: Send,
{
    const FRAME_TIME: Duration = Duration::from_nanos(16_666_667);

    pub fn new(mut gdb_client: GdbClient) -> Self {
        let (tx, rx) = std::sync::mpsc::channel();

        let running = Arc::new(Mutex::new(false));
        let state = Arc::new(Mutex::new(S::new()));
        let requested_data = Arc::new(Mutex::new(S::RequestedData::default()));
        let update_thread = {
            let running = running.clone();
            let state = state.clone();
            let requested_data = requested_data.clone();
            std::thread::spawn(move || {
                *running.lock().unwrap() = true;

                // Continue execution in case "Break on startup" is enabled
                gdb_client.continue_execution().unwrap_or_else(|e| {
                    log::error!("Failed to continue execution: {e}");
                });

                let mut next_time = Instant::now();
                let mut frame_count = 0;
                let mut last_fps_report = Instant::now();
                while gdb_client.is_connected() {
                    if let Ok(cmd) = rx.try_recv() {
                        Self::handle_command(cmd, &mut gdb_client).unwrap_or_else(|e| {
                            log::error!("Failed to handle command: {e}");
                        });
                        continue;
                    }

                    gdb_client.stop_execution().unwrap_or_else(|e| {
                        log::error!("Failed to stop execution: {e}");
                    });
                    {
                        let mut state = state.lock().unwrap();
                        let requested_data = requested_data.lock().unwrap();
                        state.update(&mut gdb_client, &requested_data).unwrap_or_else(|e| {
                            log::error!("Failed to update player: {e}");
                        });
                    }
                    gdb_client.continue_execution().unwrap_or_else(|e| {
                        log::error!("Failed to continue execution: {e}");
                    });

                    frame_count += 1;
                    if last_fps_report.elapsed() >= Duration::from_secs(1) {
                        log::debug!("FPS: {frame_count}");
                        frame_count = 0;
                        last_fps_report = Instant::now();
                    }

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
                *running.lock().unwrap() = false;
            })
        };

        Client { running, tx, state, requested_data, update_thread: Some(update_thread) }
    }

    pub fn is_running(&self) -> bool {
        *self.running.lock().unwrap()
    }

    pub fn send_command(&self, cmd: Command) -> Result<()> {
        if !self.is_running() {
            bail!("Not connected to GDB server");
        }
        self.tx.send(cmd).context("Failed to send command")?;
        Ok(())
    }

    pub fn handle_command(cmd: Command, gdb: &mut GdbClient) -> Result<()> {
        match cmd {
            Command::Disconnect => gdb.disconnect(),
        }
    }

    pub fn join_update_thread(&mut self) {
        if let Some(thread) = self.update_thread.take() {
            thread.join().expect("Failed to join update thread");
        }
    }
}
