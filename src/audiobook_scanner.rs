use anyhow::{Context, Result};
use regex::Regex;
use serde_json::Value;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::process::Command;
use walkdir::WalkDir;

use crate::models::book::Book;

pub struct AudiobookScanner {
    audiobook_dir: PathBuf,
    audio_extensions: Vec<String>,
}

impl AudiobookScanner {
    pub fn new(audiobook_dir: PathBuf) -> Self {
        Self {
            audiobook_dir,
            audio_extensions: vec![
                "mp3".to_string(),
                "m4a".to_string(),
                "m4b".to_string(),
                "flac".to_string(),
                "ogg".to_string(),
                "wav".to_string(),
                "aac".to_string(),
            ],
        }
    }

    pub async fn scan_audiobooks(&self) -> Result<Vec<Book>> {
        if !self.audiobook_dir.exists() {
            return Ok(Vec::new());
        }

        let mut books = Vec::new();
        let mut book_dirs = HashMap::new();

        // First pass: collect all audio files and group by directory
        for entry in WalkDir::new(&self.audiobook_dir)
            .min_depth(1)
            .max_depth(3)
            .into_iter()
            .filter_map(|e| e.ok())
        {
            let path = entry.path();

            if path.is_file() {
                if let Some(ext) = path.extension() {
                    if let Some(ext_str) = ext.to_str() {
                        if self.audio_extensions.contains(&ext_str.to_lowercase()) {
                            let book_dir = self.determine_book_directory(path)?;
                            book_dirs
                                .entry(book_dir)
                                .or_insert_with(Vec::new)
                                .push(path.to_path_buf());
                        }
                    }
                }
            }
        }

        // Second pass: process each book directory
        for (book_dir, audio_files) in book_dirs {
            if let Ok(book) = self.process_book_directory(&book_dir, audio_files).await {
                books.push(book);
            }
        }

        books.sort_by(|a, b| a.title.cmp(&b.title));
        Ok(books)
    }

    fn determine_book_directory(&self, file_path: &Path) -> Result<PathBuf> {
        let parent = file_path
            .parent()
            .context("Failed to get parent directory")?;

        // If the parent is the audiobook directory itself, treat the file as a standalone book
        if parent == self.audiobook_dir {
            // For standalone files, use the file's stem as the book directory name
            // But return the actual parent directory (the audiobook root)
            return Ok(parent.to_path_buf());
        }

        // Otherwise, use the parent directory as the book directory
        Ok(parent.to_path_buf())
    }

    async fn process_book_directory(
        &self,
        book_dir: &Path,
        mut audio_files: Vec<PathBuf>,
    ) -> Result<Book> {
        // Sort audio files naturally
        audio_files.sort_by(|a, b| {
            self.natural_sort_key(a.file_name().unwrap_or_default())
                .cmp(&self.natural_sort_key(b.file_name().unwrap_or_default()))
        });

        // Determine book title from directory name or file name
        let book_title = if book_dir == self.audiobook_dir {
            // For standalone files in the root directory, use the first file's stem
            audio_files
                .first()
                .and_then(|f| f.file_stem())
                .and_then(|stem| stem.to_str())
                .unwrap_or("Unknown Book")
                .to_string()
        } else {
            // For files in subdirectories, use the directory name
            book_dir
                .file_name()
                .and_then(|name| name.to_str())
                .unwrap_or("Unknown Book")
                .to_string()
        };

        let mut book = Book::new(
            book_title,
            "Unknown Author".to_string(),
            book_dir.to_string_lossy().to_string(),
        );

        // Try to extract metadata from the first audio file
        if let Some(first_file) = audio_files.first() {
            if let Ok(metadata) = self.extract_metadata(first_file).await {
                if let Some(title) = metadata.get("title") {
                    if let Some(title_str) = title.as_str() {
                        if !title_str.is_empty() {
                            book.title = title_str.to_string();
                        }
                    }
                }
                if let Some(artist) = metadata
                    .get("artist")
                    .or_else(|| metadata.get("album_artist"))
                {
                    if let Some(artist_str) = artist.as_str() {
                        if !artist_str.is_empty() {
                            book.author = artist_str.to_string();
                        }
                    }
                }
                if let Some(album) = metadata.get("album") {
                    if let Some(album_str) = album.as_str() {
                        if !album_str.is_empty() {
                            book.title = album_str.to_string();
                        }
                    }
                }
            }
        }

        // Extract chapters from audio files
        for audio_file in &audio_files {
            if let Ok(chapters) = self.extract_chapters(audio_file).await {
                if !chapters.is_empty() {
                    book.chapters = chapters;
                    break; // Use chapters from first file that has them
                }
            }
        }

        // If no chapters found, use filenames as chapters
        if book.chapters.is_empty() {
            book.chapters = audio_files
                .iter()
                .map(|path| {
                    path.file_stem()
                        .and_then(|name| name.to_str())
                        .unwrap_or("Unknown Chapter")
                        .to_string()
                })
                .collect();
        }

        Ok(book)
    }

    async fn extract_metadata(&self, file_path: &Path) -> Result<HashMap<String, Value>> {
        let output = Command::new("ffprobe")
            .arg("-v")
            .arg("quiet")
            .arg("-print_format")
            .arg("json")
            .arg("-show_format")
            .arg(file_path)
            .output()
            .context("Failed to run ffprobe for metadata")?;

        if !output.status.success() {
            return Err(anyhow::anyhow!(
                "ffprobe failed: {}",
                String::from_utf8_lossy(&output.stderr)
            ));
        }

        let json_str =
            String::from_utf8(output.stdout).context("Failed to parse ffprobe output as UTF-8")?;

        let json: Value =
            serde_json::from_str(&json_str).context("Failed to parse ffprobe JSON output")?;

        let mut metadata = HashMap::new();

        if let Some(format) = json.get("format") {
            if let Some(tags) = format.get("tags") {
                if let Some(tags_obj) = tags.as_object() {
                    for (key, value) in tags_obj {
                        metadata.insert(key.to_lowercase(), value.clone());
                    }
                }
            }
        }

        Ok(metadata)
    }

    async fn extract_chapters(&self, file_path: &Path) -> Result<Vec<String>> {
        let output = Command::new("ffprobe")
            .arg("-v")
            .arg("quiet")
            .arg("-print_format")
            .arg("json")
            .arg("-show_chapters")
            .arg(file_path)
            .output()
            .context("Failed to run ffprobe for chapters")?;

        if !output.status.success() {
            return Ok(Vec::new()); // No chapters, not an error
        }

        let json_str =
            String::from_utf8(output.stdout).context("Failed to parse ffprobe output as UTF-8")?;

        let json: Value =
            serde_json::from_str(&json_str).context("Failed to parse ffprobe JSON output")?;

        let mut chapters = Vec::new();

        if let Some(chapters_array) = json.get("chapters") {
            if let Some(chapters_vec) = chapters_array.as_array() {
                for (i, chapter) in chapters_vec.iter().enumerate() {
                    let chapter_title = chapter
                        .get("tags")
                        .and_then(|tags| tags.get("title"))
                        .and_then(|title| title.as_str())
                        .map(|s| s.to_string())
                        .unwrap_or_else(|| format!("Chapter {}", i + 1));

                    chapters.push(chapter_title);
                }
            }
        }

        Ok(chapters)
    }

    fn natural_sort_key(&self, name: &std::ffi::OsStr) -> String {
        let name_str = name.to_string_lossy();

        // Extract numbers and pad them for natural sorting
        let re = Regex::new(r"\d+").unwrap();
        let mut result = String::new();
        let mut last_end = 0;

        for mat in re.find_iter(&name_str) {
            // Add the text before the number
            result.push_str(&name_str[last_end..mat.start()]);

            // Add the number, zero-padded to 10 digits
            let num: u64 = mat.as_str().parse().unwrap_or(0);
            result.push_str(&format!("{:010}", num));

            last_end = mat.end();
        }

        // Add any remaining text
        result.push_str(&name_str[last_end..]);

        result
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    #[tokio::test]
    async fn test_scanner_empty_directory() {
        let temp_dir = TempDir::new().unwrap();
        let scanner = AudiobookScanner::new(temp_dir.path().to_path_buf());

        let books = scanner.scan_audiobooks().await.unwrap();
        assert!(books.is_empty());
    }

    #[test]
    fn test_natural_sort_key() {
        let scanner = AudiobookScanner::new(PathBuf::from("."));

        let mut files = vec![
            std::ffi::OsStr::new("chapter1.mp3"),
            std::ffi::OsStr::new("chapter10.mp3"),
            std::ffi::OsStr::new("chapter2.mp3"),
        ];

        files.sort_by(|a, b| {
            scanner
                .natural_sort_key(a)
                .cmp(&scanner.natural_sort_key(b))
        });

        assert_eq!(files[0], "chapter1.mp3");
        assert_eq!(files[1], "chapter2.mp3");
        assert_eq!(files[2], "chapter10.mp3");
    }
}
