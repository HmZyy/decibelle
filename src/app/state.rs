use std::path::PathBuf;
use std::sync::mpsc;
use std::time::Duration;

use crossterm::event::{KeyCode, KeyEvent, MouseButton, MouseEvent, MouseEventKind};
use ratatui::layout::Rect;

use crate::api::models::{AudioTrack, Chapter, Library, LibraryItem};
use crate::api::thread::ApiCommand;
use crate::app::{decrement, incrememnt};
use crate::events::types::TrackInfo;
use crate::player::commands::{PlayerCommand, PlayerState};
use crate::ui::notifications::NotificationManager;

#[derive(Default, Clone)]
pub struct LayoutRegions {
    pub library_list: Option<Rect>,
    pub chapters: Option<Rect>,
    pub controls: Option<Rect>,
    pub progress_bar: Option<Rect>,
    pub info_panel: Option<Rect>,
}

pub struct App {
    // Data
    pub selected_library_index: usize,
    pub selected_library_item_index: usize,
    pub selected_chapter_index: usize,

    pub libraries: Vec<Library>,
    pub library_items: Vec<LibraryItem>,
    pub chapters: Vec<Chapter>,

    pub current_chapter: Option<Chapter>,
    pub current_library_item: Option<LibraryItem>,
    pub current_item_id: Option<String>,

    pub loading_libraries: bool,
    pub loading_items: bool,
    pub loading_chapters: bool,

    // Selection state
    pub focus: Focus,

    // Info panel scroll
    pub info_scroll: u16,

    // Playback state
    pub player_state: PlayerState,
    pub current_position: Duration,
    pub total_duration: Duration,
    pub playback_speed: f32,

    pub current_track_info: Option<TrackInfo>,
    pub current_tracks: Vec<AudioTrack>,

    // Communication
    pub player_tx: mpsc::Sender<PlayerCommand>,
    pub api_tx: mpsc::Sender<ApiCommand>,

    // Notifications
    pub notifications: NotificationManager,

    // Control
    pub should_quit: bool,
    pub auto_resume_pending: bool,
    pub error_message: Option<String>,
    pub layout_regions: LayoutRegions,
}

#[derive(PartialEq, Clone, Copy)]
pub enum Focus {
    Libraries,
    Chapters,
    Controls,
    InfoPanel,
}

impl App {
    pub fn new(player_tx: mpsc::Sender<PlayerCommand>, api_tx: mpsc::Sender<ApiCommand>) -> Self {
        Self {
            selected_library_index: 0,
            selected_library_item_index: 0,
            selected_chapter_index: 0,

            libraries: Vec::new(),
            library_items: Vec::new(),
            chapters: Vec::new(),

            current_chapter: None,
            current_item_id: None,
            current_library_item: None,

            loading_libraries: false,
            loading_items: false,
            loading_chapters: false,

            focus: Focus::Libraries,

            info_scroll: 0,

            player_state: PlayerState::Stopped,
            current_position: Duration::ZERO,
            total_duration: Duration::ZERO,
            playback_speed: 1.0,

            current_track_info: None,
            current_tracks: Vec::new(),

            player_tx,
            api_tx,

            notifications: NotificationManager::new(),

            should_quit: false,
            auto_resume_pending: true,
            error_message: None,
            layout_regions: LayoutRegions::default(),
        }
    }

    pub fn load_libraries(&mut self) {
        self.loading_libraries = true;
        let _ = self.api_tx.send(ApiCommand::FetchLibraries);
    }

    pub fn load_library_items(&mut self, library_id: &str) {
        self.loading_items = true;
        let _ = self
            .api_tx
            .send(ApiCommand::FetchLibraryItems(library_id.to_string()));
    }

    pub fn load_chapters(&mut self, item_id: &str) {
        self.loading_chapters = true;
        let _ = self
            .api_tx
            .send(ApiCommand::FetchItemChapters(item_id.to_string()));
    }

    fn sync_progress(&self) {
        if let Some(ref item_id) = self.current_item_id {
            let current_time = self.current_position.as_secs_f64();
            let duration = self.get_total_duration();
            let is_finished = duration > 0.0 && current_time >= duration - 1.0;

            let _ = self.api_tx.send(ApiCommand::UpdateProgress {
                item_id: item_id.clone(),
                current_time,
                duration,
                is_finished,
            });
        }
    }

    fn get_total_duration(&self) -> f64 {
        self.current_library_item
            .as_ref()
            .and_then(|item| item.media.as_ref())
            .and_then(|media| media.duration)
            .unwrap_or(self.total_duration.as_secs_f64())
    }

    pub fn on_libraries_loaded(&mut self, libraries: Vec<Library>) {
        self.loading_libraries = false;
        self.libraries = libraries;
        self.selected_library_index = 0;

        if let Some(lib) = self.libraries.clone().first() {
            self.load_library_items(&lib.id);

            if self.auto_resume_pending {
                let _ = self
                    .api_tx
                    .send(ApiCommand::FetchContinueListening(lib.id.clone()));
            }
        }
    }

    pub fn on_items_loaded(&mut self, items: Vec<LibraryItem>) {
        self.loading_items = false;
        self.library_items = items;
        self.selected_library_item_index = 0;
        self.chapters.clear();
    }

    pub fn on_chapters_loaded(&mut self, chapters: Vec<Chapter>) {
        self.loading_chapters = false;
        self.chapters = chapters;
        self.selected_chapter_index = 0;
    }

    pub fn on_download_finished(
        &mut self,
        path: PathBuf,
        local_position: f64,
        track_info: TrackInfo,
    ) {
        self.current_track_info = Some(track_info);

        let position = Duration::from_secs_f64(local_position);
        let _ = self.player_tx.send(PlayerCommand::Play { path, position });
    }

    pub fn on_continue_listening_loaded(&mut self, item: LibraryItem, position: f64) {
        if !self.auto_resume_pending {
            return;
        }

        if let Some(index) = self.library_items.iter().position(|i| i.id == item.id) {
            self.selected_library_item_index = index;
        }

        self.current_library_item = Some(item.clone());
        self.current_item_id = Some(item.id.clone());

        self.load_chapters(&item.id);

        if let Some(ref media) = item.media {
            if let Some(ref tracks) = media.tracks {
                self.current_tracks = tracks.clone();
            }
        }

        self.focus = Focus::Chapters;
        let resume_position = (position - 10.0).max(0.0);
        let _ = self.api_tx.send(ApiCommand::DownloadForPlayback(
            item.id.clone(),
            resume_position,
        ));
    }

    pub fn on_api_error(&mut self, error: String) {
        self.loading_libraries = false;
        self.loading_items = false;
        self.loading_chapters = false;
        self.error_message = Some(error.clone());
        self.notifications.error(format!("API Error: {}", error));
    }

    pub fn on_player_state_changed(&mut self, state: PlayerState) {
        let previous_state = self.player_state;
        self.player_state = state;

        if self.auto_resume_pending {
            self.auto_resume_pending = false;
            let _ = self.player_tx.send(PlayerCommand::Pause);
            return;
        }

        // Sync progress when playback state changes
        match (previous_state, state) {
            (PlayerState::Playing, PlayerState::Paused) => {
                self.sync_progress();
            }
            (PlayerState::Playing, PlayerState::Stopped) => {
                self.sync_progress();
            }
            (PlayerState::Paused, PlayerState::Stopped) => {
                self.sync_progress();
            }
            _ => {}
        }
    }

    pub fn on_position_update(&mut self, position: Duration) {
        // Convert track-local position to global position
        if let Some(ref track_info) = self.current_track_info {
            self.current_position =
                Duration::from_secs_f64(track_info.start_offset + position.as_secs_f64());
        } else {
            self.current_position = position;
        }

        self.update_current_chapter();
        self.check_track_boundary();
    }

    fn check_track_boundary(&mut self) {
        let Some(ref track_info) = self.current_track_info else {
            return;
        };

        let track_end = track_info.start_offset + track_info.duration;
        let global_pos = self.current_position.as_secs_f64();

        if global_pos >= track_end - 0.5 {
            let has_next = self
                .current_tracks
                .iter()
                .any(|t| t.index == track_info.index + 1);

            if has_next {
                if let Some(ref item) = self.current_library_item {
                    let _ = self.api_tx.send(ApiCommand::DownloadForPlayback(
                        item.id.clone(),
                        track_end + 0.1,
                    ));
                }
            }
        }
    }

    fn update_current_chapter(&mut self) {
        if self.current_item_id.is_none() {
            return;
        }

        let pos_secs = self.current_position.as_secs_f64();

        if let Some(ref current) = self.current_chapter {
            if pos_secs >= current.start && pos_secs < current.end {
                return; // Still in the same chapter
            }
        }

        for (i, chapter) in self.chapters.iter().enumerate() {
            if pos_secs >= chapter.start && pos_secs < chapter.end {
                self.current_chapter = Some(chapter.clone());
                self.selected_chapter_index = i;
                return;
            }
        }

        if let Some(last) = self.chapters.last() {
            if pos_secs >= last.end {
                self.current_chapter = None;
            }
        }
    }

    pub fn on_duration_changed(&mut self, duration: Duration) {
        self.total_duration = duration;
    }

    pub fn on_track_ended(&mut self) {}

    pub fn on_player_error(&mut self, error: String) {
        self.error_message = Some(format!("Player error: {}", error));
        self.notifications.error(format!("Player: {}", error));
        self.player_state = PlayerState::Stopped;
    }

    pub fn play_current_chapter(&mut self) {
        if let Some(chapter) = self.chapters.get(self.selected_chapter_index) {
            if let Some(item) = self.library_items.get(self.selected_library_item_index) {
                let path = self.get_audio_file_path(item);
                let position = Duration::from_secs_f64(chapter.start);

                self.current_chapter = Some(chapter.clone());
                self.current_item_id = Some(item.id.clone());

                let _ = self.player_tx.send(PlayerCommand::Play { path, position });
            }
        }
    }

    pub fn scroll_info_up(&mut self) {
        self.info_scroll = self.info_scroll.saturating_sub(1);
    }

    pub fn scroll_info_down(&mut self, max_scroll: u16) {
        if self.info_scroll < max_scroll {
            self.info_scroll = self.info_scroll.saturating_add(1);
        }
    }

    pub fn handle_input(&mut self, key: KeyEvent) -> () {
        match key.code {
            KeyCode::Char('q') => {
                self.sync_progress();
                self.should_quit = true;
            }
            KeyCode::Tab => {
                self.cycle_focus(false);
            }
            KeyCode::BackTab => {
                self.cycle_focus(true);
            }
            KeyCode::Char('L') => {
                if self.focus == Focus::Libraries {
                    self.next_library();
                    self.load_library_items(
                        &self.libraries.clone()[self.selected_library_index].id,
                    );
                }
            }
            KeyCode::Char('H') => {
                if self.focus == Focus::Libraries {
                    self.previous_library();
                    self.load_library_items(
                        &self.libraries.clone()[self.selected_library_index].id,
                    );
                }
            }
            KeyCode::Char('l') | KeyCode::Right => {
                if self.focus == Focus::Libraries {
                    self.current_library_item = self
                        .library_items
                        .get(self.selected_library_item_index)
                        .cloned();

                    self.load_chapters(
                        &self.library_items.clone()[self.selected_library_item_index].id,
                    );

                    self.cycle_focus(false);
                } else if self.focus == Focus::Chapters {
                } else if self.focus == Focus::Controls {
                    self.seek_forward(5.0);
                } else if self.focus == Focus::InfoPanel {
                }
            }
            KeyCode::Char('h') | KeyCode::Left => {
                if self.focus == Focus::Libraries {
                } else if self.focus == Focus::Chapters {
                    self.cycle_focus(true);
                } else if self.focus == Focus::Controls {
                    self.seek_backward(5.0);
                } else if self.focus == Focus::InfoPanel {
                }
            }
            KeyCode::Char('j') | KeyCode::Down => {
                if self.focus == Focus::Libraries {
                    self.next_library_item();
                } else if self.focus == Focus::Chapters {
                    self.next_chapter();
                } else if self.focus == Focus::InfoPanel {
                    self.scroll_info_down(100);
                }
            }
            KeyCode::Char('k') | KeyCode::Up => {
                if self.focus == Focus::Libraries {
                    self.previous_libaray_item();
                } else if self.focus == Focus::Chapters {
                    self.previous_chapter();
                } else if self.focus == Focus::InfoPanel {
                    self.scroll_info_up();
                }
            }
            KeyCode::Enter => {
                if self.focus == Focus::Libraries {
                    self.current_library_item = self
                        .library_items
                        .get(self.selected_library_item_index)
                        .cloned();

                    self.load_chapters(
                        &self.library_items.clone()[self.selected_library_item_index].id,
                    );
                } else if self.focus == Focus::Chapters {
                    if let (Some(selected_chapter), Some(selected_item)) = (
                        self.chapters.get(self.selected_chapter_index),
                        self.library_items.get(self.selected_library_item_index),
                    ) {
                        self.current_chapter = Some(selected_chapter.clone());
                        self.current_item_id = Some(selected_item.id.clone());

                        let _ = self.api_tx.send(ApiCommand::DownloadForPlayback(
                            selected_item.id.clone(),
                            selected_chapter.start,
                        ));
                    }
                }
            }
            KeyCode::Char(' ') => {
                self.toggle_playback();
            }
            _ => {}
        }
    }

    fn point_in_rect(&self, x: u16, y: u16, rect: &Rect) -> bool {
        x >= rect.x && x < rect.x + rect.width && y >= rect.y && y < rect.y + rect.height
    }

    pub fn handle_mouse(&mut self, event: MouseEvent) {
        match event.kind {
            MouseEventKind::Down(MouseButton::Left) => {
                let x = event.column;
                let y = event.row;

                if let Some(ref region) = self.layout_regions.library_list {
                    if self.point_in_rect(x, y, region) {
                        self.focus = Focus::Libraries;
                        if y > region.y && y < region.y + region.height - 1 {
                            let clicked_index = (y - region.y - 1) as usize;
                            if clicked_index < self.library_items.len() {
                                self.selected_library_item_index = clicked_index;
                            }
                        }
                        return;
                    }
                }

                if let Some(ref region) = self.layout_regions.chapters {
                    if self.point_in_rect(x, y, region) {
                        self.focus = Focus::Chapters;
                        if y > region.y && y < region.y + region.height - 1 {
                            let clicked_index = (y - region.y - 1) as usize;
                            if clicked_index < self.chapters.len() {
                                self.selected_chapter_index = clicked_index;
                            }
                        }
                        return;
                    }
                }

                if let Some(ref region) = self.layout_regions.controls {
                    if self.point_in_rect(x, y, region) {
                        self.focus = Focus::Controls;
                        return;
                    }
                }

                if let Some(ref region) = self.layout_regions.info_panel {
                    if self.point_in_rect(x, y, region) {
                        self.focus = Focus::InfoPanel;
                        return;
                    }
                }
            }

            MouseEventKind::Down(MouseButton::Right) => {
                let x = event.column;
                let y = event.row;

                if let Some(ref region) = self.layout_regions.library_list {
                    if self.point_in_rect(x, y, region) {
                        self.focus = Focus::Libraries;
                        if y > region.y && y < region.y + region.height - 1 {
                            let clicked_index = (y - region.y - 1) as usize;
                            if clicked_index < self.library_items.len() {
                                self.selected_library_item_index = clicked_index;
                                self.current_library_item = self
                                    .library_items
                                    .get(self.selected_library_item_index)
                                    .cloned();
                                self.load_chapters(
                                    &self.library_items.clone()[self.selected_library_item_index]
                                        .id,
                                );
                                self.focus = Focus::Chapters;
                            }
                        }
                        return;
                    }
                }

                if let Some(ref region) = self.layout_regions.chapters {
                    if self.point_in_rect(x, y, region) {
                        self.focus = Focus::Chapters;
                        if y > region.y && y < region.y + region.height - 1 {
                            let clicked_index = (y - region.y - 1) as usize;
                            if clicked_index < self.chapters.len() {
                                self.selected_chapter_index = clicked_index;
                                if let (Some(selected_chapter), Some(selected_item)) = (
                                    self.chapters.get(self.selected_chapter_index),
                                    self.library_items.get(self.selected_library_item_index),
                                ) {
                                    self.current_chapter = Some(selected_chapter.clone());
                                    self.current_item_id = Some(selected_item.id.clone());
                                    let _ = self.api_tx.send(ApiCommand::DownloadForPlayback(
                                        selected_item.id.clone(),
                                        selected_chapter.start,
                                    ));
                                }
                            }
                        }
                        return;
                    }
                }
            }

            MouseEventKind::Down(MouseButton::Middle) => {
                self.toggle_playback();
            }

            MouseEventKind::ScrollUp => match self.focus {
                Focus::Libraries => self.previous_libaray_item(),
                Focus::Chapters => self.previous_chapter(),
                Focus::Controls => self.seek_forward(5.0),
                Focus::InfoPanel => self.scroll_info_up(),
            },

            MouseEventKind::ScrollDown => match self.focus {
                Focus::Libraries => self.next_library_item(),
                Focus::Chapters => self.next_chapter(),
                Focus::Controls => self.seek_backward(5.0),
                Focus::InfoPanel => self.scroll_info_down(100),
            },

            _ => {}
        }
    }

    pub fn cycle_focus(&mut self, reverse: bool) {
        self.focus = match (self.focus, reverse) {
            (Focus::Libraries, false) => Focus::Chapters,
            (Focus::Chapters, false) => Focus::InfoPanel,
            (Focus::InfoPanel, false) => Focus::Controls,
            (Focus::Controls, false) => Focus::Libraries,

            (Focus::Libraries, true) => Focus::Controls,
            (Focus::Chapters, true) => Focus::Libraries,
            (Focus::InfoPanel, true) => Focus::Chapters,
            (Focus::Controls, true) => Focus::InfoPanel,
        };
    }

    pub fn next_library(&mut self) {
        let library_count = self.libraries.len();
        self.selected_library_index = incrememnt(self.selected_library_index, library_count, false);
    }

    pub fn previous_library(&mut self) {
        let library_count = self.libraries.len();
        self.selected_library_index = decrement(self.selected_library_index, library_count, false);
    }

    pub fn next_library_item(&mut self) {
        let library_items_count = self.library_items.len();
        self.selected_library_item_index =
            incrememnt(self.selected_library_item_index, library_items_count, false);
    }

    pub fn previous_libaray_item(&mut self) {
        let library_items_count = self.library_items.len();
        self.selected_library_item_index =
            decrement(self.selected_library_item_index, library_items_count, false);
    }

    pub fn next_chapter(&mut self) {
        let chapters_count = self.chapters.len();
        self.selected_chapter_index =
            incrememnt(self.selected_chapter_index, chapters_count, false);
    }

    pub fn previous_chapter(&mut self) {
        let chapters_count = self.chapters.len();
        self.selected_chapter_index = decrement(self.selected_chapter_index, chapters_count, false);
    }

    pub fn toggle_playback(&mut self) {
        match self.player_state {
            PlayerState::Playing => {
                let _ = self.player_tx.send(PlayerCommand::Pause);
            }
            PlayerState::Paused => {
                let _ = self.player_tx.send(PlayerCommand::Resume);
            }
            PlayerState::Stopped => {
                self.play_current_chapter();
            }
            PlayerState::Loading => {}
        }
    }

    pub fn stop_playback(&mut self) {
        let _ = self.player_tx.send(PlayerCommand::Stop);
    }

    pub fn seek_forward(&mut self, secs: f64) {
        let new_global_pos = self.current_position + Duration::from_secs_f64(secs);
        let clamped = new_global_pos.min(self.total_duration);

        self.seek_to_global_position(clamped.as_secs_f64());
    }

    pub fn seek_backward(&mut self, secs: f64) {
        let new_global_pos = self
            .current_position
            .saturating_sub(Duration::from_secs_f64(secs));

        self.seek_to_global_position(new_global_pos.as_secs_f64());
    }

    fn seek_to_global_position(&mut self, global_pos: f64) {
        if let Some(ref track_info) = self.current_track_info {
            let track_start = track_info.start_offset;
            let track_end = track_start + track_info.duration;

            if global_pos >= track_start && global_pos < track_end {
                // Seek within current track
                let local_pos = global_pos - track_start;
                let _ = self
                    .player_tx
                    .send(PlayerCommand::Seek(Duration::from_secs_f64(local_pos)));
            } else {
                // Need to switch tracks - download and play the right one
                if let Some(ref item) = self.current_library_item {
                    let _ = self.player_tx.send(PlayerCommand::Stop);
                    let _ = self
                        .api_tx
                        .send(ApiCommand::DownloadForPlayback(item.id.clone(), global_pos));
                }
            }
        } else {
            // Single file mode - just seek directly
            let _ = self
                .player_tx
                .send(PlayerCommand::Seek(Duration::from_secs_f64(global_pos)));
        }
    }

    fn get_audio_file_path(&self, item: &LibraryItem) -> PathBuf {
        PathBuf::from(format!("/tmp/decibelle_{}.audio", item.id))
    }
}
