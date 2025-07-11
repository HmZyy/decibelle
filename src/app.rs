use crate::audiobook_scanner::AudiobookScanner;
use crate::models::book::Book;
use anyhow::Result;
use crossterm::event::KeyEvent;
use std::path::PathBuf;

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
        }
    }

    pub async fn initialize(&mut self) -> Result<()> {
        self.is_loading = true;
        self.error_message = None;

        let audiobook_dir = dirs::home_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join("Audiobooks");

        let scanner = AudiobookScanner::new(audiobook_dir);

        match scanner.scan_audiobooks().await {
            Ok(books) => {
                self.books = books;
                if self.books.is_empty() {
                    self.error_message =
                        Some("No audiobooks found in ~/Audiobooks directory".to_string());
                } else {
                    self.selected_book_index = 0;
                    self.selected_chapter_index = 0;
                }
            }
            Err(e) => {
                self.error_message = Some(format!("Error scanning audiobooks: {}", e));
            }
        }

        self.is_loading = false;
        Ok(())
    }

    pub fn handle_key_event(&mut self, key: KeyEvent) {
        use crossterm::event::KeyCode;

        // Don't handle input while loading
        if self.is_loading {
            return;
        }

        match key.code {
            KeyCode::Char('h') => self.cycle_pane_left(),
            KeyCode::Char('l') => self.cycle_pane_right(),
            KeyCode::Char('j') => self.move_down(),
            KeyCode::Char('k') => self.move_up(),
            KeyCode::Enter => self.select_current_item(),
            KeyCode::Char(' ') => self.toggle_playback(),
            KeyCode::Char('r') => {
                // Refresh/reload audiobooks
                self.is_loading = true;
                // This will be handled in the main loop
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

    fn move_down(&mut self) {
        if self.books.is_empty() {
            return;
        }

        match self.focused_pane {
            FocusedPane::BookList => {
                if self.selected_book_index < self.books.len().saturating_sub(1) {
                    self.selected_book_index += 1;
                    self.selected_chapter_index = 0; // Reset chapter selection
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

    fn move_up(&mut self) {
        if self.books.is_empty() {
            return;
        }

        match self.focused_pane {
            FocusedPane::BookList => {
                if self.selected_book_index > 0 {
                    self.selected_book_index -= 1;
                    self.selected_chapter_index = 0; // Reset chapter selection
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

    fn select_current_item(&mut self) {
        if self.books.is_empty() {
            return;
        }

        match self.focused_pane {
            FocusedPane::BookList => {
                // Move focus to chapter list when a book is selected
                self.focused_pane = FocusedPane::ChapterList;
                self.update_current_side();
            }
            FocusedPane::ChapterList => {
                // Start playing the selected chapter
                self.is_playing = true;
                // TODO: Implement actual audio playback
            }
            _ => {}
        }
    }

    fn toggle_playback(&mut self) {
        if self.books.is_empty() {
            return;
        }

        self.is_playing = !self.is_playing;
        // TODO: Implement actual audio playback control
    }

    pub fn on_tick(&mut self) {
        // Update progress and time if playing
        if self.is_playing {
            self.progress += 0.01;
            if self.progress > 1.0 {
                self.progress = 1.0;
                self.is_playing = false;
            }
            // TODO: Update current_time based on actual playback
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
