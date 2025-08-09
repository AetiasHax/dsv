use std::{
    net::ToSocketAddrs,
    sync::{Arc, Mutex},
};

use anyhow::{Context, Result};
use dzv_core::gdb::client::GdbClient;
use eframe::egui::{self, Color32, Widget};

use crate::{
    tasks::load_types::{LoadTypesTask, LoadTypesTaskOptions},
    views::{View, ph},
};

pub struct DzvApp {
    address: String,

    project_modal_open: bool,
    project_path: String,
    include_paths: Vec<String>,
    ignore_paths: Vec<String>,
    types: Arc<Mutex<type_crawler::Types>>,
    load_types_task: Option<LoadTypesTask>,

    view: Option<Box<dyn View>>,
}

impl Default for DzvApp {
    fn default() -> Self {
        DzvApp {
            address: "127.0.0.1:3333".to_string(),

            project_modal_open: false,
            project_path: String::new(),
            include_paths: vec![],
            ignore_paths: vec![],
            types: Arc::new(Mutex::new(type_crawler::Types::new())),
            load_types_task: None,

            view: None,
        }
    }
}

impl eframe::App for DzvApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        ctx.request_repaint();

        egui::TopBottomPanel::top("dzv_top_panel")
            .frame(egui::Frame::new().inner_margin(4).fill(Color32::from_gray(20)))
            .show(ctx, |ui| {
                ui.horizontal_wrapped(|ui| {
                    egui::TextEdit::singleline(&mut self.address)
                        .desired_width(100.0)
                        .hint_text("Address")
                        .show(ui);
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

                    ui.separator();
                    if ui.button("Configure project...").clicked() {
                        self.project_modal_open = true;
                    }
                    if ui.button("Load types").clicked() {
                        if let Some(mut task) = self.load_types_task.take() {
                            task.terminate();
                        }
                        let options = LoadTypesTaskOptions {
                            decomp_root: self.project_path.clone().into(),
                            include_paths: self.include_paths.iter().map(|s| s.into()).collect(),
                            ignore_paths: self.ignore_paths.iter().map(|s| s.into()).collect(),
                            types: self.types.clone(),
                        };
                        let mut task = LoadTypesTask::new(options);
                        if let Err(e) = task.run() {
                            log::error!("Failed to start type loading task: {e}");
                        } else {
                            self.load_types_task = Some(task);
                        }
                    }
                });
            });

        egui::TopBottomPanel::bottom("dzv_bottom_panel")
            .frame(egui::Frame::new().inner_margin(4).fill(Color32::from_gray(20)))
            .show(ctx, |ui| {
                ui.horizontal(|ui| {
                    if let Some(task) = &self.load_types_task {
                        ui.label(format!("Status: {}", task.status()));
                    } else {
                        ui.label("No type loading task running");
                    }
                    if ui.button("Cancel").clicked() {
                        if let Some(mut task) = self.load_types_task.take() {
                            task.terminate();
                        }
                    }
                });
            });

        egui::SidePanel::right("dzv_side_panel")
            .frame(egui::Frame::new().inner_margin(4).fill(Color32::from_gray(20)))
            .show(ctx, |ui| {
                if let Some(view) = &mut self.view {
                    view.render_side_panel(ctx, ui);
                }
            });

        egui::CentralPanel::default().show(ctx, |ui| {
            if self.project_modal_open {
                egui::Window::new("Configure project").show(ctx, |ui| {
                    egui::TextEdit::singleline(&mut self.project_path)
                        .desired_width(200.0)
                        .hint_text("Project path")
                        .show(ui);
                    ui.separator();
                    let mut remove_index = None;
                    egui_extras::TableBuilder::new(ui)
                        .id_salt("dzv_include_paths")
                        .striped(true)
                        .column(egui_extras::Column::exact(220.0))
                        .column(egui_extras::Column::exact(50.0))
                        .body(|mut body| {
                            for i in 0..self.include_paths.len() {
                                body.row(20.0, |mut row| {
                                    row.col(|ui| {
                                        egui::TextEdit::singleline(&mut self.include_paths[i])
                                            .desired_width(200.0)
                                            .hint_text("Include path")
                                            .show(ui);
                                    });
                                    row.col(|ui| {
                                        if egui::Button::new("Remove")
                                            .wrap_mode(egui::TextWrapMode::Extend)
                                            .ui(ui)
                                            .clicked()
                                        {
                                            remove_index = Some(i);
                                        }
                                    });
                                });
                            }
                            body.row(20.0, |mut row| {
                                row.col(|ui| {
                                    if ui.button("Add include path").clicked() {
                                        self.include_paths.push(String::new());
                                    }
                                });
                            });
                        });
                    if let Some(index) = remove_index {
                        self.include_paths.remove(index);
                    }
                    ui.separator();
                    let mut remove_index = None;
                    egui_extras::TableBuilder::new(ui)
                        .id_salt("dzv_ignore_paths")
                        .striped(true)
                        .column(egui_extras::Column::exact(220.0))
                        .column(egui_extras::Column::exact(50.0))
                        .body(|mut body| {
                            for i in 0..self.ignore_paths.len() {
                                body.row(20.0, |mut row| {
                                    row.col(|ui| {
                                        egui::TextEdit::singleline(&mut self.ignore_paths[i])
                                            .desired_width(200.0)
                                            .hint_text("Ignore path")
                                            .show(ui);
                                    });
                                    row.col(|ui| {
                                        if egui::Button::new("Remove")
                                            .wrap_mode(egui::TextWrapMode::Extend)
                                            .ui(ui)
                                            .clicked()
                                        {
                                            remove_index = Some(i);
                                        }
                                    });
                                });
                            }
                            body.row(20.0, |mut row| {
                                row.col(|ui| {
                                    if ui.button("Add ignore path").clicked() {
                                        self.ignore_paths.push(String::new());
                                    }
                                });
                            });
                        });
                    if let Some(index) = remove_index {
                        self.ignore_paths.remove(index);
                    }
                });
            }

            if let Some(view) = self.view.as_mut() {
                view.render_central_panel(ctx, ui);
            }
        });
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
