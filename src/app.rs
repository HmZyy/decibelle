use crate::models::book::Book;
use crossterm::event::KeyEvent;

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
}

impl App {
    pub fn new() -> Self {
        Self {
            should_quit: false,
            focused_pane: FocusedPane::BookList,
            current_side: Side::Left,
            books: Self::load_sample_books(),
            selected_book_index: 0,
            selected_chapter_index: 0,
            is_playing: false,
            progress: 0.0,
            current_time: "00:00".to_string(),
            total_time: "00:00".to_string(),
        }
    }

    pub fn handle_key_event(&mut self, key: KeyEvent) {
        use crossterm::event::KeyCode;

        match key.code {
            KeyCode::Char('h') => self.cycle_pane_left(),
            KeyCode::Char('l') => self.cycle_pane_right(),
            KeyCode::Char('j') => self.move_down(),
            KeyCode::Char('k') => self.move_up(),
            KeyCode::Enter => self.select_current_item(),
            KeyCode::Char(' ') => self.toggle_playback(),
            _ => {}
        }
    }

    fn cycle_pane_left(&mut self) {
        self.focused_pane = match self.focused_pane {
            FocusedPane::BookList => FocusedPane::AudioControls,
            FocusedPane::ChapterList => FocusedPane::BookList,
            FocusedPane::BookInfo => FocusedPane::ChapterList,
            FocusedPane::AudioControls => FocusedPane::BookInfo,
        };
        self.update_current_side();
    }

    fn cycle_pane_right(&mut self) {
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
            }
            FocusedPane::AudioControls => {
                // Stay in audio controls or cycle back to book info
                self.focused_pane = FocusedPane::BookInfo;
            }
        }
    }

    fn move_up(&mut self) {
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
            }
            FocusedPane::AudioControls => {
                self.focused_pane = FocusedPane::BookInfo;
            }
        }
    }

    fn select_current_item(&mut self) {
        match self.focused_pane {
            FocusedPane::BookList => {
                // Move focus to chapter list when a book is selected
                self.focused_pane = FocusedPane::ChapterList;
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

    fn load_sample_books() -> Vec<Book> {
        vec![
            Book {
                title: "The Rust Programming Language".to_string(),
                author: "Steve Klabnik & Carol Nichols".to_string(),
                description: "The official book on the Rust programming language, written by the Rust development team at Mozilla.".to_string(),
                chapters: vec![
                    "Chapter 1: Getting Started".to_string(),
                    "Chapter 2: Programming a Guessing Game".to_string(),
                    "Chapter 3: Common Programming Concepts".to_string(),
                    "Chapter 4: Understanding Ownership".to_string(),
                    "Chapter 5: Using Structs".to_string(),
                ],
                cover_path: None,
                path: "~/Audiobooks/rust-book/".to_string(),
            },
            Book {
                title: "Dune".to_string(),
                author: "Frank Herbert".to_string(),
                description: "A science fiction novel about the desert planet Arrakis and the noble family caught in a struggle for control of the most valuable substance in the universe.".to_string(),
                chapters: vec![
                    "Book One: Dune".to_string(),
                    "Book Two: Muad'Dib".to_string(),
                    "Book Three: The Prophet".to_string(),
                ],
                cover_path: None,
                path: "~/Audiobooks/dune/".to_string(),
            },
            Book {
                title: "The Hobbit".to_string(),
                author: "J.R.R. Tolkien".to_string(),
                description: "A fantasy novel about Bilbo Baggins, a hobbit who joins a company of dwarves on a quest to reclaim their homeland from the dragon Smaug.".to_string(),
                chapters: vec![
                    "An Unexpected Party".to_string(),
                    "Roast Mutton".to_string(),
                    "A Short Rest".to_string(),
                    "Over Hill and Under Hill".to_string(),
                    "Riddles in the Dark".to_string(),
                    "Out of the Frying-Pan into the Fire".to_string(),
                ],
                cover_path: None,
                path: "~/Audiobooks/hobbit/".to_string(),
            },
        ]
    }
}

