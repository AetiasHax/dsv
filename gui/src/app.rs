use std::{
    net::ToSocketAddrs,
    path::PathBuf,
    sync::{Arc, Mutex},
};

use anyhow::{Context, Result};
use dzv_core::gdb::client::GdbClient;
use eframe::egui::{self, Color32, Widget};

use crate::{
    config::Config,
    tasks::load_types::{LoadTypesTask, LoadTypesTaskOptions},
    views::{View, ph},
};

pub struct DzvApp {
    config_path: Option<PathBuf>,
    config: Config,

    project_modal_open: bool,
    types: Arc<Mutex<type_crawler::Types>>,
    load_types_task: Option<LoadTypesTask>,

    view: Option<Box<dyn View>>,
}

impl Default for DzvApp {
    fn default() -> Self {
        DzvApp {
            config_path: None,
            config: Config::new(),

            project_modal_open: false,
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
                    if ui.button("Open").clicked() {
                        let file =
                            rfd::FileDialog::new().add_filter("dzv project", &["toml"]).pick_file();
                        if let Some(file) = file {
                            self.load_config(file);
                        }
                    }

                    ui.separator();

                    if egui::TextEdit::singleline(&mut self.config.gdb.address)
                        .desired_width(100.0)
                        .hint_text("Address")
                        .show(ui)
                        .response
                        .lost_focus()
                    {
                        self.save_config();
                    }
                    if self.view.is_none() {
                        if ui.button("Connect").clicked()
                            && let Err(e) = self.connect()
                        {
                            log::error!("Failed to connect: {e}");
                        }
                    } else if ui.button("Disconnect").clicked()
                        && let Some(view) = &mut self.view
                    {
                        match view.exit() {
                            Ok(_) => self.view = None,
                            Err(e) => log::error!("Failed to disconnect: {e}"),
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
                        let project_root = self.config.types.project_root.clone().into();
                        let include_paths =
                            self.config.types.include_paths.iter().map(|s| s.into()).collect();
                        let ignore_paths =
                            self.config.types.ignore_paths.iter().map(|s| s.into()).collect();
                        let options = LoadTypesTaskOptions {
                            project_root,
                            include_paths,
                            ignore_paths,
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
                    if ui.button("Cancel").clicked()
                        && let Some(mut task) = self.load_types_task.take()
                    {
                        task.terminate();
                    }
                });
            });

        egui::SidePanel::right("dzv_side_panel")
            .frame(egui::Frame::new().inner_margin(4).fill(Color32::from_gray(20)))
            .show(ctx, |ui| {
                if let Some(view) = &mut self.view {
                    view.render_side_panel(ctx, ui, &self.types.lock().unwrap());
                }
            });

        egui::CentralPanel::default().show(ctx, |ui| {
            if self.project_modal_open {
                let mut open = self.project_modal_open;
                egui::Window::new("Configure project").open(&mut open).show(ctx, |ui| {
                    if egui::TextEdit::singleline(&mut self.config.types.project_root)
                        .desired_width(200.0)
                        .hint_text("Project path")
                        .show(ui)
                        .response
                        .lost_focus()
                    {
                        self.save_config();
                    }
                    ui.separator();
                    let mut remove_index = None;
                    egui_extras::TableBuilder::new(ui)
                        .id_salt("dzv_include_paths")
                        .striped(true)
                        .column(egui_extras::Column::exact(220.0))
                        .column(egui_extras::Column::exact(50.0))
                        .body(|mut body| {
                            for i in 0..self.config.types.include_paths.len() {
                                body.row(20.0, |mut row| {
                                    row.col(|ui| {
                                        if egui::TextEdit::singleline(
                                            &mut self.config.types.include_paths[i],
                                        )
                                        .desired_width(200.0)
                                        .hint_text("Include path")
                                        .show(ui)
                                        .response
                                        .lost_focus()
                                        {
                                            self.save_config();
                                        }
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
                                        self.config.types.include_paths.push(String::new());
                                    }
                                });
                            });
                        });
                    if let Some(index) = remove_index {
                        self.config.types.include_paths.remove(index);
                        self.save_config();
                    }
                    ui.separator();
                    let mut remove_index = None;
                    egui_extras::TableBuilder::new(ui)
                        .id_salt("dzv_ignore_paths")
                        .striped(true)
                        .column(egui_extras::Column::exact(220.0))
                        .column(egui_extras::Column::exact(50.0))
                        .body(|mut body| {
                            for i in 0..self.config.types.ignore_paths.len() {
                                body.row(20.0, |mut row| {
                                    row.col(|ui| {
                                        if egui::TextEdit::singleline(
                                            &mut self.config.types.ignore_paths[i],
                                        )
                                        .desired_width(200.0)
                                        .hint_text("Ignore path")
                                        .show(ui)
                                        .response
                                        .lost_focus()
                                        {
                                            self.save_config();
                                        }
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
                                        self.config.types.ignore_paths.push(String::new());
                                    }
                                });
                            });
                        });
                    if let Some(index) = remove_index {
                        self.config.types.ignore_paths.remove(index);
                        self.save_config();
                    }
                    if ui.button("Save").clicked() {
                        let file =
                            rfd::FileDialog::new().add_filter("dzv config", &["toml"]).save_file();
                        if let Some(file) = file {
                            self.config_path = Some(file);
                            self.save_config();
                        }
                    }
                });
                self.project_modal_open = open;
            }

            if let Some(view) = self.view.as_mut() {
                view.render_central_panel(ctx, ui, &self.types.lock().unwrap());
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
    fn save_config(&self) {
        let Some(path) = &self.config_path else {
            return;
        };
        self.config.save_to_file(path).unwrap_or_else(|e| {
            log::error!("Failed to save config: {e}");
        });
    }

    fn load_config(&mut self, path: PathBuf) {
        match Config::load_from_file(&path) {
            Ok(config) => {
                log::info!("Loaded config from {}", path.display());
                self.config = config;
                self.config_path = Some(path);
            }
            Err(e) => {
                log::error!("Failed to load config from {}: {e}", path.display());
            }
        }
    }

    fn connect(&mut self) -> Result<()> {
        log::info!("Connecting to GDB server at {}", self.config.gdb.address);

        let addr = self
            .config
            .gdb
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
