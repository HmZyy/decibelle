use std::collections::VecDeque;
use std::fs::File;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{self, TryRecvError};
use std::sync::{Arc, Mutex};
use std::thread::JoinHandle;
use std::time::Duration;

use cpal::Sample;
use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use symphonia::core::audio::{SampleBuffer, SignalSpec};
use symphonia::core::codecs::{CODEC_TYPE_NULL, DecoderOptions};
use symphonia::core::errors::Error as SymphoniaError;
use symphonia::core::formats::{FormatOptions, FormatReader, SeekMode, SeekTo};
use symphonia::core::io::MediaSourceStream;
use symphonia::core::meta::MetadataOptions;
use symphonia::core::probe::Hint;
use symphonia::core::units::Time;

use crate::events::types::AppEvent;
use crate::player::commands::{PlayerCommand, PlayerState};

struct AudioOutput {
    ring_buffer: Arc<Mutex<VecDeque<f32>>>,
    spec: SignalSpec,
    _stream: cpal::Stream,
    paused: Arc<AtomicBool>,
}

impl AudioOutput {
    fn new(spec: SignalSpec) -> Result<Self, Box<dyn std::error::Error + Send + Sync>> {
        let host = cpal::default_host();
        let device = host
            .default_output_device()
            .ok_or("No output device available")?;

        let config = cpal::StreamConfig {
            channels: spec.channels.count() as u16,
            sample_rate: cpal::SampleRate(spec.rate),
            buffer_size: cpal::BufferSize::Default,
        };

        let buffer_capacity = spec.rate as usize * spec.channels.count() * 5;
        let ring_buffer = Arc::new(Mutex::new(VecDeque::with_capacity(buffer_capacity)));
        let ring_buffer_clone = ring_buffer.clone();
        let paused = Arc::new(AtomicBool::new(false));
        let paused_clone = paused.clone();

        let stream = device.build_output_stream(
            &config,
            move |data: &mut [f32], _: &cpal::OutputCallbackInfo| {
                let mut buffer = ring_buffer_clone.lock().unwrap();
                for sample in data.iter_mut() {
                    if paused_clone.load(Ordering::Relaxed) {
                        *sample = Sample::EQUILIBRIUM;
                    } else {
                        *sample = buffer.pop_front().unwrap_or(Sample::EQUILIBRIUM);
                    }
                }
            },
            |err| eprintln!("Audio stream error: {}", err),
            None,
        )?;

        stream.play()?;

        Ok(AudioOutput {
            ring_buffer,
            spec,
            _stream: stream,
            paused,
        })
    }

    fn write_samples(&self, samples: &[f32]) {
        let mut buffer = self.ring_buffer.lock().unwrap();
        buffer.extend(samples.iter().copied());
    }

    fn buffer_len(&self) -> usize {
        self.ring_buffer.lock().unwrap().len()
    }

    fn set_paused(&self, paused: bool) {
        self.paused.store(paused, Ordering::Relaxed);
    }

    #[allow(dead_code)]
    fn is_paused(&self) -> bool {
        self.paused.load(Ordering::Relaxed)
    }

    fn clear_buffer(&self) {
        self.ring_buffer.lock().unwrap().clear();
    }
}

struct PlaybackContext {
    format: Box<dyn FormatReader>,
    decoder: Box<dyn symphonia::core::codecs::Decoder>,
    track_id: u32,
    audio_output: AudioOutput,
    sample_buf: SampleBuffer<f32>,
    total_frames_decoded: u64,
    total_duration: Option<Duration>,
}

pub fn spawn(
    cmd_rx: mpsc::Receiver<PlayerCommand>,
    event_tx: mpsc::Sender<AppEvent>,
) -> JoinHandle<()> {
    std::thread::spawn(move || {
        let mut ctx: Option<PlaybackContext> = None;
        let mut is_paused = false;
        let mut last_position_update = std::time::Instant::now();

        loop {
            // Check for commands
            match cmd_rx.try_recv() {
                Ok(cmd) => match cmd {
                    PlayerCommand::Play { path, position } => {
                        // Stop current playback
                        ctx = None;
                        is_paused = false;

                        let _ = event_tx.send(AppEvent::PlayerStateChanged(PlayerState::Loading));

                        match load_audio(&path, position) {
                            Ok(new_ctx) => {
                                if let Some(dur) = new_ctx.total_duration {
                                    let _ = event_tx.send(AppEvent::DurationChanged(dur));
                                }
                                ctx = Some(new_ctx);
                                let _ = event_tx
                                    .send(AppEvent::PlayerStateChanged(PlayerState::Playing));
                            }
                            Err(e) => {
                                let _ = event_tx.send(AppEvent::PlayerError(e.to_string()));
                                let _ = event_tx
                                    .send(AppEvent::PlayerStateChanged(PlayerState::Stopped));
                            }
                        }
                    }

                    PlayerCommand::Pause => {
                        if let Some(ref c) = ctx {
                            c.audio_output.set_paused(true);
                            is_paused = true;
                            let _ =
                                event_tx.send(AppEvent::PlayerStateChanged(PlayerState::Paused));
                        }
                    }

                    PlayerCommand::Resume => {
                        if let Some(ref c) = ctx {
                            c.audio_output.set_paused(false);
                            is_paused = false;
                            let _ =
                                event_tx.send(AppEvent::PlayerStateChanged(PlayerState::Playing));
                        }
                    }

                    PlayerCommand::Stop => {
                        if let Some(ref c) = ctx {
                            c.audio_output.clear_buffer();
                        }
                        ctx = None;
                        is_paused = false;
                        let _ = event_tx.send(AppEvent::PlayerStateChanged(PlayerState::Stopped));
                    }

                    PlayerCommand::Seek(position) => {
                        if let Some(ref mut c) = ctx {
                            c.audio_output.clear_buffer();

                            let seek_to = SeekTo::Time {
                                time: Time::from(position.as_secs_f64()),
                                track_id: Some(c.track_id),
                            };

                            match c.format.seek(SeekMode::Accurate, seek_to) {
                                Ok(_seeked_to) => {
                                    c.decoder.reset();
                                    c.total_frames_decoded = (position.as_secs_f64()
                                        * c.audio_output.spec.rate as f64)
                                        as u64;
                                    let _ = event_tx.send(AppEvent::PositionUpdate(position));
                                }
                                Err(e) => {
                                    let _ = event_tx
                                        .send(AppEvent::PlayerError(format!("Seek error: {}", e)));
                                }
                            }
                        }
                    }

                    PlayerCommand::SetSpeed(_speed) => {
                        todo!()
                    }
                },

                Err(TryRecvError::Empty) => {
                    // No command, continue
                }

                Err(TryRecvError::Disconnected) => {
                    // Main thread is gone, exit
                    break;
                }
            }

            // Process audio if we have a context and not paused
            if let Some(ref mut c) = ctx {
                if is_paused {
                    std::thread::sleep(Duration::from_millis(50));
                    continue;
                }

                // Throttle if buffer is full
                let max_buffer =
                    c.audio_output.spec.rate as usize * c.audio_output.spec.channels.count() * 3;
                if c.audio_output.buffer_len() > max_buffer {
                    std::thread::sleep(Duration::from_millis(10));
                    continue;
                }

                // Decode next packet
                match c.format.next_packet() {
                    Ok(packet) => {
                        if packet.track_id() != c.track_id {
                            continue;
                        }

                        match c.decoder.decode(&packet) {
                            Ok(decoded) => {
                                c.sample_buf.copy_interleaved_ref(decoded);
                                c.total_frames_decoded += c.sample_buf.len() as u64
                                    / c.audio_output.spec.channels.count() as u64;

                                c.audio_output.write_samples(c.sample_buf.samples());

                                // Send position update every 100ms
                                if last_position_update.elapsed() >= Duration::from_millis(100) {
                                    let current_secs = c.total_frames_decoded as f64
                                        / c.audio_output.spec.rate as f64;
                                    let _ = event_tx.send(AppEvent::PositionUpdate(
                                        Duration::from_secs_f64(current_secs),
                                    ));
                                    last_position_update = std::time::Instant::now();
                                }
                            }
                            Err(SymphoniaError::DecodeError(e)) => {
                                // Non-fatal, continue
                                eprintln!("Decode error: {}", e);
                            }
                            Err(e) => {
                                let _ = event_tx
                                    .send(AppEvent::PlayerError(format!("Decode error: {}", e)));
                            }
                        }
                    }

                    Err(SymphoniaError::IoError(e))
                        if e.kind() == std::io::ErrorKind::UnexpectedEof =>
                    {
                        // Wait for buffer to drain
                        while c.audio_output.buffer_len() > 0 {
                            std::thread::sleep(Duration::from_millis(50));
                        }
                        std::thread::sleep(Duration::from_millis(200));

                        ctx = None;
                        let _ = event_tx.send(AppEvent::TrackEnded);
                        let _ = event_tx.send(AppEvent::PlayerStateChanged(PlayerState::Stopped));
                    }

                    Err(e) => {
                        let _ = event_tx.send(AppEvent::PlayerError(format!("Read error: {}", e)));
                        ctx = None;
                        let _ = event_tx.send(AppEvent::PlayerStateChanged(PlayerState::Stopped));
                    }
                }
            } else {
                // No playback, sleep to avoid busy loop
                std::thread::sleep(Duration::from_millis(50));
            }
        }
    })
}

fn load_audio(
    path: &PathBuf,
    start_position: Duration,
) -> Result<PlaybackContext, Box<dyn std::error::Error + Send + Sync>> {
    let codecs = symphonia::default::get_codecs();
    let probe = symphonia::default::get_probe();

    let file = File::open(path)?;
    let mss = MediaSourceStream::new(Box::new(file), Default::default());

    let mut hint = Hint::new();
    if let Some(ext) = path.extension().and_then(|e| e.to_str()) {
        hint.with_extension(ext);
    }

    let probed = probe.format(
        &hint,
        mss,
        &FormatOptions::default(),
        &MetadataOptions::default(),
    )?;
    let mut format = probed.format;

    let track = format
        .tracks()
        .iter()
        .find(|t| t.codec_params.codec != CODEC_TYPE_NULL)
        .ok_or("No audio track found")?;

    let track_id = track.id;
    let codec_params = track.codec_params.clone();

    let total_duration = codec_params.time_base.and_then(|tb| {
        codec_params.n_frames.map(|frames| {
            let time = tb.calc_time(frames);
            Duration::from_secs_f64(time.seconds as f64 + time.frac)
        })
    });

    let mut decoder = codecs.make(&codec_params, &DecoderOptions::default())?;

    // Seek to start position if needed
    if start_position > Duration::ZERO {
        let seek_to = SeekTo::Time {
            time: Time::from(start_position.as_secs_f64()),
            track_id: Some(track_id),
        };
        format.seek(SeekMode::Accurate, seek_to)?;
        decoder.reset();
    }

    // Decode packets until we get valid audio (handles decoder warm-up after seek)
    let (spec, first_samples) = loop {
        let packet = format.next_packet()?;

        if packet.track_id() != track_id {
            continue;
        }

        match decoder.decode(&packet) {
            Ok(decoded) => {
                let spec = *decoded.spec();
                let mut sample_buf = SampleBuffer::new(decoded.capacity() as u64, spec);
                sample_buf.copy_interleaved_ref(decoded);
                break (spec, sample_buf);
            }
            Err(SymphoniaError::DecodeError(_)) => {
                // Decoder still warming up after seek, try next packet
                continue;
            }
            Err(e) => return Err(e.into()),
        }
    };

    let audio_output = AudioOutput::new(spec)?;

    audio_output.write_samples(first_samples.samples());

    let initial_frames = if start_position > Duration::ZERO {
        (start_position.as_secs_f64() * spec.rate as f64) as u64
    } else {
        first_samples.len() as u64 / spec.channels.count() as u64
    };

    let sample_buf = SampleBuffer::new(first_samples.len() as u64, spec);

    Ok(PlaybackContext {
        format,
        decoder,
        track_id,
        audio_output,
        sample_buf,
        total_frames_decoded: initial_frames,
        total_duration,
    })
}
