use crate::audio::AudioPlayer;
use crate::audiobook_scanner::AudiobookScanner;
use crate::models::book::Book;
use anyhow::Result;
use crossterm::event::KeyEvent;
use std::path::PathBuf;
use std::time::Duration;

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum FocusedPane {
    BookList,
    ChapterList,
    BookInfo,
    AudioControls,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Side {
    Left,
    Right,
}

pub struct App {
    pub should_quit: bool,
    pub focused_pane: FocusedPane,
    pub current_side: Side,
    pub books: Vec<Book>,
    pub selected_book_index: usize,
    pub selected_chapter_index: usize,
    pub is_playing: bool,
    pub progress: f64, // 0.0 to 1.0
    pub current_time: String,
    pub total_time: String,
    pub is_loading: bool,
    pub error_message: Option<String>,
    pub audio_player: Option<AudioPlayer>,
    pub current_audio_files: Vec<PathBuf>,
}

impl App {
    pub fn new() -> Self {
        Self {
            should_quit: false,
            focused_pane: FocusedPane::BookList,
            current_side: Side::Left,
            books: Vec::new(),
            selected_book_index: 0,
            selected_chapter_index: 0,
            is_playing: false,
            progress: 0.0,
            current_time: "00:00".to_string(),
            total_time: "00:00".to_string(),
            is_loading: true,
            error_message: None,
            audio_player: None,
            current_audio_files: Vec::new(),
        }
    }

    pub async fn initialize(&mut self) -> Result<()> {
        self.is_loading = true;
        self.error_message = None;

        // Initialize audio player
        match AudioPlayer::new() {
            Ok(player) => {
                self.audio_player = Some(player);
            }
            Err(e) => {
                self.error_message = Some(format!("Failed to initialize audio player: {}", e));
            }
        }

        let audiobook_dir = dirs::home_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join("Audiobooks");

        let scanner = AudiobookScanner::new(audiobook_dir.clone());

        match scanner.scan_audiobooks().await {
            Ok(books) => {
                self.books = books;
                if self.books.is_empty() {
                    self.error_message =
                        Some("No audiobooks found in ~/Audiobooks directory".to_string());
                } else {
                    self.selected_book_index = 0;
                    self.selected_chapter_index = 0;
                    // Load audio files for the first book
                    self.load_book_audio_files().await;
                }
            }
            Err(e) => {
                self.error_message = Some(format!("Error scanning audiobooks: {}", e));
            }
        }

        self.is_loading = false;
        Ok(())
    }

    async fn load_book_audio_files(&mut self) {
        if let Some(book) = self.books.get(self.selected_book_index) {
            // Find all audio files in the book directory
            let book_path = PathBuf::from(&book.path);
            let mut audio_files = Vec::new();

            if let Ok(entries) = std::fs::read_dir(&book_path) {
                for entry in entries.flatten() {
                    if let Some(ext) = entry.path().extension() {
                        if let Some(ext_str) = ext.to_str() {
                            match ext_str.to_lowercase().as_str() {
                                "mp3" | "m4a" | "m4b" | "flac" | "ogg" | "wav" | "aac" => {
                                    audio_files.push(entry.path());
                                }
                                _ => {}
                            }
                        }
                    }
                }
            }

            // Sort files naturally
            audio_files.sort_by(|a, b| {
                let a_name = a.file_name().unwrap_or_default();
                let b_name = b.file_name().unwrap_or_default();
                a_name.cmp(&b_name)
            });

            self.current_audio_files = audio_files;
        }
    }

    pub async fn handle_key_event(&mut self, key: KeyEvent) {
        use crossterm::event::KeyCode;

        // Don't handle input while loading
        if self.is_loading {
            return;
        }

        match key.code {
            KeyCode::Char('h') => self.cycle_pane_left(),
            KeyCode::Char('l') => self.cycle_pane_right(),
            KeyCode::Char('j') => self.move_down().await,
            KeyCode::Char('k') => self.move_up().await,
            KeyCode::Enter => self.select_current_item().await,
            KeyCode::Char(' ') => self.toggle_playback().await,
            KeyCode::Char('r') => {
                // Refresh/reload audiobooks
                self.is_loading = true;
                // This will be handled in the main loop
            }
            KeyCode::Char('+') | KeyCode::Char('=') => {
                if let Some(audio_player) = &self.audio_player {
                    let current_state = audio_player.get_state().await;
                    let _ = audio_player
                        .set_volume((current_state.volume + 0.1).min(1.0))
                        .await;
                }
            }
            KeyCode::Char('-') => {
                if let Some(audio_player) = &self.audio_player {
                    let current_state = audio_player.get_state().await;
                    let _ = audio_player
                        .set_volume((current_state.volume - 0.1).max(0.0))
                        .await;
                }
            }
            _ => {}
        }
    }

    fn cycle_pane_left(&mut self) {
        if self.books.is_empty() {
            return;
        }

        self.focused_pane = match self.focused_pane {
            FocusedPane::BookList => FocusedPane::AudioControls,
            FocusedPane::ChapterList => FocusedPane::BookList,
            FocusedPane::BookInfo => FocusedPane::ChapterList,
            FocusedPane::AudioControls => FocusedPane::BookInfo,
        };
        self.update_current_side();
    }

    fn cycle_pane_right(&mut self) {
        if self.books.is_empty() {
            return;
        }

        self.focused_pane = match self.focused_pane {
            FocusedPane::BookList => FocusedPane::ChapterList,
            FocusedPane::ChapterList => FocusedPane::BookInfo,
            FocusedPane::BookInfo => FocusedPane::AudioControls,
            FocusedPane::AudioControls => FocusedPane::BookList,
        };
        self.update_current_side();
    }

    fn update_current_side(&mut self) {
        self.current_side = match self.focused_pane {
            FocusedPane::BookList | FocusedPane::ChapterList => Side::Left,
            FocusedPane::BookInfo | FocusedPane::AudioControls => Side::Right,
        };
    }

    async fn move_down(&mut self) {
        if self.books.is_empty() {
            return;
        }

        match self.focused_pane {
            FocusedPane::BookList => {
                if self.selected_book_index < self.books.len().saturating_sub(1) {
                    self.selected_book_index += 1;
                    self.selected_chapter_index = 0; // Reset chapter selection
                    self.load_book_audio_files().await;
                }
            }
            FocusedPane::ChapterList => {
                if let Some(book) = self.books.get(self.selected_book_index) {
                    if self.selected_chapter_index < book.chapters.len().saturating_sub(1) {
                        self.selected_chapter_index += 1;
                    }
                }
            }
            FocusedPane::BookInfo => {
                self.focused_pane = FocusedPane::AudioControls;
                self.update_current_side();
            }
            FocusedPane::AudioControls => {
                self.focused_pane = FocusedPane::BookInfo;
                self.update_current_side();
            }
        }
    }

    async fn move_up(&mut self) {
        if self.books.is_empty() {
            return;
        }

        match self.focused_pane {
            FocusedPane::BookList => {
                if self.selected_book_index > 0 {
                    self.selected_book_index -= 1;
                    self.selected_chapter_index = 0; // Reset chapter selection
                    self.load_book_audio_files().await;
                }
            }
            FocusedPane::ChapterList => {
                if self.selected_chapter_index > 0 {
                    self.selected_chapter_index -= 1;
                }
            }
            FocusedPane::BookInfo => {
                self.focused_pane = FocusedPane::AudioControls;
                self.update_current_side();
            }
            FocusedPane::AudioControls => {
                self.focused_pane = FocusedPane::BookInfo;
                self.update_current_side();
            }
        }
    }

    async fn select_current_item(&mut self) {
        if self.books.is_empty() {
            return;
        }

        match self.focused_pane {
            FocusedPane::BookList => {
                // Move focus to chapter list when a book is selected
                self.focused_pane = FocusedPane::ChapterList;
                self.update_current_side();

                // Load audio files for the selected book
                self.load_book_audio_files().await;
            }
            FocusedPane::ChapterList => {
                // Load and start playing the selected chapter
                if let Some(audio_player) = &self.audio_player {
                    if let Some(audio_file) =
                        self.current_audio_files.get(self.selected_chapter_index)
                    {
                        if let Err(e) = audio_player.load_file(audio_file.clone()).await {
                            self.error_message = Some(format!("Failed to load audio file: {}", e));
                        } else {
                            if let Err(e) = audio_player.play().await {
                                self.error_message =
                                    Some(format!("Failed to start playback: {}", e));
                            }
                        }
                    }
                }
            }
            _ => {}
        }
    }

    async fn toggle_playback(&mut self) {
        if self.books.is_empty() {
            return;
        }

        if let Some(audio_player) = &self.audio_player {
            if let Err(e) = audio_player.toggle_playback().await {
                self.error_message = Some(format!("Failed to toggle playback: {}", e));
            }
        }
    }

    pub async fn on_tick(&mut self) {
        // Update audio position
        if let Some(audio_player) = &self.audio_player {
            let _ = audio_player.update_position().await;

            // Update UI state from audio player
            let state = audio_player.get_state().await;
            self.is_playing = state.is_playing;

            // Update progress
            if state.total_duration.as_secs() > 0 {
                self.progress =
                    state.current_position.as_secs_f64() / state.total_duration.as_secs_f64();
            }

            // Update time strings
            self.current_time = Self::format_duration(state.current_position);
            self.total_time = Self::format_duration(state.total_duration);

            // Check if current track finished
            if audio_player.is_finished().await && self.is_playing {
                // Auto-advance to next chapter
                if self.selected_chapter_index < self.current_audio_files.len().saturating_sub(1) {
                    self.selected_chapter_index += 1;
                    if let Some(audio_file) =
                        self.current_audio_files.get(self.selected_chapter_index)
                    {
                        let _ = audio_player.load_file(audio_file.clone()).await;
                        let _ = audio_player.play().await;
                    }
                } else {
                    // End of book
                    self.is_playing = false;
                }
            }
        }
    }

    fn format_duration(duration: Duration) -> String {
        let total_seconds = duration.as_secs();
        let hours = total_seconds / 3600;
        let minutes = (total_seconds % 3600) / 60;
        let seconds = total_seconds % 60;

        if hours > 0 {
            format!("{:02}:{:02}:{:02}", hours, minutes, seconds)
        } else {
            format!("{:02}:{:02}", minutes, seconds)
        }
    }

    pub fn get_current_book(&self) -> Option<&Book> {
        self.books.get(self.selected_book_index)
    }

    pub fn get_current_chapter(&self) -> Option<&str> {
        self.get_current_book()?
            .chapters
            .get(self.selected_chapter_index)
            .map(|x| x.as_str())
    }

    pub fn needs_refresh(&self) -> bool {
        self.is_loading
    }
}
