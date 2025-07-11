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
    pub chapters: Vec<Chapter>,
    pub current_chapter: Option<usize>,
}

#[derive(Debug, Clone)]
pub struct Chapter {
    pub title: String,
    pub start_time: Duration,
    pub end_time: Duration,
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
            chapters: Vec::new(),
            current_chapter: None,
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
    SeekToChapter(usize),
    GetState,
}

#[derive(Debug, Clone)]
pub enum AudioEvent {
    StateChanged(PlaybackState),
    Error(String),
    EndOfFile,
}
