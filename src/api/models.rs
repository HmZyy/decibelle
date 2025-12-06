use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Library {
    pub id: String,
    pub name: String,
    pub media_type: String,
    pub display_order: Option<i32>,
    pub icon: Option<String>,
    pub provider: Option<String>,
    pub folders: Option<Vec<Folder>>,
    pub settings: Option<LibrarySettings>,
    pub created_at: Option<i64>,
    pub last_update: Option<i64>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LibrarySettings {
    pub cover_aspect_ratio: Option<i32>,
    pub disable_watcher: Option<bool>,
    pub skip_matching_media_with_asin: Option<bool>,
    pub skip_matching_media_with_isbn: Option<bool>,
    pub auto_scan_cron_expression: Option<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Folder {
    pub id: String,
    pub full_path: String,
    pub library_id: Option<String>,
    pub added_at: Option<i64>,
}

#[derive(Debug, Deserialize)]
pub struct LibrariesResponse {
    pub libraries: Vec<Library>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LibraryItem {
    pub id: String,
    pub library_id: String,
    pub folder_id: Option<String>,
    pub path: Option<String>,
    pub rel_path: Option<String>,
    pub is_file: Option<bool>,
    pub mtime_ms: Option<i64>,
    pub ctime_ms: Option<i64>,
    pub birthtime_ms: Option<i64>,
    pub added_at: Option<i64>,
    pub updated_at: Option<i64>,
    pub last_scan: Option<i64>,
    pub scan_version: Option<String>,
    pub is_missing: Option<bool>,
    pub is_invalid: Option<bool>,
    pub media_type: Option<String>,
    pub media: Option<Media>,
    pub library_files: Option<Vec<LibraryFile>>,
    pub size: Option<i64>,
    pub num_files: Option<i32>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Media {
    pub library_item_id: Option<String>,
    pub metadata: MediaMetadata,
    pub cover_path: Option<String>,
    pub tags: Option<Vec<String>>,
    pub audio_files: Option<Vec<AudioFile>>,
    pub chapters: Option<Vec<Chapter>>,
    pub duration: Option<f64>,
    pub size: Option<i64>,
    pub tracks: Option<Vec<AudioTrack>>,
    pub ebook_file: Option<EBookFile>,
    // Minified fields (present in list responses)
    pub num_tracks: Option<i32>,
    pub num_audio_files: Option<i32>,
    pub num_chapters: Option<i32>,
    pub ebook_file_format: Option<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MediaMetadata {
    pub title: Option<String>,
    pub title_ignore_prefix: Option<String>,
    pub subtitle: Option<String>,
    pub authors: Option<Vec<Author>>,
    pub narrators: Option<Vec<String>>,
    pub series: Option<Vec<SeriesSequence>>,
    pub genres: Option<Vec<String>>,
    pub published_year: Option<String>,
    pub published_date: Option<String>,
    pub publisher: Option<String>,
    pub description: Option<String>,
    pub isbn: Option<String>,
    pub asin: Option<String>,
    pub language: Option<String>,
    pub explicit: Option<bool>,
    pub abridged: Option<bool>,
    // Minified/expanded computed fields
    pub author_name: Option<String>,
    pub author_name_lf: Option<String>,
    pub narrator_name: Option<String>,
    pub series_name: Option<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Author {
    pub id: String,
    pub name: String,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct SeriesSequence {
    pub id: String,
    pub name: String,
    pub sequence: Option<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Chapter {
    pub id: i32,
    pub start: f64,
    pub end: f64,
    pub title: String,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AudioFile {
    pub index: Option<i32>,
    pub metadata: FileMetadata,
    pub added_at: Option<i64>,
    pub updated_at: Option<i64>,
    pub track_num_from_meta: Option<i32>,
    pub disc_num_from_meta: Option<i32>,
    pub track_num_from_filename: Option<i32>,
    pub disc_num_from_filename: Option<i32>,
    pub manually_verified: Option<bool>,
    pub exclude: Option<bool>,
    pub error: Option<String>,
    pub format: Option<String>,
    pub duration: Option<f64>,
    pub bit_rate: Option<i64>,
    pub language: Option<String>,
    pub codec: Option<String>,
    pub time_base: Option<String>,
    pub channels: Option<i32>,
    pub channel_layout: Option<String>,
    pub chapters: Option<Vec<Chapter>>,
    pub embedded_cover_art: Option<String>,
    pub meta_tags: Option<AudioMetaTags>,
    pub mime_type: Option<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AudioTrack {
    pub index: i32,
    pub start_offset: f64,
    pub duration: f64,
    pub title: String,
    pub content_url: String,
    pub mime_type: String,
    pub metadata: Option<FileMetadata>,
}

impl AudioTrack {
    pub fn end_offset(&self) -> f64 {
        self.start_offset + self.duration
    }

    pub fn contains_timestamp(&self, timestamp: f64) -> bool {
        timestamp >= self.start_offset && timestamp < self.end_offset()
    }
}

pub fn find_track_for_position(tracks: &[AudioTrack], position: f64) -> Option<&AudioTrack> {
    tracks.iter().find(|t| t.contains_timestamp(position))
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FileMetadata {
    pub filename: String,
    pub ext: String,
    pub path: String,
    pub rel_path: String,
    pub size: i64,
    pub mtime_ms: Option<i64>,
    pub ctime_ms: Option<i64>,
    pub birthtime_ms: Option<i64>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LibraryFile {
    pub metadata: FileMetadata,
    pub added_at: Option<i64>,
    pub updated_at: Option<i64>,
    pub file_type: String,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct EBookFile {
    pub metadata: FileMetadata,
    pub ebook_format: String,
    pub added_at: Option<i64>,
    pub updated_at: Option<i64>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AudioMetaTags {
    pub tag_album: Option<String>,
    pub tag_artist: Option<String>,
    pub tag_genre: Option<String>,
    pub tag_title: Option<String>,
    pub tag_track: Option<String>,
    pub tag_album_artist: Option<String>,
    pub tag_composer: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LibraryItemsResponse {
    pub results: Vec<LibraryItem>,
    pub total: Option<i32>,
    pub limit: Option<i32>,
    pub page: Option<i32>,
    pub sort_by: Option<String>,
    pub sort_desc: Option<bool>,
    pub filter_by: Option<String>,
    pub media_type: Option<String>,
    pub minified: Option<bool>,
    pub collapseseries: Option<bool>,
    pub include: Option<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PersonalizedShelf {
    pub id: String,
    pub label: String,
    pub label_string_key: Option<String>,
    #[serde(rename = "type")]
    pub shelf_type: String,
    pub entities: Vec<LibraryItem>,
    pub category: Option<String>,
    pub total: Option<i32>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MediaProgress {
    pub id: String,
    pub library_item_id: String,
    pub episode_id: Option<String>,
    pub duration: f64,
    pub progress: f64,
    pub current_time: f64,
    pub is_finished: bool,
    pub hide_from_continue_listening: Option<bool>,
    pub last_update: i64,
    pub started_at: i64,
    pub finished_at: Option<i64>,
}
