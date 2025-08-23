use std::{
    path::{Path, PathBuf},
    sync::{Arc, Mutex, mpsc},
    thread::JoinHandle,
    time::Instant,
};

use anyhow::{Context, Result};
use type_crawler::{Env, EnvOptions, TypeCrawler, Types, WordSize};

pub struct LoadTypesTask {
    decomp_root: PathBuf,
    include_paths: Vec<PathBuf>,
    ignore_paths: Vec<PathBuf>,
    types: Arc<Mutex<type_crawler::Types>>,
    status: Arc<Mutex<String>>,
    thread_handle: Option<JoinHandle<()>>,
    terminate_tx: Option<mpsc::Sender<()>>,
}

pub struct LoadTypesTaskOptions {
    pub project_root: PathBuf,
    pub include_paths: Vec<PathBuf>,
    pub ignore_paths: Vec<PathBuf>,
    pub types: Arc<Mutex<type_crawler::Types>>,
}

impl LoadTypesTask {
    pub fn new(options: LoadTypesTaskOptions) -> Self {
        LoadTypesTask {
            decomp_root: options.project_root,
            include_paths: options.include_paths,
            ignore_paths: options.ignore_paths,
            types: options.types,
            status: Arc::new(Mutex::new(String::new())),
            thread_handle: None,
            terminate_tx: None,
        }
    }

    pub fn run(&mut self) -> Result<()> {
        if self.thread_handle.is_some() {
            log::warn!("Type loading task is already running.");
            return Ok(());
        }

        let include_paths = self.include_paths.to_vec();
        let types_result = self.types.clone();
        let status = self.status.clone();
        let headers = self.find_header_files(&self.decomp_root);

        let (terminate_tx, terminate_rx) = mpsc::channel();
        self.terminate_tx = Some(terminate_tx);

        self.thread_handle = Some(std::thread::spawn(move || {
            let env = Env::new(EnvOptions {
                word_size: WordSize::Size32,
                short_enums: false,
                signed_char: true,
            });
            let mut crawler =
                TypeCrawler::new(env).context("Failed to create type crawler").unwrap();
            include_paths.iter().for_each(|path| {
                crawler.add_include_path(path).unwrap();
            });

            let start = Instant::now();
            let mut types = Types::new();
            for header in &headers {
                if terminate_rx.try_recv().is_ok() {
                    log::info!("Type loading task terminated early.");
                    return;
                }

                *status.lock().unwrap() = format!("{}", header.display());
                let new_types = crawler.parse_file(header).unwrap();
                match types.extend(new_types) {
                    Ok(()) => {}
                    Err(err) => panic!("Error extending types: {err}"),
                }
            }
            let end = Instant::now();
            *status.lock().unwrap() =
                format!("Loaded {} types in {:.2}s", types.len(), (end - start).as_secs_f32());

            *types_result.lock().unwrap() = types;
        }));
        Ok(())
    }

    pub fn terminate(&mut self) {
        if let Some(tx) = self.terminate_tx.take() {
            let _ = tx.send(());
        }
        if let Some(handle) = self.thread_handle.take() {
            let _ = handle.join();
        }
    }

    pub fn status(&self) -> String {
        self.status.lock().unwrap().clone()
    }

    fn find_header_files<P: AsRef<Path>>(&self, dir: P) -> Vec<PathBuf> {
        let dir = dir.as_ref();
        if self.ignore_paths.iter().any(|p| p.starts_with(dir)) {
            return Vec::new();
        }
        let mut header_files = Vec::new();
        if dir.is_dir() {
            for entry in std::fs::read_dir(dir).unwrap() {
                let entry = entry.unwrap();
                let path = entry.path();
                if path.is_dir() {
                    header_files.extend(self.find_header_files(&path));
                } else if path.extension().is_some_and(|ext| ext == "hpp" || ext == "h") {
                    header_files.push(path);
                }
            }
        }
        header_files
    }
}
