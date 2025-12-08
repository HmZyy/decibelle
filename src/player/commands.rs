use std::path::PathBuf;
use std::time::Duration;

#[derive(Debug, Clone)]
#[allow(dead_code)]
pub enum PlayerCommand {
    Play { path: PathBuf, position: Duration },
    Pause,
    Resume,
    Stop,
    Seek(Duration),
    SetSpeed(f32),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PlayerState {
    Stopped,
    Loading,
    Playing,
    Paused,
}
