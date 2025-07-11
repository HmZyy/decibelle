use anyhow::{Context, Result};
use rodio::{Decoder, OutputStream, OutputStreamHandle, Sink, Source};
use std::fs::File;
use std::io::{Cursor, Read};
use std::path::PathBuf;
use std::process::Command;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::{Mutex, RwLock};

use super::types::*;

pub struct AudioPlayer {
    sink: Arc<Mutex<Option<Sink>>>,
    _stream: OutputStream,
    stream_handle: OutputStreamHandle,
    state: Arc<RwLock<PlaybackState>>,
    current_file_path: Arc<Mutex<Option<PathBuf>>>,
    audio_data: Arc<Mutex<Option<Vec<u8>>>>,
}

impl AudioPlayer {
    pub fn new() -> Result<Self> {
        let (stream, stream_handle) =
            OutputStream::try_default().context("Failed to create audio output stream")?;

        Ok(Self {
            sink: Arc::new(Mutex::new(None)),
            _stream: stream,
            stream_handle,
            state: Arc::new(RwLock::new(PlaybackState::default())),
            current_file_path: Arc::new(Mutex::new(None)),
            audio_data: Arc::new(Mutex::new(None)),
        })
    }

    pub async fn load_file(&self, path: PathBuf, mut logger: impl FnMut(&str, &str)) -> Result<()> {
        logger("INFO", &format!("Loading file: {}", path.display()));

        // First, check if file exists
        if !path.exists() {
            let error = format!("File does not exist: {}", path.display());
            logger("ERROR", &error);
            return Err(anyhow::anyhow!(error));
        }

        {
            let mut file_path = self.current_file_path.lock().await;
            *file_path = Some(path.clone());
        }

        let chapters = self.extract_chapters(&path, &mut logger).await?;

        logger("INFO", "FFmpeg conversion");
        let result = self.load_file_with_ffmpeg(path, logger).await;

        if result.is_ok() {
            // Update state with chapters
            let mut state = self.state.write().await;
            state.chapters = chapters.clone();
            state.current_chapter = if chapters.is_empty() { None } else { Some(0) };
        }

        result
    }

    async fn extract_chapters(
        &self,
        path: &PathBuf,
        logger: &mut impl FnMut(&str, &str),
    ) -> Result<Vec<Chapter>> {
        logger("DEBUG", "Extracting chapters from file");

        let output = Command::new("ffprobe")
            .arg("-v")
            .arg("quiet")
            .arg("-print_format")
            .arg("json")
            .arg("-show_chapters")
            .arg(path)
            .output()
            .context("Failed to run ffprobe for chapters")?;

        if !output.status.success() {
            logger("DEBUG", "No chapters found in file");
            return Ok(Vec::new());
        }

        let json_str =
            String::from_utf8(output.stdout).context("Failed to parse ffprobe output as UTF-8")?;

        let json: serde_json::Value =
            serde_json::from_str(&json_str).context("Failed to parse ffprobe JSON output")?;

        let mut chapters = Vec::new();

        if let Some(chapters_array) = json.get("chapters") {
            if let Some(chapters_vec) = chapters_array.as_array() {
                for (i, chapter) in chapters_vec.iter().enumerate() {
                    let title = chapter
                        .get("tags")
                        .and_then(|tags| tags.get("title"))
                        .and_then(|title| title.as_str())
                        .map(|s| s.to_string())
                        .unwrap_or_else(|| format!("Chapter {}", i + 1));

                    let start_time = chapter
                        .get("start_time")
                        .and_then(|t| t.as_str())
                        .and_then(|t| t.parse::<f64>().ok())
                        .map(|t| Duration::from_secs_f64(t))
                        .unwrap_or_else(|| Duration::from_secs(0));

                    let end_time = chapter
                        .get("end_time")
                        .and_then(|t| t.as_str())
                        .and_then(|t| t.parse::<f64>().ok())
                        .map(|t| Duration::from_secs_f64(t))
                        .unwrap_or_else(|| Duration::from_secs(0));

                    chapters.push(Chapter {
                        title,
                        start_time,
                        end_time,
                    });
                }
            }
        }

        logger("DEBUG", &format!("Found {} chapters", chapters.len()));
        Ok(chapters)
    }

    pub async fn seek_to_chapter(&self, chapter_index: usize) -> Result<()> {
        let chapters = {
            let state = self.state.read().await;
            state.chapters.clone()
        };

        if let Some(chapter) = chapters.get(chapter_index) {
            // Actually seek to the chapter position
            self.seek_to_position(chapter.start_time).await?;

            // Update current chapter
            {
                let mut state = self.state.write().await;
                state.current_chapter = Some(chapter_index);
            }
        }

        Ok(())
    }

    pub async fn seek_to_position(&self, position: Duration) -> Result<()> {
        // Get the current file path
        let file_path = {
            let path = self.current_file_path.lock().await;
            path.clone()
        };

        if let Some(_path) = file_path {
            // Stop current playback
            {
                let sink = self.sink.lock().await;
                if let Some(ref s) = *sink {
                    s.stop();
                }
            }

            // Get audio data (either from cache or by re-converting)
            let audio_data = {
                let data = self.audio_data.lock().await;
                data.clone()
            };

            if let Some(data) = audio_data {
                // Create a new sink and decoder from the cached audio data
                let cursor = Cursor::new(data);
                let source = Decoder::new(cursor).context("Failed to decode audio data")?;

                // Calculate how many samples to skip based on the position
                let sample_rate = source.sample_rate();
                let channels = source.channels();
                let _samples_to_skip =
                    (position.as_secs_f64() * sample_rate as f64 * channels as f64) as usize;

                // Skip to the desired position
                let source_at_position = source.skip_duration(position);

                let new_sink =
                    Sink::try_new(&self.stream_handle).context("Failed to create audio sink")?;
                new_sink.append(source_at_position);

                // Update state
                {
                    let mut state = self.state.write().await;
                    state.current_position = position;
                    state.is_playing = true;
                }

                // Replace the sink
                {
                    let mut sink = self.sink.lock().await;
                    *sink = Some(new_sink);
                }
            } else {
                return Err(anyhow::anyhow!("No audio data cached for seeking"));
            }
        } else {
            return Err(anyhow::anyhow!("No file loaded"));
        }

        Ok(())
    }

    async fn load_file_with_ffmpeg(
        &self,
        path: PathBuf,
        mut logger: impl FnMut(&str, &str),
    ) -> Result<()> {
        logger("INFO", "Starting FFmpeg conversion");

        // Check if FFmpeg is available
        let ffmpeg_check = Command::new("ffmpeg").arg("-version").output();

        match ffmpeg_check {
            Ok(output) => {
                if output.status.success() {
                    logger("DEBUG", "FFmpeg is available");
                } else {
                    logger("WARN", "FFmpeg version command failed");
                }
            }
            Err(e) => {
                let error = format!("FFmpeg not found in PATH: {}", e);
                logger("ERROR", &error);
                return Err(anyhow::anyhow!(error));
            }
        }

        // First, probe the file to get information
        let probe_output = Command::new("ffprobe")
            .arg("-v")
            .arg("quiet")
            .arg("-print_format")
            .arg("json")
            .arg("-show_format")
            .arg("-show_streams")
            .arg(&path)
            .output()
            .context("Failed to run ffprobe")?;

        let mut actual_duration = Duration::from_secs(0);

        if probe_output.status.success() {
            let probe_json = String::from_utf8_lossy(&probe_output.stdout);
            logger("DEBUG", &format!("FFprobe output: {}", probe_json));

            if let Ok(json_value) = serde_json::from_str::<serde_json::Value>(&probe_json) {
                if let Some(format) = json_value.get("format") {
                    if let Some(duration_str) = format.get("duration") {
                        if let Some(duration_str) = duration_str.as_str() {
                            if let Ok(duration_f64) = duration_str.parse::<f64>() {
                                actual_duration = Duration::from_secs_f64(duration_f64);
                                logger(
                                    "DEBUG",
                                    &format!("Detected duration: {:?}", actual_duration),
                                );
                            }
                        }
                    }
                }
            }
        } else {
            let probe_error = String::from_utf8_lossy(&probe_output.stderr);
            logger("ERROR", &format!("FFprobe error: {}", probe_error));
        }

        // Convert with FFmpeg
        logger("INFO", "Running FFmpeg conversion for full file...");
        let output = Command::new("ffmpeg")
            .arg("-i")
            .arg(&path)
            .arg("-f")
            .arg("wav")
            .arg("-acodec")
            .arg("pcm_f32le")
            .arg("-ac")
            .arg("2") // stereo
            .arg("-ar")
            .arg("44100") // sample rate
            .arg("-")
            .output()
            .context("Failed to run FFmpeg - make sure it's installed and in PATH")?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            let error = format!("FFmpeg conversion failed: {}", stderr);
            logger("ERROR", &error);
            return Err(anyhow::anyhow!(error));
        }

        logger(
            "INFO",
            &format!(
                "FFmpeg conversion successful, output size: {} bytes",
                output.stdout.len()
            ),
        );

        // Cache the converted audio data
        {
            let mut audio_data = self.audio_data.lock().await;
            *audio_data = Some(output.stdout.clone());
        }

        // Create a cursor from the converted audio data
        let cursor = Cursor::new(output.stdout);

        // Decode the converted WAV data
        let source = Decoder::new(cursor).context("Failed to decode converted audio data")?;

        let total_duration = if actual_duration.as_secs() > 0 {
            actual_duration
        } else {
            source.total_duration().unwrap_or(Duration::from_secs(0))
        };

        logger(
            "DEBUG",
            &format!("FFmpeg decode - Total duration: {:?}", total_duration),
        );

        let sample_rate = source.sample_rate();
        let channels = source.channels();
        logger(
            "DEBUG",
            &format!(
                "FFmpeg decode - Sample rate: {}, Channels: {}",
                sample_rate, channels
            ),
        );

        let source_f32 = source.convert_samples::<f32>();

        let new_sink = Sink::try_new(&self.stream_handle).context("Failed to create audio sink")?;

        new_sink.append(source_f32);
        new_sink.pause();

        // Update state
        {
            let mut state = self.state.write().await;
            state.current_file = Some(path);
            state.total_duration = total_duration;
            state.current_position = Duration::from_secs(0);
            state.is_playing = false;
        }

        // Store the new sink
        {
            let mut sink = self.sink.lock().await;
            *sink = Some(new_sink);
        }

        logger("INFO", "FFmpeg load successful - full file loaded");
        Ok(())
    }

    pub async fn play(&self) -> Result<()> {
        let sink = self.sink.lock().await;
        if let Some(ref sink) = *sink {
            sink.play();
            let mut state = self.state.write().await;
            state.is_playing = true;
        } else {
            return Err(anyhow::anyhow!("No audio loaded"));
        }
        Ok(())
    }

    pub async fn pause(&self) -> Result<()> {
        let sink = self.sink.lock().await;
        if let Some(ref sink) = *sink {
            sink.pause();
            let mut state = self.state.write().await;
            state.is_playing = false;
        }
        Ok(())
    }

    pub async fn stop(&self) -> Result<()> {
        let sink = self.sink.lock().await;
        if let Some(ref sink) = *sink {
            sink.stop();
            let mut state = self.state.write().await;
            state.is_playing = false;
            state.current_position = Duration::from_secs(0);
        }
        Ok(())
    }

    pub async fn set_volume(&self, volume: f32) -> Result<()> {
        let clamped_volume = volume.clamp(0.0, 1.0);

        let sink = self.sink.lock().await;
        if let Some(ref sink) = *sink {
            sink.set_volume(clamped_volume);
            let mut state = self.state.write().await;
            state.volume = clamped_volume;
        }
        Ok(())
    }

    pub async fn set_speed(&self, speed: f32) -> Result<()> {
        let clamped_speed = speed.clamp(0.25, 4.0);

        let sink = self.sink.lock().await;
        if let Some(ref sink) = *sink {
            sink.set_speed(clamped_speed);
            let mut state = self.state.write().await;
            state.playback_speed = clamped_speed;
        }
        Ok(())
    }

    pub async fn toggle_playback(&self) -> Result<()> {
        let is_playing = {
            let state = self.state.read().await;
            state.is_playing
        };

        if is_playing {
            self.pause().await
        } else {
            self.play().await
        }
    }

    pub async fn get_state(&self) -> PlaybackState {
        self.state.read().await.clone()
    }

    pub async fn try_receive_event(&self) -> Option<AudioEvent> {
        None
    }

    pub async fn update_position(&self) -> Result<()> {
        let sink = self.sink.lock().await;
        if let Some(ref sink) = *sink {
            let mut state = self.state.write().await;
            if state.is_playing && !sink.is_paused() {
                state.current_position += Duration::from_millis(100);

                if state.current_position >= state.total_duration {
                    state.current_position = state.total_duration;
                    state.is_playing = false;
                }

                // Update current chapter based on position
                if !state.chapters.is_empty() {
                    for (i, chapter) in state.chapters.iter().enumerate() {
                        if state.current_position >= chapter.start_time
                            && state.current_position < chapter.end_time
                        {
                            state.current_chapter = Some(i);
                            break;
                        }
                    }
                }
            }
        }
        Ok(())
    }

    pub async fn is_finished(&self) -> bool {
        let state = self.state.read().await;

        // Check if we're at the end of the current chapter (for single file with chapters)
        if !state.chapters.is_empty() {
            if let Some(current_chapter_idx) = state.current_chapter {
                if let Some(chapter) = state.chapters.get(current_chapter_idx) {
                    // Consider finished if we're at the end of the current chapter
                    if state.current_position >= chapter.end_time {
                        return true;
                    }
                }
            }
        }

        // Otherwise check if sink is empty (for multi-file books)
        let sink = self.sink.lock().await;
        if let Some(ref sink) = *sink {
            sink.empty()
        } else {
            true
        }
    }
}

