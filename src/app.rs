use crate::audio::AudioPlayer;
use crate::audiobook_scanner::AudiobookScanner;
use crate::models::book::Book;
use anyhow::Result;
use crossterm::event::KeyEvent;
use regex::Regex;
use std::collections::VecDeque;
use std::path::PathBuf;
use std::time::Duration;
use walkdir::WalkDir;

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum FocusedPane {
    BookList,
    ChapterList,
    BookInfo,
    AudioControls,
    Console,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Side {
    Left,
    Right,
    Bottom,
}

#[derive(Debug, Clone)]
pub struct ConsoleMessage {
    pub timestamp: String,
    pub level: String,
    pub message: String,
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
    pub console_messages: VecDeque<ConsoleMessage>,
    pub console_scroll_offset: usize,
    pub console_viewport_height: usize,
    pub audiobook_directory: PathBuf,
}

impl App {
    pub fn new() -> Self {
        let audiobook_dir = dirs::home_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join("Audiobooks");

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
            console_messages: VecDeque::new(),
            console_scroll_offset: 0,
            console_viewport_height: 10,
            audiobook_directory: audiobook_dir,
        }
    }

    pub fn log_message(&mut self, level: &str, message: &str) {
        let timestamp = chrono::Local::now().format("%H:%M:%S").to_string();
        let console_msg = ConsoleMessage {
            timestamp,
            level: level.to_string(),
            message: message.to_string(),
        };

        self.console_messages.push_back(console_msg);

        // Keep only last 1000 messages
        if self.console_messages.len() > 1000 {
            self.console_messages.pop_front();
            // Adjust scroll offset if needed
            if self.console_scroll_offset > 0 {
                self.console_scroll_offset -= 1;
            }
        }

        // Auto-scroll to bottom when new message arrives (if we're already at the bottom)
        let total_messages = self.console_messages.len();
        if self.console_scroll_offset + self.console_viewport_height
            >= total_messages.saturating_sub(1)
        {
            self.console_scroll_offset =
                total_messages.saturating_sub(self.console_viewport_height);
        }
    }

    pub fn scroll_console_up(&mut self) {
        if self.console_scroll_offset > 0 {
            self.console_scroll_offset -= 1;
        }
    }

    pub fn scroll_console_down(&mut self) {
        let max_offset = self
            .console_messages
            .len()
            .saturating_sub(self.console_viewport_height);
        if self.console_scroll_offset < max_offset {
            self.console_scroll_offset += 1;
        }
    }

    pub fn scroll_console_page_up(&mut self) {
        self.console_scroll_offset = self
            .console_scroll_offset
            .saturating_sub(self.console_viewport_height);
    }

    pub fn scroll_console_page_down(&mut self) {
        let max_offset = self
            .console_messages
            .len()
            .saturating_sub(self.console_viewport_height);
        self.console_scroll_offset =
            (self.console_scroll_offset + self.console_viewport_height).min(max_offset);
    }

    pub fn scroll_console_to_top(&mut self) {
        self.console_scroll_offset = 0;
    }

    pub fn scroll_console_to_bottom(&mut self) {
        let max_offset = self
            .console_messages
            .len()
            .saturating_sub(self.console_viewport_height);
        self.console_scroll_offset = max_offset;
    }

    pub async fn initialize(&mut self) -> Result<()> {
        self.is_loading = true;
        self.error_message = None;
        self.log_message("INFO", "Initializing Decibelle...");

        // Check if audiobook directory exists
        if !self.audiobook_directory.exists() {
            self.log_message(
                "WARN",
                &format!(
                    "Audiobook directory does not exist: {}",
                    self.audiobook_directory.display()
                ),
            );
            self.log_message("INFO", "Creating audiobook directory...");

            if let Err(e) = std::fs::create_dir_all(&self.audiobook_directory) {
                let error_msg = format!("Failed to create audiobook directory: {}", e);
                self.log_message("ERROR", &error_msg);
                self.error_message = Some(error_msg);
                self.is_loading = false;
                return Ok(());
            }
        }

        // Initialize audio player
        self.log_message("INFO", "Initializing audio player...");
        match AudioPlayer::new() {
            Ok(player) => {
                self.audio_player = Some(player);
                self.log_message("INFO", "Audio player initialized successfully");
            }
            Err(e) => {
                let error_msg = format!("Failed to initialize audio player: {}", e);
                self.log_message("ERROR", &error_msg);
                self.error_message = Some(error_msg);
            }
        }

        self.log_message(
            "INFO",
            &format!("Scanning directory: {}", self.audiobook_directory.display()),
        );

        let scanner = AudiobookScanner::new(self.audiobook_directory.clone());

        match scanner.scan_audiobooks().await {
            Ok(books) => {
                self.log_message("INFO", &format!("Found {} audiobooks", books.len()));
                self.books = books;

                if self.books.is_empty() {
                    let error_msg = format!(
                        "No audiobooks found in {} directory",
                        self.audiobook_directory.display()
                    );
                    self.log_message("WARN", &error_msg);
                    self.error_message = Some(error_msg);
                } else {
                    self.selected_book_index = 0;
                    self.selected_chapter_index = 0;
                    // Load audio files for the first book
                    self.load_book_audio_files().await;
                }
            }
            Err(e) => {
                let error_msg = format!("Error scanning audiobooks: {}", e);
                self.log_message("ERROR", &error_msg);
                self.error_message = Some(error_msg);
            }
        }

        self.is_loading = false;
        self.log_message("INFO", "Initialization complete");
        Ok(())
    }

    async fn load_book_audio_files(&mut self) {
        if let Some(book) = self.books.get(self.selected_book_index) {
            let book_title = book.title.clone();
            let book_path = PathBuf::from(&book.path);

            self.log_message("INFO", &format!("Loading audio files for: {}", book_title));
            self.log_message("DEBUG", &format!("Scanning path: {}", book_path.display()));

            // Check if the path exists
            if !book_path.exists() {
                self.log_message(
                    "ERROR",
                    &format!("Book path does not exist: {}", book_path.display()),
                );

                // If the exact path doesn't exist, try to find it by searching for the book title
                self.log_message("INFO", "Attempting to find book directory by title...");
                if let Some(found_path) = self.find_book_directory_by_title(&book_title).await {
                    self.log_message("INFO", &format!("Found book at: {}", found_path.display()));
                    // Update the book path and continue
                    if let Some(book_mut) = self.books.get_mut(self.selected_book_index) {
                        book_mut.path = found_path.to_string_lossy().to_string();
                    }
                    self.load_book_audio_files_from_path(&found_path, &book_title)
                        .await;
                } else {
                    self.log_message("ERROR", "Could not find book directory");
                }
                return;
            }

            self.load_book_audio_files_from_path(&book_path, &book_title)
                .await;
        }
    }

    async fn find_book_directory_by_title(&self, title: &str) -> Option<PathBuf> {
        // Search for a directory or files that match the book title
        for entry in WalkDir::new(&self.audiobook_directory)
            .max_depth(2)
            .into_iter()
            .filter_map(|e| e.ok())
        {
            let path = entry.path();

            // Check if it's a directory that matches the title
            if path.is_dir() {
                if let Some(dir_name) = path.file_name().and_then(|n| n.to_str()) {
                    if dir_name.contains(title) || title.contains(dir_name) {
                        return Some(path.to_path_buf());
                    }
                }
            }

            // Check if it's an audio file that matches the title
            if path.is_file() {
                if let Some(ext) = path.extension() {
                    if let Some(ext_str) = ext.to_str() {
                        let ext_lower = ext_str.to_lowercase();
                        if ["mp3", "m4a", "m4b", "flac", "ogg", "wav", "aac"]
                            .contains(&ext_lower.as_str())
                        {
                            if let Some(file_stem) = path.file_stem().and_then(|s| s.to_str()) {
                                if file_stem.contains(title) || title.contains(file_stem) {
                                    return path.parent().map(|p| p.to_path_buf());
                                }
                            }
                        }
                    }
                }
            }
        }
        None
    }

    async fn load_book_audio_files_from_path(&mut self, book_path: &PathBuf, book_title: &str) {
        self.log_message(
            "DEBUG",
            &format!("Loading audio files from path: {}", book_path.display()),
        );

        // Clear previous audio files
        self.current_audio_files.clear();

        // Find all audio files in the book directory (including subdirectories)
        let mut audio_files = Vec::new();
        let supported_extensions = vec!["mp3", "m4a", "m4b", "flac", "ogg", "wav", "aac"];

        // Special case: if the book path is the audiobook root directory,
        // look for files that match the book title
        if book_path == &self.audiobook_directory {
            self.log_message(
                "DEBUG",
                "Book path is audiobook root, searching for title-specific files",
            );

            // Look for files that contain the book title
            for entry in WalkDir::new(book_path)
                .max_depth(2)
                .into_iter()
                .filter_map(|e| e.ok())
                .filter(|e| e.path().is_file())
            {
                let path = entry.path();
                if let Some(ext) = path.extension() {
                    if let Some(ext_str) = ext.to_str() {
                        let ext_lower = ext_str.to_lowercase();
                        if supported_extensions.contains(&ext_lower.as_str()) {
                            if let Some(file_stem) = path.file_stem().and_then(|s| s.to_str()) {
                                // Check if the file name contains part of the book title
                                let title_words: Vec<&str> =
                                    book_title.split_whitespace().collect();
                                if title_words.iter().any(|word| {
                                    file_stem.to_lowercase().contains(&word.to_lowercase())
                                }) {
                                    audio_files.push(path.to_path_buf());
                                }
                            }
                        }
                    }
                }
            }
        } else {
            // Normal case: use WalkDir to recursively search for audio files
            let walkdir_results = WalkDir::new(book_path)
                .into_iter()
                .filter_map(|e| e.ok())
                .filter(|e| e.path().is_file())
                .filter_map(|e| {
                    let path = e.path();
                    if let Some(ext) = path.extension() {
                        if let Some(ext_str) = ext.to_str() {
                            let ext_lower = ext_str.to_lowercase();
                            if supported_extensions.contains(&ext_lower.as_str()) {
                                return Some(path.to_path_buf());
                            }
                        }
                    }
                    None
                })
                .collect::<Vec<PathBuf>>();

            audio_files.extend(walkdir_results);
        }

        // If no files found with WalkDir, try manual directory traversal
        if audio_files.is_empty() {
            self.log_message(
                "WARN",
                "No audio files found with WalkDir, trying manual search",
            );
            self.manual_search_audio_files(book_path, &supported_extensions, &mut audio_files);
        }

        // Sort files naturally (accounting for numbers in filenames)
        audio_files.sort_by(|a, b| {
            let a_name = a.file_name().unwrap_or_default().to_string_lossy();
            let b_name = b.file_name().unwrap_or_default().to_string_lossy();
            self.natural_sort(&a_name, &b_name)
        });

        self.current_audio_files = audio_files;
        self.log_message(
            "INFO",
            &format!("Loaded {} audio files", self.current_audio_files.len()),
        );

        // Debug: Compare chapters vs audio files
        if let Some(book) = self.books.get(self.selected_book_index) {
            self.log_message(
                "DEBUG",
                &format!(
                    "Book has {} chapters, found {} audio files",
                    book.chapters.len(),
                    self.current_audio_files.len()
                ),
            );
        }

        // If still no files found, check if the path is actually a file
        if self.current_audio_files.is_empty() && book_path.is_file() {
            self.log_message(
                "INFO",
                "Book path is a single file, checking if it's an audio file",
            );
            if let Some(ext) = book_path.extension() {
                if let Some(ext_str) = ext.to_str() {
                    let ext_lower = ext_str.to_lowercase();
                    if supported_extensions.contains(&ext_lower.as_str()) {
                        self.current_audio_files.push(book_path.clone());
                        self.log_message("INFO", "Added single audio file");
                    }
                }
            }
        }
    }

    fn manual_search_audio_files(
        &mut self,
        path: &PathBuf,
        supported_extensions: &[&str],
        audio_files: &mut Vec<PathBuf>,
    ) {
        self.log_message("DEBUG", &format!("Manual search in: {}", path.display()));

        match std::fs::read_dir(path) {
            Ok(entries) => {
                for entry in entries.flatten() {
                    let entry_path = entry.path();

                    if entry_path.is_file() {
                        if let Some(ext) = entry_path.extension() {
                            if let Some(ext_str) = ext.to_str() {
                                let ext_lower = ext_str.to_lowercase();
                                if supported_extensions.contains(&ext_lower.as_str()) {
                                    audio_files.push(entry_path.clone());
                                    self.log_message(
                                        "DEBUG",
                                        &format!(
                                            "Found audio file (manual): {}",
                                            entry_path.display()
                                        ),
                                    );
                                }
                            }
                        }
                    } else if entry_path.is_dir() {
                        // Recursively search subdirectories (but limit depth to avoid infinite loops)
                        self.manual_search_audio_files(
                            &entry_path,
                            supported_extensions,
                            audio_files,
                        );
                    }
                }
            }
            Err(e) => {
                self.log_message(
                    "ERROR",
                    &format!("Failed to read directory {}: {}", path.display(), e),
                );
            }
        }
    }

    fn natural_sort(&self, a: &str, b: &str) -> std::cmp::Ordering {
        // Create regex to find numbers in the string
        let re = Regex::new(r"\d+").unwrap();

        // Split both strings into parts (text and numbers)
        let mut a_parts = Vec::new();
        let mut b_parts = Vec::new();

        let mut last_end = 0;
        for mat in re.find_iter(a) {
            // Add text before number
            if mat.start() > last_end {
                a_parts.push((&a[last_end..mat.start()], None));
            }
            // Add number
            let num: u64 = mat.as_str().parse().unwrap_or(0);
            a_parts.push((mat.as_str(), Some(num)));
            last_end = mat.end();
        }
        // Add remaining text
        if last_end < a.len() {
            a_parts.push((&a[last_end..], None));
        }

        last_end = 0;
        for mat in re.find_iter(b) {
            // Add text before number
            if mat.start() > last_end {
                b_parts.push((&b[last_end..mat.start()], None));
            }
            // Add number
            let num: u64 = mat.as_str().parse().unwrap_or(0);
            b_parts.push((mat.as_str(), Some(num)));
            last_end = mat.end();
        }
        // Add remaining text
        if last_end < b.len() {
            b_parts.push((&b[last_end..], None));
        }

        // Compare parts
        for (a_part, b_part) in a_parts.iter().zip(b_parts.iter()) {
            match (a_part.1, b_part.1) {
                (Some(a_num), Some(b_num)) => {
                    let cmp = a_num.cmp(&b_num);
                    if cmp != std::cmp::Ordering::Equal {
                        return cmp;
                    }
                }
                (None, None) => {
                    let cmp = a_part.0.cmp(b_part.0);
                    if cmp != std::cmp::Ordering::Equal {
                        return cmp;
                    }
                }
                (Some(_), None) => return std::cmp::Ordering::Less,
                (None, Some(_)) => return std::cmp::Ordering::Greater,
            }
        }

        a_parts.len().cmp(&b_parts.len())
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
                self.log_message("INFO", "Refreshing audiobook library...");
                // This will be handled in the main loop
            }
            KeyCode::Char('c') => {
                if self.focused_pane == FocusedPane::Console {
                    // Clear console
                    self.console_messages.clear();
                    self.console_scroll_offset = 0;
                    self.log_message("INFO", "Console cleared");
                } else {
                    // Focus console
                    self.focused_pane = FocusedPane::Console;
                    self.current_side = Side::Bottom;
                    self.scroll_console_to_bottom();
                }
            }
            KeyCode::Char('+') | KeyCode::Char('=') => {
                self.adjust_volume(0.1).await;
            }
            KeyCode::Char('-') => {
                self.adjust_volume(-0.1).await;
            }
            KeyCode::Char('s') => {
                // Stop playback
                self.stop_playback().await;
            }
            KeyCode::Char('n') => {
                // Next chapter
                self.next_chapter().await;
            }
            KeyCode::Char('p') => {
                // Previous chapter
                self.previous_chapter().await;
            }
            KeyCode::Char('f') => {
                // Seek forward 30 seconds
                self.seek_relative(30.0).await;
            }
            KeyCode::Char('b') => {
                // Seek backward 30 seconds
                self.seek_relative(-30.0).await;
            }
            KeyCode::Char('g') => {
                if self.focused_pane == FocusedPane::Console {
                    self.scroll_console_to_top();
                }
            }
            KeyCode::Char('G') => {
                if self.focused_pane == FocusedPane::Console {
                    self.scroll_console_to_bottom();
                }
            }
            KeyCode::PageUp => {
                if self.focused_pane == FocusedPane::Console {
                    self.scroll_console_page_up();
                }
            }
            KeyCode::PageDown => {
                if self.focused_pane == FocusedPane::Console {
                    self.scroll_console_page_down();
                }
            }
            KeyCode::Esc => {
                if self.focused_pane == FocusedPane::Console {
                    // Exit console and go back to previous pane
                    self.focused_pane = FocusedPane::BookList;
                    self.current_side = Side::Left;
                }
            }
            _ => {}
        }
    }

    async fn adjust_volume(&mut self, delta: f32) {
        if let Some(audio_player) = &self.audio_player {
            let current_state = audio_player.get_state().await;
            let new_volume = (current_state.volume + delta).clamp(0.0, 1.0);

            if let Err(e) = audio_player.set_volume(new_volume).await {
                self.log_message("ERROR", &format!("Failed to set volume: {}", e));
            } else {
                self.log_message("INFO", &format!("Volume set to {:.1}", new_volume));
            }
        }
    }

    async fn stop_playback(&mut self) {
        if let Some(audio_player) = &self.audio_player {
            if let Err(e) = audio_player.stop().await {
                self.log_message("ERROR", &format!("Failed to stop playback: {}", e));
            } else {
                self.log_message("INFO", "Playback stopped");
            }
        }
    }

    async fn next_chapter(&mut self) {
        let max_chapters = if let Some(book) = self.books.get(self.selected_book_index) {
            std::cmp::max(book.chapters.len(), self.current_audio_files.len())
        } else {
            self.current_audio_files.len()
        };

        if self.selected_chapter_index < max_chapters.saturating_sub(1) {
            self.selected_chapter_index += 1;
            self.log_message(
                "INFO",
                &format!(
                    "Moving to next chapter: {}",
                    self.selected_chapter_index + 1
                ),
            );
            self.load_selected_chapter().await;
        } else {
            self.log_message("INFO", "Already at the last chapter");
        }
    }

    async fn previous_chapter(&mut self) {
        if self.selected_chapter_index > 0 {
            self.selected_chapter_index -= 1;
            self.log_message(
                "INFO",
                &format!(
                    "Moving to previous chapter: {}",
                    self.selected_chapter_index + 1
                ),
            );
            self.load_selected_chapter().await;
        } else {
            self.log_message("INFO", "Already at the first chapter");
        }
    }

    async fn seek_relative(&mut self, seconds: f32) {
        if let Some(audio_player) = &self.audio_player {
            let current_state = audio_player.get_state().await;
            let _new_position = if seconds > 0.0 {
                current_state.current_position + Duration::from_secs_f32(seconds)
            } else {
                current_state
                    .current_position
                    .saturating_sub(Duration::from_secs_f32(seconds.abs()))
            };

            // Note: This would require implementing seek functionality in the audio player
            self.log_message(
                "INFO",
                &format!("Seeking {} seconds (seek not yet implemented)", seconds),
            );
        }
    }

    async fn load_selected_chapter(&mut self) {
        // Check if this is a single file with embedded chapters
        if self.current_audio_files.len() == 1 {
            // Single file with embedded chapters
            if let Some(audio_player) = &self.audio_player {
                let _chapter_num = self.selected_chapter_index + 1;

                if let Err(e) = audio_player
                    .seek_to_chapter(self.selected_chapter_index)
                    .await
                {
                    self.log_message("ERROR", &format!("Failed to seek to chapter: {}", e));
                } else {
                    // Auto-play after seeking to chapter
                    if let Err(e) = audio_player.play().await {
                        self.log_message("ERROR", &format!("Failed to start playback: {}", e));
                    }
                }
            }
        } else {
            // Multiple files - existing behavior
            if let Some(audio_file) = self
                .current_audio_files
                .get(self.selected_chapter_index)
                .cloned()
            {
                let chapter_num = self.selected_chapter_index + 1;
                self.log_message("INFO", &format!("Loading chapter {}", chapter_num));
                self.load_and_play_file(audio_file).await;
            }
        }
    }

    fn cycle_pane_left(&mut self) {
        if self.books.is_empty() {
            return;
        }

        self.focused_pane = match self.focused_pane {
            FocusedPane::BookList => FocusedPane::Console,
            FocusedPane::ChapterList => FocusedPane::BookList,
            FocusedPane::BookInfo => FocusedPane::ChapterList,
            FocusedPane::AudioControls => FocusedPane::BookInfo,
            FocusedPane::Console => FocusedPane::AudioControls,
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
            FocusedPane::AudioControls => FocusedPane::Console,
            FocusedPane::Console => FocusedPane::BookList,
        };
        self.update_current_side();
    }

    fn update_current_side(&mut self) {
        self.current_side = match self.focused_pane {
            FocusedPane::BookList | FocusedPane::ChapterList => Side::Left,
            FocusedPane::BookInfo | FocusedPane::AudioControls => Side::Right,
            FocusedPane::Console => Side::Bottom,
        };
    }

    async fn move_down(&mut self) {
        if self.books.is_empty() && self.focused_pane != FocusedPane::Console {
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
                let max_chapters = if let Some(book) = self.books.get(self.selected_book_index) {
                    // Use the larger of the two: book chapters or actual audio files
                    std::cmp::max(book.chapters.len(), self.current_audio_files.len())
                } else {
                    self.current_audio_files.len()
                };

                if self.selected_chapter_index < max_chapters.saturating_sub(1) {
                    self.selected_chapter_index += 1;
                    self.log_message(
                        "DEBUG",
                        &format!(
                            "Chapter index now: {} / {}",
                            self.selected_chapter_index + 1,
                            max_chapters
                        ),
                    );
                } else {
                    self.log_message(
                        "DEBUG",
                        &format!(
                            "Already at last chapter: {} / {}",
                            self.selected_chapter_index + 1,
                            max_chapters
                        ),
                    );
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
            FocusedPane::Console => {
                self.scroll_console_down();
            }
        }
    }

    async fn move_up(&mut self) {
        if self.books.is_empty() && self.focused_pane != FocusedPane::Console {
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
                    self.log_message(
                        "DEBUG",
                        &format!(
                            "Chapter index now: {} / {}",
                            self.selected_chapter_index + 1,
                            if let Some(book) = self.books.get(self.selected_book_index) {
                                std::cmp::max(book.chapters.len(), self.current_audio_files.len())
                            } else {
                                self.current_audio_files.len()
                            }
                        ),
                    );
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
            FocusedPane::Console => {
                self.scroll_console_up();
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
                self.load_selected_chapter().await;
            }
            FocusedPane::Console => {
                // Maybe implement copying selected log line to clipboard in the future
            }
            _ => {}
        }
    }

    async fn load_and_play_file(&mut self, audio_file: PathBuf) {
        // Check if we have an audio player first
        if self.audio_player.is_none() {
            self.log_message("ERROR", "No audio player available");
            return;
        }

        let file_display = audio_file.display().to_string();
        self.log_message("INFO", &format!("Loading file: {}", file_display));

        // Check if file exists
        if !audio_file.exists() {
            self.log_message(
                "ERROR",
                &format!("Audio file does not exist: {}", file_display),
            );
            return;
        }

        // Create a logger closure that captures messages
        let mut log_messages = Vec::new();
        let logger = |level: &str, message: &str| {
            log_messages.push((level.to_string(), message.to_string()));
        };

        // Get a reference to the audio player and perform operations
        let load_result = {
            let audio_player = self.audio_player.as_ref().unwrap();
            audio_player.load_file(audio_file, logger).await
        };

        // Now log all the captured messages
        for (level, message) in log_messages {
            self.log_message(&level, &message);
        }

        match load_result {
            Ok(_) => {
                self.log_message("INFO", "File loaded successfully");

                // Now try to play
                let play_result = {
                    let audio_player = self.audio_player.as_ref().unwrap();
                    audio_player.play().await
                };

                match play_result {
                    Ok(_) => {
                        self.log_message("INFO", "Playback started");
                    }
                    Err(e) => {
                        let error_msg = format!("Failed to start playback: {}", e);
                        self.log_message("ERROR", &error_msg);
                        self.error_message = Some(error_msg);
                    }
                }
            }
            Err(e) => {
                let error_msg = format!("Failed to load audio file: {}", e);
                self.log_message("ERROR", &error_msg);
                self.error_message = Some(error_msg);
            }
        }
    }

    async fn toggle_playback(&mut self) {
        if self.books.is_empty() {
            return;
        }

        if self.audio_player.is_none() {
            self.log_message("ERROR", "No audio player available");
            return;
        }

        let toggle_result = {
            let audio_player = self.audio_player.as_ref().unwrap();
            audio_player.toggle_playback().await
        };

        match toggle_result {
            Ok(_) => {
                let action = if self.is_playing { "Paused" } else { "Resumed" };
                self.log_message("INFO", &format!("Playback {}", action));
            }
            Err(e) => {
                let error_msg = format!("Failed to toggle playback: {}", e);
                self.log_message("ERROR", &error_msg);
                self.error_message = Some(error_msg);
            }
        }
    }

    pub async fn on_tick(&mut self) {
        // Update audio position
        if self.audio_player.is_none() {
            return;
        }

        // Get all the values we need from the audio player first
        let (state, is_finished) = {
            let audio_player = self.audio_player.as_ref().unwrap();
            let _ = audio_player.update_position().await;
            let state = audio_player.get_state().await;
            let is_finished = audio_player.is_finished().await;
            (state, is_finished)
        };

        // Now we can safely update self without borrowing conflicts
        self.is_playing = state.is_playing;

        // Update progress
        if state.total_duration.as_secs() > 0 {
            self.progress =
                state.current_position.as_secs_f64() / state.total_duration.as_secs_f64();
        }

        // Update time strings
        self.current_time = Self::format_duration(state.current_position);
        self.total_time = Self::format_duration(state.total_duration);

        // Update selected chapter based on current position for embedded chapters
        if self.current_audio_files.len() == 1 && !state.chapters.is_empty() {
            if let Some(current_chapter) = state.current_chapter {
                if current_chapter != self.selected_chapter_index {
                    self.selected_chapter_index = current_chapter;
                    self.log_message(
                        "DEBUG",
                        &format!("Auto-updated to chapter {}", current_chapter + 1),
                    );
                }
            }
        }

        // Check if current track/chapter finished
        if is_finished && self.is_playing {
            self.log_message("INFO", "Chapter finished");

            // For single files with embedded chapters, move to next chapter
            if self.current_audio_files.len() == 1 && !state.chapters.is_empty() {
                let max_chapters = state.chapters.len();

                if self.selected_chapter_index < max_chapters.saturating_sub(1) {
                    self.selected_chapter_index += 1;
                    self.log_message(
                        "INFO",
                        &format!(
                            "Auto-advancing to chapter {}",
                            self.selected_chapter_index + 1
                        ),
                    );
                    self.load_selected_chapter().await;
                } else {
                    self.log_message("INFO", "End of book reached");
                    self.is_playing = false;
                }
            } else {
                // Multiple files - existing behavior
                if self.selected_chapter_index < self.current_audio_files.len().saturating_sub(1) {
                    self.selected_chapter_index += 1;
                    if let Some(audio_file) = self
                        .current_audio_files
                        .get(self.selected_chapter_index)
                        .cloned()
                    {
                        let file_display = audio_file.display().to_string();
                        self.log_message(
                            "INFO",
                            &format!("Auto-loading next chapter: {}", file_display),
                        );
                        self.load_and_play_file(audio_file).await;
                    }
                } else {
                    self.log_message("INFO", "End of book reached");
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

    pub fn update_console_viewport_height(&mut self, height: usize) {
        self.console_viewport_height = height.saturating_sub(4); // Account for borders and title
                                                                 // Adjust scroll offset if needed
        let max_offset = self
            .console_messages
            .len()
            .saturating_sub(self.console_viewport_height);
        if self.console_scroll_offset > max_offset {
            self.console_scroll_offset = max_offset;
        }
    }
}

