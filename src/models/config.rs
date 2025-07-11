

use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    pub audiobook_directory: PathBuf,
    pub volume: f32,
    pub playback_speed: f32,
    pub auto_save_position: bool,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            audiobook_directory: dirs::home_dir()
                .unwrap_or_else(|| PathBuf::from("."))
                .join("Audiobooks"),
            volume: 1.0,
            playback_speed: 1.0,
            auto_save_position: true,
        }
    }
}
