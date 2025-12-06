use crate::player::commands::PlayerState;
use crossterm::event::KeyEvent;
use std::{path::PathBuf, time::Duration};

#[derive(Debug, Clone)]
pub enum AppEvent {
    // From input thread
    Input(KeyEvent),
    Resize(u16, u16),

    // From player thread
    PlayerStateChanged(PlayerState),
    PositionUpdate(Duration),
    DurationChanged(Duration),
    TrackEnded,
    PlayerError(String),

    // From API thread
    LibrariesLoaded(Vec<crate::api::models::Library>),
    ItemsLoaded(Vec<crate::api::models::LibraryItem>),
    ChaptersLoaded(Vec<crate::api::models::Chapter>),

    DownloadFinished(PathBuf, f64, TrackInfo),

    ApiError(String),
}

#[derive(Debug, Clone)]
pub struct TrackInfo {
    pub index: i32,
    pub start_offset: f64,
    pub duration: f64,
}

impl TrackInfo {
    pub fn single_file() -> Self {
        Self {
            index: 0,
            start_offset: 0.0,
            duration: 0.0,
        }
    }
}
