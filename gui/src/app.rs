use std::net::ToSocketAddrs;

use anyhow::{Context, Result};
use dzv_core::gdb::client::GdbClient;
use eframe::egui::{self, Color32};

use crate::views::{View, ph};

pub struct DzvApp {
    address: String,
    view: Option<Box<dyn View>>,
}

impl Default for DzvApp {
    fn default() -> Self {
        DzvApp { address: "127.0.0.1:3333".to_string(), view: None }
    }
}

impl eframe::App for DzvApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        ctx.request_repaint();

        egui::TopBottomPanel::top("dzv_top_panel")
            .frame(egui::Frame::new().inner_margin(4).fill(Color32::from_gray(20)))
            .show(ctx, |ui| {
                ui.horizontal(|ui| {
                    ui.label("Address");
                    egui::TextEdit::singleline(&mut self.address).desired_width(150.0).show(ui);
                    if self.view.is_none() {
                        if ui.button("Connect").clicked() {
                            if let Err(e) = self.connect() {
                                log::error!("Failed to connect: {e}");
                            }
                        }
                    } else if ui.button("Disconnect").clicked() {
                        if let Some(view) = &mut self.view {
                            match view.exit() {
                                Ok(_) => self.view = None,
                                Err(e) => log::error!("Failed to disconnect: {e}"),
                            }
                        }
                    }
                });
            });
        if let Some(view) = self.view.as_mut() {
            view.render(ctx);
        }
    }

    fn on_exit(&mut self, _gl: Option<&eframe::glow::Context>) {
        if let Some(mut view) = self.view.take() {
            view.exit().context("Failed to exit view").unwrap();
        }
    }
}

impl DzvApp {
    fn connect(&mut self) -> Result<()> {
        log::info!("Connecting to GDB server at {}", self.address);

        let addr = self
            .address
            .to_socket_addrs()
            .context("Failed to resolve address")?
            .next()
            .context("No socket address found")?;

        let mut gdb_client = GdbClient::new();
        gdb_client.connect(addr)?;
        gdb_client.continue_execution()?;
        // TODO: Ask emulator which game is running
        self.view = Some(Box::new(ph::View::new(gdb_client)));
        Ok(())
    }
}
