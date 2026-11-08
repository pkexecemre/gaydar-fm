//! Background SoundCloud + Gaydar worker.
//!
//! The native UI must never block on the network, so this thread owns the
//! SoundCloud client and the recommendation engine. Results are pushed into a
//! shared queue and the UI is woken with BUI's UiSender.

use crate::gaydar::GaydarEngine;
use crate::model::{Dials, Recommendation, StreamSource, Track};
use crate::soundcloud::{SoundCloudClient, StreamUnavailable};
use anyhow::Result;
use blenderui::prelude::*;
use std::collections::VecDeque;
use std::path::{Path, PathBuf};
use std::sync::mpsc::{self, Sender};
use std::sync::{Arc, Mutex};
use std::thread;

#[derive(Debug)]
pub struct Autoplay {
    pub track: Track,
    pub reason: String,
    pub source: StreamSource,
}

#[derive(Debug)]
pub enum RadioEvent {
    Ready {
        client_id: String,
    },
    Bootstrap {
        queue: Vec<Recommendation>,
        autoplay: Option<Autoplay>,
        liked: Vec<Track>,
        history: Vec<Track>,
        taste: Vec<(String, f32)>,
        top_artists: Vec<String>,
        total_plays: u64,
        dials: Dials,
    },
    Refill {
        recs: Vec<Recommendation>,
    },
    Station {
        name: String,
        queue: Vec<Recommendation>,
        autoplay: Option<Autoplay>,
    },
    Search {
        generation: u64,
        query: String,
        results: Vec<Track>,
        error: Option<String>,
    },
    StreamReady {
        token: u64,
        track: Track,
        source: StreamSource,
    },
    StreamError {
        token: u64,
        track: Track,
        error: String,
        unavailable: bool,
    },
    Artwork {
        track_id: u64,
        path: String,
    },
    ArtworkFailed {
        track_id: u64,
        error: String,
    },
    Taste {
        liked: Vec<Track>,
        history: Vec<Track>,
        taste: Vec<(String, f32)>,
        top_artists: Vec<String>,
        total_plays: u64,
    },
    Error(String),
    Info(String),
}

pub enum RadioCommand {
    Bootstrap {
        count: usize,
    },
    Refill {
        count: usize,
    },
    Station {
        name: String,
        queries: Vec<String>,
        dials: Dials,
    },
    Search {
        generation: u64,
        query: String,
    },
    Play {
        token: u64,
        track: Track,
        reason: String,
    },
    Played(Track),
    Complete(Track),
    Skip {
        track: Track,
        fraction: f32,
    },
    Like(Track),
    Unlike(Track),
    SetDials(Dials),
    Artwork {
        track_id: u64,
        url: String,
        dest: PathBuf,
    },
    Retry,
    Shutdown,
}

pub struct RadioService {
    tx: Sender<RadioCommand>,
    events: Arc<Mutex<VecDeque<RadioEvent>>>,
    join: Option<thread::JoinHandle<()>>,
}

impl RadioService {
    pub fn new(ui: UiSender, state_dir: PathBuf) -> Self {
        let (tx, rx) = mpsc::channel::<RadioCommand>();
        let events = Arc::new(Mutex::new(VecDeque::new()));
        let event_sink = Arc::clone(&events);
        let join = thread::Builder::new()
            .name("gaydar-radio".into())
            .spawn(move || {
                let mut worker = Worker::new(ui, state_dir, event_sink);
                worker.run(rx);
            })
            .ok();
        Self { tx, events, join }
    }

    pub fn send(&self, command: RadioCommand) {
        let _ = self.tx.send(command);
    }

    pub fn drain(&self) -> Vec<RadioEvent> {
        let mut queue = match self.events.lock() {
            Ok(q) => q,
            Err(p) => p.into_inner(),
        };
        queue.drain(..).collect()
    }
}

impl Drop for RadioService {
    fn drop(&mut self) {
        let _ = self.tx.send(RadioCommand::Shutdown);
        if let Some(join) = self.join.take() {
            let _ = join.join();
        }
    }
}

struct Worker {
    ui: UiSender,
    state_dir: PathBuf,
    events: Arc<Mutex<VecDeque<RadioEvent>>>,
    client: Option<Arc<SoundCloudClient>>,
    engine: Option<GaydarEngine>,
    last_error: Option<String>,
}

impl Worker {
    fn new(ui: UiSender, state_dir: PathBuf, events: Arc<Mutex<VecDeque<RadioEvent>>>) -> Self {
        Self {
            ui,
            state_dir,
            events,
            client: None,
            engine: None,
            last_error: None,
        }
    }

    fn push(&self, event: RadioEvent) {
        if let Ok(mut q) = self.events.lock() {
            q.push_back(event);
        }
        let _ = self.ui.send(|cx| cx.notify_all());
    }

    fn initialize(&mut self) -> Result<()> {
        if self.client.is_some() {
            return Ok(());
        }
        let client = Arc::new(SoundCloudClient::new()?);
        let state_path = self.state_dir.join("state.json");
        let engine = GaydarEngine::new(Arc::clone(&client), state_path);
        let client_id = client.client_id();
        self.client = Some(client);
        self.engine = Some(engine);
        self.push(RadioEvent::Ready { client_id });
        Ok(())
    }

    fn snapshot(
        &self,
    ) -> (
        Vec<Track>,
        Vec<Track>,
        Vec<(String, f32)>,
        Vec<String>,
        u64,
        Dials,
    ) {
        let engine = self.engine.as_ref();
        match engine {
            Some(e) => (
                e.liked_tracks(),
                e.history(),
                e.taste_summary(),
                e.top_artists(5),
                e.total_plays(),
                e.dials(),
            ),
            None => (
                Vec::new(),
                Vec::new(),
                Vec::new(),
                Vec::new(),
                0,
                Dials::default(),
            ),
        }
    }

    fn resolve_first(
        &self,
        mut picks: Vec<Recommendation>,
    ) -> (Option<Autoplay>, Vec<Recommendation>) {
        let Some(client) = &self.client else {
            return (None, picks);
        };
        for _ in 0..picks.len().min(7) {
            let rec = picks.remove(0);
            match client.resolve_stream(rec.track.id) {
                Ok(source) => {
                    return (
                        Some(Autoplay {
                            track: rec.track,
                            reason: rec.reason,
                            source,
                        }),
                        picks,
                    );
                }
                Err(_error) => {}
            }
        }
        (None, picks)
    }

    fn playable_picks(
        &self,
        mut tracks: Vec<Track>,
        reason: &str,
        limit: usize,
    ) -> Vec<Recommendation> {
        tracks.truncate(limit);
        tracks
            .into_iter()
            .map(|track| Recommendation {
                track,
                reason: reason.to_string(),
                score: 0.8,
            })
            .collect()
    }

    fn bootstrap(&mut self, count: usize) {
        if let Err(error) = self.initialize() {
            self.push(RadioEvent::Error(format!(
                "could not reach SoundCloud: {error}"
            )));
            return;
        }
        let Some(engine) = &mut self.engine else {
            return;
        };
        match engine.bootstrap(count) {
            Ok(picks) => {
                let (autoplay, queue) = self.resolve_first(picks);
                let (liked, history, taste, top_artists, total_plays, dials) = self.snapshot();
                self.push(RadioEvent::Bootstrap {
                    queue,
                    autoplay,
                    liked,
                    history,
                    taste,
                    top_artists,
                    total_plays,
                    dials,
                });
            }
            Err(error) => self.push(RadioEvent::Error(format!(
                "could not tune the gaydar: {error}"
            ))),
        }
    }

    fn refill(&mut self, count: usize) {
        let Some(engine) = &mut self.engine else {
            return;
        };
        match engine.refill(count) {
            Ok(recs) => self.push(RadioEvent::Refill { recs }),
            Err(error) => self.push(RadioEvent::Error(format!("gaydar refill failed: {error}"))),
        }
    }

    fn station(&mut self, name: String, queries: Vec<String>, dials: Dials) {
        let Some(engine) = &mut self.engine else {
            return;
        };
        engine.set_dials(dials);
        let Some(client) = &self.client else { return };
        let mut last_error: Option<String> = None;
        for query in queries {
            match client.search_tracks(&query, 50) {
                Ok(tracks) if !tracks.is_empty() => {
                    let picks = self.playable_picks(
                        tracks,
                        &format!("{name} · hand-picked by the gaydar"),
                        12,
                    );
                    let (autoplay, queue) = self.resolve_first(picks);
                    if let Some(autoplay) = autoplay {
                        self.push(RadioEvent::Station {
                            name,
                            queue,
                            autoplay: Some(autoplay),
                        });
                        return;
                    }
                    last_error = Some("the search results were all DRM-locked".into());
                }
                Ok(_) => last_error = Some(format!("no playable results for {query:?}")),
                Err(error) => last_error = Some(error.to_string()),
            }
        }
        self.push(RadioEvent::Error(format!(
            "{name} is quiet right now ({}). Try another station.",
            last_error.unwrap_or_else(|| "no playable tracks".into())
        )));
    }

    fn search(&mut self, generation: u64, query: String) {
        if query.trim().is_empty() {
            self.push(RadioEvent::Search {
                generation,
                query,
                results: Vec::new(),
                error: None,
            });
            return;
        }
        let Some(client) = &self.client else {
            self.push(RadioEvent::Search {
                generation,
                query,
                results: Vec::new(),
                error: Some("SoundCloud client is not ready".into()),
            });
            return;
        };
        match client.search_tracks(&query, 50) {
            Ok(results) => self.push(RadioEvent::Search {
                generation,
                query,
                results,
                error: None,
            }),
            Err(error) => self.push(RadioEvent::Search {
                generation,
                query,
                results: Vec::new(),
                error: Some(error.to_string()),
            }),
        }
    }

    fn play(&mut self, token: u64, track: Track, _reason: String) {
        let Some(client) = &self.client else {
            self.push(RadioEvent::StreamError {
                token,
                track,
                error: "SoundCloud client is not ready".into(),
                unavailable: false,
            });
            return;
        };
        match client.resolve_stream(track.id) {
            Ok(source) => self.push(RadioEvent::StreamReady {
                token,
                track,
                source,
            }),
            Err(error) => {
                let unavailable = error.downcast_ref::<StreamUnavailable>().is_some();
                self.push(RadioEvent::StreamError {
                    token,
                    track,
                    error: error.to_string(),
                    unavailable,
                });
            }
        }
    }

    fn taste(&mut self) {
        let (liked, history, taste, top_artists, total_plays, _dials) = self.snapshot();
        self.push(RadioEvent::Taste {
            liked,
            history,
            taste,
            top_artists,
            total_plays,
        });
    }

    fn artwork(&mut self, track_id: u64, url: String, dest: PathBuf) {
        let Some(client) = &self.client else { return };
        if let Some(parent) = dest.parent() {
            let _ = std::fs::create_dir_all(parent);
        }
        match client.fetch_artwork(&url, Path::new(&dest)) {
            Ok(path) => self.push(RadioEvent::Artwork {
                track_id,
                path: path.to_string_lossy().into_owned(),
            }),
            Err(error) => self.push(RadioEvent::ArtworkFailed {
                track_id,
                error: error.to_string(),
            }),
        }
    }

    fn run(&mut self, rx: mpsc::Receiver<RadioCommand>) {
        if let Err(error) = self.initialize() {
            self.last_error = Some(error.to_string());
            self.push(RadioEvent::Error(format!(
                "could not reach SoundCloud: {error}"
            )));
        }
        while let Ok(command) = rx.recv() {
            match command {
                RadioCommand::Shutdown => break,
                RadioCommand::Retry => self.bootstrap(10),
                RadioCommand::Bootstrap { count } => self.bootstrap(count),
                RadioCommand::Refill { count } => self.refill(count),
                RadioCommand::Station {
                    name,
                    queries,
                    dials,
                } => self.station(name, queries, dials),
                RadioCommand::Search { generation, query } => self.search(generation, query),
                RadioCommand::Play {
                    token,
                    track,
                    reason,
                } => self.play(token, track, reason),
                RadioCommand::Played(track) => {
                    if let Some(engine) = &mut self.engine {
                        engine.record_play(&track);
                    }
                    self.taste();
                }
                RadioCommand::Complete(track) => {
                    if let Some(engine) = &mut self.engine {
                        engine.record_complete(&track);
                    }
                    self.taste();
                }
                RadioCommand::Skip { track, fraction } => {
                    if let Some(engine) = &mut self.engine {
                        engine.record_skip(&track, fraction);
                    }
                    self.taste();
                }
                RadioCommand::Like(track) => {
                    if let Some(engine) = &mut self.engine {
                        engine.record_like(&track);
                    }
                    self.taste();
                }
                RadioCommand::Unlike(track) => {
                    if let Some(engine) = &mut self.engine {
                        engine.record_unlike(&track);
                    }
                    self.taste();
                }
                RadioCommand::SetDials(dials) => {
                    if let Some(engine) = &mut self.engine {
                        engine.set_dials(dials);
                    }
                }
                RadioCommand::Artwork {
                    track_id,
                    url,
                    dest,
                } => self.artwork(track_id, url, dest),
            }
        }
    }
}
