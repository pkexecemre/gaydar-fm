//! Real Linux ALSA playback for GAYDAR FM.
//!
//! A dedicated thread owns the ALSA PCM handle and the ffmpeg child process.
//! The UI only sends commands over an mpsc channel and reads a shared,
//! lock-protected status snapshot, so no ALSA or pipe call can block the UI.
//!
//! Audio path: ffmpeg (HTTP/HLS/progressive/local, s16le stereo) -> bounded
//! chunk channel -> software volume + 32-band analyser -> ALSA PCM.
//! Backpressure from the bounded channel is what pauses the decoder; the
//! child is killed and reaped on stop, seek, device switch, and engine drop.

use alsa::device_name::HintIter;
use alsa::pcm::{Access, Format, HwParams, PCM, State};
use alsa::{Direction, ValueOr};
use anyhow::Result;
use std::collections::HashSet;
use std::io::{ErrorKind, Read};
use std::process::{Child, Command, Stdio};
use std::sync::mpsc::{self, Receiver, RecvTimeoutError, Sender, TryRecvError};
use std::sync::{Arc, Mutex};
use std::thread::{self, JoinHandle};
use std::time::{Duration, Instant};

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum PlaybackState {
    #[default]
    Idle,
    Loading,
    Playing,
    Paused,
    Error,
}

#[derive(Clone, Debug)]
pub struct AudioStatus {
    pub state: PlaybackState,
    pub track_id: Option<u64>,
    pub position_ms: u64,
    pub duration_ms: u64,
    pub volume: f32,
    pub device: String,
    pub sample_rate: u32,
    pub channels: u16,
    pub error: Option<String>,
    pub levels: [f32; 32],
}

impl Default for AudioStatus {
    fn default() -> Self {
        Self {
            state: PlaybackState::Idle,
            track_id: None,
            position_ms: 0,
            duration_ms: 0,
            volume: 0.8,
            device: "default".into(),
            sample_rate: 48000,
            channels: 2,
            error: None,
            levels: [0.0; 32],
        }
    }
}

#[derive(Clone, Debug)]
pub struct AlsaDevice {
    pub name: String,
    pub description: String,
    pub is_default: bool,
}

#[derive(Clone, Debug)]
pub enum AudioEvent {
    Finished(u64),
    Error(String),
}

enum AudioCommand {
    Play {
        track_id: u64,
        url: String,
        duration_ms: u64,
    },
    Stop,
    Pause,
    Resume,
    Toggle,
    Seek(u64),
    SetVolume(f32),
    SetDevice(String),
    Shutdown,
}

const USER_AGENT: &str = "Mozilla/5.0 (X11; Linux x86_64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/129.0.0.0 Safari/537.36";
const HTTP_HEADERS: &str =
    "Referer: https://soundcloud.com/\r\nOrigin: https://soundcloud.com/\r\n";
const SAMPLE_RATES: [u32; 2] = [48_000, 44_100];
const CHANNELS: u16 = 2;
const BYTES_PER_FRAME: usize = (CHANNELS as usize) * 2;
const FFMPEG_READ_SIZE: usize = 4096;
const CHUNK_QUEUE_DEPTH: usize = 8;
const PCM_PERIOD_FRAMES: i64 = 512;
const PCM_BUFFER_FRAMES: i64 = 2048;

const LEVEL_WINDOW: usize = 1024;
const LEVEL_HOP: usize = 512;
const LEVEL_BANDS: usize = 32;
const LEVEL_MIN_HZ: f32 = 45.0;
const LEVEL_MAX_HZ: f32 = 16_000.0;
const LEVEL_TILT_DB: f32 = 14.0;
const LEVEL_FLOOR_DB: f32 = -66.0;
const LEVEL_CEIL_DB: f32 = -5.0;

const PACING_TOLERANCE_MS: u64 = 40;
const PACING_MAX_SLEEP_MS: u64 = 40;
const STALL_RESET: Duration = Duration::from_millis(250);
/// ffmpeg gets this long to produce its first PCM byte before the stream is
/// considered dead. Network/HLS startup normally takes well under a second.
const FIRST_AUDIO_TIMEOUT: Duration = Duration::from_secs(15);
/// A healthy stream can buffer for a moment, but a quarter minute of silence
/// while unpaused means the URL has gone away.
const STALL_TIMEOUT: Duration = Duration::from_secs(25);

/// ALSA PCM handles are created on the audio thread and stay there. Only the
/// senders and the shared status cross threads.
pub struct AudioEngine {
    tx: Sender<AudioCommand>,
    status: Arc<Mutex<AudioStatus>>,
    events: Receiver<AudioEvent>,
    done_rx: Receiver<()>,
    thread: Option<JoinHandle<()>>,
}

impl AudioEngine {
    pub fn new(device: Option<&str>, volume: f32) -> Result<Self> {
        let device = normalize_device(device.unwrap_or("default"));
        let volume = normalize_volume(volume);
        let status = Arc::new(Mutex::new(AudioStatus {
            device: device.clone(),
            volume,
            ..Default::default()
        }));
        let (tx, rx) = mpsc::channel();
        let (event_tx, events) = mpsc::channel();
        let (done_tx, done_rx) = mpsc::channel();
        let status_thread = Arc::clone(&status);
        let handle = thread::Builder::new()
            .name("gaydar-audio".into())
            .spawn(move || {
                let mut engine = ThreadEngine {
                    rx,
                    events: event_tx,
                    status: status_thread,
                    device,
                    volume,
                    playback: None,
                };
                engine.run();
                let _ = done_tx.send(());
            })
            .map_err(|e| anyhow::anyhow!("could not start the audio thread: {e}"))?;
        Ok(Self {
            tx,
            status,
            events,
            done_rx,
            thread: Some(handle),
        })
    }

    /// Enumerate ALSA playback hints, always including "default" first.
    /// Hints may be duplicated across cards/plugins; names are de-duplicated.
    pub fn list_devices() -> Vec<AlsaDevice> {
        let mut devices = Vec::new();
        let mut seen = HashSet::new();
        seen.insert("default".to_string());
        let mut default_description = None;

        if let Ok(hints) = HintIter::new_str(None, "pcm") {
            for hint in hints {
                if hint.direction == Some(Direction::Capture) {
                    continue;
                }
                let Some(raw_name) = hint.name else { continue };
                let name = raw_name.trim().to_string();
                if name.is_empty() {
                    continue;
                }
                let description = clean_description(hint.desc.as_deref().unwrap_or(""));
                if name == "default" {
                    if !description.is_empty() {
                        default_description = Some(description);
                    }
                    continue;
                }
                if seen.insert(name.clone()) {
                    devices.push(AlsaDevice {
                        description: if description.is_empty() {
                            name.clone()
                        } else {
                            description
                        },
                        name,
                        is_default: false,
                    });
                }
            }
        }

        let mut out = Vec::with_capacity(devices.len() + 1);
        out.push(AlsaDevice {
            name: "default".into(),
            description: default_description.unwrap_or_else(|| "Default ALSA output".into()),
            is_default: true,
        });
        out.extend(devices);
        out
    }

    /// Start a new track. Any previous decoder/PCM is torn down first by the
    /// audio thread, so calling play repeatedly never leaks ffmpeg processes.
    pub fn play(&self, track_id: u64, url: String, duration_ms: u64) {
        {
            let mut status = lock_status(&self.status);
            status.state = PlaybackState::Loading;
            status.track_id = Some(track_id);
            status.position_ms = 0;
            status.duration_ms = duration_ms;
            status.error = None;
            status.levels = [0.0; 32];
        }
        let _ = self.tx.send(AudioCommand::Play {
            track_id,
            url,
            duration_ms,
        });
    }

    pub fn pause(&self) {
        let _ = self.tx.send(AudioCommand::Pause);
    }
    pub fn resume(&self) {
        let _ = self.tx.send(AudioCommand::Resume);
    }
    pub fn toggle(&self) {
        let _ = self.tx.send(AudioCommand::Toggle);
    }
    pub fn seek(&self, position_ms: u64) {
        let _ = self.tx.send(AudioCommand::Seek(position_ms));
    }
    pub fn stop(&self) {
        let _ = self.tx.send(AudioCommand::Stop);
    }

    pub fn set_volume(&self, volume: f32) {
        let volume = normalize_volume(volume);
        lock_status(&self.status).volume = volume;
        let _ = self.tx.send(AudioCommand::SetVolume(volume));
    }

    /// Stored immediately; if a track is active the audio thread restarts the
    /// stream on the new device at the current position, preserving pause.
    pub fn set_device(&self, device: String) {
        let device = normalize_device(&device);
        lock_status(&self.status).device = device.clone();
        let _ = self.tx.send(AudioCommand::SetDevice(device));
    }

    pub fn status(&self) -> AudioStatus {
        lock_status(&self.status).clone()
    }

    pub fn try_recv(&self) -> Option<AudioEvent> {
        self.events.try_recv().ok()
    }
}

impl Drop for AudioEngine {
    fn drop(&mut self) {
        let _ = self.tx.send(AudioCommand::Stop);
        let _ = self.tx.send(AudioCommand::Shutdown);
        // Bounded join: the audio thread polls commands every few ms and kills
        // ffmpeg immediately. If it cannot finish in time we detach instead of
        // blocking application shutdown forever.
        if self
            .done_rx
            .recv_timeout(Duration::from_millis(1500))
            .is_ok()
        {
            if let Some(handle) = self.thread.take() {
                let _ = handle.join();
            }
        }
    }
}

struct ThreadEngine {
    rx: Receiver<AudioCommand>,
    events: Sender<AudioEvent>,
    status: Arc<Mutex<AudioStatus>>,
    device: String,
    volume: f32,
    playback: Option<Playback>,
}

impl ThreadEngine {
    fn run(&mut self) {
        loop {
            if !self.drain_commands() {
                break;
            }

            if self.playback.is_none() {
                match self.rx.recv_timeout(Duration::from_millis(100)) {
                    Ok(command) => {
                        if !self.handle_command(command) {
                            break;
                        }
                    }
                    Err(RecvTimeoutError::Timeout) => {}
                    Err(RecvTimeoutError::Disconnected) => break,
                }
                continue;
            }

            if self.playback.as_ref().is_some_and(|p| p.paused) {
                match self.rx.recv_timeout(Duration::from_millis(40)) {
                    Ok(command) => {
                        if !self.handle_command(command) {
                            break;
                        }
                    }
                    Err(RecvTimeoutError::Timeout) => self.pause_tick(),
                    Err(RecvTimeoutError::Disconnected) => break,
                }
                continue;
            }

            let mut playback = self.playback.take().expect("playback checked above");
            let outcome = playback.pump(&self.status, self.volume);
            match outcome {
                PumpOutcome::Continue => self.playback = Some(playback),
                PumpOutcome::Eof => {
                    let track_id = playback.track_id;
                    let position = playback.final_position_ms();
                    drop(playback);
                    let volume = self.volume;
                    with_status(&self.status, |status| {
                        status.state = PlaybackState::Idle;
                        status.position_ms = position;
                        status.volume = volume;
                        status.error = None;
                        status.levels = [0.0; 32];
                    });
                    let _ = self.events.send(AudioEvent::Finished(track_id));
                }
                PumpOutcome::Error(message) => {
                    let position = playback.position_ms();
                    drop(playback);
                    with_status(&self.status, |status| {
                        status.state = PlaybackState::Error;
                        status.position_ms = position;
                        status.error = Some(message.clone());
                        status.levels = [0.0; 32];
                    });
                    let _ = self.events.send(AudioEvent::Error(message));
                }
            }
        }
        self.stop_and_idle();
    }

    fn drain_commands(&mut self) -> bool {
        loop {
            match self.rx.try_recv() {
                Ok(command) => {
                    if !self.handle_command(command) {
                        return false;
                    }
                }
                Err(TryRecvError::Empty) => return true,
                Err(TryRecvError::Disconnected) => return false,
            }
        }
    }

    fn handle_command(&mut self, command: AudioCommand) -> bool {
        match command {
            AudioCommand::Shutdown => {
                self.stop_and_idle();
                return false;
            }
            AudioCommand::Stop => self.stop_and_idle(),
            AudioCommand::Play {
                track_id,
                url,
                duration_ms,
            } => {
                drop(self.playback.take());
                self.start_playback(track_id, url, duration_ms, 0, false);
            }
            AudioCommand::Pause => {
                if let Some(playback) = self.playback.as_mut() {
                    if !playback.paused {
                        let position = playback.pause();
                        let levels = playback.levels();
                        with_status(&self.status, |status| {
                            status.state = PlaybackState::Paused;
                            status.position_ms = position;
                            status.error = None;
                            status.levels = levels;
                        });
                    }
                }
            }
            AudioCommand::Resume => {
                let resumed = match self.playback.as_mut() {
                    Some(playback) if playback.paused => Some(playback.resume()),
                    _ => None,
                };
                match resumed {
                    Some(Ok((position, started_audio))) => {
                        let levels = self
                            .playback
                            .as_ref()
                            .map(|p| p.levels())
                            .unwrap_or([0.0; 32]);
                        with_status(&self.status, |status| {
                            status.state = if started_audio {
                                PlaybackState::Playing
                            } else {
                                PlaybackState::Loading
                            };
                            status.position_ms = position;
                            status.error = None;
                            status.levels = levels;
                        });
                    }
                    Some(Err(message)) => self.fail_current(message),
                    None => {}
                }
            }
            AudioCommand::Toggle => {
                let should_pause = self.playback.as_ref().map(|p| !p.paused).unwrap_or(false);
                if should_pause {
                    if let Some(playback) = self.playback.as_mut() {
                        let position = playback.pause();
                        let levels = playback.levels();
                        with_status(&self.status, |status| {
                            status.state = PlaybackState::Paused;
                            status.position_ms = position;
                            status.levels = levels;
                        });
                    }
                } else if self.playback.as_ref().is_some_and(|p| p.paused) {
                    let resumed = self.playback.as_mut().map(|playback| playback.resume());
                    match resumed {
                        Some(Ok((position, started_audio))) => {
                            with_status(&self.status, |status| {
                                status.state = if started_audio {
                                    PlaybackState::Playing
                                } else {
                                    PlaybackState::Loading
                                };
                                status.position_ms = position;
                                status.error = None;
                            });
                        }
                        Some(Err(message)) => self.fail_current(message),
                        None => {}
                    }
                }
            }
            AudioCommand::Seek(position_ms) => {
                let duration = self.playback.as_ref().map(|p| p.duration_ms).unwrap_or(0);
                let target = clamp_to_duration(position_ms, duration);
                let seeked = self.playback.as_mut().map(|playback| playback.seek(target));
                match seeked {
                    Some(Ok(position)) => {
                        let state = match self.playback.as_ref() {
                            Some(p) if p.paused => PlaybackState::Paused,
                            Some(p) if !p.started => PlaybackState::Loading,
                            Some(_) => PlaybackState::Playing,
                            None => PlaybackState::Idle,
                        };
                        with_status(&self.status, |status| {
                            status.state = state;
                            status.position_ms = position;
                            status.error = None;
                        });
                    }
                    Some(Err(message)) => self.fail_current(message),
                    None => {}
                }
            }
            AudioCommand::SetVolume(volume) => {
                self.volume = normalize_volume(volume);
                let volume = self.volume;
                with_status(&self.status, |status| status.volume = volume);
            }
            AudioCommand::SetDevice(device) => {
                let device = normalize_device(&device);
                self.device = device.clone();
                with_status(&self.status, |status| status.device = device.clone());
                if let Some(previous) = self.playback.take() {
                    let track_id = previous.track_id;
                    let url = previous.url.clone();
                    let duration_ms = previous.duration_ms;
                    let position = previous.position_ms();
                    let paused = previous.paused;
                    drop(previous);
                    self.start_playback(track_id, url, duration_ms, position, paused);
                }
            }
        }
        true
    }

    fn start_playback(
        &mut self,
        track_id: u64,
        url: String,
        duration_ms: u64,
        start_ms: u64,
        paused: bool,
    ) {
        let start_ms = clamp_to_duration(start_ms, duration_ms);
        let device = self.device.clone();
        let volume = self.volume;
        with_status(&self.status, |status| {
            status.state = if paused {
                PlaybackState::Paused
            } else {
                PlaybackState::Loading
            };
            status.track_id = Some(track_id);
            status.position_ms = start_ms;
            status.duration_ms = duration_ms;
            status.volume = volume;
            status.device = device.clone();
            status.error = None;
            status.levels = [0.0; 32];
        });

        match Playback::start(
            track_id,
            url,
            duration_ms,
            start_ms,
            paused,
            &device,
            volume,
        ) {
            Ok(playback) => {
                let sample_rate = playback.sample_rate;
                let device = playback.device.clone();
                self.playback = Some(playback);
                with_status(&self.status, |status| {
                    status.device = device;
                    status.sample_rate = sample_rate;
                    status.channels = CHANNELS;
                });
            }
            Err(message) => {
                with_status(&self.status, |status| {
                    status.state = PlaybackState::Error;
                    status.error = Some(message.clone());
                    status.levels = [0.0; 32];
                });
                let _ = self.events.send(AudioEvent::Error(message));
            }
        }
    }

    fn pause_tick(&mut self) {
        let (position, levels) = match self.playback.as_mut() {
            Some(playback) => {
                playback.decay_levels();
                (playback.frozen_position_ms(), playback.levels())
            }
            None => return,
        };
        with_status(&self.status, |status| {
            status.position_ms = position;
            status.levels = levels;
        });
    }

    fn fail_current(&mut self, message: String) {
        let position = self.playback.as_ref().map(|p| p.position_ms()).unwrap_or(0);
        drop(self.playback.take());
        with_status(&self.status, |status| {
            status.state = PlaybackState::Error;
            status.position_ms = position;
            status.error = Some(message.clone());
            status.levels = [0.0; 32];
        });
        let _ = self.events.send(AudioEvent::Error(message));
    }

    fn stop_and_idle(&mut self) {
        drop(self.playback.take());
        with_status(&self.status, |status| {
            status.state = PlaybackState::Idle;
            status.track_id = None;
            status.position_ms = 0;
            status.duration_ms = 0;
            status.error = None;
            status.levels = [0.0; 32];
        });
    }
}

enum PumpOutcome {
    Continue,
    Eof,
    Error(String),
}

struct Playback {
    track_id: u64,
    url: String,
    duration_ms: u64,
    device: String,
    sample_rate: u32,
    pcm: PCM,
    child: Option<Child>,
    chunk_rx: Option<Receiver<Vec<u8>>>,
    reader: Option<JoinHandle<()>>,
    stderr_reader: Option<JoinHandle<String>>,
    pending: Vec<u8>,
    scratch: Vec<i16>,
    base_ms: u64,
    frames_written: u64,
    paused: bool,
    pause_locked: bool,
    frozen_ms: u64,
    started: bool,
    current_gain: f32,
    level: LevelMeter,
    pacing_origin_pos: u64,
    pacing_origin_wall: Instant,
    spawned_at: Instant,
    last_chunk_at: Option<Instant>,
}

impl Playback {
    fn start(
        track_id: u64,
        url: String,
        duration_ms: u64,
        start_ms: u64,
        paused: bool,
        device: &str,
        volume: f32,
    ) -> std::result::Result<Self, String> {
        let (pcm, sample_rate) = open_pcm_with_fallback(device)?;
        let spawned = spawn_ffmpeg(&url, sample_rate, start_ms)?;
        Ok(Self {
            track_id,
            url,
            duration_ms,
            device: device.to_string(),
            sample_rate,
            pcm,
            child: Some(spawned.child),
            chunk_rx: Some(spawned.chunks),
            reader: Some(spawned.reader),
            stderr_reader: Some(spawned.stderr),
            pending: Vec::new(),
            scratch: Vec::new(),
            base_ms: start_ms,
            frames_written: 0,
            paused,
            pause_locked: false,
            frozen_ms: start_ms,
            started: false,
            current_gain: perceptual_gain(volume),
            level: LevelMeter::new(sample_rate),
            pacing_origin_pos: start_ms,
            pacing_origin_wall: Instant::now(),
            spawned_at: Instant::now(),
            last_chunk_at: None,
        })
    }

    fn pump(&mut self, status: &Arc<Mutex<AudioStatus>>, volume: f32) -> PumpOutcome {
        self.recover_pcm_state();

        let received = match self.chunk_rx.as_ref() {
            Some(rx) => rx.recv_timeout(Duration::from_millis(12)),
            None => return self.finish_eof(),
        };

        match received {
            Err(RecvTimeoutError::Timeout) => {
                if !self.started && self.spawned_at.elapsed() > FIRST_AUDIO_TIMEOUT {
                    let context = format!(
                        "ffmpeg produced no audio within {} seconds (expired URL, dead stream, or blocked network)",
                        FIRST_AUDIO_TIMEOUT.as_secs()
                    );
                    let message = self.abort_stream(&context);
                    return PumpOutcome::Error(message);
                }
                if self.started
                    && self
                        .last_chunk_at
                        .is_some_and(|last| last.elapsed() > STALL_TIMEOUT)
                {
                    let context = format!(
                        "audio stream stalled for {} seconds while unpaused; ffmpeg was stopped",
                        STALL_TIMEOUT.as_secs()
                    );
                    let message = self.abort_stream(&context);
                    return PumpOutcome::Error(message);
                }
                self.level.decay();
                let raw_position = self.raw_position_ms();
                let position = clamp_to_duration(raw_position, self.duration_ms);
                self.publish(status, volume, position);
                self.pace(raw_position);
                PumpOutcome::Continue
            }
            Err(RecvTimeoutError::Disconnected) => self.finish_eof(),
            Ok(chunk) => {
                if self.started
                    && self
                        .last_chunk_at
                        .is_some_and(|last| last.elapsed() > STALL_RESET)
                {
                    let position = self.raw_position_ms();
                    self.reset_pacing(position);
                }
                self.last_chunk_at = Some(Instant::now());

                let mut data = std::mem::take(&mut self.pending);
                data.extend_from_slice(&chunk);
                let usable = (data.len() / BYTES_PER_FRAME) * BYTES_PER_FRAME;
                let was_started = self.started;

                if usable > 0 {
                    if let Err(message) = self.write_audio_block(&data[..usable], volume) {
                        return PumpOutcome::Error(message);
                    }
                    self.started = true;
                }
                self.pending = data[usable..].to_vec();

                let raw_position = self.raw_position_ms();
                let position = clamp_to_duration(raw_position, self.duration_ms);
                if self.started && !was_started {
                    self.reset_pacing(raw_position);
                }
                self.publish(status, volume, position);
                if self.started && was_started {
                    self.pace(raw_position);
                }
                PumpOutcome::Continue
            }
        }
    }

    fn write_audio_block(&mut self, data: &[u8], volume: f32) -> std::result::Result<(), String> {
        let frames = data.len() / BYTES_PER_FRAME;
        if frames == 0 {
            return Ok(());
        }
        let target_gain = perceptual_gain(volume);
        let start_gain = self.current_gain;
        self.scratch.clear();
        self.scratch.reserve(frames * CHANNELS as usize);

        for (index, frame) in data.chunks_exact(BYTES_PER_FRAME).enumerate() {
            let left = i16::from_le_bytes([frame[0], frame[1]]);
            let right = i16::from_le_bytes([frame[2], frame[3]]);
            self.level
                .push((left as f32 + right as f32) * (0.5 / 32768.0));

            let progress = (index + 1) as f32 / frames as f32;
            let gain = start_gain + (target_gain - start_gain) * progress;
            self.scratch.push(
                (left as f32 * gain)
                    .round()
                    .clamp(i16::MIN as f32, i16::MAX as f32) as i16,
            );
            self.scratch.push(
                (right as f32 * gain)
                    .round()
                    .clamp(i16::MIN as f32, i16::MAX as f32) as i16,
            );
        }

        self.current_gain = target_gain;
        self.level.maybe_analyze();
        self.write_scratch(frames)
    }

    fn write_scratch(&mut self, frames: usize) -> std::result::Result<(), String> {
        let Self {
            pcm,
            scratch,
            device,
            frames_written,
            ..
        } = self;
        let mut written_samples = 0usize;
        let mut zero_retries = 0u32;
        while written_samples < scratch.len() {
            let result = {
                let io = pcm
                    .io_i16()
                    .map_err(|e| format!("ALSA rejected s16le access on \"{device}\": {e}"))?;
                io.writei(&scratch[written_samples..])
            };
            match result {
                Ok(0) => {
                    zero_retries += 1;
                    if zero_retries > 8 {
                        return Err(format!(
                            "ALSA device \"{device}\" accepted no frames while writing"
                        ));
                    }
                    thread::yield_now();
                }
                Ok(written_frames) => {
                    zero_retries = 0;
                    written_samples += written_frames * CHANNELS as usize;
                }
                Err(error) => match pcm.try_recover(error, true) {
                    Ok(()) => {}
                    Err(recovery_error) => {
                        return Err(format!(
                            "ALSA playback failed on \"{device}\" and underrun recovery failed: {recovery_error}"
                        ));
                    }
                },
            }
        }
        *frames_written += frames as u64;
        Ok(())
    }

    fn recover_pcm_state(&self) {
        match self.pcm.state() {
            State::XRun | State::Suspended => {
                let _ = self.pcm.prepare();
            }
            State::Disconnected => {}
            _ => {}
        }
    }

    fn pause(&mut self) -> u64 {
        if self.paused {
            return self.frozen_ms;
        }
        let position = self.position_ms();
        self.frozen_ms = position;
        self.paused = true;
        self.pause_locked = self.pcm.pause(true).is_ok();
        self.level.decay();
        position
    }

    fn resume(&mut self) -> std::result::Result<(u64, bool), String> {
        if !self.paused {
            return Ok((self.position_ms(), self.started));
        }
        self.paused = false;
        if self.pause_locked {
            if self.pcm.pause(false).is_err() {
                self.pcm.prepare().map_err(|e| {
                    format!("ALSA could not resume playback on \"{}\": {e}", self.device)
                })?;
            }
            self.pause_locked = false;
        } else if matches!(self.pcm.state(), State::XRun | State::Suspended) {
            self.pcm.prepare().map_err(|e| {
                format!("ALSA could not resume playback on \"{}\": {e}", self.device)
            })?;
        }
        let position = self.position_ms();
        self.reset_pacing(position);
        self.last_chunk_at = Some(Instant::now());
        Ok((position, self.started))
    }

    fn seek(&mut self, target_ms: u64) -> std::result::Result<u64, String> {
        let target = clamp_to_duration(target_ms, self.duration_ms);
        self.terminate_stream();
        self.pcm
            .drop()
            .map_err(|e| format!("ALSA could not flush \"{}\" for seek: {e}", self.device))?;
        self.pcm
            .prepare()
            .map_err(|e| format!("ALSA could not prepare \"{}\" after seek: {e}", self.device))?;

        self.base_ms = target;
        self.frames_written = 0;
        self.frozen_ms = target;
        self.started = false;
        self.pending.clear();
        self.level.reset();
        self.reset_pacing(target);
        self.last_chunk_at = None;
        self.pause_locked = false;
        self.spawn_stream()?;
        Ok(target)
    }

    fn spawn_stream(&mut self) -> std::result::Result<(), String> {
        let spawned = spawn_ffmpeg(&self.url, self.sample_rate, self.base_ms)?;
        self.child = Some(spawned.child);
        self.chunk_rx = Some(spawned.chunks);
        self.reader = Some(spawned.reader);
        self.stderr_reader = Some(spawned.stderr);
        self.spawned_at = Instant::now();
        self.last_chunk_at = None;
        Ok(())
    }

    /// Wait for the decoder after EOF, but never hang the audio thread if it
    /// closed stdout and then stayed alive; kill it after two seconds.
    fn reap_child(&mut self) -> Option<std::io::Result<std::process::ExitStatus>> {
        let mut child = self.child.take()?;
        let deadline = Instant::now() + Duration::from_secs(2);
        loop {
            match child.try_wait() {
                Ok(Some(status)) => return Some(Ok(status)),
                Ok(None) => {
                    if Instant::now() >= deadline {
                        let _ = child.kill();
                        return Some(child.wait());
                    }
                    thread::sleep(Duration::from_millis(10));
                }
                Err(error) => return Some(Err(error)),
            }
        }
    }

    /// Kill and reap a stuck decoder, collecting its stderr for the UI event.
    fn abort_stream(&mut self, context: &str) -> String {
        if let Some(mut child) = self.child.take() {
            let _ = child.kill();
            let _ = child.wait();
        }
        self.chunk_rx.take();
        if let Some(handle) = self.reader.take() {
            let _ = handle.join();
        }
        let stderr = self
            .stderr_reader
            .take()
            .and_then(|handle| handle.join().ok())
            .unwrap_or_default();
        format!("{context}{}", format_ffmpeg_detail(&stderr))
    }

    fn finish_eof(&mut self) -> PumpOutcome {
        if let Some(handle) = self.reader.take() {
            let _ = handle.join();
        }
        let had_audio = self.frames_written > 0;
        if had_audio {
            let _ = self.pcm.drain();
        }
        let waited = self.reap_child();
        let stderr = self
            .stderr_reader
            .take()
            .and_then(|handle| handle.join().ok())
            .unwrap_or_default();
        let detail = format_ffmpeg_detail(&stderr);

        match waited {
            Some(Ok(exit)) if exit.success() => {
                if !had_audio && self.base_ms == 0 {
                    PumpOutcome::Error(format!("ffmpeg decoded no audio for this stream{detail}"))
                } else {
                    PumpOutcome::Eof
                }
            }
            Some(Ok(exit)) => PumpOutcome::Error(format!(
                "ffmpeg exited with status {exit} (the stream may have expired){detail}"
            )),
            Some(Err(error)) => PumpOutcome::Error(format!("could not reap ffmpeg: {error}")),
            None => PumpOutcome::Error("ffmpeg output closed unexpectedly".into()),
        }
    }

    fn terminate_stream(&mut self) {
        if let Some(mut child) = self.child.take() {
            let _ = child.kill();
            let _ = child.wait();
        }
        self.chunk_rx.take();
        if let Some(handle) = self.reader.take() {
            let _ = handle.join();
        }
        if let Some(handle) = self.stderr_reader.take() {
            let _ = handle.join();
        }
    }

    fn position_ms(&self) -> u64 {
        if self.paused {
            return clamp_to_duration(self.frozen_ms, self.duration_ms);
        }
        clamp_to_duration(self.raw_position_ms(), self.duration_ms)
    }

    /// Unclamped audible position used for real-time pacing. Track metadata can
    /// be shorter than the actual stream; clamping the pacing clock would make
    /// a fast sink blast the remaining audio after the reported duration.
    fn raw_position_ms(&self) -> u64 {
        let delay = self.pcm.delay().map(|d| d.max(0) as u64).unwrap_or(0);
        let buffered = delay.min(self.frames_written);
        let played_frames = self.frames_written.saturating_sub(buffered);
        let played_ms = played_frames.saturating_mul(1000) / self.sample_rate.max(1) as u64;
        self.base_ms.saturating_add(played_ms)
    }

    fn frozen_position_ms(&self) -> u64 {
        clamp_to_duration(self.frozen_ms, self.duration_ms)
    }

    /// At natural EOF every frame handed to ALSA has been drained, so use the
    /// written-frame clock rather than the (possibly stale) hardware delay.
    fn final_position_ms(&self) -> u64 {
        let played_ms = self.frames_written.saturating_mul(1000) / self.sample_rate.max(1) as u64;
        clamp_to_duration(self.base_ms.saturating_add(played_ms), self.duration_ms)
    }

    fn publish(&self, status: &Arc<Mutex<AudioStatus>>, volume: f32, position: u64) {
        let levels = self.level.bands;
        let state = if self.started {
            PlaybackState::Playing
        } else {
            PlaybackState::Loading
        };
        let track_id = self.track_id;
        let duration_ms = self.duration_ms;
        let sample_rate = self.sample_rate;
        let device = self.device.clone();
        with_status(status, |status| {
            status.state = state;
            status.track_id = Some(track_id);
            status.position_ms = position;
            status.duration_ms = duration_ms;
            status.volume = volume;
            status.device = device;
            status.sample_rate = sample_rate;
            status.channels = CHANNELS;
            status.error = None;
            status.levels = levels;
        });
    }

    fn pace(&mut self, position: u64) {
        let wall_ms = self.pacing_origin_wall.elapsed().as_millis() as u64;
        let audio_ms = position.saturating_sub(self.pacing_origin_pos);
        if audio_ms > wall_ms.saturating_add(PACING_TOLERANCE_MS) {
            let ahead = audio_ms - wall_ms - PACING_TOLERANCE_MS;
            if ahead > 0 {
                thread::sleep(Duration::from_millis(ahead.min(PACING_MAX_SLEEP_MS)));
            }
        }
    }

    fn reset_pacing(&mut self, position: u64) {
        self.pacing_origin_pos = position;
        self.pacing_origin_wall = Instant::now();
    }

    fn decay_levels(&mut self) {
        self.level.decay();
    }
    fn levels(&self) -> [f32; LEVEL_BANDS] {
        self.level.bands
    }
}

impl Drop for Playback {
    fn drop(&mut self) {
        self.terminate_stream();
    }
}

struct SpawnedFfmpeg {
    child: Child,
    chunks: Receiver<Vec<u8>>,
    reader: JoinHandle<()>,
    stderr: JoinHandle<String>,
}

fn spawn_ffmpeg(
    url: &str,
    sample_rate: u32,
    start_ms: u64,
) -> std::result::Result<SpawnedFfmpeg, String> {
    let mut command = Command::new("ffmpeg");
    command
        .arg("-nostdin")
        .arg("-hide_banner")
        .arg("-loglevel")
        .arg("error");
    // HTTP protocol AVOptions are rejected outright by the file:// demuxer,
    // so they are only attached to network streams.
    if is_network_url(url) {
        command
            .arg("-user_agent")
            .arg(USER_AGENT)
            .arg("-headers")
            .arg(HTTP_HEADERS);
    }
    if start_ms > 0 {
        command
            .arg("-ss")
            .arg(format!("{:.3}", start_ms as f64 / 1000.0));
    }
    command
        .arg("-i")
        .arg(url)
        .arg("-vn")
        .arg("-f")
        .arg("s16le")
        .arg("-ac")
        .arg(CHANNELS.to_string())
        .arg("-ar")
        .arg(sample_rate.to_string())
        .arg("-")
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());

    let mut child = command
        .spawn()
        .map_err(|e| format!("could not start ffmpeg (is it installed?): {e}"))?;
    let Some(stdout) = child.stdout.take() else {
        let _ = child.kill();
        let _ = child.wait();
        return Err("ffmpeg stdout pipe unavailable".into());
    };
    let Some(stderr) = child.stderr.take() else {
        let _ = child.kill();
        let _ = child.wait();
        return Err("ffmpeg stderr pipe unavailable".into());
    };
    let (chunk_tx, chunk_rx) = mpsc::sync_channel::<Vec<u8>>(CHUNK_QUEUE_DEPTH);

    let reader = thread::Builder::new()
        .name("gaydar-ffmpeg-out".into())
        .spawn(move || {
            let mut stdout = stdout;
            let mut buffer = vec![0u8; FFMPEG_READ_SIZE];
            loop {
                match stdout.read(&mut buffer) {
                    Ok(0) => break,
                    Ok(read) => {
                        if chunk_tx.send(buffer[..read].to_vec()).is_err() {
                            break;
                        }
                    }
                    Err(error) if error.kind() == ErrorKind::Interrupted => continue,
                    Err(_) => break,
                }
            }
        })
        .map_err(|e| {
            let _ = child.kill();
            let _ = child.wait();
            format!("could not start the ffmpeg reader thread: {e}")
        })?;

    let stderr_reader = match thread::Builder::new()
        .name("gaydar-ffmpeg-err".into())
        .spawn(move || read_stderr(stderr))
    {
        Ok(handle) => handle,
        Err(error) => {
            let _ = child.kill();
            let _ = child.wait();
            drop(chunk_rx);
            let _ = reader.join();
            return Err(format!("could not start the ffmpeg stderr thread: {error}"));
        }
    };

    Ok(SpawnedFfmpeg {
        child,
        chunks: chunk_rx,
        reader,
        stderr: stderr_reader,
    })
}

fn read_stderr(mut stderr: std::process::ChildStderr) -> String {
    const LIMIT: usize = 16 * 1024;
    let mut collected = Vec::new();
    let mut buffer = [0u8; 1024];
    loop {
        match stderr.read(&mut buffer) {
            Ok(0) => break,
            Ok(read) => {
                if collected.len() < LIMIT {
                    let take = read.min(LIMIT - collected.len());
                    collected.extend_from_slice(&buffer[..take]);
                }
            }
            Err(error) if error.kind() == ErrorKind::Interrupted => continue,
            Err(_) => break,
        }
    }
    String::from_utf8_lossy(&collected).into_owned()
}

fn open_pcm_with_fallback(device: &str) -> std::result::Result<(PCM, u32), String> {
    let mut errors = Vec::new();
    for rate in SAMPLE_RATES {
        match open_pcm(device, rate) {
            Ok(pcm) => return Ok((pcm, rate)),
            Err(error) => errors.push(format!("{rate} Hz: {error}")),
        }
    }
    Err(format!(
        "could not open ALSA output \"{device}\" as 16-bit stereo at 48 kHz or 44.1 kHz: {}",
        errors.join("; ")
    ))
}

fn open_pcm(device: &str, rate: u32) -> std::result::Result<PCM, String> {
    let pcm =
        PCM::new(device, Direction::Playback, false).map_err(|e| format!("open failed: {e}"))?;
    {
        let hwp = HwParams::any(&pcm).map_err(|e| format!("hardware query failed: {e}"))?;
        hwp.set_access(Access::RWInterleaved)
            .map_err(|e| format!("interleaved access unsupported: {e}"))?;
        hwp.set_format(Format::s16())
            .map_err(|e| format!("s16le format unsupported: {e}"))?;
        hwp.set_channels(CHANNELS as u32)
            .map_err(|e| format!("stereo unsupported: {e}"))?;
        hwp.set_rate(rate, ValueOr::Nearest)
            .map_err(|e| format!("sample rate {rate} Hz unsupported: {e}"))?;
        let actual_rate = hwp
            .get_rate()
            .map_err(|e| format!("could not read the negotiated rate: {e}"))?;
        if actual_rate != rate {
            return Err(format!(
                "device negotiated {actual_rate} Hz instead of {rate} Hz"
            ));
        }
        let period = hwp
            .set_period_size_near(PCM_PERIOD_FRAMES, ValueOr::Nearest)
            .map_err(|e| format!("could not set a playback period: {e}"))?;
        let buffer_target = PCM_BUFFER_FRAMES.max(period.saturating_mul(4));
        hwp.set_buffer_size_near(buffer_target)
            .map_err(|e| format!("could not set the playback buffer: {e}"))?;
        pcm.hw_params(&hwp)
            .map_err(|e| format!("hardware parameters rejected: {e}"))?;
    }
    pcm.prepare().map_err(|e| format!("prepare failed: {e}"))?;
    Ok(pcm)
}

struct LevelMeter {
    window: [f32; LEVEL_WINDOW],
    cursor: usize,
    filled: usize,
    since_analysis: usize,
    hann: [f32; LEVEL_WINDOW],
    coefficients: Vec<f32>,
    tilt: [f32; LEVEL_BANDS],
    bands: [f32; LEVEL_BANDS],
}

impl LevelMeter {
    fn new(sample_rate: u32) -> Self {
        let mut hann = [0.0f32; LEVEL_WINDOW];
        for (index, value) in hann.iter_mut().enumerate() {
            *value = 0.5
                - 0.5
                    * (2.0 * std::f32::consts::PI * index as f32 / (LEVEL_WINDOW - 1) as f32).cos();
        }

        let ratio = (LEVEL_MAX_HZ / LEVEL_MIN_HZ).powf(1.0 / LEVEL_BANDS as f32);
        let mut coefficients = Vec::with_capacity(LEVEL_BANDS * 3);
        let mut tilt = [0.0f32; LEVEL_BANDS];
        let mut lower = LEVEL_MIN_HZ;
        for band in 0..LEVEL_BANDS {
            let upper = lower * ratio;
            for probe in 0..3 {
                let fraction = (probe as f32 + 0.5) / 3.0;
                let frequency = lower * (upper / lower).powf(fraction);
                coefficients.push(
                    2.0 * (2.0 * std::f32::consts::PI * frequency / sample_rate.max(1) as f32)
                        .cos(),
                );
            }
            tilt[band] = LEVEL_TILT_DB * band as f32 / (LEVEL_BANDS - 1) as f32;
            lower = upper;
        }

        Self {
            window: [0.0; LEVEL_WINDOW],
            cursor: 0,
            filled: 0,
            since_analysis: 0,
            hann,
            coefficients,
            tilt,
            bands: [0.0; LEVEL_BANDS],
        }
    }

    fn push(&mut self, sample: f32) {
        self.window[self.cursor] = sample;
        self.cursor = (self.cursor + 1) % LEVEL_WINDOW;
        self.filled = (self.filled + 1).min(LEVEL_WINDOW);
        self.since_analysis = self.since_analysis.saturating_add(1);
    }

    fn maybe_analyze(&mut self) {
        if self.filled == LEVEL_WINDOW && self.since_analysis >= LEVEL_HOP {
            self.analyze();
            self.since_analysis = 0;
        }
    }

    fn analyze(&mut self) {
        // cursor points at the oldest sample once the window is full.
        let mut windowed = [0.0f32; LEVEL_WINDOW];
        for (index, value) in windowed.iter_mut().enumerate() {
            *value = self.window[(self.cursor + index) % LEVEL_WINDOW] * self.hann[index];
        }

        for band in 0..LEVEL_BANDS {
            let mut power = 0.0f32;
            for probe in 0..3 {
                let coefficient = self.coefficients[band * 3 + probe];
                let (mut s1, mut s2) = (0.0f32, 0.0f32);
                for sample in windowed {
                    let s0 = sample + coefficient * s1 - s2;
                    s2 = s1;
                    s1 = s0;
                }
                let candidate = s1 * s1 + s2 * s2 - coefficient * s1 * s2;
                if candidate > power {
                    power = candidate;
                }
            }

            // Hann coherent gain is 0.5, so a full-scale sine gives |X| = N/4.
            let amplitude = 4.0 * power.max(0.0).sqrt() / LEVEL_WINDOW as f32;
            let db = 20.0 * (amplitude + 1e-9).log10() + self.tilt[band];
            let target = ((db - LEVEL_FLOOR_DB) / (LEVEL_CEIL_DB - LEVEL_FLOOR_DB)).clamp(0.0, 1.0);
            let previous = self.bands[band];
            let smoothed = if target > previous {
                previous + (target - previous) * 0.55
            } else {
                previous + (target - previous) * 0.10
            };
            self.bands[band] = smoothed;
        }
    }

    fn decay(&mut self) {
        for level in &mut self.bands {
            *level *= 0.88;
            if *level < 0.001 {
                *level = 0.0;
            }
        }
    }

    fn reset(&mut self) {
        self.bands = [0.0; LEVEL_BANDS];
        self.filled = 0;
        self.cursor = 0;
        self.since_analysis = 0;
        self.window = [0.0; LEVEL_WINDOW];
    }
}

fn perceptual_gain(volume: f32) -> f32 {
    let volume = normalize_volume(volume);
    volume * volume
}

fn normalize_volume(volume: f32) -> f32 {
    if volume.is_finite() {
        volume.clamp(0.0, 1.0)
    } else {
        0.8
    }
}

fn is_network_url(url: &str) -> bool {
    let url = url.trim_start().to_ascii_lowercase();
    url.starts_with("http://") || url.starts_with("https://")
}

fn normalize_device(device: &str) -> String {
    let device = device.trim();
    if device.is_empty() {
        "default".into()
    } else {
        device.to_string()
    }
}

fn clamp_to_duration(position_ms: u64, duration_ms: u64) -> u64 {
    if duration_ms == 0 {
        position_ms
    } else {
        position_ms.min(duration_ms)
    }
}

fn lock_status(status: &Arc<Mutex<AudioStatus>>) -> std::sync::MutexGuard<'_, AudioStatus> {
    status
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
}

fn with_status(status: &Arc<Mutex<AudioStatus>>, update: impl FnOnce(&mut AudioStatus)) {
    update(&mut lock_status(status));
}

fn clean_description(description: &str) -> String {
    description.split_whitespace().collect::<Vec<_>>().join(" ")
}

fn format_ffmpeg_detail(stderr: &str) -> String {
    let cleaned = clean_description(stderr);
    if cleaned.is_empty() {
        String::new()
    } else {
        let truncated: String = cleaned.chars().take(400).collect();
        format!(": {truncated}")
    }
}
