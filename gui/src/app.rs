use std::thread::JoinHandle;

use eframe::egui;

use crate::client::ph::{PhClient, PhCommand};

pub struct DzvApp {
    address: String,
    client: PhClient,
    update_thread: Option<JoinHandle<()>>,
}

impl Default for DzvApp {
    fn default() -> Self {
        DzvApp {
            address: "127.0.0.1:3333".to_string(),
            client: PhClient::new(),
            update_thread: None,
        }
    }
}

impl eframe::App for DzvApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        egui::CentralPanel::default().show(ctx, |ui| {
            ui.heading("dzv");

            ui.separator();
            ui.add_enabled_ui(!self.client.is_connected(), |ui| {
                ui.label("Connect to GDB server:");
                ui.horizontal(|ui| {
                    ui.text_edit_singleline(&mut self.address);
                    ui.label("Address");
                });
            });
            if !self.client.is_connected() {
                if ui.button("Connect").clicked() {
                    self.connect();
                }
            } else if ui.button("Disconnect").clicked() {
                log::info!("Disconnecting from GDB server");
                self.send_command(PhCommand::Disconnect);
            }

            if self.client.is_connected() {
                ui.separator();
                if ui.button("Stop Execution").clicked() {
                    log::info!("Stopping execution on GDB server");
                    self.send_command(PhCommand::StopExecution);
                }
                if ui.button("Continue Execution").clicked() {
                    log::info!("Continuing execution on GDB server");
                    self.send_command(PhCommand::ContinueExecution);
                }

                let state = self.client.state.lock().unwrap();

                let x = state.x;
                let y = state.y;
                let z = state.z;
                ui.label(format!("x: {x:x?}"));
                ui.label(format!("y: {y:x?}"));
                ui.label(format!("z: {z:x?}"));
            }
        });
    }

    fn on_exit(&mut self, _gl: Option<&eframe::glow::Context>) {
        if self.client.is_connected() {
            log::info!("Disconnecting from GDB server on exit");
            self.send_command(PhCommand::Disconnect);
        }
        if let Some(thread) = self.update_thread.take() {
            log::info!("Waiting for update thread to finish");
            let _ = thread.join();
        }
    }
}

impl DzvApp {
    fn connect(&mut self) {
        log::info!("Connecting to GDB server at {}", self.address);
        match self.client.connect(&self.address) {
            Ok(thread) => {
                self.update_thread = Some(thread);
            }
            Err(e) => {
                log::error!("Failed to connect to GDB server: {e}");
            }
        }
    }

    fn send_command(&mut self, cmd: PhCommand) {
        if let Err(e) = self.client.send_command(cmd) {
            log::error!("{e}");
        }
    }
}
