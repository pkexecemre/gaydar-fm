//! Application state for GAYDAR FM.

use crate::audio::{AudioEngine, AudioEvent, AudioStatus, PlaybackState};
use crate::model::{Dials, Recommendation, StreamSource, Track, format_time};
use crate::radio::{Autoplay, RadioCommand, RadioEvent, RadioService};
use blenderui::prelude::*;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet, VecDeque};
use std::path::PathBuf;
use std::time::Instant;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Screen {
    Home,
    Gaydar,
    Search,
    Liked,
    History,
    Settings,
    NowPlaying,
}

impl Screen {
    pub fn label(self) -> &'static str {
        match self {
            Screen::Home => "For You",
            Screen::Gaydar => "The Gaydar",
            Screen::Search => "Search",
            Screen::Liked => "Liked",
            Screen::History => "History",
            Screen::Settings => "Settings",
            Screen::NowPlaying => "Now Serving",
        }
    }

    pub fn eyebrow(self) -> &'static str {
        match self {
            Screen::Home => "GAYDAR FM · 88.8",
            Screen::Gaydar => "THE ALGORITHM HAS TASTE",
            Screen::Search => "DIG THROUGH THE CRATE",
            Screen::Liked => "YOUR PERSONAL CANON",
            Screen::History => "ALREADY SERVED",
            Screen::Settings => "TECHNICAL, HENNY",
            Screen::NowPlaying => "THE MAIN EVENT",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ToastKind {
    Info,
    Good,
    Bad,
}

#[derive(Clone, Debug)]
pub struct Toast {
    pub text: String,
    pub kind: ToastKind,
    pub born: Instant,
}

impl Toast {
    pub fn new(text: impl Into<String>, kind: ToastKind) -> Self {
        Self {
            text: text.into(),
            kind,
            born: Instant::now(),
        }
    }

    pub fn alive(&self) -> bool {
        self.born.elapsed().as_secs_f32() < 3.2
    }
}

#[derive(Clone, Copy, Debug)]
pub struct Station {
    pub id: &'static str,
    pub name: &'static str,
    pub tag: &'static str,
    pub blurb: &'static str,
    pub query: &'static str,
    pub fallback_query: &'static str,
    pub dials: Dials,
}

pub const STATIONS: [Station; 4] = [
    Station {
        id: "camp",
        name: "CAMP CLASSICS",
        tag: "VOL. 01",
        blurb: "Showtunes, divas and unnecessary drama.",
        query: "Jinkx Monsoon",
        fallback_query: "drag queen dj set",
        dials: Dials {
            camp: 1.0,
            club: 0.55,
            soft: 0.30,
            chaos: 0.45,
        },
    },
    Station {
        id: "ballroom",
        name: "BALLROOM 4AM",
        tag: "VOL. 02",
        blurb: "Vogue beats, runway stomps, no survivors.",
        query: "ballroom vogue",
        fallback_query: "queer rave",
        dials: Dials {
            camp: 0.82,
            club: 1.0,
            soft: 0.18,
            chaos: 0.62,
        },
    },
    Station {
        id: "soft",
        name: "SOFT SERVE",
        tag: "VOL. 03",
        blurb: "Sapphic slow burns for crying in the club.",
        query: "sapphic pop",
        fallback_query: "trans ambient",
        dials: Dials {
            camp: 0.35,
            club: 0.28,
            soft: 1.0,
            chaos: 0.20,
        },
    },
    Station {
        id: "hyper",
        name: "HYPERPOP COMMUNION",
        tag: "VOL. 04",
        blurb: "Glitched angels and pocket-sized chaos.",
        query: "queer hyperpop",
        fallback_query: "queercore",
        dials: Dials {
            camp: 0.68,
            club: 0.86,
            soft: 0.25,
            chaos: 1.0,
        },
    },
];

#[derive(Serialize, Deserialize, Default)]
struct StoredSettings {
    device: Option<String>,
    volume: Option<f32>,
}

pub struct GaydarApp {
    pub screen: Screen,
    pub previous_screen: Screen,
    pub current: Option<Track>,
    pub current_reason: String,
    pub queue: VecDeque<Recommendation>,
    pub history: Vec<Track>,
    pub liked_tracks: Vec<Track>,
    pub liked: HashSet<u64>,
    pub search_query: String,
    pub search_results: Vec<Track>,
    pub searching: bool,
    pub search_generation: u64,
    pub dials: Dials,
    pub taste: Vec<(String, f32)>,
    pub total_plays: u64,
    pub top_artists: Vec<String>,
    pub audio: Option<AudioEngine>,
    pub playback: AudioStatus,
    pub devices: Vec<crate::audio::AlsaDevice>,
    pub device: String,
    pub volume: f32,
    pub booting: bool,
    pub loading_track: bool,
    pub connecting: bool,
    pub error: Option<String>,
    pub toast: Option<Toast>,
    pub art_cache: HashMap<u64, String>,
    pub pending_art: HashSet<u64>,
    pub demo: bool,
    pub now_playing_open: bool,
    pub queue_open: bool,
    pub pulse: f32,
    pub radio_status: String,
    pub client_ready: bool,
    pub settings_open: bool,
    pub screenshot_screen: Option<Screen>,
    pub playback_token: u64,
    pub liked_filter: bool,
    pub radar_phase: f32,
    pub radio: Option<RadioService>,
    pub current_source: Option<StreamSource>,
    pub data_dir: PathBuf,
    pub cache_dir: PathBuf,
    pub settings_path: PathBuf,
    pub search_error: Option<String>,
    pub refilling: bool,
    pub consecutive_failures: u32,
    pub started: bool,
    pub needs_autoplay: bool,
}

impl GaydarApp {
    pub fn demo(screen: Screen) -> Self {
        let tracks = demo_tracks();
        let current = tracks[0].clone();
        let mut liked = HashSet::new();
        for t in tracks.iter().skip(1).take(4) {
            liked.insert(t.id);
        }
        let queue = tracks
            .iter()
            .skip(1)
            .take(5)
            .enumerate()
            .map(|(i, t)| Recommendation {
                track: t.clone(),
                reason: demo_reasons()[i % demo_reasons().len()].to_string(),
                score: 0.9 - i as f32 * 0.07,
            })
            .collect::<VecDeque<_>>();
        let mut playback = AudioStatus::default();
        playback.state = PlaybackState::Playing;
        playback.track_id = Some(current.id);
        playback.position_ms = 83_000;
        playback.duration_ms = current.duration_ms;
        playback.volume = 0.72;
        playback.device = "default".into();
        playback.sample_rate = 48_000;
        playback.channels = 2;
        playback.levels = demo_levels();
        Self {
            screen,
            previous_screen: Screen::Home,
            current: Some(current),
            current_reason: "camp canon · because you keep replaying diva pop".into(),
            queue,
            history: tracks.iter().skip(2).take(5).cloned().collect(),
            liked_tracks: tracks
                .iter()
                .filter(|t| liked.contains(&t.id))
                .cloned()
                .collect(),
            liked,
            search_query: String::new(),
            search_results: tracks.iter().take(6).cloned().collect(),
            searching: false,
            search_generation: 0,
            dials: Dials::default(),
            taste: vec![
                ("hyperpop".into(), 0.92),
                ("camp".into(), 0.86),
                ("ballroom".into(), 0.71),
                ("sapphic".into(), 0.64),
                ("club".into(), 0.58),
                ("trans joy".into(), 0.47),
            ],
            total_plays: 128,
            top_artists: vec![
                "Chappell Roan".into(),
                "SOPHIE".into(),
                "MUNA".into(),
                "Slayyyter".into(),
            ],
            audio: None,
            playback,
            devices: vec![
                crate::audio::AlsaDevice {
                    name: "default".into(),
                    description: "Default ALSA device".into(),
                    is_default: true,
                },
                crate::audio::AlsaDevice {
                    name: "pipewire".into(),
                    description: "PipeWire Sound Server".into(),
                    is_default: false,
                },
                crate::audio::AlsaDevice {
                    name: "hw:0,0".into(),
                    description: "Built-in Audio Analog Stereo".into(),
                    is_default: false,
                },
            ],
            device: "default".into(),
            volume: 0.72,
            booting: false,
            loading_track: false,
            connecting: false,
            error: None,
            toast: None,
            art_cache: HashMap::new(),
            pending_art: HashSet::new(),
            demo: true,
            now_playing_open: screen == Screen::NowPlaying,
            queue_open: false,
            pulse: 0.0,
            radio_status: "ON AIR · LIVE FROM THE CLOUD".into(),
            client_ready: true,
            settings_open: false,
            screenshot_screen: None,
            playback_token: 0,
            liked_filter: false,
            radar_phase: 0.85,
            radio: None,
            current_source: None,
            data_dir: PathBuf::new(),
            cache_dir: PathBuf::new(),
            settings_path: PathBuf::new(),
            search_error: None,
            refilling: false,
            consecutive_failures: 0,
            started: false,
            needs_autoplay: false,
        }
    }

    pub fn live() -> Self {
        let mut app = Self::demo(Screen::Home);
        app.demo = false;
        app.current = None;
        app.current_reason = String::new();
        app.queue.clear();
        app.history.clear();
        app.liked_tracks.clear();
        app.liked.clear();
        app.taste.clear();
        app.total_plays = 0;
        app.top_artists.clear();
        app.search_results.clear();
        app.playback = AudioStatus::default();
        app.volume = 0.78;
        app.booting = true;
        app.client_ready = false;
        app.radio_status = "TUNING THE GAYDAR…".into();
        app
    }

    pub fn is_liked(&self, id: u64) -> bool {
        self.liked.contains(&id)
    }

    pub fn current_id(&self) -> Option<u64> {
        self.current.as_ref().map(|t| t.id)
    }

    pub fn current_duration(&self) -> u64 {
        self.current.as_ref().map(|t| t.duration_ms).unwrap_or(0)
    }

    pub fn progress(&self) -> f32 {
        if self.playback.duration_ms == 0 {
            return 0.0;
        }
        (self.playback.position_ms as f32 / self.playback.duration_ms as f32).clamp(0.0, 1.0)
    }

    pub fn position_label(&self) -> String {
        format_time(self.playback.position_ms)
    }

    pub fn duration_label(&self) -> String {
        format_time(self.current_duration().max(self.playback.duration_ms))
    }

    pub fn art_path(&self, track: &Track) -> Option<String> {
        self.art_cache.get(&track.id).cloned()
    }

    pub fn notify(&mut self, text: impl Into<String>, kind: ToastKind) {
        self.toast = Some(Toast::new(text, kind));
    }

    pub fn dismiss_error(&mut self) {
        self.error = None;
    }

    pub fn refresh_devices(&mut self) {
        self.devices = AudioEngine::list_devices();
        self.notify(
            format!("{} ALSA outputs in the building", self.devices.len()),
            ToastKind::Info,
        );
    }

    pub fn goto(&mut self, screen: Screen) {
        if screen != self.screen {
            self.previous_screen = self.screen;
            self.screen = screen;
        }
        if screen == Screen::Settings {
            self.devices = AudioEngine::list_devices();
        }
        if !self.demo {
            let tracks: Vec<Track> = match screen {
                Screen::Liked => self.liked_tracks.clone(),
                Screen::History => self.history.clone(),
                Screen::Search => self.search_results.clone(),
                _ => Vec::new(),
            };
            for track in tracks {
                self.ensure_art(&track);
            }
        }
    }

    pub fn toggle_now_playing(&mut self) {
        if self.screen == Screen::NowPlaying {
            self.screen = self.previous_screen;
        } else {
            self.previous_screen = self.screen;
            self.screen = Screen::NowPlaying;
        }
    }

    pub fn push_current_to_history(&mut self) {
        if let Some(t) = &self.current {
            self.history.retain(|h| h.id != t.id);
            self.history.insert(0, t.clone());
        }
    }

    pub fn next_in_queue(&mut self) -> Option<Recommendation> {
        self.queue.pop_front()
    }

    pub fn demo_like_toggle(&mut self, track: &Track) {
        if self.liked.remove(&track.id) {
            self.liked_tracks.retain(|t| t.id != track.id);
            self.notify("removed from the canon, unfortunately", ToastKind::Info);
        } else {
            self.liked.insert(track.id);
            self.liked_tracks.retain(|t| t.id != track.id);
            self.liked_tracks.insert(0, track.clone());
            self.notify("added to the canon. taste!", ToastKind::Good);
        }
    }

    pub fn demo_play_track(&mut self, track: Track, reason: impl Into<String>) {
        self.current = Some(track.clone());
        self.current_reason = reason.into();
        self.playback.state = PlaybackState::Playing;
        self.playback.track_id = Some(track.id);
        self.playback.position_ms = 0;
        self.playback.duration_ms = track.duration_ms;
        self.playback.levels = demo_levels();
        self.loading_track = false;
        self.total_plays += 1;
        self.push_current_to_history();
    }

    pub fn demo_search(&mut self, query: &str) {
        self.search_query = query.to_string();
        self.searching = false;
        if query.trim().is_empty() {
            self.search_results.clear();
        } else {
            let all = demo_tracks();
            let q = query.to_lowercase();
            let mut results: Vec<Track> = all
                .iter()
                .filter(|t| t.seed_text().to_lowercase().contains(&q))
                .cloned()
                .collect();
            if results.is_empty() {
                results = all.into_iter().take(5).collect();
            }
            self.search_results = results;
        }
    }

    // -- demo / UI actions -------------------------------------------------

    pub fn toggle_play(&mut self) {
        if self.demo {
            match self.playback.state {
                PlaybackState::Playing => self.playback.state = PlaybackState::Paused,
                PlaybackState::Paused => self.playback.state = PlaybackState::Playing,
                PlaybackState::Idle => {
                    if let Some(t) = self.current.clone() {
                        self.demo_play_track(t, self.current_reason.clone());
                    }
                }
                _ => {}
            }
            return;
        }
        match self.playback.state {
            PlaybackState::Playing => {
                if let Some(audio) = &self.audio {
                    audio.pause();
                    self.playback.state = PlaybackState::Paused;
                }
            }
            PlaybackState::Paused => {
                if let Some(audio) = &self.audio {
                    audio.resume();
                    self.playback.state = PlaybackState::Playing;
                }
            }
            PlaybackState::Loading => {}
            _ => {
                if let Some(source) = self.current_source.clone() {
                    if let (Some(audio), Some(track)) = (&self.audio, self.current.clone()) {
                        audio.play(track.id, source.url, track.duration_ms);
                    }
                } else if let Some(track) = self.current.clone() {
                    self.request_play(track, self.current_reason.clone());
                } else {
                    self.start_radio();
                }
            }
        }
    }

    pub fn toggle_like(&mut self, track: &Track) {
        let was_liked = self.liked.contains(&track.id);
        self.demo_like_toggle(track);
        if !self.demo {
            if let Some(radio) = &self.radio {
                if was_liked {
                    radio.send(RadioCommand::Unlike(track.clone()));
                } else {
                    radio.send(RadioCommand::Like(track.clone()));
                }
            }
        }
    }

    pub fn play_track(&mut self, track: Track, reason: impl Into<String>) {
        if self.demo {
            self.demo_play_track(track, reason);
        } else {
            self.request_play(track, reason.into());
        }
    }

    pub fn next_track(&mut self) {
        if self.demo {
            self.push_current_to_history();
            if let Some(rec) = self.queue.pop_front() {
                self.play_track(rec.track, rec.reason);
            } else {
                let all = demo_tracks();
                if let Some(t) = all.into_iter().find(|t| Some(t.id) != self.current_id()) {
                    self.play_track(t, "the gaydar stays serving");
                }
            }
        } else {
            self.advance_live(false);
        }
    }

    pub fn prev_track(&mut self) {
        if self.demo {
            self.playback.position_ms = 0;
            if let Some(t) = self.history.first().cloned() {
                self.play_track(t, "a little rewind, a little cunt");
            }
            return;
        }
        if self.playback.position_ms > 3_000 {
            self.seek_fraction(0.0);
            return;
        }
        let current = self.current_id();
        if let Some(previous) = self.history.iter().find(|t| Some(t.id) != current).cloned() {
            self.request_play(previous, "a little rewind, a little cunt".into());
        } else {
            self.seek_fraction(0.0);
        }
    }

    pub fn seek_fraction(&mut self, fraction: f32) {
        let duration = self.current_duration().max(self.playback.duration_ms);
        let ms = (duration as f32 * fraction.clamp(0.0, 1.0)) as u64;
        self.playback.position_ms = ms;
        if !self.demo {
            if let Some(audio) = &self.audio {
                audio.seek(ms);
            }
        }
    }

    pub fn set_volume(&mut self, volume: f32) {
        self.volume = volume.clamp(0.0, 1.0);
        if let Some(audio) = &self.audio {
            audio.set_volume(self.volume);
        }
        if !self.demo {
            self.save_settings();
        }
    }

    pub fn set_device(&mut self, device: String) {
        self.device = device.clone();
        if let Some(audio) = &self.audio {
            audio.set_device(device);
        }
        if !self.demo {
            self.save_settings();
        }
        self.notify("ALSA device switched. hear the girls?", ToastKind::Good);
    }

    pub fn set_dials_all(&mut self, dials: Dials) {
        self.dials = dials.clamped();
        if !self.demo {
            if let Some(radio) = &self.radio {
                radio.send(RadioCommand::SetDials(self.dials));
            }
        }
    }

    pub fn set_dial(&mut self, index: usize, value: f32) {
        let value = value.clamp(0.0, 1.0);
        match index {
            0 => self.dials.camp = value,
            1 => self.dials.club = value,
            2 => self.dials.soft = value,
            _ => self.dials.chaos = value,
        }
        if !self.demo {
            if let Some(radio) = &self.radio {
                radio.send(RadioCommand::SetDials(self.dials));
            }
        }
    }

    pub fn search_push(&mut self, text: &str) {
        if text.is_empty() || text.chars().any(|c| c.is_control()) {
            return;
        }
        self.search_query.push_str(text);
        self.search_error = None;
        if self.demo {
            self.demo_search(&self.search_query.clone());
        }
    }

    pub fn search_backspace(&mut self) {
        self.search_query.pop();
        self.search_error = None;
        if self.demo {
            self.demo_search(&self.search_query.clone());
        }
    }

    pub fn search_clear(&mut self) {
        self.search_query.clear();
        self.search_error = None;
        if self.demo {
            self.search_results.clear();
            self.searching = false;
        }
    }

    pub fn set_search_query(&mut self, value: String) {
        self.search_query = value;
        self.search_error = None;
        if self.demo {
            self.demo_search(&self.search_query.clone());
        }
    }

    pub fn submit_search(&mut self) {
        if self.demo {
            self.demo_search(&self.search_query.clone());
            return;
        }
        if self.search_query.trim().is_empty() {
            self.search_results.clear();
            self.searching = false;
            return;
        }
        self.search_generation += 1;
        let generation = self.search_generation;
        self.searching = true;
        self.search_error = None;
        if let Some(radio) = &self.radio {
            radio.send(RadioCommand::Search {
                generation,
                query: self.search_query.clone(),
            });
        } else {
            self.searching = false;
            self.search_error = Some("SoundCloud is still getting ready".into());
        }
    }

    pub fn start_radio(&mut self) {
        if self.demo {
            if let Some(t) = self.current.clone() {
                self.demo_play_track(t, "GAYDAR LIVE · the agenda starts now");
                self.notify("the gaydar is live. behave.", ToastKind::Good);
            }
            return;
        }
        if self.booting {
            self.notify("still tuning, gorgeous", ToastKind::Info);
            return;
        }
        if self.current_source.is_some() || self.current.is_some() {
            self.toggle_play();
        } else {
            self.booting = true;
            self.error = None;
            if let Some(radio) = &self.radio {
                radio.send(RadioCommand::Bootstrap { count: 10 });
            }
        }
    }

    pub fn apply_station(&mut self, station: &Station) {
        self.dials = station.dials;
        if self.demo {
            let track = demo_tracks()
                .into_iter()
                .find(|t| t.tags.iter().any(|tag| station.query.contains(tag)))
                .or_else(|| demo_tracks().first().cloned());
            if let Some(track) = track {
                self.play_track(track, format!("{} · {}", station.name, station.blurb));
            }
            self.notify(
                format!("{} is now serving", station.name.to_lowercase()),
                ToastKind::Good,
            );
        } else {
            // Do not mark the player as loading until the worker returns a
            // playable station. A failed search must leave the current track
            // and transport state untouched.
            if let Some(radio) = &self.radio {
                radio.send(RadioCommand::Station {
                    name: station.name.to_string(),
                    queries: vec![
                        station.query.to_string(),
                        station.fallback_query.to_string(),
                    ],
                    dials: station.dials,
                });
            }
            self.notify(
                format!("tuning {}…", station.name.to_lowercase()),
                ToastKind::Info,
            );
        }
    }

    // -- live service --------------------------------------------------------

    pub fn start_live(&mut self, cx: &mut Context<Self>) {
        if self.started {
            return;
        }
        self.started = true;
        let data = std::env::var_os("GAYDAR_DATA_DIR")
            .map(PathBuf::from)
            .unwrap_or_else(|| {
                dirs::data_dir()
                    .unwrap_or_else(|| PathBuf::from("."))
                    .join("gaydar")
            });
        let cache = std::env::var_os("GAYDAR_CACHE_DIR")
            .map(PathBuf::from)
            .unwrap_or_else(|| {
                dirs::cache_dir()
                    .unwrap_or_else(|| PathBuf::from("."))
                    .join("gaydar")
                    .join("art")
            });
        let _ = std::fs::create_dir_all(&data);
        let _ = std::fs::create_dir_all(&cache);
        self.data_dir = data.clone();
        self.cache_dir = cache;
        self.settings_path = data.join("settings.json");

        let stored: StoredSettings = std::fs::read(&self.settings_path)
            .ok()
            .and_then(|bytes| serde_json::from_slice(&bytes).ok())
            .unwrap_or_default();
        if let Some(device) = stored.device {
            self.device = device;
        }
        if let Some(volume) = stored.volume {
            self.volume = volume.clamp(0.0, 1.0);
        }
        if let Ok(device) = std::env::var("GAYDAR_ALSA_DEVICE") {
            if !device.trim().is_empty() {
                self.device = device;
            }
        }

        self.devices = AudioEngine::list_devices();
        match AudioEngine::new(Some(&self.device), self.volume) {
            Ok(audio) => {
                self.playback = audio.status();
                self.audio = Some(audio);
            }
            Err(error) => {
                self.error = Some(format!("ALSA could not start: {error}"));
                self.notify(format!("ALSA is being difficult: {error}"), ToastKind::Bad);
            }
        }

        self.radio = Some(RadioService::new(cx.ui_sender(), data));
        if let Some(radio) = &self.radio {
            radio.send(RadioCommand::Bootstrap { count: 12 });
        }
        self.booting = true;
        self.client_ready = false;
        self.radio_status = "TUNING THE GAYDAR…".into();
    }

    pub fn pump(&mut self, _cx: &mut Context<Self>) {
        let events = self
            .radio
            .as_ref()
            .map(|radio| radio.drain())
            .unwrap_or_default();
        for event in events {
            self.handle_radio_event(event);
        }

        let mut audio_events = Vec::new();
        let mut latest_status = None;
        if let Some(audio) = &self.audio {
            while let Some(event) = audio.try_recv() {
                audio_events.push(event);
            }
            latest_status = Some(audio.status());
        }
        if let Some(status) = latest_status {
            self.playback = status;
        }
        for event in audio_events {
            self.handle_audio_event(event);
        }
    }

    fn handle_audio_event(&mut self, event: AudioEvent) {
        match event {
            AudioEvent::Finished(track_id) => {
                if self.current_id() == Some(track_id) && !self.loading_track {
                    self.playback.state = PlaybackState::Idle;
                    self.advance_live(true);
                }
            }
            AudioEvent::Error(message) => {
                eprintln!("[gaydar audio] {message}");
                self.loading_track = false;
                self.error = Some(message.clone());
                self.notify(format!("stream trouble: {message}"), ToastKind::Bad);
            }
        }
    }

    fn handle_radio_event(&mut self, event: RadioEvent) {
        match event {
            RadioEvent::Ready { client_id } => {
                self.client_ready = true;
                self.radio_status = format!(
                    "SOUNDCLOUD CONNECTED · {}",
                    &client_id[..client_id.len().min(6)]
                );
            }
            RadioEvent::Bootstrap {
                queue,
                autoplay,
                liked,
                history,
                taste,
                top_artists,
                total_plays,
                dials,
            } => {
                self.booting = false;
                self.loading_track = false;
                self.error = None;
                self.client_ready = true;
                self.queue = VecDeque::from(queue);
                self.liked = liked.iter().map(|t| t.id).collect();
                self.liked_tracks = liked;
                self.history = history;
                self.taste = taste;
                self.top_artists = top_artists;
                self.total_plays = total_plays;
                self.dials = dials;
                let queue_art: Vec<Track> =
                    self.queue.iter().map(|rec| rec.track.clone()).collect();
                for track in queue_art {
                    self.ensure_art(&track);
                }
                if let Some(auto) = autoplay {
                    self.start_autoplay(auto);
                } else if let Some(rec) = self.queue.pop_front() {
                    self.request_play(rec.track, rec.reason);
                } else {
                    self.error =
                        Some("the gaydar came back empty. the clubs must be closed.".into());
                }
            }
            RadioEvent::Station {
                name,
                queue,
                autoplay,
            } => {
                self.booting = false;
                self.loading_track = false;
                self.error = None;
                self.queue = VecDeque::from(queue);
                self.screen = Screen::Home;
                self.radio_status = format!("{} · LIVE", name.to_uppercase());
                self.notify(format!("{name} is now serving"), ToastKind::Good);
                let queue_art: Vec<Track> =
                    self.queue.iter().map(|rec| rec.track.clone()).collect();
                for track in queue_art {
                    self.ensure_art(&track);
                }
                if let Some(auto) = autoplay {
                    self.start_autoplay(auto);
                } else {
                    self.error = Some(format!("{name} came back empty"));
                }
            }
            RadioEvent::Refill { recs } => {
                self.refilling = false;
                for rec in &recs {
                    self.ensure_art(&rec.track);
                }
                let mut recs = recs;
                if self.needs_autoplay && !recs.is_empty() {
                    self.needs_autoplay = false;
                    let first = recs.remove(0);
                    self.queue.extend(recs);
                    self.request_play(first.track, first.reason);
                } else {
                    self.queue.extend(recs);
                }
            }
            RadioEvent::Search {
                generation,
                query,
                results,
                error,
            } => {
                if generation != self.search_generation {
                    return;
                }
                self.searching = false;
                if let Some(error) = error {
                    self.search_error = Some(error.clone());
                    self.notify(format!("search failed: {error}"), ToastKind::Bad);
                } else {
                    self.search_error = None;
                    self.search_results = results;
                    let search_art: Vec<Track> =
                        self.search_results.iter().take(16).cloned().collect();
                    for track in search_art {
                        self.ensure_art(&track);
                    }
                }
                let _ = query;
            }
            RadioEvent::StreamReady {
                token,
                track,
                source,
            } => {
                if token != self.playback_token {
                    return;
                }
                self.current = Some(track.clone());
                self.current_source = Some(source.clone());
                self.loading_track = false;
                self.error = None;
                self.consecutive_failures = 0;
                self.playback.track_id = Some(track.id);
                self.playback.duration_ms = track.duration_ms;
                self.playback.position_ms = 0;
                self.playback.error = None;
                self.ensure_art(&track);
                if let Some(audio) = &self.audio {
                    audio.play(track.id, source.url, track.duration_ms);
                }
                if let Some(radio) = &self.radio {
                    radio.send(RadioCommand::Played(track.clone()));
                }
                self.maybe_refill();
            }
            RadioEvent::StreamError {
                token,
                track,
                error,
                unavailable,
            } => {
                if token != self.playback_token {
                    return;
                }
                self.loading_track = false;
                self.consecutive_failures += 1;
                if unavailable && self.consecutive_failures <= 3 {
                    self.notify("DRM locked. not today, Satan.", ToastKind::Info);
                    self.current_source = None;
                    if let Some(rec) = self.queue.pop_front() {
                        self.request_play(rec.track, rec.reason);
                    } else {
                        self.needs_autoplay = true;
                        self.refilling = true;
                        if let Some(radio) = &self.radio {
                            radio.send(RadioCommand::Refill { count: 8 });
                        }
                    }
                } else {
                    self.error = Some(error.clone());
                    self.playback.state = PlaybackState::Error;
                    self.notify(
                        format!("could not play {}: {error}", track.title),
                        ToastKind::Bad,
                    );
                }
            }
            RadioEvent::Artwork { track_id, path } => {
                self.pending_art.remove(&track_id);
                self.art_cache.insert(track_id, path);
            }
            RadioEvent::ArtworkFailed { track_id, error } => {
                self.pending_art.remove(&track_id);
                let _ = error;
            }
            RadioEvent::Taste {
                liked,
                history,
                taste,
                top_artists,
                total_plays,
            } => {
                self.liked = liked.iter().map(|t| t.id).collect();
                self.liked_tracks = liked;
                self.history = history;
                self.taste = taste;
                self.top_artists = top_artists;
                self.total_plays = total_plays;
            }
            RadioEvent::Error(error) => {
                eprintln!("[gaydar radio] {error}");
                self.loading_track = false;
                if self.booting {
                    // Keep the boot screen up so it can show the error and a
                    // retry button instead of dropping into a fake home page.
                    self.error = Some(error);
                } else {
                    self.notify(error, ToastKind::Bad);
                }
            }
            RadioEvent::Info(text) => self.notify(text, ToastKind::Info),
        }
    }

    fn start_autoplay(&mut self, auto: Autoplay) {
        self.playback_token += 1;
        self.current = Some(auto.track.clone());
        self.current_reason = auto.reason;
        self.current_source = Some(auto.source.clone());
        self.loading_track = false;
        self.booting = false;
        self.needs_autoplay = false;
        self.error = None;
        self.playback.state = PlaybackState::Loading;
        self.playback.track_id = Some(auto.track.id);
        self.playback.position_ms = 0;
        self.playback.duration_ms = auto.track.duration_ms;
        self.playback.levels = [0.0; 32];
        self.ensure_art(&auto.track);
        if let Some(audio) = &self.audio {
            audio.play(auto.track.id, auto.source.url, auto.track.duration_ms);
        }
        if let Some(radio) = &self.radio {
            radio.send(RadioCommand::Played(auto.track));
        }
        self.maybe_refill();
    }

    fn request_play(&mut self, track: Track, reason: String) {
        self.playback_token += 1;
        let token = self.playback_token;
        self.current = Some(track.clone());
        self.current_reason = reason.clone();
        self.current_source = None;
        self.loading_track = true;
        self.error = None;
        self.needs_autoplay = false;
        self.playback.state = PlaybackState::Loading;
        self.playback.track_id = Some(track.id);
        self.playback.position_ms = 0;
        self.playback.duration_ms = track.duration_ms;
        self.playback.levels = [0.0; 32];
        if let Some(audio) = &self.audio {
            audio.stop();
        }
        self.ensure_art(&track);
        if let Some(radio) = &self.radio {
            radio.send(RadioCommand::Play {
                token,
                track,
                reason,
            });
        }
    }

    fn maybe_refill(&mut self) {
        if self.queue.len() < 4 && !self.refilling {
            if let Some(radio) = &self.radio {
                self.refilling = true;
                radio.send(RadioCommand::Refill { count: 8 });
            }
        }
    }

    pub fn advance_live(&mut self, completed: bool) {
        if let Some(current) = self.current.clone() {
            let fraction = if self.playback.duration_ms > 0 {
                self.playback.position_ms as f32 / self.playback.duration_ms as f32
            } else {
                0.0
            };
            if let Some(radio) = &self.radio {
                if completed {
                    radio.send(RadioCommand::Complete(current));
                } else {
                    radio.send(RadioCommand::Skip {
                        track: current,
                        fraction,
                    });
                }
            }
        }
        self.current_source = None;
        if let Some(rec) = self.queue.pop_front() {
            self.request_play(rec.track, rec.reason);
            self.maybe_refill();
        } else {
            self.needs_autoplay = true;
            self.refilling = true;
            self.loading_track = true;
            self.playback.state = PlaybackState::Loading;
            if let Some(radio) = &self.radio {
                radio.send(RadioCommand::Refill { count: 8 });
            }
            self.notify("the gaydar is digging for more…", ToastKind::Info);
        }
    }

    pub fn retry_boot(&mut self) {
        self.booting = true;
        self.error = None;
        self.loading_track = true;
        self.client_ready = false;
        if let Some(radio) = &self.radio {
            radio.send(RadioCommand::Retry);
        }
    }

    fn ensure_art(&mut self, track: &Track) {
        if self.cache_dir.as_os_str().is_empty()
            || self.art_cache.contains_key(&track.id)
            || self.pending_art.contains(&track.id)
        {
            return;
        }
        let Some(url) = track.artwork_large().or_else(|| track.artwork_url.clone()) else {
            return;
        };
        if url.trim().is_empty() {
            return;
        }
        let dest = self.cache_dir.join(format!("{}.jpg", track.id));
        if dest.is_file() {
            self.art_cache
                .insert(track.id, dest.to_string_lossy().into_owned());
            return;
        }
        self.pending_art.insert(track.id);
        if let Some(radio) = &self.radio {
            radio.send(RadioCommand::Artwork {
                track_id: track.id,
                url,
                dest,
            });
        }
    }

    fn save_settings(&self) {
        if self.settings_path.as_os_str().is_empty() {
            return;
        }
        if let Some(parent) = self.settings_path.parent() {
            let _ = std::fs::create_dir_all(parent);
        }
        let stored = StoredSettings {
            device: Some(self.device.clone()),
            volume: Some(self.volume),
        };
        if let Ok(json) = serde_json::to_vec_pretty(&stored) {
            let _ = std::fs::write(&self.settings_path, json);
        }
    }

    pub fn station_for(screen: Screen) -> Option<&'static Station> {
        let _ = screen;
        None
    }
}

pub fn demo_tracks() -> Vec<Track> {
    let rows = [
        (
            1u64,
            "PINK PONY CLUB",
            "Chappell Roan",
            231_000,
            "camp",
            "Certified camp canon. The gaydar is weeping.",
        ),
        (
            2,
            "IMMATERIAL",
            "SOPHIE",
            228_000,
            "hyper",
            "Hyperpop communion. Chrome, but make it holy.",
        ),
        (
            3,
            "SILK CHIFFON",
            "MUNA",
            203_000,
            "soft",
            "Sapphic slow burn. Soft serve, extra feelings.",
        ),
        (
            4,
            "VOGUE (STRIKE A POSE)",
            "BALLROOM 4AM",
            302_000,
            "ballroom",
            "Certified ballroom. The floor is yours.",
        ),
        (
            5,
            "HEAVEN",
            "Troye Sivan",
            196_000,
            "club",
            "Club kid catechism. Hands up, little angel.",
        ),
        (
            6,
            "PADAM PADAM",
            "Kim Petras",
            172_000,
            "hyper",
            "Because you keep replaying diva pop.",
        ),
        (
            7,
            "MOTORSPORT",
            "Shygirl",
            189_000,
            "club",
            "The gaydar says: floor, immediately.",
        ),
        (
            8,
            "GOOD LUCK, BABE!",
            "Chappell Roan",
            217_000,
            "camp",
            "Camp canon · a little dramatic, on purpose.",
        ),
        (
            9,
            "AEON",
            "Kelela",
            244_000,
            "soft",
            "Soft sapphic detour. Aquatic, expensive.",
        ),
        (
            10,
            "QUEEN OF THE NIGHT",
            "Pabllo Vittar",
            168_000,
            "club",
            "Brazilian gay club. You're welcome.",
        ),
        (
            11,
            "SPELL",
            "Arca",
            201_000,
            "chaos",
            "Chaos dial at 11. Hold on to your wig.",
        ),
        (
            12,
            "TINY DANCER",
            "Elton John",
            372_000,
            "camp",
            "Auntie canon. Everybody's favourite gay uncle.",
        ),
    ];
    rows.iter()
        .map(|(id, title, artist, dur, genre, _reason)| Track {
            id: *id,
            title: (*title).into(),
            artist: (*artist).into(),
            artist_id: Some(1000 + *id),
            artist_avatar_url: None,
            artwork_url: None,
            duration_ms: *dur,
            genre: Some((*genre).into()),
            tags: vec![(*genre).into(), "queer".into(), "gay".into()],
            description: None,
            playback_count: 120_000 + id * 9_221,
            likes_count: 8_000 + id * 211,
            user_followers: 5_000 + id * 800,
            created_at: Some("2026-10-01T00:00:00Z".into()),
            permalink_url: format!("https://soundcloud.com/demo/track-{id}"),
            streamable: true,
            policy: "ALLOW".into(),
        })
        .collect()
}

pub fn demo_reasons() -> [&'static str; 8] {
    [
        "camp canon · the gaydar is weeping",
        "because you keep replaying diva pop",
        "certified ballroom · the floor is yours",
        "soft sapphic detour, handle with care",
        "the gaydar says: explore, whore",
        "high chaos, high camp, high ponytail",
        "a little trans joy for your afternoon",
        "this one came out of the club with you",
    ]
}

pub fn demo_levels() -> [f32; 32] {
    let seed = [
        0.82, 0.58, 0.91, 0.44, 0.72, 0.36, 0.64, 0.88, 0.51, 0.76, 0.40, 0.93, 0.62, 0.28, 0.71,
        0.55, 0.85, 0.34, 0.67, 0.48, 0.78, 0.24, 0.59, 0.83, 0.41, 0.69, 0.52, 0.87, 0.31, 0.74,
        0.46, 0.61,
    ];
    seed
}

pub fn demo_cover_class(id: u64) -> &'static str {
    match id % 6 {
        0 => "cover-a",
        1 => "cover-b",
        2 => "cover-c",
        3 => "cover-d",
        4 => "cover-e",
        _ => "cover-f",
    }
}

pub fn status_label(state: PlaybackState) -> &'static str {
    match state {
        PlaybackState::Idle => "idle",
        PlaybackState::Loading => "tuning in",
        PlaybackState::Playing => "on air",
        PlaybackState::Paused => "paused, diva",
        PlaybackState::Error => "technical difficulties",
    }
}
