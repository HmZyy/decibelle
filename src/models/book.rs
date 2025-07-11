

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Book {
    pub title: String,
    pub author: String,
    pub description: String,
    pub chapters: Vec<String>,
    pub cover_path: Option<String>,
    pub path: String,
}

impl Book {
    pub fn new(title: String, author: String, path: String) -> Self {
        Self {
            title,
            author,
            description: String::new(),
            chapters: Vec::new(),
            cover_path: None,
            path,
        }
    }

    pub fn chapter_count(&self) -> usize {
        self.chapters.len()
    }

    pub fn has_chapters(&self) -> bool {
        !self.chapters.is_empty()
    }
}
