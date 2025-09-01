// Simplified configuration structures
use serde::{Deserialize, Serialize};
use std::fs;
use anyhow::Result;

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
        // Try to load from config.toml, or use defaults if it doesn't exist
        if let Ok(content) = fs::read_to_string("config.toml") {
            Ok(toml::from_str(&content)?)
        } else {
            // Return default config if config.toml doesn't exist
            Ok(Self::default())
        }
    }
    
    pub fn save(&self) -> Result<()> {
        let content = toml::to_string_pretty(self)?;
        fs::write("config.toml", content)?;
        Ok(())
    }
}