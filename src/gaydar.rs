//! GAYDAR's adaptive queer radio engine.
//!
//! The engine keeps a candidate pool built from curated queer searches,
//! related tracks, artist uploads, and tag searches derived from what the
//! listener actually plays. Every candidate is scored with an explainable
//! blend of tag affinity, artist affinity, a curated queer lexicon, dial fit,
//! popularity, freshness, novelty, and diversity. Each pick carries a human
//! reason, and everything the algorithm learns is persisted as JSON.

use crate::model::{Dials, Recommendation, Track};
use crate::soundcloud::{SoundCloudClient, StreamUnavailable, is_playable};
use anyhow::{Context, Result, anyhow, bail};
use chrono::{DateTime, Utc};
use once_cell::sync::Lazy;
use rand::{Rng, RngExt};
use regex::Regex;
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet, HashMap, HashSet};
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::mpsc;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

const STATE_VERSION: u32 = 1;
const MAX_HISTORY: usize = 300;
const MAX_LIKED: usize = 500;
const MAX_SKIPPED: usize = 3000;
const MAX_POOL: usize = 400;
const MAX_WEIGHT: f32 = 10.0;
const BOOTSTRAP_BUDGET: Duration = Duration::from_secs(18);
const REFILL_BUDGET: Duration = Duration::from_secs(10);

// ---------------------------------------------------------------------------
// Curated seeds
// ---------------------------------------------------------------------------

const CORE_SEEDS: &[&str] = &[
    "queer anthem",
    "queer",
    "lgbtq",
    "pride",
    "drag queen",
    "ballroom",
    "hyperpop",
    "sapphic",
    "wlw",
    "gay club",
    "trans joy",
    "queercore",
    "vogue",
    "camp",
    "queer pop",
    "queer electronic",
];

const CAMP_SEEDS: &[&str] = &[
    "camp",
    "drag queen",
    "pride anthem",
    "queer pop",
    "diva pop",
    "hyperpop",
    "queer anthem",
    "drag race",
    "eurovision queer",
    "bubblegum pop",
];

const CLUB_SEEDS: &[&str] = &[
    "gay club",
    "ballroom",
    "vogue",
    "queer house",
    "queer techno",
    "ballroom beats",
    "pride",
    "honey dijon",
    "jodie harsh",
    "gorgon city",
];

const SOFT_SEEDS: &[&str] = &[
    "sapphic",
    "wlw",
    "lesbian indie",
    "queer folk",
    "soft queer",
    "trans joy",
    "queer r&b",
    "sad queer",
    "kelela",
    "perfume genius",
];

const CHAOS_SEEDS: &[&str] = &[
    "hyperpop",
    "queercore",
    "queer punk",
    "experimental queer",
    "SOPHIE",
    "Arca",
    "100 gecs",
    "deconstructed club",
    "glitch queer",
];

/// Artists that anchor the radio. They double as artist search seeds and as a
/// strong queer signal even when an upload has no useful tags.
const ARTIST_SEEDS: &[&str] = &[
    "Chappell Roan",
    "Rina Sawayama",
    "MUNA",
    "Troye Sivan",
    "SOPHIE",
    "Kim Petras",
    "Slayyyter",
    "Pabllo Vittar",
    "Arca",
    "Kehlani",
    "Janelle Monáe",
    "Janelle Monae",
    "Omar Apollo",
    "MNEK",
    "Kelela",
    "Shygirl",
    "COBRAH",
    "Tove Lo",
    "SG Lewis",
    "Jodie Harsh",
    "Gorgon City",
    "Lil Nas X",
    "Sam Smith",
    "Hayley Kiyoko",
    "King Princess",
    "girl in red",
    "Fletcher",
    "Dorian Electra",
    "100 gecs",
    "Honey Dijon",
    "The Blessed Madonna",
    "Romy",
    "Olly Alexander",
    "Years & Years",
    "Perfume Genius",
    "Anohni",
    "Christine and the Queens",
    "Tegan and Sara",
    "Beth Ditto",
    "Gossip",
    "Syd",
    "Frank Ocean",
    "Steve Lacy",
    "Lady Gaga",
    "Madonna",
    "Charli XCX",
    "Caroline Polachek",
    "Shura",
    "Mabel",
];

// ---------------------------------------------------------------------------
// Lexicons
// ---------------------------------------------------------------------------

const QUEER_STRONG_TERMS: &[&str] = &[
    "queer",
    "lgbt",
    "lgbtq",
    "lgbtqia",
    "pride",
    "sapphic",
    "wlw",
    "lesbian",
    "dyke",
    "drag",
    "drag queen",
    "drag king",
    "ballroom",
    "vogue",
    "voguing",
    "trans",
    "transgender",
    "trans joy",
    "nonbinary",
    "non-binary",
    "genderfluid",
    "gender fluid",
    "genderqueer",
    "intersex",
    "two-spirit",
    "butch",
    "femme",
    "queercore",
    "homocore",
    "coming out",
    "same-sex",
    "bisexual",
    "pansexual",
    "asexual",
    "aromantic",
    "queer anthem",
    "gay anthem",
    "pride anthem",
];

const QUEER_MILD_TERMS: &[&str] = &[
    "gay",
    "bi",
    "pan",
    "ace",
    "aro",
    "enby",
    "homo",
    "homosexual",
    "camp",
    "kitsch",
    "diva",
    "slay",
    "hyperpop",
    "polyamorous",
    "poly",
    "leather",
    "bear",
    "twink",
    "doll",
    "the dolls",
    "yas",
    "yass",
    "mother",
    "chosen family",
    "love is love",
    "born this way",
];

const CAMP_TERMS: &[&str] = &[
    "camp",
    "kitsch",
    "diva",
    "drag",
    "disco",
    "musical",
    "theatre",
    "theater",
    "parody",
    "bubblegum",
    "hyperpop",
    "eurovision",
    "abba",
    "girl group",
    "pop princess",
    "campy",
    "cabaret",
    "burlesque",
    "drag race",
    "slay",
    "pink",
];

const CLUB_TERMS: &[&str] = &[
    "club",
    "house",
    "techno",
    "dance",
    "rave",
    "edm",
    "bass",
    "garage",
    "drum and bass",
    "dnb",
    "jersey club",
    "ballroom",
    "vogue",
    "voguing",
    "tech house",
    "deep house",
    "disco house",
    "ghetto house",
    "acid",
    "trance",
    "electro",
    "party",
    "bootleg",
    "bpm",
    "mix",
];

const SOFT_TERMS: &[&str] = &[
    "soft",
    "ambient",
    "acoustic",
    "ballad",
    "piano",
    "dreamy",
    "dream pop",
    "r&b",
    "soul",
    "lo-fi",
    "lofi",
    "sad",
    "indie",
    "singer-songwriter",
    "folk",
    "gentle",
    "slow",
    "intimate",
    "bedroom pop",
    "shoegaze",
    "ethereal",
    "hushed",
];

const CHAOS_TERMS: &[&str] = &[
    "experimental",
    "noise",
    "glitch",
    "breakcore",
    "hyperpop",
    "industrial",
    "hardcore",
    "speedcore",
    "deconstructed",
    "avant",
    "pc music",
    "digicore",
    "hexd",
    "scene",
    "screamo",
    "metal",
    "punk",
    "distorted",
    "chaotic",
    "weird",
];

/// Build a case-insensitive, whole-word alternation for a lexicon.
fn terms_regex(terms: &[&str]) -> Regex {
    let alternatives: Vec<String> = terms.iter().map(|term| regex::escape(term)).collect();
    Regex::new(&format!(r"(?i)\b(?:{})\b", alternatives.join("|")))
        .expect("lexicon regex is built from escaped literals")
}

static QUEER_STRONG_RE: Lazy<Regex> = Lazy::new(|| terms_regex(QUEER_STRONG_TERMS));
static QUEER_MILD_RE: Lazy<Regex> = Lazy::new(|| terms_regex(QUEER_MILD_TERMS));
static CAMP_RE: Lazy<Regex> = Lazy::new(|| terms_regex(CAMP_TERMS));
static CLUB_RE: Lazy<Regex> = Lazy::new(|| terms_regex(CLUB_TERMS));
static SOFT_RE: Lazy<Regex> = Lazy::new(|| terms_regex(SOFT_TERMS));
static CHAOS_RE: Lazy<Regex> = Lazy::new(|| terms_regex(CHAOS_TERMS));
static QUEER_ARTIST_KEYS: Lazy<HashSet<String>> = Lazy::new(|| {
    ARTIST_SEEDS
        .iter()
        .map(|name| normalize_name(name))
        .collect()
});

// ---------------------------------------------------------------------------
// Persisted state
// ---------------------------------------------------------------------------

fn now_unix() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_secs() as i64)
        .unwrap_or(0)
}

fn now_nanos() -> u128 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_nanos())
        .unwrap_or(0)
}

fn default_version() -> u32 {
    STATE_VERSION
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
enum PlayOutcome {
    #[default]
    Started,
    Completed,
    Skipped,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
struct HistoryEntry {
    track: Track,
    #[serde(default)]
    played_at: i64,
    #[serde(default)]
    fraction: f32,
    #[serde(default)]
    outcome: PlayOutcome,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
struct PersistedState {
    #[serde(default = "default_version")]
    version: u32,
    #[serde(default)]
    liked: Vec<Track>,
    #[serde(default)]
    history: Vec<HistoryEntry>,
    #[serde(default)]
    tag_weights: BTreeMap<String, f32>,
    #[serde(default)]
    artist_weights: BTreeMap<String, f32>,
    #[serde(default)]
    skipped: Vec<u64>,
    #[serde(default)]
    dials: Dials,
    #[serde(default)]
    plays: u64,
    #[serde(default)]
    completions: u64,
    #[serde(default)]
    last_updated: i64,
}

impl Default for PersistedState {
    fn default() -> Self {
        Self {
            version: STATE_VERSION,
            liked: Vec::new(),
            history: Vec::new(),
            tag_weights: BTreeMap::new(),
            artist_weights: BTreeMap::new(),
            skipped: Vec::new(),
            dials: Dials::default(),
            plays: 0,
            completions: 0,
            last_updated: now_unix(),
        }
    }
}

// ---------------------------------------------------------------------------
// Candidate pool
// ---------------------------------------------------------------------------

#[derive(Clone, Debug)]
struct Candidate {
    track: Track,
    source: CandidateSource,
    added_at: Instant,
}

#[derive(Clone, Debug)]
enum CandidateSource {
    Seed(String),
    Related(String),
    Artist(String),
    Tag(String),
    Chart,
}

impl CandidateSource {
    fn bonus(&self) -> f32 {
        match self {
            Self::Seed(_) => 0.0,
            Self::Related(_) => 0.12,
            Self::Artist(_) => 0.06,
            Self::Tag(_) => 0.05,
            Self::Chart => 0.03,
        }
    }
}

#[derive(Clone, Debug)]
enum FetchJob {
    Search(String),
    TagSearch(String),
    Related { track_id: u64, artist: String },
    ArtistTracks { user_id: u64, artist: String },
    Trending,
}

struct Fetched {
    job: FetchJob,
    result: Result<Vec<Track>>,
}

fn source_for_job(job: &FetchJob) -> CandidateSource {
    match job {
        FetchJob::Search(query) => CandidateSource::Seed(query.clone()),
        FetchJob::TagSearch(tag) => CandidateSource::Tag(tag.clone()),
        FetchJob::Related { artist, .. } => CandidateSource::Related(artist.clone()),
        FetchJob::ArtistTracks { artist, .. } => CandidateSource::Artist(artist.clone()),
        FetchJob::Trending => CandidateSource::Chart,
    }
}

// ---------------------------------------------------------------------------
// Engine
// ---------------------------------------------------------------------------

/// The stateful Gaydar radio.
///
/// The engine is intentionally synchronous: the UI can call bootstrap or
/// refill from a worker thread while it keeps painting. All network work is
/// bounded by an internal deadline so a sick connection cannot wedge the
/// caller.
pub struct GaydarEngine {
    client: Arc<SoundCloudClient>,
    state_path: PathBuf,
    state: PersistedState,
    pool: Vec<Candidate>,
    served: BTreeSet<u64>,
    last_refill: Option<Instant>,
}

impl GaydarEngine {
    /// Load the persisted taste state and create an empty candidate pool.
    ///
    /// A missing, corrupt, or unreadable state file degrades to a fresh start;
    /// the previous file is moved aside instead of being deleted.
    pub fn new(client: Arc<SoundCloudClient>, state_path: PathBuf) -> Self {
        let state = load_state(&state_path);
        Self {
            client,
            state_path,
            state,
            pool: Vec::new(),
            served: BTreeSet::new(),
            last_refill: None,
        }
    }

    /// The underlying SoundCloud client.
    pub fn client(&self) -> &Arc<SoundCloudClient> {
        &self.client
    }

    /// The current recommendation dials.
    pub fn dials(&self) -> Dials {
        self.state.dials
    }

    /// Replace the dials and persist them.
    pub fn set_dials(&mut self, dials: Dials) {
        self.state.dials = dials.clamped();
        self.state.last_updated = now_unix();
        let _ = self.save();
    }

    /// Build the first batch of the session.
    ///
    /// Several seed searches run in parallel, bounded by an overall budget.
    /// The pool is topped up with the trending chart or one last broad search
    /// only when the parallel searches produced nothing, so a healthy network
    /// costs four requests and roughly a second.
    pub fn bootstrap(&mut self, count: usize) -> Result<Vec<Recommendation>> {
        let count = count.clamp(1, 50);
        self.prune_pool();
        let started = Instant::now();
        let mut errors: Vec<String> = Vec::new();

        let minimum = count.min(3);
        if self.available_count() < count {
            let queries = self.bootstrap_queries();
            let used_queries = queries.clone();
            let jobs: Vec<FetchJob> = queries.into_iter().map(FetchJob::Search).collect();
            let expected = jobs.len();
            let fetched = self.run_jobs(
                jobs,
                50,
                remaining_budget(started, BOOTSTRAP_BUDGET)
                    .min(Duration::from_secs(10))
                    .max(Duration::from_millis(500)),
            );
            if fetched.len() < expected {
                errors.push("timed out waiting for SoundCloud searches".to_string());
            }
            for item in fetched {
                let source = source_for_job(&item.job);
                match item.result {
                    Ok(tracks) => self.merge_tracks(tracks, source),
                    Err(error) => errors.push(error.to_string()),
                }
            }

            if self.available_count() < minimum
                && !remaining_budget(started, BOOTSTRAP_BUDGET).is_zero()
            {
                let fetched = self.run_jobs(
                    vec![FetchJob::Trending],
                    50,
                    remaining_budget(started, BOOTSTRAP_BUDGET).min(Duration::from_secs(5)),
                );
                for item in fetched {
                    match item.result {
                        Ok(tracks) => self.merge_tracks(tracks, CandidateSource::Chart),
                        Err(error) => errors.push(error.to_string()),
                    }
                }
            }

            if self.available_count() < minimum
                && !remaining_budget(started, BOOTSTRAP_BUDGET).is_zero()
            {
                let fallback_query = ["pride", "queer", "queer punk", "drag queen", "sapphic"]
                    .into_iter()
                    .find(|candidate| {
                        !used_queries
                            .iter()
                            .any(|used| used.eq_ignore_ascii_case(candidate))
                    })
                    .unwrap_or("queer");
                let fetched = self.run_jobs(
                    vec![FetchJob::Search(fallback_query.to_string())],
                    50,
                    remaining_budget(started, BOOTSTRAP_BUDGET).min(Duration::from_secs(5)),
                );
                for item in fetched {
                    match item.result {
                        Ok(tracks) => self.merge_tracks(
                            tracks,
                            CandidateSource::Seed(fallback_query.to_string()),
                        ),
                        Err(error) => errors.push(error.to_string()),
                    }
                }
            }
        }

        let selected = self.select(count);
        let verify_budget = remaining_budget(started, BOOTSTRAP_BUDGET).min(Duration::from_secs(8));
        let picks = self.verify_bootstrap_picks(selected, verify_budget);
        if picks.is_empty() {
            let detail = errors
                .into_iter()
                .next()
                .unwrap_or_else(|| "SoundCloud returned no usable tracks".to_string());
            bail!("could not start the Gaydar radio: {detail}");
        }
        self.last_refill = Some(Instant::now());
        let _ = self.save();
        Ok(picks)
    }

    /// Top up the radio from current taste.
    ///
    /// Related tracks, artist uploads, tag searches, and one weighted random
    /// exploration query are fetched in parallel. Network failures never erase
    /// the existing pool; if it is exhausted anyway, a fresh bootstrap is
    /// attempted.
    pub fn refill(&mut self, count: usize) -> Result<Vec<Recommendation>> {
        let count = count.clamp(1, 50);
        self.prune_pool();

        let enough = self.available_count() >= count;
        let recently_filled = self
            .last_refill
            .map(|instant| instant.elapsed() < Duration::from_secs(90))
            .unwrap_or(false);

        if !(enough && recently_filled) {
            let jobs = self.refill_jobs();
            if !jobs.is_empty() {
                let fetched = self.run_jobs(jobs, 40, REFILL_BUDGET);
                for item in fetched {
                    let source = source_for_job(&item.job);
                    if let Ok(tracks) = item.result {
                        self.merge_tracks(tracks, source);
                    }
                }
            }
        }

        let mut picks = self.select(count);
        if picks.is_empty() {
            picks = self.bootstrap(count)?;
        }
        self.last_refill = Some(Instant::now());
        let _ = self.save();
        Ok(picks)
    }

    /// Record that a track started playing.
    ///
    /// The play is added to history, counts toward the play total, and applies
    /// a small positive nudge to its tags and artist.
    pub fn record_play(&mut self, track: &Track) {
        if track.id == 0 {
            return;
        }
        self.state.plays = self.state.plays.saturating_add(1);
        self.state.history.push(HistoryEntry {
            track: track.clone(),
            played_at: now_unix(),
            fraction: 0.0,
            outcome: PlayOutcome::Started,
        });
        self.trim_history();
        self.reinforce(track, 0.18, 0.12);
        self.state.last_updated = now_unix();
        let _ = self.save();
    }

    /// Record a full listen: a strong positive signal.
    pub fn record_complete(&mut self, track: &Track) {
        if track.id == 0 {
            return;
        }
        self.state.completions = self.state.completions.saturating_add(1);
        self.update_last_history(track, 1.0, PlayOutcome::Completed);
        self.reinforce(track, 1.4, 1.2);
        self.last_refill = None;
        self.state.last_updated = now_unix();
        let _ = self.save();
    }

    /// Record a skip. Early skips (under 35 percent) are a real negative
    /// signal; late skips are treated as a gentle "not right now".
    pub fn record_skip(&mut self, track: &Track, played_fraction: f32) {
        if track.id == 0 {
            return;
        }
        let fraction = played_fraction.clamp(0.0, 1.0);
        if !self.state.skipped.contains(&track.id) {
            self.state.skipped.push(track.id);
        }
        self.trim_skipped();
        self.update_last_history(track, fraction, PlayOutcome::Skipped);
        if fraction < 0.35 {
            self.reinforce(track, -1.8, -1.4);
        } else {
            self.reinforce(track, -0.15, -0.1);
        }
        self.last_refill = None;
        self.state.last_updated = now_unix();
        let _ = self.save();
    }

    /// Record a like: the strongest positive signal the engine gets.
    pub fn record_like(&mut self, track: &Track) {
        if track.id == 0 || self.is_liked(track.id) {
            return;
        }
        self.state.liked.push(track.clone());
        if self.state.liked.len() > MAX_LIKED {
            let excess = self.state.liked.len() - MAX_LIKED;
            self.state.liked.drain(0..excess);
        }
        self.state.skipped.retain(|id| *id != track.id);
        self.reinforce(track, 2.6, 2.2);
        self.last_refill = None;
        self.state.last_updated = now_unix();
        let _ = self.save();
    }

    /// Remove a like and undo part of its influence.
    pub fn record_unlike(&mut self, track: &Track) {
        let before = self.state.liked.len();
        self.state.liked.retain(|liked| liked.id != track.id);
        if self.state.liked.len() != before {
            self.reinforce(track, -1.4, -1.2);
            self.last_refill = None;
            self.state.last_updated = now_unix();
            let _ = self.save();
        }
    }

    /// Whether a track is currently liked.
    pub fn is_liked(&self, id: u64) -> bool {
        self.state.liked.iter().any(|track| track.id == id)
    }

    /// Liked tracks, most recently liked first.
    pub fn liked_tracks(&self) -> Vec<Track> {
        self.state.liked.iter().rev().cloned().collect()
    }

    /// Recently played tracks, newest first, deduplicated by track id.
    pub fn history(&self) -> Vec<Track> {
        let mut seen = BTreeSet::new();
        self.state
            .history
            .iter()
            .rev()
            .filter(|entry| seen.insert(entry.track.id))
            .map(|entry| entry.track.clone())
            .take(200)
            .collect()
    }

    /// Top positive tag affinities, strongest first, with readable labels.
    pub fn taste_summary(&self) -> Vec<(String, f32)> {
        let mut entries: Vec<(String, f32)> = self
            .state
            .tag_weights
            .iter()
            .filter(|(_, weight)| **weight > 0.05)
            .map(|(tag, weight)| (readable_tag(tag), (weight * 10.0).round() / 10.0))
            .collect();
        entries.sort_by(|left, right| {
            right
                .1
                .partial_cmp(&left.1)
                .unwrap_or(std::cmp::Ordering::Equal)
                .then_with(|| left.0.cmp(&right.0))
        });
        entries.truncate(8);
        entries
    }

    /// Top artists by learned affinity.
    pub fn top_artists(&self, n: usize) -> Vec<String> {
        let mut entries: Vec<(&String, f32)> = self
            .state
            .artist_weights
            .iter()
            .filter(|(_, weight)| **weight > 0.05)
            .map(|(artist, weight)| (artist, *weight))
            .collect();
        entries.sort_by(|left, right| {
            right
                .1
                .partial_cmp(&left.1)
                .unwrap_or(std::cmp::Ordering::Equal)
                .then_with(|| left.0.cmp(right.0))
        });
        entries
            .into_iter()
            .take(n)
            .map(|(artist, _)| artist.clone())
            .collect()
    }

    /// Total number of plays recorded.
    pub fn total_plays(&self) -> u64 {
        self.state.plays
    }

    /// Persist the state atomically.
    pub fn save(&self) -> Result<()> {
        let parent = self
            .state_path
            .parent()
            .filter(|path| !path.as_os_str().is_empty());
        if let Some(parent) = parent {
            fs::create_dir_all(parent)
                .with_context(|| format!("creating state directory {}", parent.display()))?;
        }
        let file_name = self
            .state_path
            .file_name()
            .and_then(|name| name.to_str())
            .unwrap_or("gaydar-state.json");
        let bytes = serde_json::to_vec_pretty(&self.state).context("serialising Gaydar state")?;
        let temp = self.state_path.with_file_name(format!(
            ".{file_name}.{}.{}.tmp",
            std::process::id(),
            now_nanos()
        ));
        let write_result = (|| -> Result<()> {
            let mut file =
                fs::File::create(&temp).with_context(|| format!("creating {}", temp.display()))?;
            file.write_all(&bytes)?;
            file.sync_all().ok();
            Ok(())
        })();
        if let Err(error) = write_result {
            let _ = fs::remove_file(&temp);
            return Err(error);
        }
        let backup = self.state_path.with_file_name(format!("{file_name}.bak"));
        let _ = fs::copy(&self.state_path, &backup);
        if let Err(error) = fs::rename(&temp, &self.state_path) {
            let _ = fs::remove_file(&temp);
            return Err(anyhow!("replacing {}: {error}", self.state_path.display()));
        }
        Ok(())
    }

    // -- recommendation internals -------------------------------------------

    fn bootstrap_queries(&self) -> Vec<String> {
        let mut queries: Vec<String> = Vec::new();
        let mut rng = rand::rng();

        if let Some(artist) = self.top_artists(1).into_iter().next() {
            if self.artist_weight(&artist) > 1.2 {
                push_unique(&mut queries, artist);
            }
        }
        if let Some((tag, weight)) = self.top_tag_pairs(1).into_iter().next() {
            if weight > 0.8 {
                push_unique(&mut queries, tag);
            }
        }

        push_unique(
            &mut queries,
            ARTIST_SEEDS[rng.random_range(0..ARTIST_SEEDS.len())].to_string(),
        );
        push_unique(&mut queries, "queer anthem".to_string());

        let mut pool = self.seed_pool();
        let mut attempts = 0;
        while queries.len() < 4 && !pool.is_empty() && attempts < 64 {
            attempts += 1;
            let query = weighted_pick(&mut rng, &mut pool);
            push_unique(&mut queries, query);
        }
        for seed in CORE_SEEDS {
            if queries.len() >= 4 {
                break;
            }
            push_unique(&mut queries, (*seed).to_string());
        }
        queries.truncate(4);
        queries
    }

    /// Prove that the first few picks really resolve before the UI starts.
    ///
    /// The first up to three picks are resolved in parallel with a hard
    /// deadline. Confirmed tracks move to the front; typed DRM-only or removed
    /// tracks are dropped; transient network failures are kept (the UI will
    /// retry when it actually plays them). The returned order is still the
    /// original score order among each group.
    fn verify_bootstrap_picks(
        &self,
        picks: Vec<Recommendation>,
        budget: Duration,
    ) -> Vec<Recommendation> {
        if picks.is_empty() || budget.is_zero() {
            return picks;
        }
        let probe_count = picks.len().min(3);
        let fallback = picks.clone();
        let (sender, receiver) = mpsc::channel::<(usize, Result<()>)>();
        for (index, pick) in picks.iter().take(probe_count).enumerate() {
            let client = Arc::clone(&self.client);
            let track_id = pick.track.id;
            let sender = sender.clone();
            std::thread::spawn(move || {
                let result = client.resolve_stream(track_id).map(|_| ());
                let _ = sender.send((index, result));
            });
        }
        drop(sender);

        let end = Instant::now() + budget;
        let mut results: HashMap<usize, Result<()>> = HashMap::new();
        while results.len() < probe_count {
            let remaining = end.saturating_duration_since(Instant::now());
            if remaining.is_zero() {
                break;
            }
            match receiver.recv_timeout(remaining) {
                Ok((index, result)) => {
                    results.insert(index, result);
                }
                Err(_) => break,
            }
        }

        let mut verified = Vec::new();
        let mut unverified = Vec::new();
        for (index, pick) in picks.into_iter().enumerate() {
            if index < probe_count {
                match results.get(&index) {
                    Some(Ok(())) => verified.push(pick),
                    Some(Err(error)) if error.downcast_ref::<StreamUnavailable>().is_some() => {
                        // DRM-only or withdrawn: do not hand it to the queue.
                    }
                    _ => unverified.push(pick),
                }
            } else {
                unverified.push(pick);
            }
        }
        verified.extend(unverified);
        if verified.is_empty() {
            fallback
        } else {
            verified
        }
    }

    fn refill_jobs(&self) -> Vec<FetchJob> {
        let mut jobs: Vec<FetchJob> = Vec::new();

        for (track_id, artist) in self.related_seeds(2) {
            jobs.push(FetchJob::Related { track_id, artist });
        }
        for (user_id, artist) in self.artist_jobs(2) {
            jobs.push(FetchJob::ArtistTracks { user_id, artist });
        }

        let mut rng = rand::rng();
        if let Some(tag) = self.top_tags(1).into_iter().next() {
            push_tag_job(&mut jobs, tag);
        }
        if let Some(tag) = self.random_top_tag(&mut rng, 6) {
            push_tag_job(&mut jobs, tag);
        }
        if jobs.len() < 2 {
            for query in self.bootstrap_queries().into_iter().take(2) {
                push_search_job(&mut jobs, query);
            }
        }
        push_search_job(&mut jobs, self.random_seed_query(&mut rng));
        jobs.truncate(6);
        jobs
    }

    fn score_candidate(&self, candidate: &Candidate, rng: &mut impl Rng) -> (f32, String) {
        let track = &candidate.track;
        let tag_affinity = self.tag_affinity(track);
        let artist_weight = self.artist_weight(&track.artist);
        let artist_affinity = (artist_weight / 4.0).tanh();
        let queer = queer_signal(track);
        let popularity = popularity(track);
        let freshness = freshness(track);
        let fit = dial_fit(track, self.state.dials);
        let recent_artist = self.recent_artist_count(&track.artist, 12) as f32;
        let unseen_artist = artist_weight == 0.0
            && !self
                .state
                .history
                .iter()
                .any(|entry| same_artist(&entry.track.artist, &track.artist));
        let long_form_penalty = if track.duration_ms > 15 * 60 * 1000 {
            0.12
        } else {
            0.0
        };
        let chaos = self.state.dials.chaos;
        let explore = rng.random::<f32>() < (0.06 + 0.18 * chaos);
        let jitter = rng.random_range(-0.025..0.025);
        let exploration_bonus = if explore {
            rng.random_range(0.02..0.14)
        } else {
            0.0
        };

        let mut score = 0.42
            + 0.30 * tag_affinity
            + 0.24 * artist_affinity
            + 0.20 * queer
            + 0.07 * popularity
            + 0.10 * freshness
            + 0.20 * (fit - 0.35)
            + candidate.source.bonus()
            + if unseen_artist { 0.05 } else { 0.0 }
            + exploration_bonus
            + jitter
            - long_form_penalty
            - 0.055 * recent_artist;
        score = score.clamp(0.0, 3.0);

        let reason = self.reason_for(
            track,
            artist_weight,
            queer,
            fit,
            freshness,
            explore,
            candidate,
        );
        (score, reason)
    }

    #[allow(clippy::too_many_arguments)]
    fn reason_for(
        &self,
        track: &Track,
        artist_weight: f32,
        queer: f32,
        fit: f32,
        freshness: f32,
        explore: bool,
        candidate: &Candidate,
    ) -> String {
        if has_any_tag(track, &["ballroom", "vogue", "voguing", "house ball"]) && queer >= 0.3 {
            return "certified ballroom".to_string();
        }
        if artist_weight >= 2.0 || self.is_liked_artist(&track.artist) {
            return format!("because you keep replaying {}", track.artist);
        }
        if let Some((tag, weight)) = self.best_tag(track) {
            if weight >= 1.4 {
                return format!("more {} in your veins", readable_tag(&tag));
            }
        }
        if let CandidateSource::Related(seed_artist) = &candidate.source {
            return format!("more like {seed_artist}");
        }

        let text = track_text(track);
        let camp = category_score(&CAMP_RE, &text);
        let soft = category_score(&SOFT_RE, &text);
        let chaos = category_score(&CHAOS_RE, &text);
        if self.state.dials.camp >= 0.55 && camp >= 0.4 {
            return "camp canon".to_string();
        }
        if self.state.dials.soft >= 0.5 && soft >= 0.4 && queer >= 0.2 {
            return "soft sapphic detour".to_string();
        }
        if self.state.dials.chaos >= 0.45 && chaos >= 0.4 {
            return "chaos dial, no notes".to_string();
        }
        if freshness >= 0.55 {
            return "fresh out the group chat".to_string();
        }
        if explore {
            return "the gaydar says explore".to_string();
        }
        if queer >= 0.35 {
            return "the gaydar says yes".to_string();
        }
        if fit >= 0.5 {
            return "right on the dial".to_string();
        }
        match &candidate.source {
            CandidateSource::Artist(artist) => return format!("deep cut from {artist}"),
            CandidateSource::Tag(tag) => return format!("tagged {tag}, kept it"),
            CandidateSource::Seed(query) if !query.trim().is_empty() => {
                return format!("dug out of the {} crate", query.to_lowercase());
            }
            _ => {}
        }
        if let Some(genre) = &track.genre {
            return format!("{} with a queer accent", genre.to_lowercase());
        }
        "queer radio staple".to_string()
    }

    fn select(&mut self, count: usize) -> Vec<Recommendation> {
        let count = count.max(1);
        let excluded = self.excluded_ids();
        let mut rng = rand::rng();
        let mut scored: Vec<(f32, String, usize)> = Vec::new();
        for (index, candidate) in self.pool.iter().enumerate() {
            if excluded.contains(&candidate.track.id) || !is_playable(&candidate.track) {
                continue;
            }
            let (score, reason) = self.score_candidate(candidate, &mut rng);
            scored.push((score, reason, index));
        }
        if scored.is_empty() {
            return Vec::new();
        }
        scored.sort_by(|left, right| {
            right
                .0
                .partial_cmp(&left.0)
                .unwrap_or(std::cmp::Ordering::Equal)
        });

        let distinct_artists = scored
            .iter()
            .map(|(_, _, index)| artist_key(&self.pool[*index].track))
            .collect::<HashSet<_>>()
            .len()
            .max(1);
        let max_per_artist = if distinct_artists >= count {
            1
        } else {
            count.div_ceil(distinct_artists).max(2)
        };

        let mut picks: Vec<Recommendation> = Vec::new();
        let mut picked_ids: BTreeSet<u64> = BTreeSet::new();
        let mut per_artist: HashMap<String, usize> = HashMap::new();
        for (score, reason, index) in &scored {
            if picks.len() >= count {
                break;
            }
            let track = &self.pool[*index].track;
            if picked_ids.contains(&track.id) {
                continue;
            }
            let key = artist_key(track);
            let used = per_artist.get(&key).copied().unwrap_or(0);
            if used >= max_per_artist {
                continue;
            }
            picked_ids.insert(track.id);
            *per_artist.entry(key).or_insert(0) += 1;
            picks.push(Recommendation {
                track: track.clone(),
                reason: reason.clone(),
                score: *score,
            });
        }

        if picks.len() < count {
            for (score, reason, index) in &scored {
                if picks.len() >= count {
                    break;
                }
                let track = &self.pool[*index].track;
                if picked_ids.insert(track.id) {
                    picks.push(Recommendation {
                        track: track.clone(),
                        reason: reason.clone(),
                        score: *score,
                    });
                }
            }
        }

        if picks.is_empty() {
            return Vec::new();
        }
        for pick in &picks {
            self.served.insert(pick.track.id);
        }
        if self.served.len() > 5000 {
            self.served.clear();
            for pick in &picks {
                self.served.insert(pick.track.id);
            }
        }
        self.pool
            .retain(|candidate| !picked_ids.contains(&candidate.track.id));
        picks
    }

    fn merge_tracks(&mut self, tracks: Vec<Track>, source: CandidateSource) {
        let excluded = self.excluded_ids();
        for track in tracks {
            if !is_playable(&track) || excluded.contains(&track.id) {
                continue;
            }
            if let Some(existing) = self
                .pool
                .iter_mut()
                .find(|candidate| candidate.track.id == track.id)
            {
                if existing.source.bonus() < source.bonus() {
                    existing.source = source.clone();
                }
                existing.added_at = Instant::now();
                continue;
            }
            if self.pool.len() >= MAX_POOL {
                if let Some((oldest, _)) = self
                    .pool
                    .iter()
                    .enumerate()
                    .min_by_key(|(_, candidate)| candidate.added_at)
                {
                    self.pool.remove(oldest);
                }
            }
            self.pool.push(Candidate {
                track,
                source: source.clone(),
                added_at: Instant::now(),
            });
        }
    }

    fn prune_pool(&mut self) {
        let excluded = self.excluded_ids();
        let now = Instant::now();
        self.pool.retain(|candidate| {
            !excluded.contains(&candidate.track.id)
                && is_playable(&candidate.track)
                && now.saturating_duration_since(candidate.added_at) < Duration::from_secs(6 * 3600)
        });
    }

    fn available_count(&self) -> usize {
        let excluded = self.excluded_ids();
        self.pool
            .iter()
            .filter(|candidate| {
                !excluded.contains(&candidate.track.id) && is_playable(&candidate.track)
            })
            .count()
    }

    fn excluded_ids(&self) -> BTreeSet<u64> {
        let mut ids: BTreeSet<u64> = self
            .state
            .history
            .iter()
            .map(|entry| entry.track.id)
            .collect();
        ids.extend(self.state.skipped.iter().copied());
        ids.extend(self.state.liked.iter().map(|track| track.id));
        ids.extend(self.served.iter().copied());
        ids
    }

    fn run_jobs(&self, jobs: Vec<FetchJob>, limit: usize, deadline: Duration) -> Vec<Fetched> {
        if jobs.is_empty() || deadline.is_zero() {
            return Vec::new();
        }
        let (sender, receiver) = mpsc::channel::<Fetched>();
        let expected = jobs.len();
        for job in jobs {
            let client = Arc::clone(&self.client);
            let sender = sender.clone();
            std::thread::spawn(move || {
                let result = match &job {
                    FetchJob::Search(query) => client.search_tracks(query, limit),
                    FetchJob::TagSearch(tag) => client.search_tracks(tag, limit),
                    FetchJob::Related { track_id, .. } => client.related_tracks(*track_id, limit),
                    FetchJob::ArtistTracks { user_id, .. } => client.user_tracks(*user_id, limit),
                    FetchJob::Trending => client.trending_tracks(limit),
                };
                let _ = sender.send(Fetched { job, result });
            });
        }
        drop(sender);

        let end = Instant::now() + deadline;
        let mut fetched: Vec<Fetched> = Vec::with_capacity(expected);
        while fetched.len() < expected {
            let remaining = end.saturating_duration_since(Instant::now());
            if remaining.is_zero() {
                break;
            }
            match receiver.recv_timeout(remaining) {
                Ok(item) => fetched.push(item),
                Err(_) => break,
            }
        }
        fetched
    }

    fn related_seeds(&self, n: usize) -> Vec<(u64, String)> {
        let mut seen = HashSet::new();
        let mut seeds = Vec::new();
        let skipped: HashSet<u64> = self.state.skipped.iter().copied().collect();
        for entry in self.state.history.iter().rev() {
            if entry.outcome == PlayOutcome::Skipped
                || skipped.contains(&entry.track.id)
                || !is_playable(&entry.track)
            {
                continue;
            }
            if !seen.insert(entry.track.id) {
                continue;
            }
            seeds.push((entry.track.id, entry.track.artist.clone()));
            if seeds.len() >= n {
                return seeds;
            }
        }
        for track in self.state.liked.iter().rev() {
            if skipped.contains(&track.id) || !is_playable(track) {
                continue;
            }
            if !seen.insert(track.id) {
                continue;
            }
            seeds.push((track.id, track.artist.clone()));
            if seeds.len() >= n {
                break;
            }
        }
        seeds
    }

    fn artist_jobs(&self, n: usize) -> Vec<(u64, String)> {
        let mut jobs = Vec::new();
        for artist in self.top_artists(n.max(1).saturating_mul(3)) {
            let id = self
                .state
                .liked
                .iter()
                .rev()
                .chain(self.state.history.iter().rev().map(|entry| &entry.track))
                .find(|track| same_artist(&track.artist, &artist))
                .and_then(|track| track.artist_id);
            if let Some(user_id) = id {
                jobs.push((user_id, artist));
                if jobs.len() >= n {
                    break;
                }
            }
        }
        jobs
    }

    fn top_tag_pairs(&self, n: usize) -> Vec<(String, f32)> {
        let mut entries: Vec<(String, f32)> = self
            .state
            .tag_weights
            .iter()
            .filter(|(tag, weight)| **weight > 0.05 && tag.chars().count() >= 2)
            .map(|(tag, weight)| (tag.clone(), *weight))
            .collect();
        entries.sort_by(|left, right| {
            right
                .1
                .partial_cmp(&left.1)
                .unwrap_or(std::cmp::Ordering::Equal)
        });
        entries.truncate(n);
        entries
    }

    fn top_tags(&self, n: usize) -> Vec<String> {
        self.top_tag_pairs(n)
            .into_iter()
            .map(|(tag, _)| tag)
            .collect()
    }

    fn random_top_tag(&self, rng: &mut impl Rng, candidates: usize) -> Option<String> {
        let tags = self.top_tags(candidates);
        if tags.is_empty() {
            None
        } else {
            Some(tags[rng.random_range(0..tags.len())].clone())
        }
    }

    fn tag_affinity(&self, track: &Track) -> f32 {
        let mut total = 0.0;
        let mut count = 0usize;
        for tag in track_tag_keys(track) {
            if let Some(weight) = self.state.tag_weights.get(&tag) {
                total += *weight;
                count += 1;
            }
        }
        if count == 0 {
            0.0
        } else {
            (total / (4.0 * (count as f32).sqrt())).tanh()
        }
    }

    fn best_tag(&self, track: &Track) -> Option<(String, f32)> {
        let mut best: Option<(String, f32)> = None;
        for tag in track_tag_keys(track) {
            if let Some(weight) = self.state.tag_weights.get(&tag).copied() {
                if weight > 0.05
                    && best
                        .as_ref()
                        .map(|(_, best_weight)| weight > *best_weight)
                        .unwrap_or(true)
                {
                    best = Some((tag, weight));
                }
            }
        }
        best
    }

    fn artist_weight(&self, artist: &str) -> f32 {
        if let Some(weight) = self.state.artist_weights.get(artist) {
            return *weight;
        }
        self.state
            .artist_weights
            .iter()
            .find(|(key, _)| same_artist(key, artist))
            .map(|(_, weight)| *weight)
            .unwrap_or(0.0)
    }

    fn is_liked_artist(&self, artist: &str) -> bool {
        self.state
            .liked
            .iter()
            .any(|track| same_artist(&track.artist, artist))
    }

    fn recent_artist_count(&self, artist: &str, window: usize) -> usize {
        self.state
            .history
            .iter()
            .rev()
            .take(window)
            .filter(|entry| same_artist(&entry.track.artist, artist))
            .count()
    }

    fn update_last_history(&mut self, track: &Track, fraction: f32, outcome: PlayOutcome) {
        if let Some(entry) = self
            .state
            .history
            .iter_mut()
            .rev()
            .find(|entry| entry.track.id == track.id && entry.outcome == PlayOutcome::Started)
        {
            entry.track = track.clone();
            entry.fraction = fraction;
            entry.outcome = outcome;
            entry.played_at = now_unix();
            return;
        }
        self.state.history.push(HistoryEntry {
            track: track.clone(),
            played_at: now_unix(),
            fraction,
            outcome,
        });
        self.trim_history();
    }

    fn trim_history(&mut self) {
        if self.state.history.len() > MAX_HISTORY {
            let excess = self.state.history.len() - MAX_HISTORY;
            self.state.history.drain(0..excess);
        }
    }

    fn trim_skipped(&mut self) {
        let mut seen = BTreeSet::new();
        self.state.skipped.retain(|id| *id != 0 && seen.insert(*id));
        if self.state.skipped.len() > MAX_SKIPPED {
            let excess = self.state.skipped.len() - MAX_SKIPPED;
            self.state.skipped.drain(0..excess);
        }
    }

    fn reinforce(&mut self, track: &Track, tag_delta: f32, artist_delta: f32) {
        let tags = track_tag_keys(track);
        if !tags.is_empty() {
            let spread = tag_delta / (tags.len() as f32).sqrt();
            for tag in tags {
                adjust_weight(&mut self.state.tag_weights, tag, spread);
            }
        }
        let artist = track.artist.trim();
        if !artist.is_empty() {
            adjust_weight(
                &mut self.state.artist_weights,
                artist.to_string(),
                artist_delta,
            );
        }
        prune_weights(&mut self.state.tag_weights, 500, 350);
        prune_weights(&mut self.state.artist_weights, 400, 300);
    }

    fn seed_pool(&self) -> Vec<(String, f32)> {
        let dials = self.state.dials;
        let mut weights: BTreeMap<String, f32> = BTreeMap::new();
        let mut add = |seeds: &[&str], weight: f32| {
            for seed in seeds {
                let entry = weights.entry((*seed).to_string()).or_insert(0.0);
                *entry = entry.max(weight);
            }
        };
        add(CORE_SEEDS, 0.7);
        add(CAMP_SEEDS, 0.25 + dials.camp);
        add(CLUB_SEEDS, 0.25 + dials.club);
        add(SOFT_SEEDS, 0.25 + dials.soft);
        add(CHAOS_SEEDS, 0.25 + dials.chaos);
        add(ARTIST_SEEDS, 0.45 + 0.35 * dials.camp.max(dials.club));
        weights.into_iter().collect()
    }

    fn random_seed_query(&self, rng: &mut impl Rng) -> String {
        let mut pool = self.seed_pool();
        if pool.is_empty() {
            "queer anthem".to_string()
        } else {
            weighted_pick(rng, &mut pool)
        }
    }
}

// ---------------------------------------------------------------------------
// Scoring helpers
// ---------------------------------------------------------------------------

fn normalize_name(name: &str) -> String {
    name.chars()
        .filter(|character| character.is_alphanumeric())
        .flat_map(|character| character.to_lowercase())
        .collect()
}

fn same_artist(left: &str, right: &str) -> bool {
    left.trim().eq_ignore_ascii_case(right.trim())
}

fn artist_key(track: &Track) -> String {
    let key = track.artist.trim().to_lowercase();
    if key.is_empty() {
        format!("#{}", track.id)
    } else {
        key
    }
}

fn track_tag_keys(track: &Track) -> Vec<String> {
    let mut keys = track.tags.clone();
    if let Some(genre) = &track.genre {
        let key = genre.trim().to_lowercase();
        if !key.is_empty() && !keys.iter().any(|existing| existing == &key) {
            keys.push(key);
        }
    }
    keys
}

fn track_text(track: &Track) -> String {
    let mut text = track.seed_text().to_lowercase();
    if let Some(description) = &track.description {
        text.push(' ');
        text.push_str(&description.to_lowercase());
    }
    text
}

fn has_any_tag(track: &Track, needles: &[&str]) -> bool {
    track_tag_keys(track)
        .iter()
        .any(|tag| needles.iter().any(|needle| tag.contains(needle)))
}

fn category_score(regex: &Regex, text: &str) -> f32 {
    let hits = regex.find_iter(text).count() as f32;
    (1.0 - (-0.9 * hits).exp()).clamp(0.0, 1.0)
}

fn queer_signal(track: &Track) -> f32 {
    let text = track_text(track);
    let strong = QUEER_STRONG_RE.find_iter(&text).count() as f32;
    let mild = QUEER_MILD_RE.find_iter(&text).count() as f32;
    let mut score = 1.0 - (-(0.75 * strong + 0.25 * mild)).exp();
    if QUEER_ARTIST_KEYS.contains(&normalize_name(&track.artist)) {
        score += 0.55;
    }
    score.clamp(0.0, 1.0)
}

fn log_norm(value: u64, ceiling: f32) -> f32 {
    ((1.0 + value as f64).log10() as f32 / ceiling).clamp(0.0, 1.0)
}

fn popularity(track: &Track) -> f32 {
    0.7 * log_norm(track.playback_count, 6.5)
        + 0.2 * log_norm(track.likes_count, 5.0)
        + 0.1 * log_norm(track.user_followers, 6.0)
}

fn freshness(track: &Track) -> f32 {
    let created = track
        .created_at
        .as_deref()
        .and_then(|value| DateTime::parse_from_rfc3339(value).ok());
    let Some(created) = created else {
        return 0.0;
    };
    let days = Utc::now()
        .signed_duration_since(created.with_timezone(&Utc))
        .num_days()
        .max(0) as f32;
    (-days / 540.0).exp()
}

fn dial_fit(track: &Track, dials: Dials) -> f32 {
    let text = track_text(track);
    let camp = category_score(&CAMP_RE, &text);
    let club = category_score(&CLUB_RE, &text);
    let soft = category_score(&SOFT_RE, &text);
    let chaos = category_score(&CHAOS_RE, &text);
    let total = dials.camp + dials.club + dials.soft + dials.chaos + 0.001;
    ((dials.camp * camp + dials.club * club + dials.soft * soft + dials.chaos * chaos) / total)
        .clamp(0.0, 1.0)
}

fn readable_tag(tag: &str) -> String {
    match tag {
        "wlw" => "WLW",
        "lgbtq" => "LGBTQ+",
        "lgbt" => "LGBT",
        "lgbtqia" => "LGBTQIA+",
        "r&b" => "R&B",
        "dnb" => "DnB",
        "edm" => "EDM",
        "mnek" => "MNEK",
        _ => tag,
    }
    .to_string()
}

fn adjust_weight(map: &mut BTreeMap<String, f32>, key: String, delta: f32) {
    if key.trim().is_empty() || !delta.is_finite() {
        return;
    }
    let entry = map.entry(key).or_insert(0.0);
    *entry = (*entry + delta).clamp(-MAX_WEIGHT, MAX_WEIGHT);
}

fn prune_weights(map: &mut BTreeMap<String, f32>, limit: usize, keep: usize) {
    if map.len() <= limit {
        return;
    }
    let mut entries: Vec<(String, f32)> = map
        .iter()
        .map(|(key, weight)| (key.clone(), *weight))
        .collect();
    entries.sort_by(|left, right| {
        right
            .1
            .abs()
            .partial_cmp(&left.1.abs())
            .unwrap_or(std::cmp::Ordering::Equal)
    });
    entries.truncate(keep);
    *map = entries.into_iter().collect();
}

fn push_unique(list: &mut Vec<String>, value: String) {
    let value = value.trim().to_string();
    if value.is_empty() {
        return;
    }
    if !list
        .iter()
        .any(|existing| existing.eq_ignore_ascii_case(&value))
    {
        list.push(value);
    }
}

fn push_tag_job(jobs: &mut Vec<FetchJob>, tag: String) {
    let tag = tag.trim().to_string();
    if tag.is_empty() {
        return;
    }
    if jobs.iter().any(
        |job| matches!(job, FetchJob::TagSearch(existing) if existing.eq_ignore_ascii_case(&tag)),
    ) {
        return;
    }
    jobs.push(FetchJob::TagSearch(tag));
}

fn push_search_job(jobs: &mut Vec<FetchJob>, query: String) {
    let query = query.trim().to_string();
    if query.is_empty() {
        return;
    }
    if jobs.iter().any(
        |job| matches!(job, FetchJob::Search(existing) if existing.eq_ignore_ascii_case(&query)),
    ) {
        return;
    }
    jobs.push(FetchJob::Search(query));
}

fn weighted_pick(rng: &mut impl Rng, pool: &mut Vec<(String, f32)>) -> String {
    let total: f32 = pool.iter().map(|(_, weight)| weight.max(0.01)).sum();
    let mut draw = rng.random::<f32>() * total;
    for index in 0..pool.len() {
        draw -= pool[index].1.max(0.01);
        if draw <= 0.0 {
            return pool.swap_remove(index).0;
        }
    }
    pool.pop()
        .map(|(query, _)| query)
        .unwrap_or_else(|| "queer anthem".to_string())
}

fn remaining_budget(started: Instant, total: Duration) -> Duration {
    total.saturating_sub(started.elapsed())
}

// ---------------------------------------------------------------------------
// Persistence helpers
// ---------------------------------------------------------------------------

fn backup_path(state_path: &Path) -> PathBuf {
    let file_name = state_path
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or("gaydar-state.json");
    state_path.with_file_name(format!("{file_name}.bak"))
}

fn read_state_file(path: &Path) -> Option<PersistedState> {
    let bytes = fs::read(path).ok()?;
    if bytes.is_empty() {
        return None;
    }
    serde_json::from_slice(&bytes).ok()
}

fn backup_corrupt(path: &Path) {
    let file_name = path
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or("gaydar-state.json");
    let target = path.with_file_name(format!("{file_name}.corrupt-{}", now_unix()));
    let _ = fs::rename(path, target);
}

fn load_state(path: &Path) -> PersistedState {
    if let Some(state) = read_state_file(path) {
        return sanitize_state(state);
    }
    if path.exists() {
        backup_corrupt(path);
    }
    if let Some(state) = read_state_file(&backup_path(path)) {
        return sanitize_state(state);
    }
    PersistedState::default()
}

fn sanitize_state(mut state: PersistedState) -> PersistedState {
    state.version = STATE_VERSION;
    state.dials = state.dials.clamped();

    state
        .tag_weights
        .retain(|key, weight| !key.trim().is_empty() && weight.is_finite());
    for weight in state.tag_weights.values_mut() {
        *weight = weight.clamp(-MAX_WEIGHT, MAX_WEIGHT);
    }
    state
        .artist_weights
        .retain(|key, weight| !key.trim().is_empty() && weight.is_finite());
    for weight in state.artist_weights.values_mut() {
        *weight = weight.clamp(-MAX_WEIGHT, MAX_WEIGHT);
    }

    let mut liked_ids = BTreeSet::new();
    state.liked.retain(|track| {
        track.id != 0 && !track.title.trim().is_empty() && liked_ids.insert(track.id)
    });
    if state.liked.len() > MAX_LIKED {
        let excess = state.liked.len() - MAX_LIKED;
        state.liked.drain(0..excess);
    }

    state.history.retain(|entry| entry.track.id != 0);
    for entry in state.history.iter_mut() {
        entry.fraction = entry.fraction.clamp(0.0, 1.0);
    }
    if state.history.len() > MAX_HISTORY {
        let excess = state.history.len() - MAX_HISTORY;
        state.history.drain(0..excess);
    }

    let mut skipped_ids = BTreeSet::new();
    state
        .skipped
        .retain(|id| *id != 0 && skipped_ids.insert(*id));
    if state.skipped.len() > MAX_SKIPPED {
        let excess = state.skipped.len() - MAX_SKIPPED;
        state.skipped.drain(0..excess);
    }

    let now = now_unix();
    if state.last_updated > 0 && now > state.last_updated {
        let days = (now - state.last_updated) as f32 / 86_400.0;
        if days >= 1.0 {
            let factor = 0.99f32.powf((days / 7.0).min(52.0));
            for weight in state.tag_weights.values_mut() {
                *weight *= factor;
            }
            for weight in state.artist_weights.values_mut() {
                *weight *= factor;
            }
        }
    }
    state.last_updated = now;
    state
}
