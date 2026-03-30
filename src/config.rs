use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;
use std::sync::RwLock;
use std::sync::atomic::{AtomicBool, AtomicU32};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Config {
    pub api_key: String,
    pub min_brightness: u32,
    pub max_brightness: u32,
    pub update_interval_secs: u64,
    pub start_on_startup: bool,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            api_key: String::new(),
            min_brightness: 0,
            max_brightness: 100,
            update_interval_secs: 180,
            start_on_startup: true,
        }
    }
}

pub struct SharedState {
    pub config: RwLock<Config>,

    pub needs_refetch: AtomicBool,

    pub status: RwLock<String>,

    pub current_brightness: AtomicU32,

    pub sunrise_str: RwLock<String>,
    pub noon_str: RwLock<String>,
}

impl SharedState {
    pub fn new(config: Config) -> Self {
        Self {
            config: RwLock::new(config),
            needs_refetch: AtomicBool::new(true),
            status: RwLock::new("Starting...".into()),
            current_brightness: AtomicU32::new(50),
            sunrise_str: RwLock::new(String::new()),
            noon_str: RwLock::new(String::new()),
        }
    }

    pub fn set_status(&self, s: &str) {
        if let Ok(mut w) = self.status.write() {
            *w = s.to_string();
        }
    }
}

pub fn config_path() -> PathBuf {
    directories::ProjectDirs::from("", "", "sunrise-brightness")
        .map(|d| d.config_dir().to_path_buf())
        .unwrap_or_else(|| PathBuf::from("."))
        .join("config.toml")
}

pub fn load_config() -> Config {
    let path = config_path();
    match fs::read_to_string(&path) {
        Ok(content) => toml::from_str(&content).unwrap_or_default(),
        Err(_) => Config::default(),
    }
}

pub fn save_config(config: &Config) -> Result<()> {
    let path = config_path();
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(&path, toml::to_string_pretty(config)?)?;
    Ok(())
}
