use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::fs;

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Config {
    pub app_name: String,
    pub version: String,
    pub debug: bool,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            app_name: "chonker8".to_string(),
            version: "8.9.0".to_string(),
            debug: false,
        }
    }
}

impl Config {
    pub fn load() -> Result<Self> {
        match fs::read_to_string("config.toml") {
            Ok(content) => toml::from_str(&content).map_err(Into::into),
            Err(_) => Ok(Self::default()),
        }
    }
    
    pub fn save(&self) -> Result<()> {
        fs::write("config.toml", toml::to_string_pretty(self)?).map_err(Into::into)
    }
}