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

    pub async fn load_file(&self, path: PathBuf) -> Result<()> {
        match self.load_file_with_ffmpeg(path.clone()).await {
            Ok(_) => return Ok(()),
            Err(ffmpeg_err) => {
                return Err(anyhow::anyhow!("FFmpeg conversion failed.: {}", ffmpeg_err));
            }
        }
    }

    async fn load_file_with_ffmpeg(&self, path: PathBuf) -> Result<()> {
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
            .arg("30") // Only convert first 30 seconds for testing
            .arg("-")
            .output()
            .context("Failed to run FFmpeg - make sure it's installed and in PATH")?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(anyhow::anyhow!("FFmpeg conversion failed: {}", stderr));
        }

        // Create a cursor from the converted audio data
        let cursor = Cursor::new(output.stdout);

        // Decode the converted WAV data
        let source = Decoder::new(cursor).context("Failed to decode converted audio data")?;

        let total_duration = source.total_duration().unwrap_or(Duration::from_secs(30));
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

        Ok(())
    }

    pub async fn play(&self) -> Result<()> {
        let sink = self.sink.lock().await;
        if let Some(ref sink) = *sink {
            sink.play();
            let mut state = self.state.write().await;
            state.is_playing = true;
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
        let sink = self.sink.lock().await;
        if let Some(ref sink) = *sink {
            sink.set_volume(volume.clamp(0.0, 1.0));
            let mut state = self.state.write().await;
            state.volume = volume;
        }
        Ok(())
    }

    pub async fn set_speed(&self, speed: f32) -> Result<()> {
        let sink = self.sink.lock().await;
        if let Some(ref sink) = *sink {
            sink.set_speed(speed.clamp(0.25, 4.0));
            let mut state = self.state.write().await;
            state.playback_speed = speed;
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

