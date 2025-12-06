use std::sync::mpsc::{self, Receiver, Sender, TryRecvError};
use std::thread;

use ratatui_image::{picker::Picker, protocol::StatefulProtocol};

use crate::config::Config;

pub enum CoverMessage {
    Loaded { item_id: String, data: Vec<u8> },
    Error { item_id: String, error: String },
}

pub struct CoverFetcher {
    rx: Receiver<CoverMessage>,
    tx: Sender<CoverMessage>,
    config: Config,
    client: reqwest::blocking::Client,
}

impl CoverFetcher {
    pub fn new(config: Config) -> Self {
        let (tx, rx) = mpsc::channel();
        let client = reqwest::blocking::Client::new();
        Self {
            rx,
            tx,
            config,
            client,
        }
    }

    /// Request to fetch a cover image asynchronously
    pub fn fetch(&self, item_id: String) {
        let tx = self.tx.clone();
        let config = self.config.clone();
        let client = self.client.clone();

        thread::spawn(move || {
            let cover_url = format!("{}/api/items/{}/cover", config.server_url, item_id);

            match client
                .get(&cover_url)
                .header("Authorization", format!("Bearer {}", config.api_key))
                .send()
            {
                Ok(response) => {
                    if !response.status().is_success() {
                        let _ = tx.send(CoverMessage::Error {
                            item_id,
                            error: format!("HTTP error: {}", response.status()),
                        });
                        return;
                    }

                    match response.bytes() {
                        Ok(data) => {
                            let _ = tx.send(CoverMessage::Loaded {
                                item_id,
                                data: data.to_vec(),
                            });
                        }
                        Err(e) => {
                            let _ = tx.send(CoverMessage::Error {
                                item_id,
                                error: format!("Read error: {}", e),
                            });
                        }
                    }
                }
                Err(e) => {
                    let _ = tx.send(CoverMessage::Error {
                        item_id,
                        error: format!("Fetch error: {}", e),
                    });
                }
            }
        });
    }

    /// Non-blocking check for received cover data
    pub fn try_recv(&self) -> Result<CoverMessage, TryRecvError> {
        self.rx.try_recv()
    }
}

/// Caches loaded images for rendering
pub struct ImageCache {
    pub picker: Picker,
    pub current_image: Option<Box<dyn StatefulProtocol>>,
    pub current_item_id: Option<String>,
}

impl ImageCache {
    pub fn new() -> Self {
        let mut picker = Picker::from_termios().unwrap_or_else(|_| Picker::new((8, 16)));
        picker.guess_protocol();

        Self {
            picker,
            current_image: None,
            current_item_id: None,
        }
    }

    pub fn load_cover(&mut self, item_id: &str, image_data: &[u8]) -> Result<(), String> {
        if self.current_item_id.as_deref() == Some(item_id) {
            return Ok(());
        }

        let img = image::load_from_memory(image_data)
            .map_err(|e| format!("Failed to decode image: {}", e))?;

        let protocol = self.picker.new_resize_protocol(img);
        self.current_image = Some(protocol);
        self.current_item_id = Some(item_id.to_string());

        Ok(())
    }

    pub fn clear(&mut self) {
        self.current_image = None;
        self.current_item_id = None;
    }
}

impl Default for ImageCache {
    fn default() -> Self {
        Self::new()
    }
}
