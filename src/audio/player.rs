use anyhow::{Context, Result};
use rodio::{Decoder, OutputStream, OutputStreamHandle, Sink, Source};
use std::fs::File;
use std::io::{BufReader, Cursor};
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

        // Check file size
        if let Ok(metadata) = std::fs::metadata(&path) {
            logger("DEBUG", &format!("File size: {} bytes", metadata.len()));
        }

        // Check file extension
        if let Some(ext) = path.extension() {
            logger(
                "DEBUG",
                &format!("File extension: {}", ext.to_string_lossy()),
            );
        }

        // Try direct decode first for simpler formats
        if let Ok(result) = self.try_direct_decode(&path, &mut logger).await {
            logger("INFO", "Successfully loaded file using direct decode");
            return Ok(result);
        }

        // Fall back to FFmpeg
        logger("INFO", "Direct decode failed, trying FFmpeg conversion");
        self.load_file_with_ffmpeg(path, logger).await
    }

    async fn try_direct_decode(
        &self,
        path: &PathBuf,
        logger: &mut impl FnMut(&str, &str),
    ) -> Result<()> {
        logger("DEBUG", "Attempting direct decode");

        let file =
            File::open(path).with_context(|| format!("Failed to open file: {}", path.display()))?;

        let buf_reader = BufReader::new(file);

        let decoder = Decoder::new(buf_reader)
            .with_context(|| format!("Failed to decode audio file: {}", path.display()))?;

        let total_duration = decoder.total_duration().unwrap_or(Duration::from_secs(0));
        logger(
            "DEBUG",
            &format!("Direct decode - Total duration: {:?}", total_duration),
        );

        let sample_rate = decoder.sample_rate();
        let channels = decoder.channels();
        logger(
            "DEBUG",
            &format!(
                "Direct decode - Sample rate: {}, Channels: {}",
                sample_rate, channels
            ),
        );

        let new_sink = Sink::try_new(&self.stream_handle).context("Failed to create audio sink")?;

        new_sink.append(decoder);
        new_sink.pause();

        // Update state
        {
            let mut state = self.state.write().await;
            state.current_file = Some(path.clone());
            state.total_duration = total_duration;
            state.current_position = Duration::from_secs(0);
            state.is_playing = false;
        }

        // Store the new sink
        {
            let mut sink = self.sink.lock().await;
            *sink = Some(new_sink);
        }

        logger("DEBUG", "Direct decode successful");
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

        if probe_output.status.success() {
            let probe_json = String::from_utf8_lossy(&probe_output.stdout);
            logger("DEBUG", &format!("FFprobe output: {}", probe_json));
        } else {
            let probe_error = String::from_utf8_lossy(&probe_output.stderr);
            logger("ERROR", &format!("FFprobe error: {}", probe_error));
        }

        // Convert with FFmpeg
        logger("INFO", "Running FFmpeg conversion...");
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
            .arg("-t")
            .arg("60") // Convert first 60 seconds for testing
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

        // Create a cursor from the converted audio data
        let cursor = Cursor::new(output.stdout);

        // Decode the converted WAV data
        let source = Decoder::new(cursor).context("Failed to decode converted audio data")?;

        let total_duration = source.total_duration().unwrap_or(Duration::from_secs(60));
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

        logger("INFO", "FFmpeg load successful");
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
            }
        }
        Ok(())
    }

    pub async fn is_finished(&self) -> bool {
        let sink = self.sink.lock().await;
        if let Some(ref sink) = *sink {
            sink.empty()
        } else {
            true
        }
    }
}

