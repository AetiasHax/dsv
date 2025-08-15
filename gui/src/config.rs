use std::path::Path;

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use toml::Table;

#[derive(Serialize, Deserialize)]
pub struct Config {
    pub gdb: GdbConfig,
    pub types: TypesConfig,
    #[serde(default)]
    pub games: Table,
}

#[derive(Serialize, Deserialize)]
pub struct GdbConfig {
    pub address: String,
}

#[derive(Serialize, Deserialize)]
pub struct TypesConfig {
    pub project_root: String,
    pub include_paths: Vec<String>,
    pub ignore_paths: Vec<String>,
}

impl Config {
    pub fn new() -> Self {
        Config {
            gdb: GdbConfig { address: "127.0.0.1:3333".into() },
            types: TypesConfig {
                project_root: String::new(),
                include_paths: Vec::new(),
                ignore_paths: Vec::new(),
            },
            games: Table::new(),
        }
    }

    pub fn save_to_file<P: AsRef<Path>>(&self, path: P) -> Result<()> {
        let toml_string = toml::to_string(self).context("Failed to serialize config")?;
        std::fs::write(path, toml_string).context("Failed to write config file")
    }

    pub fn load_from_file<P: AsRef<Path>>(path: P) -> Result<Self> {
        let toml_string = std::fs::read_to_string(path).context("Failed to read config file")?;
        let config: Config = toml::from_str(&toml_string).context("Failed to parse config")?;
        Ok(config)
    }
}
