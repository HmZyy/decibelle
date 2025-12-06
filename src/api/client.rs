use std::path::PathBuf;

use crate::api::models::{
    AudioTrack, Chapter, LibrariesResponse, Library, LibraryItem, LibraryItemsResponse,
    MediaProgress, PersonalizedShelf,
};
use crate::config::Config;
use reqwest::blocking::Client;

pub struct ApiClient {
    client: Client,
    base_url: String,
    api_key: String,
}

impl ApiClient {
    pub fn new(config: &Config) -> Self {
        Self {
            client: Client::new(),
            base_url: config.server_url.trim_end_matches('/').to_string(),
            api_key: config.api_key.clone(),
        }
    }

    pub fn get_libraries(&self) -> Result<Vec<Library>, ApiError> {
        let resp = self
            .client
            .get(format!("{}/api/libraries", self.base_url))
            .header("Authorization", format!("Bearer {}", self.api_key))
            .send()?
            .error_for_status()?;

        let wrapper: LibrariesResponse = resp.json()?;
        Ok(wrapper.libraries)
    }

    pub fn get_library_items(&self, library_id: &str) -> Result<Vec<LibraryItem>, ApiError> {
        let resp = self
            .client
            .get(format!(
                "{}/api/libraries/{}/items",
                self.base_url, library_id
            ))
            .header("Authorization", format!("Bearer {}", self.api_key))
            .send()?
            .error_for_status()?;

        let wrapper: LibraryItemsResponse = resp.json()?;
        Ok(wrapper.results)
    }

    pub fn get_library_item(&self, item_id: &str) -> Result<LibraryItem, ApiError> {
        let url = format!("{}/api/items/{}?expanded=1", self.base_url, item_id);

        let response = self.client.get(&url).bearer_auth(&self.api_key).send()?;

        if !response.status().is_success() {
            return Err(ApiError::Http(response.status().as_u16()));
        }

        let item: LibraryItem = response.json()?;
        Ok(item)
    }

    pub fn get_item_chapters(&self, item_id: &str) -> Result<Vec<Chapter>, ApiError> {
        let resp = self
            .client
            .get(format!("{}/api/items/{}", self.base_url, item_id))
            .header("Authorization", format!("Bearer {}", self.api_key))
            .send()?
            .error_for_status()?;

        let item: LibraryItem = resp.json()?;
        Ok(item
            .media
            .map(|m| m.chapters.unwrap_or_default())
            .unwrap_or_default())
    }

    pub fn download_audio(&self, item_id: &str) -> Result<PathBuf, ApiError> {
        let temp_path = PathBuf::from(format!("/tmp/decibelle_{}.audio", item_id));
        if temp_path.exists() {
            return Ok(temp_path);
        }

        let url = format!("{}/api/items/{}/play", self.base_url, item_id);

        let response = self.client
        .post(&url)
        .header("Authorization", format!("Bearer {}", self.api_key))
        .header("Content-Type", "application/json")
        .json(&serde_json::json!({
            "deviceInfo": {
                "clientName": "Decibelle",
                "clientVersion": "0.1.0"
            },
            "forceDirectPlay": true,
            "supportedMimeTypes": ["audio/flac", "audio/mpeg", "audio/mp4", "audio/ogg", "audio/aac"]
        }))
        .send()?
        .error_for_status()?;

        let session: serde_json::Value = response.json()?;
        let content_url = session["audioTracks"]
            .as_array()
            .and_then(|tracks| tracks.first())
            .and_then(|track| track["contentUrl"].as_str())
            .ok_or_else(|| "No audio tracks in playback session");

        let url = match content_url {
            Ok(url) => url,
            Err(_e) => return Err(ApiError::NotFound),
        };

        let audio_url = format!("{}{}", self.base_url, url);
        let audio_response = self
            .client
            .get(&audio_url)
            .header("Authorization", format!("Bearer {}", self.api_key))
            .send()?;

        let bytes = audio_response.bytes()?;
        let _ = std::fs::write(&temp_path, &bytes);

        Ok(temp_path)
    }

    pub fn download_track(
        &self,
        item_id: &str,
        track: &AudioTrack,
    ) -> Result<std::path::PathBuf, ApiError> {
        let url = format!("{}{}", self.base_url, track.content_url);

        let path =
            std::path::PathBuf::from(format!("/tmp/decibelle_{}_{}.audio", item_id, track.index));

        if path.exists() {
            return Ok(path);
        }

        let response = self.client.get(&url).bearer_auth(&self.api_key).send()?;

        if !response.status().is_success() {
            return Err(ApiError::Http(response.status().as_u16()));
        }

        let bytes = response.bytes()?;
        let _ = std::fs::write(&path, &bytes);

        Ok(path)
    }

    pub fn get_personalized(&self, library_id: &str) -> Result<Vec<PersonalizedShelf>, ApiError> {
        let url = format!(
            "{}/api/libraries/{}/personalized",
            self.base_url, library_id
        );

        let resp = self.client.get(&url).bearer_auth(&self.api_key).send()?;

        if !resp.status().is_success() {
            return Err(ApiError::Http(resp.status().as_u16()));
        }

        Ok(resp.json()?)
    }

    pub fn get_media_progress(&self, item_id: &str) -> Result<MediaProgress, ApiError> {
        let url = format!("{}/api/me/progress/{}", self.base_url, item_id);

        let resp = self.client.get(&url).bearer_auth(&self.api_key).send()?;

        match resp.status().as_u16() {
            200 => Ok(resp.json()?),
            404 => Err(ApiError::NotFound),
            code => Err(ApiError::Http(code)),
        }
    }

    pub fn get_continue_listening(
        &self,
        library_id: &str,
    ) -> Result<Option<(LibraryItem, f64)>, ApiError> {
        let shelves = self.get_personalized(library_id)?;

        let item = shelves
            .iter()
            .find(|s| s.id == "continue-listening")
            .and_then(|s| s.entities.iter().find(|e| e.media.is_some()).cloned());

        match item {
            Some(item) => {
                let pos = self
                    .get_media_progress(&item.id)
                    .map(|p| p.current_time)
                    .unwrap_or(0.0);
                Ok(Some((item, pos)))
            }
            None => Ok(None),
        }
    }
}

// Error type
#[derive(Debug)]
pub enum ApiError {
    Network(reqwest::Error),
    NotFound,
    Unauthorized,
    Http(u16),
}

impl From<reqwest::Error> for ApiError {
    fn from(e: reqwest::Error) -> Self {
        ApiError::Network(e)
    }
}
