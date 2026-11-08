//! Public SoundCloud API client for GAYDAR FM.
//!
//! The player never asks the listener to log in. It talks to SoundCloud's
//! public api-v2 endpoints with a web client id discovered from SoundCloud's
//! own HTML and JavaScript bundles, cached under the user cache directory, and
//! refreshed transparently when the API rejects it.
//!
//! Only public, read-only endpoints are used: search, track lookup, related
//! tracks, user tracks, charts, and the media transcoding resolver.

use crate::model::{StreamKind, StreamSource, Track};
use anyhow::{Context, Result, anyhow, bail};
use once_cell::sync::Lazy;
use regex::Regex;
use reqwest::blocking::Client;
use reqwest::header::{ACCEPT, HeaderMap, HeaderValue, ORIGIN, RANGE, REFERER};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::{HashMap, HashSet};
use std::fs;
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc;
use std::sync::{Arc, Mutex, MutexGuard};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

const API_BASE: &str = "https://api-v2.soundcloud.com";
const WEB_BASE: &str = "https://soundcloud.com";
const USER_AGENT: &str = "Mozilla/5.0 (X11; Linux x86_64) AppleWebKit/537.36 \
                          (KHTML, like Gecko) Chrome/124.0.0.0 Safari/537.36";
const REQUEST_TIMEOUT: Duration = Duration::from_secs(15);
const CONNECT_TIMEOUT: Duration = Duration::from_secs(10);
const MAX_RETRIES: usize = 2;
const MAX_SEARCH_LIMIT: usize = 200;
const MAX_ARTWORK_BYTES: usize = 32 * 1024 * 1024;
const MAX_ASSET_BYTES: usize = 8 * 1024 * 1024;
const MAX_ASSETS: usize = 12;
const DISCOVERY_DEADLINE: Duration = Duration::from_secs(12);
const TRACK_CACHE_TTL: Duration = Duration::from_secs(600);
const TRACK_CACHE_MAX: usize = 256;
const MIN_TRACK_MS: u64 = 45_000;
const MAX_TRACK_MS: u64 = 45 * 60 * 1000;

/// Public web client ids that have worked in the past. They are only tried
/// after live extraction fails, and every candidate is validated with a real
/// search call before it is accepted.
const FALLBACK_CLIENT_IDS: &[&str] = &[
    "39AVoiZVrdvJEMFLH8T0c5WGshQaAPRJ",
    "a3e059563d7fd3372b49b37f00a00bcf",
    "02gUJC0hH2ct1EGOcYXQIzRFU91c72Ea",
];

/// Matches client_id:"<32 chars>", client_id=..., "client_id":"...", and
/// URL-encoded variants found in SoundCloud's bundles.
static CLIENT_ID_RE: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r#"(?i)client[_]?id(?:["']?\s*[:=]|%3D)\s*["']?([A-Za-z0-9]{32})"#)
        .expect("client id regex is valid")
});

/// Matches script assets served from SoundCloud's webpack CDN.
static ASSET_RE: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r#"(?:https?:)?//a-v2\.sndcdn\.com/assets/[A-Za-z0-9._-]+\.js"#)
        .expect("asset regex is valid")
});

/// A process-wide request pacer shared by every client instance, so parallel
/// bootstrap threads and multiple clients together stay near 5 requests/second.
static RATE_LIMITER: Lazy<RateLimiter> = Lazy::new(|| RateLimiter::new(5));

/// Resolve a poisoned mutex instead of panicking: a cached client id or a
/// pacing timestamp is never worth taking the UI down for.
fn lock<T>(mutex: &Mutex<T>) -> MutexGuard<'_, T> {
    mutex
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
}

fn now_unix() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0)
}

fn now_nanos() -> u128 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_nanos())
        .unwrap_or(0)
}

fn backoff(attempt: u32) -> Duration {
    Duration::from_millis(250 * 2u64.pow(attempt.min(4)))
}

fn truncate_text(text: &str, max_chars: usize) -> String {
    let mut out: String = text.chars().take(max_chars).collect();
    if text.chars().count() > max_chars {
        out.push('…');
    }
    out
}

struct RateLimiter {
    next: Mutex<Instant>,
    spacing: Duration,
}

impl RateLimiter {
    fn new(requests_per_second: u32) -> Self {
        let spacing = Duration::from_micros(1_000_000 / u64::from(requests_per_second.max(1)));
        Self {
            next: Mutex::new(Instant::now()),
            spacing,
        }
    }

    /// Sleep until this caller's send slot, then reserve the next slot.
    fn wait(&self) {
        let mut next = lock(&self.next);
        let now = Instant::now();
        if *next > now {
            std::thread::sleep(*next - now);
        }
        *next = Instant::now() + self.spacing;
    }
}

#[derive(Debug)]
struct ClientIdState {
    client_id: String,
    generation: u64,
}

#[derive(Debug, Serialize, Deserialize)]
struct ClientIdCache {
    client_id: String,
    #[serde(default)]
    saved_at: i64,
    #[serde(default)]
    source: String,
}

/// The kind of unauthenticated stream SoundCloud exposes for a track.
///
/// SoundCloud's public API lists MP3, AAC, and ABR transcodings. Only the MP3
/// family is playable without DRM: AAC/ABR playlists use FairPlay keys, which
/// ffmpeg cannot open. GAYDAR therefore treats everything that is not MP3 as
/// unavailable rather than retrying it forever.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum StreamSupport {
    /// A full MP3 progressive stream is available.
    Mp3Progressive,
    /// An HLS playlist with plain MP3 segments is available.
    Mp3Hls,
    /// Transcodings exist, but only DRM-protected AAC/ABR variants.
    DrmOnly,
    /// No usable transcodings exist at all.
    Unavailable,
}

impl StreamSupport {
    /// True when [SoundCloudClient::resolve_stream] can produce a playable URL.
    pub fn is_playable(self) -> bool {
        matches!(self, Self::Mp3Progressive | Self::Mp3Hls)
    }
}

impl std::fmt::Display for StreamSupport {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Mp3Progressive => write!(f, "MP3 progressive stream available"),
            Self::Mp3Hls => write!(f, "MP3 HLS stream available"),
            Self::DrmOnly => write!(f, "only DRM-protected AAC/ABR transcodings are offered"),
            Self::Unavailable => write!(f, "no audio transcodings are available"),
        }
    }
}

/// Typed error returned when a track has no playable MP3 stream.
///
/// This lets the radio distinguish "SoundCloud will never play this without a
/// login" (drop it) from a transient network failure (keep and retry later).
/// Downcast it with anyhow::Error::downcast_ref.
#[derive(Clone, Debug)]
pub struct StreamUnavailable {
    pub track_id: u64,
    pub title: String,
    pub support: StreamSupport,
}

impl std::fmt::Display for StreamUnavailable {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "no playable mp3 stream for SoundCloud track {} (\"{}\"): {}",
            self.track_id, self.title, self.support
        )
    }
}

impl std::error::Error for StreamUnavailable {}

struct CachedTrackValue {
    fetched_at: Instant,
    value: Value,
}

/// Errors produced by the API transport, kept separate so callers can tell a
/// missing related-track list from an actual network outage.
#[derive(Debug)]
enum ApiError {
    Auth(String),
    NotFound,
    Status { code: u16, body: String },
    Transport(String),
    Decode(String),
}

impl std::fmt::Display for ApiError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Auth(message) => write!(f, "authentication failed: {message}"),
            Self::NotFound => write!(f, "not found (HTTP 404)"),
            Self::Status { code, body } => {
                if body.is_empty() {
                    write!(f, "HTTP {code}")
                } else {
                    write!(f, "HTTP {code}: {body}")
                }
            }
            Self::Transport(message) => write!(f, "network error: {message}"),
            Self::Decode(message) => write!(f, "invalid response: {message}"),
        }
    }
}

impl std::error::Error for ApiError {}

/// A read-only, unauthenticated SoundCloud API client.
///
/// Construction obtains a web client id without user interaction: from the
/// on-disk cache, from SoundCloud's web bundle, or from a validated fallback.
pub struct SoundCloudClient {
    http: Client,
    state: Mutex<ClientIdState>,
    refresh_lock: Mutex<()>,
    cache_path: PathBuf,
    track_cache: Mutex<HashMap<u64, CachedTrackValue>>,
}

impl SoundCloudClient {
    /// Create a client, reusing the cached web client id when possible.
    ///
    /// The cache is trusted without a network round trip; a rotated id is
    /// detected on the first 401/403 and refreshed automatically. When no
    /// cache exists, a client id is extracted from SoundCloud's web assets.
    pub fn new() -> Result<Self> {
        let http = build_http()?;
        let cache_path = client_id_cache_path();
        let (client_id, source) = match load_cached_client_id(&cache_path) {
            Some(id) => (id, "cache"),
            None => {
                let (id, source) =
                    acquire_client_id(&http).context("obtaining a SoundCloud web client id")?;
                (id, source)
            }
        };
        let client = Self {
            http,
            state: Mutex::new(ClientIdState {
                client_id: client_id.clone(),
                generation: 1,
            }),
            refresh_lock: Mutex::new(()),
            cache_path,
            track_cache: Mutex::new(HashMap::new()),
        };
        if source != "cache" {
            let _ = client.persist_cache(&client_id, source);
        }
        Ok(client)
    }

    /// Create a client with an explicitly supplied web client id.
    ///
    /// The id is not validated over the network here; it is checked on first
    /// use, and a rejected id triggers the normal refresh path.
    pub fn with_client_id(client_id: impl Into<String>) -> Result<Self> {
        let client_id = normalize_client_id(client_id.into())?;
        Ok(Self {
            http: build_http()?,
            state: Mutex::new(ClientIdState {
                client_id,
                generation: 1,
            }),
            refresh_lock: Mutex::new(()),
            cache_path: client_id_cache_path(),
            track_cache: Mutex::new(HashMap::new()),
        })
    }

    /// The client id currently in use.
    pub fn client_id(&self) -> String {
        lock(&self.state).client_id.clone()
    }

    /// Force a fresh client id discovery and cache it.
    ///
    /// This is exposed for diagnostics and for hosts that want to repair the
    /// cache; normal requests refresh themselves on 401/403.
    pub fn refresh_client_id(&self) -> Result<String> {
        let _guard = lock(&self.refresh_lock);
        let (client_id, source) =
            acquire_client_id(&self.http).context("refreshing the SoundCloud web client id")?;
        {
            let mut state = lock(&self.state);
            state.client_id = client_id.clone();
            state.generation = state.generation.wrapping_add(1);
        }
        let _ = self.persist_cache(&client_id, source);
        Ok(client_id)
    }

    /// Discover and validate a working web client id without touching the
    /// on-disk cache. Useful as a self-test of SoundCloud's current bundle.
    pub fn discover_client_id() -> Result<String> {
        let http = build_http()?;
        discover_with(&http)
    }

    /// Search SoundCloud's public track catalogue.
    ///
    /// Results are normalised, deduplicated, and filtered to tracks that are
    /// streamable, allowed by SoundCloud policy, and 45 s-45 min long.
    pub fn search_tracks(&self, query: &str, limit: usize) -> Result<Vec<Track>> {
        let query = query.trim();
        if query.is_empty() {
            return Ok(Vec::new());
        }
        let limit = limit.clamp(1, MAX_SEARCH_LIMIT);
        let value = self
            .api_get(
                "/search/tracks",
                &[("q", query.to_string()), ("limit", limit.to_string())],
            )
            .map_err(|error| anyhow!("SoundCloud search for {query:?} failed: {error}"))?;
        Ok(dedup_tracks(collection_tracks(self, &value, "collection")))
    }

    /// Fetch the track object for one id, without filtering its policy.
    ///
    /// Use [is_playable] and [stream_support] before queueing it; this method
    /// deliberately returns blocked, short, or DRM-only tracks so callers can
    /// display a reason.
    pub fn track(&self, track_id: u64) -> Result<Track> {
        let value = self
            .fetch_track_value(track_id)
            .map_err(|error| match error {
                ApiError::NotFound => anyhow!("SoundCloud track {track_id} does not exist"),
                other => anyhow!("fetching SoundCloud track {track_id} failed: {other}"),
            })?;
        parse_track(&value)
            .ok_or_else(|| anyhow!("SoundCloud returned an unreadable track object for {track_id}"))
    }

    /// Classify the stream availability of one track without resolving media.
    ///
    /// This costs at most one cached track lookup and lets the radio drop
    /// DRM-only uploads before they reach the queue.
    pub fn stream_support(&self, track_id: u64) -> Result<StreamSupport> {
        let value = self
            .fetch_track_value(track_id)
            .map_err(|error| match error {
                ApiError::NotFound => anyhow!("SoundCloud track {track_id} does not exist"),
                other => anyhow!("fetching SoundCloud track {track_id} failed: {other}"),
            })?;
        Ok(stream_support_from_value(&value))
    }

    /// Tracks related to one track.
    ///
    /// SoundCloud returns 404 for related tracks on some small uploads, so
    /// this falls back to the artist's own uploads and then to a search for
    /// the artist plus one of the track's tags or its genre.
    pub fn related_tracks(&self, track_id: u64, limit: usize) -> Result<Vec<Track>> {
        let limit = limit.clamp(1, MAX_SEARCH_LIMIT);
        let related = match self.api_get(
            &format!("/tracks/{track_id}/related"),
            &[("limit", limit.to_string())],
        ) {
            Ok(value) => collection_tracks(self, &value, "collection"),
            Err(ApiError::NotFound) => Vec::new(),
            Err(ApiError::Auth(message)) => {
                return Err(anyhow!(
                    "SoundCloud related tracks for {track_id} failed: authentication failed: {message}"
                ));
            }
            Err(error) => {
                return Err(anyhow!(
                    "SoundCloud related tracks for {track_id} failed: {error}"
                ));
            }
        };
        let mut related = related;
        related.retain(|track| track.id != track_id);
        if !related.is_empty() {
            return Ok(dedup_tracks(related));
        }

        let seed = match self.track(track_id) {
            Ok(track) => track,
            Err(_) => return Ok(Vec::new()),
        };

        if let Some(artist_id) = seed.artist_id {
            if let Ok(mut own) = self.user_tracks(artist_id, limit) {
                own.retain(|track| track.id != track_id);
                if !own.is_empty() {
                    return Ok(dedup_tracks(own));
                }
            }
        }

        let mut query = seed.artist.clone();
        if let Some(tag) = seed.tags.first().or(seed.genre.as_ref()) {
            query.push(' ');
            query.push_str(tag);
        }
        match self.search_tracks(&query, limit) {
            Ok(mut tracks) => {
                tracks.retain(|track| track.id != track_id);
                Ok(tracks)
            }
            Err(_) => Ok(Vec::new()),
        }
    }

    /// Fetch an artist's published tracks.
    pub fn user_tracks(&self, user_id: u64, limit: usize) -> Result<Vec<Track>> {
        let limit = limit.clamp(1, MAX_SEARCH_LIMIT);
        let value = self
            .api_get(
                &format!("/users/{user_id}/tracks"),
                &[("limit", limit.to_string())],
            )
            .map_err(|error| anyhow!("SoundCloud artist {user_id} tracks failed: {error}"))?;
        Ok(dedup_tracks(collection_tracks(self, &value, "collection")))
    }

    /// SoundCloud's current global trending chart.
    ///
    /// Charts are used only as a last-resort cold-start source; the top chart
    /// kind is not part of the public API and returns 404.
    pub fn trending_tracks(&self, limit: usize) -> Result<Vec<Track>> {
        let limit = limit.clamp(1, MAX_SEARCH_LIMIT);
        let value = self
            .api_get(
                "/charts",
                &[
                    ("kind", "trending".to_string()),
                    ("limit", limit.to_string()),
                ],
            )
            .map_err(|error| anyhow!("SoundCloud trending chart failed: {error}"))?;
        Ok(dedup_tracks(collection_tracks(self, &value, "collection")))
    }

    /// Resolve a playable, unauthenticated MP3 media URL for a track.
    ///
    /// Only MP3 transcodings are usable without login; AAC and ABR variants
    /// are FairPlay DRM. Progressive MP3 is preferred, MP3 HLS is the
    /// fallback. The resolved CDN URL is probed with a small range request
    /// before it is returned, so a URL that cannot actually be played is never
    /// handed to the audio engine. DRM-only tracks produce a typed
    /// [StreamUnavailable] error that callers can drop immediately.
    pub fn resolve_stream(&self, track_id: u64) -> Result<StreamSource> {
        let value = self
            .fetch_track_value(track_id)
            .map_err(|error| match error {
                ApiError::NotFound => anyhow!("SoundCloud track {track_id} does not exist"),
                other => anyhow!("fetching SoundCloud track {track_id} failed: {other}"),
            })?;
        let track = parse_track(&value).ok_or_else(|| {
            anyhow!("SoundCloud returned an unreadable track object for {track_id}")
        })?;

        if policy_blocked(&track.policy) {
            bail!(
                "SoundCloud track {} (\"{}\") is not playable without login (policy {})",
                track_id,
                track.title,
                track.policy
            );
        }
        if !track.streamable {
            bail!(
                "SoundCloud track {} (\"{}\") is marked non-streamable",
                track_id,
                track.title
            );
        }

        let support = stream_support_from_value(&value);
        if !support.is_playable() {
            return Err(anyhow::Error::new(StreamUnavailable {
                track_id,
                title: track.title,
                support,
            }));
        }

        let track_authorization = value
            .get("track_authorization")
            .and_then(Value::as_str)
            .map(str::to_string);
        let mut transcodings: Vec<Transcoding> = parse_transcodings(&value)
            .into_iter()
            .filter(|transcoding| transcoding.is_mp3() && !transcoding.is_snipped())
            .collect();
        if transcodings.is_empty() {
            return Err(anyhow::Error::new(StreamUnavailable {
                track_id,
                title: track.title,
                support: StreamSupport::Unavailable,
            }));
        }
        transcodings.sort_by_key(transcoding_rank);

        let mut failures: Vec<String> = Vec::new();
        for transcoding in &transcodings {
            let kind = match transcoding.protocol().to_ascii_lowercase().as_str() {
                "progressive" => StreamKind::Progressive,
                "hls" => StreamKind::Hls,
                other => {
                    failures.push(format!("unsupported protocol {other:?}"));
                    continue;
                }
            };
            let media_url = match self
                .resolve_transcoding_url(&transcoding.url, track_authorization.as_deref())
            {
                Ok(url) => url,
                Err(error) => {
                    failures.push(format!("{kind:?}: {error}"));
                    continue;
                }
            };
            match self.verify_media_url(&media_url, kind) {
                Ok(()) => {
                    return Ok(StreamSource {
                        url: media_url,
                        kind,
                    });
                }
                Err(error) => failures.push(format!("{kind:?}: {error}")),
            }
        }

        bail!(
            "could not resolve a playable mp3 stream for SoundCloud track {} (\"{}\"): {}",
            track_id,
            track.title,
            failures.join("; ")
        );
    }

    /// Download an artwork image, validate its magic bytes, and write it
    /// atomically to dest, creating parent directories as needed.
    ///
    /// If dest already contains a supported image it is reused.
    pub fn fetch_artwork(&self, url: &str, dest: &Path) -> Result<PathBuf> {
        let url = url.trim();
        if !(url.starts_with("https://") || url.starts_with("http://")) {
            bail!("artwork URL is not absolute: {url:?}");
        }
        if let Ok(existing) = fs::read(dest) {
            if is_supported_image(&existing) {
                return Ok(dest.to_path_buf());
            }
        }

        let mut last_error = String::from("unknown error");
        let mut body: Option<Vec<u8>> = None;
        for attempt in 0..=MAX_RETRIES {
            match self
                .http
                .get(url)
                .header(ACCEPT, "image/webp,image/png,image/jpeg,image/gif,*/*")
                .send()
            {
                Ok(response) => {
                    let status = response.status();
                    if !status.is_success() {
                        last_error = format!("HTTP {status}");
                        let retryable = status.as_u16() == 429 || status.is_server_error();
                        if !retryable {
                            break;
                        }
                    } else {
                        if let Some(length) = response.content_length() {
                            if length as usize > MAX_ARTWORK_BYTES {
                                bail!(
                                    "artwork at {url} is larger than {} MiB",
                                    MAX_ARTWORK_BYTES / 1024 / 1024
                                );
                            }
                        }
                        match response.bytes() {
                            Ok(bytes) if !bytes.is_empty() => {
                                if bytes.len() > MAX_ARTWORK_BYTES {
                                    bail!(
                                        "artwork at {url} is larger than {} MiB",
                                        MAX_ARTWORK_BYTES / 1024 / 1024
                                    );
                                }
                                body = Some(bytes.to_vec());
                                break;
                            }
                            Ok(_) => last_error = "empty artwork response".to_string(),
                            Err(error) => last_error = error.to_string(),
                        }
                    }
                }
                Err(error) => last_error = error.to_string(),
            }
            if attempt < MAX_RETRIES {
                std::thread::sleep(backoff(attempt as u32 + 1));
            }
        }

        let bytes =
            body.ok_or_else(|| anyhow!("downloading artwork {url} failed: {last_error}"))?;
        if !is_supported_image(&bytes) {
            bail!("artwork at {url} is not a JPEG, PNG, WebP, or GIF image");
        }

        let parent = dest
            .parent()
            .filter(|path| !path.as_os_str().is_empty())
            .unwrap_or_else(|| Path::new("."));
        fs::create_dir_all(parent)
            .with_context(|| format!("creating artwork directory {}", parent.display()))?;
        let file_name = dest
            .file_name()
            .and_then(|name| name.to_str())
            .ok_or_else(|| anyhow!("artwork destination {} has no file name", dest.display()))?;
        let temp = parent.join(format!(
            ".{file_name}.{}.{}.tmp",
            std::process::id(),
            now_nanos()
        ));
        let write_result = (|| -> Result<()> {
            let mut file = fs::File::create(&temp)
                .with_context(|| format!("creating temporary artwork {}", temp.display()))?;
            file.write_all(&bytes)?;
            file.sync_all().ok();
            Ok(())
        })();
        if let Err(error) = write_result {
            let _ = fs::remove_file(&temp);
            return Err(error);
        }
        if let Err(error) = fs::rename(&temp, dest) {
            let _ = fs::remove_file(&temp);
            return Err(anyhow!("finalising artwork {}: {error}", dest.display()));
        }
        Ok(dest.to_path_buf())
    }

    fn cached_track_value(&self, track_id: u64) -> Option<Value> {
        let cache = lock(&self.track_cache);
        cache
            .get(&track_id)
            .filter(|entry| entry.fetched_at.elapsed() < TRACK_CACHE_TTL)
            .map(|entry| entry.value.clone())
    }

    fn remember_track_value(&self, track_id: u64, value: Value) {
        let mut cache = lock(&self.track_cache);
        if cache.len() >= TRACK_CACHE_MAX {
            cache.retain(|_, entry| entry.fetched_at.elapsed() < TRACK_CACHE_TTL);
            if cache.len() >= TRACK_CACHE_MAX {
                cache.clear();
            }
        }
        cache.insert(
            track_id,
            CachedTrackValue {
                fetched_at: Instant::now(),
                value,
            },
        );
    }

    fn fetch_track_value(&self, track_id: u64) -> Result<Value, ApiError> {
        if let Some(value) = self.cached_track_value(track_id) {
            return Ok(value);
        }
        let value = self.api_get(&format!("/tracks/{track_id}"), &[])?;
        self.remember_track_value(track_id, value.clone());
        Ok(value)
    }

    fn persist_cache(&self, client_id: &str, source: &str) -> Result<()> {
        let cache = ClientIdCache {
            client_id: client_id.to_string(),
            saved_at: now_unix(),
            source: source.to_string(),
        };
        let bytes = serde_json::to_vec_pretty(&cache)?;
        write_atomic(&self.cache_path, &bytes)
    }

    fn resolve_transcoding_url(
        &self,
        transcoding_url: &str,
        track_authorization: Option<&str>,
    ) -> Result<String> {
        let mut params: Vec<(&str, String)> = Vec::new();
        if let Some(authorization) = track_authorization.filter(|value| !value.is_empty()) {
            params.push(("track_authorization", authorization.to_string()));
        }
        let value = self
            .api_get(transcoding_url, &params)
            .map_err(|error| anyhow!("transcoding request failed: {error}"))?;
        let media_url = value
            .get("url")
            .and_then(Value::as_str)
            .filter(|url| url.starts_with("http"))
            .ok_or_else(|| anyhow!("SoundCloud did not return a media URL for this transcoding"))?;
        Ok(media_url.to_string())
    }

    fn verify_media_url(&self, url: &str, kind: StreamKind) -> Result<()> {
        if !(url.starts_with("https://") || url.starts_with("http://")) {
            bail!("resolved media URL is not absolute");
        }
        let mut last_error = String::from("unknown error");
        for attempt in 0..=MAX_RETRIES {
            match self
                .http
                .get(url)
                .header(RANGE, "bytes=0-2047")
                .header(ACCEPT, "*/*")
                .send()
            {
                Ok(response) => {
                    let status = response.status();
                    if !status.is_success() {
                        last_error = format!("HTTP {status}");
                    } else {
                        match response.bytes() {
                            Ok(bytes) if !bytes.is_empty() => {
                                let encrypted = hls_playlist_encrypted(&bytes);
                                let usable = match kind {
                                    StreamKind::Progressive => {
                                        bytes.len() >= 128 && is_mpeg_audio(&bytes)
                                    }
                                    StreamKind::Hls => bytes.starts_with(b"#EXTM3U") && !encrypted,
                                };
                                if usable {
                                    return Ok(());
                                }
                                last_error = if encrypted {
                                    "HLS playlist is DRM-protected (#EXT-X-KEY)".to_string()
                                } else {
                                    match kind {
                                        StreamKind::Progressive => {
                                            "audio probe did not contain MPEG audio".to_string()
                                        }
                                        StreamKind::Hls => {
                                            "HLS playlist did not start with #EXTM3U".to_string()
                                        }
                                    }
                                };
                            }
                            Ok(_) => last_error = "empty media response".to_string(),
                            Err(error) => last_error = error.to_string(),
                        }
                    }
                }
                Err(error) => last_error = error.to_string(),
            }
            if attempt < MAX_RETRIES {
                std::thread::sleep(backoff(attempt as u32 + 1));
            }
        }
        bail!("resolved media URL is not playable: {last_error}")
    }

    fn refresh_after_auth_failure(&self, failed_generation: u64) -> Result<String> {
        let _guard = lock(&self.refresh_lock);
        {
            let state = lock(&self.state);
            if state.generation != failed_generation {
                return Ok(state.client_id.clone());
            }
        }
        let (client_id, source) =
            acquire_client_id(&self.http).context("refreshing a rejected client id")?;
        {
            let mut state = lock(&self.state);
            state.client_id = client_id.clone();
            state.generation = state.generation.wrapping_add(1);
        }
        let _ = self.persist_cache(&client_id, source);
        Ok(client_id)
    }

    /// Perform one API request with pacing, exponential backoff, and a single
    /// client-id refresh on authentication failure.
    fn api_get(&self, path_or_url: &str, params: &[(&str, String)]) -> Result<Value, ApiError> {
        let url = if path_or_url.starts_with("http") {
            path_or_url.to_string()
        } else {
            format!("{API_BASE}{path_or_url}")
        };

        let mut auth_retry_used = false;
        let mut attempt = 0usize;
        loop {
            let (client_id, generation) = {
                let state = lock(&self.state);
                (state.client_id.clone(), state.generation)
            };
            let mut query: Vec<(&str, String)> = params.to_vec();
            query.push(("client_id", client_id));

            RATE_LIMITER.wait();
            match self.http.get(&url).query(&query).send() {
                Ok(response) => {
                    let status = response.status();
                    if status.is_success() {
                        match response.bytes() {
                            Ok(bytes) => {
                                return serde_json::from_slice(&bytes).map_err(|error| {
                                    ApiError::Decode(format!("invalid JSON from {url}: {error}"))
                                });
                            }
                            Err(error) => {
                                if attempt < MAX_RETRIES {
                                    attempt += 1;
                                    std::thread::sleep(backoff(attempt as u32));
                                    continue;
                                }
                                return Err(ApiError::Transport(error.to_string()));
                            }
                        }
                    }

                    let code = status.as_u16();
                    if code == 401 || code == 403 {
                        if !auth_retry_used {
                            auth_retry_used = true;
                            match self.refresh_after_auth_failure(generation) {
                                Ok(_) => continue,
                                Err(error) => {
                                    return Err(ApiError::Auth(format!(
                                        "client id rejected and refresh failed: {error}"
                                    )));
                                }
                            }
                        }
                        return Err(ApiError::Auth(
                            "SoundCloud rejected the client id".to_string(),
                        ));
                    }
                    if code == 404 {
                        return Err(ApiError::NotFound);
                    }
                    if (code == 429 || status.is_server_error()) && attempt < MAX_RETRIES {
                        attempt += 1;
                        let retry_after = response
                            .headers()
                            .get(reqwest::header::RETRY_AFTER)
                            .and_then(|value| value.to_str().ok())
                            .and_then(|value| value.trim().parse::<u64>().ok())
                            .map(|seconds| Duration::from_secs(seconds.min(3)));
                        std::thread::sleep(retry_after.unwrap_or_else(|| backoff(attempt as u32)));
                        continue;
                    }
                    let body = response.text().unwrap_or_default();
                    return Err(ApiError::Status {
                        code,
                        body: truncate_text(&body, 240),
                    });
                }
                Err(error) => {
                    if attempt < MAX_RETRIES {
                        attempt += 1;
                        std::thread::sleep(backoff(attempt as u32));
                        continue;
                    }
                    return Err(ApiError::Transport(error.to_string()));
                }
            }
        }
    }
}

/// True when a track passes the basic queueing checks.
///
/// This enforces the duration window (45 s-45 min), streamability, and a
/// policy that is neither BLOCK nor SNIP. It cannot see transcodings because
/// [Track] does not carry them; [SoundCloudClient::search_tracks] and friends
/// additionally filter to MP3-playable uploads, and
/// [SoundCloudClient::stream_support] exposes the same check for a track id.
pub fn is_playable(track: &Track) -> bool {
    track.id != 0
        && !track.title.trim().is_empty()
        && track.streamable
        && !policy_blocked(&track.policy)
        && (MIN_TRACK_MS..=MAX_TRACK_MS).contains(&track.duration_ms)
}

fn policy_blocked(policy: &str) -> bool {
    let policy = policy.trim().to_ascii_uppercase();
    policy.contains("BLOCK") || policy.contains("SNIP")
}

fn build_http() -> Result<Client> {
    let mut headers = HeaderMap::new();
    headers.insert(
        ACCEPT,
        HeaderValue::from_static("application/json, text/plain, */*"),
    );
    headers.insert(ORIGIN, HeaderValue::from_static("https://soundcloud.com"));
    headers.insert(REFERER, HeaderValue::from_static("https://soundcloud.com/"));
    Client::builder()
        .user_agent(USER_AGENT)
        .default_headers(headers)
        .timeout(REQUEST_TIMEOUT)
        .connect_timeout(CONNECT_TIMEOUT)
        .build()
        .context("building the HTTP client")
}

fn normalize_client_id(raw: String) -> Result<String> {
    let id = raw.trim().to_string();
    let valid = (8..=128).contains(&id.len())
        && id
            .chars()
            .all(|ch| ch.is_ascii_alphanumeric() || ch == '-' || ch == '_');
    if !valid {
        bail!("invalid SoundCloud client id (expected an alphanumeric token)");
    }
    Ok(id)
}

fn client_id_cache_path() -> PathBuf {
    if let Some(directory) = std::env::var_os("GAYDAR_CACHE_DIR") {
        return PathBuf::from(directory).join("client_id.json");
    }
    let base = dirs::cache_dir().unwrap_or_else(std::env::temp_dir);
    base.join("gaydar").join("client_id.json")
}

fn load_cached_client_id(path: &Path) -> Option<String> {
    let bytes = fs::read(path).ok()?;
    let cache: ClientIdCache = serde_json::from_slice(&bytes).ok()?;
    normalize_client_id(cache.client_id).ok()
}

fn write_atomic(path: &Path, bytes: &[u8]) -> Result<()> {
    if let Some(parent) = path.parent().filter(|path| !path.as_os_str().is_empty()) {
        fs::create_dir_all(parent).with_context(|| format!("creating {}", parent.display()))?;
    }
    let file_name = path
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or("gaydar-cache.json");
    let temp = path.with_file_name(format!(
        ".{file_name}.{}.{}.tmp",
        std::process::id(),
        now_nanos()
    ));
    let result = (|| -> Result<()> {
        let mut file =
            fs::File::create(&temp).with_context(|| format!("creating {}", temp.display()))?;
        file.write_all(bytes)?;
        file.sync_all().ok();
        Ok(())
    })();
    if let Err(error) = result {
        let _ = fs::remove_file(&temp);
        return Err(error);
    }
    if let Err(error) = fs::rename(&temp, path) {
        let _ = fs::remove_file(&temp);
        return Err(error).with_context(|| format!("replacing {}", path.display()));
    }
    Ok(())
}

fn client_ids_in(text: &str) -> Vec<String> {
    CLIENT_ID_RE
        .captures_iter(text)
        .filter_map(|captures| captures.get(1).map(|m| m.as_str().to_string()))
        .collect()
}

fn asset_urls_in(html: &str) -> Vec<String> {
    let mut urls: Vec<String> = Vec::new();
    for found in ASSET_RE.find_iter(html) {
        let url = found.as_str();
        let url = match url.strip_prefix("//") {
            Some(rest) => format!("https://{rest}"),
            None => url.to_string(),
        };
        if !urls.contains(&url) {
            urls.push(url);
        }
    }
    urls
}

fn fetch_web_text(http: &Client, url: &str, max_bytes: usize) -> Result<String> {
    RATE_LIMITER.wait();
    let response = http
        .get(url)
        .header(ACCEPT, "*/*")
        .send()
        .with_context(|| format!("GET {url}"))?;
    let status = response.status();
    if !status.is_success() {
        bail!("GET {url} returned HTTP {status}");
    }
    if let Some(length) = response.content_length() {
        if length as usize > max_bytes {
            bail!("GET {url} is too large ({length} bytes)");
        }
    }
    let mut buffer = Vec::new();
    response
        .take(max_bytes as u64 + 1)
        .read_to_end(&mut buffer)
        .with_context(|| format!("reading {url}"))?;
    if buffer.len() > max_bytes {
        bail!("GET {url} exceeded the {max_bytes}-byte limit");
    }
    Ok(String::from_utf8_lossy(&buffer).into_owned())
}

fn validate_client_id_with(http: &Client, client_id: &str) -> Result<bool> {
    RATE_LIMITER.wait();
    let response = http
        .get(format!("{API_BASE}/search/tracks"))
        .query(&[("q", "pride"), ("limit", "1"), ("client_id", client_id)])
        .send()
        .map_err(|error| anyhow!("validating client id: {error}"))?;
    match response.status().as_u16() {
        200..=299 => Ok(true),
        401 | 403 => Ok(false),
        code => bail!("SoundCloud returned HTTP {code} while validating a client id"),
    }
}

fn discover_with(http: &Client) -> Result<String> {
    let html = fetch_web_text(http, &format!("{WEB_BASE}/"), 4 * 1024 * 1024)
        .context("fetching soundcloud.com")?;
    let mut seen: HashSet<String> = HashSet::new();
    for candidate in client_ids_in(&html) {
        if !seen.insert(candidate.clone()) {
            continue;
        }
        if validate_client_id_with(http, &candidate).unwrap_or(false) {
            return Ok(candidate);
        }
    }

    let assets = asset_urls_in(&html);
    if assets.is_empty() {
        bail!("soundcloud.com did not reference any JavaScript assets");
    }
    if let Some(client_id) = scan_assets_for_client_id(http, &assets, &mut seen) {
        return Ok(client_id);
    }
    bail!("no working client id found in SoundCloud's web assets")
}

fn scan_assets_for_client_id(
    http: &Client,
    assets: &[String],
    seen: &mut HashSet<String>,
) -> Option<String> {
    let (sender, receiver) = mpsc::channel::<(String, String)>();
    let done = Arc::new(AtomicBool::new(false));

    for asset in assets.iter().take(MAX_ASSETS) {
        let http = http.clone();
        let sender = sender.clone();
        let done = Arc::clone(&done);
        let asset = asset.clone();
        std::thread::spawn(move || {
            if done.load(Ordering::SeqCst) {
                return;
            }
            if let Ok(text) = fetch_web_text(&http, &asset, MAX_ASSET_BYTES) {
                if !done.load(Ordering::SeqCst) {
                    let _ = sender.send((asset, text));
                }
            }
        });
    }
    drop(sender);

    let deadline = Instant::now() + DISCOVERY_DEADLINE;
    loop {
        let remaining = deadline.saturating_duration_since(Instant::now());
        if remaining.is_zero() {
            break;
        }
        match receiver.recv_timeout(remaining) {
            Ok((_asset, text)) => {
                for candidate in client_ids_in(&text) {
                    if !seen.insert(candidate.clone()) {
                        continue;
                    }
                    if validate_client_id_with(http, &candidate).unwrap_or(false) {
                        done.store(true, Ordering::SeqCst);
                        return Some(candidate);
                    }
                }
            }
            Err(_) => break,
        }
    }
    None
}

fn acquire_client_id(http: &Client) -> Result<(String, &'static str)> {
    let extraction_error = match discover_with(http) {
        Ok(client_id) => return Ok((client_id, "extracted")),
        Err(error) => error,
    };
    for candidate in FALLBACK_CLIENT_IDS {
        if validate_client_id_with(http, candidate).unwrap_or(false) {
            return Ok(((*candidate).to_string(), "fallback"));
        }
    }
    Err(anyhow!(
        "could not obtain a working SoundCloud client id; SoundCloud may be unreachable or blocking this network (last extraction error: {extraction_error})"
    ))
}

/// Map a list endpoint (search, related, user tracks, charts) to queueable
/// tracks.
///
/// Besides the duration and policy checks, this keeps only uploads with a
/// non-snipped MP3 transcoding, which is the only stream family that plays
/// without DRM. List responses normally include the media object; if one does
/// not, up to eight details are fetched (and cached) before giving up.
fn collection_tracks(client: &SoundCloudClient, value: &Value, key: &str) -> Vec<Track> {
    let Some(items) = value.get(key).and_then(Value::as_array) else {
        return Vec::new();
    };
    let mut tracks = Vec::new();
    let mut detail_lookups = 0usize;
    for item in items {
        let track_value = item.get("track").unwrap_or(item);
        let Some(track) = parse_track(track_value) else {
            continue;
        };
        if !is_playable(&track) || !looks_like_music(&track) {
            continue;
        }
        let has_transcodings = track_value
            .get("media")
            .and_then(|media| media.get("transcodings"))
            .is_some();
        let support = if has_transcodings {
            stream_support_from_value(track_value)
        } else if detail_lookups < 8 {
            detail_lookups += 1;
            match client.stream_support(track.id) {
                Ok(support) => support,
                Err(_) => continue,
            }
        } else {
            continue;
        };
        if !support.is_playable() {
            continue;
        }
        client.remember_track_value(track.id, track_value.clone());
        tracks.push(track);
    }
    tracks
}

/// Classify the transcodings in a raw SoundCloud track JSON object.
///
/// A track is playable only when it exposes a non-snipped MP3 transcoding.
/// AAC/ABR transcodings are FairPlay DRM and are reported as
/// [StreamSupport::DrmOnly] rather than queued and retried.
pub fn stream_support_from_value(value: &Value) -> StreamSupport {
    let transcodings: Vec<Transcoding> = parse_transcodings(value)
        .into_iter()
        .filter(|transcoding| !transcoding.is_snipped())
        .collect();
    if transcodings.is_empty() {
        return StreamSupport::Unavailable;
    }
    let mp3: Vec<&Transcoding> = transcodings.iter().filter(|item| item.is_mp3()).collect();
    if mp3.is_empty() {
        return StreamSupport::DrmOnly;
    }
    if mp3
        .iter()
        .any(|item| item.protocol().eq_ignore_ascii_case("progressive"))
    {
        return StreamSupport::Mp3Progressive;
    }
    if mp3
        .iter()
        .any(|item| item.protocol().eq_ignore_ascii_case("hls"))
    {
        return StreamSupport::Mp3Hls;
    }
    StreamSupport::DrmOnly
}

/// Reject obvious talks, podcasts and slur-titled uploads before they reach
/// the queue. A music radio should not interrupt the mix with a 36-minute
/// panel discussion just because SoundCloud marked it streamable.
fn looks_like_music(track: &Track) -> bool {
    let title = track.title.to_lowercase();
    let genre = track.genre.as_deref().unwrap_or("").to_lowercase();
    let description = track.description.as_deref().unwrap_or("").to_lowercase();
    let tags = track.tags.join(" ").to_lowercase();

    const TALK: [&str; 12] = [
        "podcast",
        "interview",
        "oral history",
        "talk show",
        "panel discussion",
        "lecture",
        "audiobook",
        "radio show",
        "episode",
        "ep. ",
        "discussion",
        "review",
    ];
    if TALK
        .iter()
        .any(|needle| title.contains(needle) || description.contains(needle))
    {
        return false;
    }
    if genre.contains("podcast")
        || genre.contains("talk")
        || genre.contains("spoken word")
        || genre.contains("audiobook")
        || genre.contains("comedy")
    {
        return false;
    }
    if tags.contains("podcast") || tags.contains("interview") || tags.contains("talk show") {
        return false;
    }

    // A small, deliberately narrow safety block. Reclaimed words like
    // "queer", "dyke" and "faggot" in artistic titles are not blanket-banned;
    // these are slurs most likely to make an algorithmic radio hostile.
    const BLOCKED: [&str; 6] = ["nigger", "nigga", "tranny", "retard", "kike", "spic"];
    let haystack = format!("{title} {description} {tags}");
    !BLOCKED.iter().any(|word| haystack.contains(word))
}

fn dedup_tracks(tracks: Vec<Track>) -> Vec<Track> {
    let mut seen = HashSet::new();
    tracks
        .into_iter()
        .filter(|track| seen.insert(track.id))
        .collect()
}

fn number(value: &Value, key: &str) -> Option<u64> {
    match value.get(key) {
        Some(Value::Number(number)) => number
            .as_u64()
            .or_else(|| number.as_f64().map(|value| value.max(0.0) as u64)),
        Some(Value::String(text)) => text.trim().parse().ok(),
        _ => None,
    }
}

fn parse_track(value: &Value) -> Option<Track> {
    let id = value.get("id").and_then(Value::as_u64)?;
    if id == 0 {
        return None;
    }
    let user = value.get("user");
    let artist = user
        .and_then(|user| user.get("username"))
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|name| !name.is_empty())
        .unwrap_or("Unknown Artist")
        .to_string();
    let artist_id = user.and_then(|user| user.get("id")).and_then(Value::as_u64);
    let avatar = user
        .and_then(|user| user.get("avatar_url"))
        .and_then(Value::as_str)
        .and_then(normalize_artwork_url);
    let artwork = value
        .get("artwork_url")
        .and_then(Value::as_str)
        .and_then(normalize_artwork_url)
        .or_else(|| avatar.clone());
    let duration_ms = number(value, "duration")
        .or_else(|| number(value, "full_duration"))
        .unwrap_or(0);
    let title = value
        .get("title")
        .and_then(Value::as_str)
        .unwrap_or("")
        .trim()
        .to_string();
    let genre = value
        .get("genre")
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|genre| !genre.is_empty())
        .map(str::to_string);
    let description = value
        .get("description")
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|description| !description.is_empty())
        .map(|description| description.chars().take(4000).collect::<String>());
    let tags = parse_tags(
        value
            .get("tag_list")
            .and_then(Value::as_str)
            .unwrap_or_default(),
    );

    Some(Track {
        id,
        title,
        artist,
        artist_id,
        artist_avatar_url: avatar,
        artwork_url: artwork,
        duration_ms,
        genre,
        tags,
        description,
        playback_count: number(value, "playback_count").unwrap_or(0),
        likes_count: number(value, "likes_count").unwrap_or(0),
        user_followers: user
            .and_then(|user| number(user, "followers_count"))
            .unwrap_or(0),
        created_at: value
            .get("created_at")
            .and_then(Value::as_str)
            .map(str::to_string),
        permalink_url: permalink_url(value, user),
        streamable: value
            .get("streamable")
            .and_then(Value::as_bool)
            .unwrap_or(true),
        policy: value
            .get("policy")
            .and_then(Value::as_str)
            .unwrap_or("ALLOW")
            .to_string(),
    })
}

fn permalink_url(value: &Value, user: Option<&Value>) -> String {
    if let Some(url) = value
        .get("permalink_url")
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|url| !url.is_empty())
    {
        return url.to_string();
    }
    let track_slug = value
        .get("permalink")
        .and_then(Value::as_str)
        .unwrap_or_default();
    let user_slug = user
        .and_then(|user| user.get("permalink"))
        .and_then(Value::as_str)
        .unwrap_or_default();
    if !user_slug.is_empty() && !track_slug.is_empty() {
        format!("{WEB_BASE}/{user_slug}/{track_slug}")
    } else {
        WEB_BASE.to_string()
    }
}

/// SoundCloud tag_list is inconsistently formatted: legacy tags are
/// comma-separated, modern tags are quoted and space-separated, and plenty of
/// uploads mix both. Quoted runs are always one tag; the unquoted remainder is
/// split on commas when commas are present, otherwise on whitespace.
fn parse_tags(raw: &str) -> Vec<String> {
    if raw.trim().is_empty() {
        return Vec::new();
    }
    let mut tags: Vec<String> = Vec::new();
    let mut rest = String::with_capacity(raw.len());
    let mut current = String::new();
    let mut in_quotes = false;
    for character in raw.chars() {
        match character {
            '"' if in_quotes => {
                push_tag(&mut tags, &current);
                current.clear();
                in_quotes = false;
            }
            '"' => in_quotes = true,
            _ if in_quotes => current.push(character),
            _ => rest.push(character),
        }
    }
    if in_quotes {
        push_tag(&mut tags, &current);
    }
    if rest.contains(',') {
        for part in rest.split(',') {
            push_tag(&mut tags, part);
        }
    } else {
        for part in rest.split_whitespace() {
            push_tag(&mut tags, part);
        }
    }
    tags.truncate(12);
    tags
}

fn push_tag(tags: &mut Vec<String>, raw: &str) {
    let tag = raw.trim().trim_start_matches('#').trim();
    if tag.is_empty() {
        return;
    }
    let tag = tag.chars().take(48).collect::<String>().to_lowercase();
    if !tags.iter().any(|existing| existing == &tag) {
        tags.push(tag);
    }
}

/// Normalise artwork and avatar URLs to the 500 px square variant.
fn normalize_artwork_url(url: &str) -> Option<String> {
    let url = url.trim();
    if url.is_empty() || !(url.starts_with("https://") || url.starts_with("http://")) {
        return None;
    }
    const VARIANTS: [&str; 10] = [
        "-large.",
        "-t500x500.",
        "-t300x300.",
        "-t200x200.",
        "-t120x120.",
        "-t60x60.",
        "-small.",
        "-tiny.",
        "-mini.",
        "-badge.",
    ];
    for variant in VARIANTS {
        if let Some(index) = url.find(variant) {
            return Some(format!(
                "{}-t500x500.{}",
                &url[..index],
                &url[index + variant.len()..]
            ));
        }
    }
    Some(url.to_string())
}

#[derive(Debug, Clone, Deserialize)]
struct Transcoding {
    url: String,
    #[serde(default)]
    preset: Option<String>,
    #[serde(default)]
    snipped: Option<bool>,
    #[serde(default)]
    format: Option<TranscodingFormat>,
}

#[derive(Debug, Clone, Deserialize)]
struct TranscodingFormat {
    #[serde(default)]
    protocol: Option<String>,
    #[serde(default)]
    mime_type: Option<String>,
}

impl Transcoding {
    fn protocol(&self) -> &str {
        self.format
            .as_ref()
            .and_then(|format| format.protocol.as_deref())
            .unwrap_or_default()
    }

    fn mime_type(&self) -> &str {
        self.format
            .as_ref()
            .and_then(|format| format.mime_type.as_deref())
            .unwrap_or_default()
    }

    fn is_snipped(&self) -> bool {
        self.snipped.unwrap_or(false)
    }

    /// True for the only transcoding family that plays without DRM.
    fn is_mp3(&self) -> bool {
        if let Some(preset) = &self.preset {
            let preset = preset.trim().to_ascii_lowercase();
            if !preset.is_empty() {
                return preset.starts_with("mp3");
            }
        }
        self.mime_type().to_ascii_lowercase().contains("mpeg")
    }
}

fn parse_transcodings(value: &Value) -> Vec<Transcoding> {
    value
        .get("media")
        .and_then(|media| media.get("transcodings"))
        .and_then(Value::as_array)
        .map(|items| {
            items
                .iter()
                .filter_map(|item| serde_json::from_value::<Transcoding>(item.clone()).ok())
                .collect()
        })
        .unwrap_or_default()
}

/// Sort key: progressive before HLS, MP3 before other codecs, full streams
/// before snipped previews, unknown protocols last.
fn transcoding_rank(transcoding: &Transcoding) -> (u8, u8, u8) {
    let protocol = transcoding.protocol().to_ascii_lowercase();
    let protocol_rank = match protocol.as_str() {
        "progressive" => 0,
        "hls" => 1,
        _ => 9,
    };
    let codec_rank = if transcoding
        .mime_type()
        .to_ascii_lowercase()
        .contains("mpeg")
    {
        0
    } else {
        1
    };
    let snipped_rank = u8::from(transcoding.is_snipped());
    (snipped_rank, protocol_rank, codec_rank)
}

/// True when a media probe starts like an MPEG audio stream (MP3 frame sync
/// or an ID3 tag). This prevents an HTML error page with a 200 status from
/// being treated as a playable progressive stream.
fn is_mpeg_audio(bytes: &[u8]) -> bool {
    bytes.starts_with(b"ID3") || bytes.first() == Some(&0xFF)
}

/// Detect FairPlay or AES keys in an HLS media playlist. METHOD=NONE keys are
/// not encryption and are tolerated.
fn hls_playlist_encrypted(bytes: &[u8]) -> bool {
    let text = String::from_utf8_lossy(bytes);
    text.lines().any(|line| {
        let line = line.trim();
        (line.starts_with("#EXT-X-KEY") || line.starts_with("#EXT-X-SESSION-KEY"))
            && !line.to_ascii_uppercase().contains("METHOD=NONE")
    })
}

fn is_supported_image(bytes: &[u8]) -> bool {
    const PNG_MAGIC: [u8; 8] = [0x89, b'P', b'N', b'G', 0x0D, 0x0A, 0x1A, 0x0A];
    bytes.starts_with(&[0xFF, 0xD8, 0xFF])
        || bytes.starts_with(&PNG_MAGIC)
        || bytes.starts_with(b"GIF87a")
        || bytes.starts_with(b"GIF89a")
        || (bytes.len() >= 12 && &bytes[0..4] == b"RIFF" && &bytes[8..12] == b"WEBP")
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn parses_quoted_comma_and_space_tag_lists() {
        assert_eq!(
            parse_tags(r#""queer" "hyperpop" drag"#),
            vec!["queer", "hyperpop", "drag"]
        );
        assert_eq!(
            parse_tags("pop, queer, drag queen"),
            vec!["pop", "queer", "drag queen"]
        );
        assert_eq!(
            parse_tags("hyperpop sophie 2023"),
            vec!["hyperpop", "sophie", "2023"]
        );
        assert_eq!(
            parse_tags(r#"hyperpop "pc music " unreleased"#),
            vec!["pc music", "hyperpop", "unreleased"]
        );
        assert!(parse_tags("   ").is_empty());
    }

    #[test]
    fn normalises_artwork_and_avatar_variants() {
        assert_eq!(
            normalize_artwork_url("https://i1.sndcdn.com/artworks-x-large.jpg").as_deref(),
            Some("https://i1.sndcdn.com/artworks-x-t500x500.jpg")
        );
        assert_eq!(
            normalize_artwork_url("https://i1.sndcdn.com/avatars-y-small.png").as_deref(),
            Some("https://i1.sndcdn.com/avatars-y-t500x500.png")
        );
        assert_eq!(normalize_artwork_url("  ").as_deref(), None);
    }

    #[test]
    fn classifies_mp3_hls_and_drm_transcodings() {
        let progressive = json!({
            "media": {"transcodings": [{
                "url": "u", "preset": "mp3_1_0",
                "format": {"protocol": "progressive", "mime_type": "audio/mpeg"}
            }]}
        });
        assert_eq!(
            stream_support_from_value(&progressive),
            StreamSupport::Mp3Progressive
        );

        let hls = json!({
            "media": {"transcodings": [{
                "url": "u", "preset": "mp3_0_1",
                "format": {"protocol": "hls", "mime_type": "audio/mpeg"}
            }]}
        });
        assert_eq!(stream_support_from_value(&hls), StreamSupport::Mp3Hls);

        let drm = json!({
            "media": {"transcodings": [{
                "url": "u", "preset": "aac_160k",
                "format": {"protocol": "hls", "mime_type": "audio/mp4"}
            }]}
        });
        assert_eq!(stream_support_from_value(&drm), StreamSupport::DrmOnly);

        let snipped = json!({
            "media": {"transcodings": [{
                "url": "u", "preset": "mp3_1_0", "snipped": true,
                "format": {"protocol": "progressive", "mime_type": "audio/mpeg"}
            }]}
        });
        assert_eq!(
            stream_support_from_value(&snipped),
            StreamSupport::Unavailable
        );

        let none = json!({"media": {"transcodings": []}});
        assert_eq!(stream_support_from_value(&none), StreamSupport::Unavailable);
    }

    #[test]
    fn checks_image_magic() {
        assert!(is_supported_image(&[0xFF, 0xD8, 0xFF, 0x00]));
        assert!(is_supported_image(b"GIF89a----"));
        assert!(is_supported_image(b"RIFF0000WEBP"));
        assert!(!is_supported_image(b"<html>nope</html>"));
        assert!(!is_supported_image(&[]));
    }
}
