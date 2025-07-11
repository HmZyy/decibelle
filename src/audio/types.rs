use std::path::PathBuf;
use std::time::Duration;

#[derive(Debug, Clone)]
pub struct PlaybackState {
    pub is_playing: bool,
    pub current_position: Duration,
    pub total_duration: Duration,
    pub volume: f32,
    pub playback_speed: f32,
    pub current_file: Option<PathBuf>,
}

impl Default for PlaybackState {
    fn default() -> Self {
        Self {
            is_playing: false,
            current_position: Duration::from_secs(0),
            total_duration: Duration::from_secs(0),
            volume: 1.0,
            playback_speed: 1.0,
            current_file: None,
        }
    }
}

#[derive(Debug, Clone)]
pub enum AudioCommand {
    Play,
    Pause,
    Stop,
    LoadFile(PathBuf),
    SetVolume(f32),
    SetSpeed(f32),
    Seek(Duration),
    GetState,
}

#[derive(Debug, Clone)]
pub enum AudioEvent {
    StateChanged(PlaybackState),
    Error(String),
    EndOfFile,
}
