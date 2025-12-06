use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

use crate::ui::theme::ThemeName;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    pub server_url: String,
    pub api_key: String,
    #[serde(default)]
    pub theme: ThemeName,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            server_url: "http://localhost:13378".to_string(),
            api_key: "not set yet".to_string(),
            theme: ThemeName::default(),
        }
    }
}

fn get_config_path() -> Result<PathBuf> {
    let config_dir = dirs::config_dir()
        .context("Could not find config directory")?
        .join("decibelle");

    Ok(config_dir.join("config.yml"))
}

pub fn load_or_create_config() -> Result<Config> {
    let config_path = get_config_path()?;

    if !config_path.exists() {
        if let Some(parent) = config_path.parent() {
            fs::create_dir_all(parent).context("Failed to create config directory")?;
        }

        let default_config = Config::default();
        let yaml =
            serde_yaml::to_string(&default_config).context("Failed to serialize default config")?;

        fs::write(&config_path, yaml).context("Failed to write default config file")?;

        eprintln!("Config file created at: {}", config_path.display());
        eprintln!("\nPlease edit the config file and set your API key and server URL:");
        eprintln!("  server_url: Your Audiobookshelf server URL");
        eprintln!("  api_key: Your Audiobookshelf API key");
        eprintln!("  theme: tokyo_night or catppuccin_mocha");
        anyhow::bail!("Config file not configured. Please set your API key and server URL.");
    }

    let config_content = fs::read_to_string(&config_path).context("Failed to read config file")?;

    let config: Config =
        serde_yaml::from_str(&config_content).context("Failed to parse config file")?;

    if config.api_key == "not set yet" || config.api_key.is_empty() {
        anyhow::bail!("API key not set in config file: {}", config_path.display());
    }

    Ok(config)
}
