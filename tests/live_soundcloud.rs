//! Live SoundCloud and Gaydar checks.
//!
//! Run with:
//!     cargo test --test live_soundcloud -- --nocapture
//!
//! The tests need internet access to soundcloud.com and api-v2.soundcloud.com.
//! They exercise the same public API the player uses; no account or OAuth is
//! involved.

use gaydar::gaydar::GaydarEngine;
use gaydar::model::{Dials, StreamKind};
use gaydar::soundcloud::{
    SoundCloudClient, StreamSupport, StreamUnavailable, stream_support_from_value,
};
use serde_json::Value;
use std::collections::HashSet;
use std::fs;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

const LIVE_CLIENT_ID: &str = "39AVoiZVrdvJEMFLH8T0c5WGshQaAPRJ";
const USER_AGENT: &str = "Mozilla/5.0 (X11; Linux x86_64) AppleWebKit/537.36 \
                          (KHTML, like Gecko) Chrome/124.0.0.0 Safari/537.36";

fn now_nanos() -> u128 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_nanos())
        .unwrap_or(0)
}

fn temp_dir(tag: &str) -> PathBuf {
    let directory = std::env::temp_dir().join(format!(
        "gaydar-live-{tag}-{}-{}",
        std::process::id(),
        now_nanos()
    ));
    fs::create_dir_all(&directory).expect("create temp dir");
    directory
}

fn live_client() -> SoundCloudClient {
    SoundCloudClient::with_client_id(LIVE_CLIENT_ID).expect("create SoundCloud client")
}

fn raw_http() -> reqwest::blocking::Client {
    reqwest::blocking::Client::builder()
        .user_agent(USER_AGENT)
        .timeout(Duration::from_secs(30))
        .build()
        .expect("build raw HTTP client")
}

fn raw_get(url: &str) -> (u16, Vec<u8>, Option<String>) {
    let response = raw_http()
        .get(url)
        .header("Range", "bytes=0-65535")
        .header("Origin", "https://soundcloud.com")
        .header("Referer", "https://soundcloud.com/")
        .send()
        .expect("raw GET");
    let status = response.status().as_u16();
    let content_type = response
        .headers()
        .get("content-type")
        .and_then(|value| value.to_str().ok())
        .map(str::to_string);
    let bytes = response.bytes().expect("read body").to_vec();
    (status, bytes, content_type)
}

fn raw_search(client_id: &str, query: &str) -> Value {
    raw_http()
        .get("https://api-v2.soundcloud.com/search/tracks")
        .query(&[
            ("q", query.to_string()),
            ("limit", "50".to_string()),
            ("client_id", client_id.to_string()),
        ])
        .header("Origin", "https://soundcloud.com")
        .header("Referer", "https://soundcloud.com/")
        .send()
        .expect("raw search")
        .error_for_status()
        .expect("raw search status")
        .json()
        .expect("raw search json")
}

fn support_label(support: StreamSupport) -> &'static str {
    match support {
        StreamSupport::Mp3Progressive => "mp3-progressive",
        StreamSupport::Mp3Hls => "mp3-hls",
        StreamSupport::DrmOnly => "drm-only",
        StreamSupport::Unavailable => "unavailable",
    }
}

#[test]
fn live_client_id_discovery() {
    let started = Instant::now();
    let client_id = SoundCloudClient::discover_client_id().expect("discover client id");
    println!(
        "discovered client_id = {} ({} chars) in {:?}",
        client_id,
        client_id.len(),
        started.elapsed()
    );
    assert_eq!(client_id.len(), 32);
    assert!(
        client_id
            .chars()
            .all(|character| character.is_ascii_alphanumeric())
    );

    let client = SoundCloudClient::with_client_id(client_id).expect("client");
    let tracks = client
        .search_tracks("queer anthem", 20)
        .expect("search with discovered id");
    println!(
        "discovered id validated by search, {} mp3-capable tracks",
        tracks.len()
    );
    assert!(!tracks.is_empty(), "validated client id must return tracks");
}

#[test]
fn live_cached_client_id_startup() {
    // First construction may discover and cache; the second must reuse the
    // cache without another web-bundle scan.
    let first = SoundCloudClient::new().expect("first client");
    let client_id = first.client_id();
    let tracks = first
        .search_tracks("queer anthem", 5)
        .expect("search with cached client");
    println!(
        "cache startup client_id = {}, {} mp3-capable tracks",
        client_id,
        tracks.len()
    );

    let started = Instant::now();
    let second = SoundCloudClient::new().expect("second client");
    let elapsed = started.elapsed();
    println!("cached client construction took {elapsed:?}");
    assert_eq!(second.client_id(), client_id);
    assert!(
        elapsed < Duration::from_secs(2),
        "cached startup should not rescan SoundCloud's bundles"
    );
}

#[test]
fn live_search_resolve_and_download() {
    let client = live_client();
    let queries = [
        "Rina Sawayama",
        "queer anthem",
        "Chappell Roan",
        "Kim Petras",
    ];
    let mut candidates = Vec::new();
    let mut seen = HashSet::new();

    for query in queries {
        let tracks = client.search_tracks(query, 50).expect("search");
        println!(
            "search {query:?}: {} mp3-capable tracks; first: {}",
            tracks.len(),
            tracks
                .first()
                .map(|track| format!("{} - {}", track.artist, track.title))
                .unwrap_or_else(|| "<none>".to_string())
        );
        for track in tracks {
            if seen.insert(track.id) {
                candidates.push(track);
            }
        }
    }
    assert!(!candidates.is_empty(), "search must yield playable tracks");

    let mut verified = 0usize;
    let mut bytes_total = 0usize;
    for track in candidates.iter().take(12) {
        let support = client.stream_support(track.id).expect("stream support");
        println!(
            "candidate {} | {} - {} | support={} | duration={} | tags={:?}",
            track.id,
            track.artist,
            track.title,
            support_label(support),
            track.duration_label(),
            track.tags
        );
        assert!(
            support.is_playable(),
            "search must only return mp3-capable tracks"
        );

        let stream = match client.resolve_stream(track.id) {
            Ok(stream) => stream,
            Err(error) => {
                println!("  resolve failed: {error}");
                continue;
            }
        };
        let (status, bytes, content_type) = raw_get(&stream.url);
        println!(
            "  stream kind={:?} status={} bytes={} content-type={:?} url={}",
            stream.kind,
            status,
            bytes.len(),
            content_type,
            &stream.url[..stream.url.len().min(90)]
        );
        assert!(status == 200 || status == 206, "media GET status {status}");
        assert!(!bytes.is_empty(), "media GET must return bytes");
        match stream.kind {
            StreamKind::Progressive => {
                assert!(
                    bytes[0] == 0xFF || bytes.starts_with(b"ID3"),
                    "progressive stream does not look like MPEG audio"
                );
            }
            StreamKind::Hls => {
                assert!(
                    bytes.starts_with(b"#EXTM3U"),
                    "HLS stream is not a playlist"
                );
                assert!(
                    !bytes.windows(10).any(|window| window == b"#EXT-X-KEY"),
                    "HLS playlist is DRM protected"
                );
            }
        }
        bytes_total += bytes.len();
        verified += 1;
        if verified >= 3 {
            break;
        }
    }
    println!("verified {verified} streams, {bytes_total} bytes downloaded");
    assert!(
        verified >= 3,
        "expected at least three playable mp3 streams"
    );

    // Artwork: normalised to the 500 px variant, magic validated, atomic write.
    let artwork_track = candidates
        .iter()
        .find(|track| track.artwork_url.is_some())
        .expect("at least one track with artwork");
    let artwork_url = artwork_track.artwork_url.clone().unwrap();
    println!("artwork url = {artwork_url}");
    assert!(artwork_url.contains("t500x500"));
    let destination = temp_dir("artwork").join("nested/cover.jpg");
    let path = client
        .fetch_artwork(&artwork_url, &destination)
        .expect("fetch artwork");
    let bytes = fs::read(&path).expect("read artwork");
    println!(
        "artwork written to {} ({} bytes)",
        path.display(),
        bytes.len()
    );
    assert!(!bytes.is_empty());
    assert!(
        bytes.starts_with(&[0xFF, 0xD8, 0xFF])
            || bytes.starts_with(&[0x89, b'P', b'N', b'G'])
            || bytes.starts_with(b"GIF8")
            || (bytes.len() > 12 && &bytes[0..4] == b"RIFF" && &bytes[8..12] == b"WEBP"),
        "artwork magic bytes"
    );
    let again = client
        .fetch_artwork(&artwork_url, &path)
        .expect("cached artwork");
    assert_eq!(again, path);
}

#[test]
fn live_drm_only_track_errors_without_hanging() {
    let client = live_client();
    let collection = raw_search(&client.client_id(), "queer anthem");
    let mut drm_track: Option<(u64, String)> = None;
    for item in collection["collection"]
        .as_array()
        .expect("search collection")
    {
        let support = stream_support_from_value(item);
        let policy = item["policy"].as_str().unwrap_or("ALLOW");
        let streamable = item["streamable"].as_bool().unwrap_or(true);
        let duration = item["duration"].as_u64().unwrap_or(0);
        if support == StreamSupport::DrmOnly
            && policy == "ALLOW"
            && streamable
            && (45_000..=2_700_000).contains(&duration)
        {
            drm_track = Some((
                item["id"].as_u64().expect("track id"),
                item["title"].as_str().unwrap_or("<untitled>").to_string(),
            ));
            break;
        }
    }
    let (track_id, title) = drm_track.expect("sample search should contain a DRM-only track");
    println!("DRM-only sample: {track_id} - {title}");
    assert_eq!(
        client.stream_support(track_id).expect("support"),
        StreamSupport::DrmOnly
    );

    let started = Instant::now();
    let error = client
        .resolve_stream(track_id)
        .expect_err("DRM-only track must not resolve");
    let elapsed = started.elapsed();
    println!("resolve_stream error after {:?}: {}", elapsed, error);
    assert!(
        error.downcast_ref::<StreamUnavailable>().is_some(),
        "DRM-only error must be typed StreamUnavailable, got: {error:?}"
    );
    assert!(
        error.to_string().contains("no playable mp3 stream"),
        "error should say no playable mp3 stream: {error}"
    );
    assert!(elapsed < Duration::from_secs(5), "DRM check must be fast");
}

#[test]
fn live_engine_bootstrap_taste_and_state() {
    let client = Arc::new(live_client());
    let directory = temp_dir("engine");
    let state_path = directory.join("state.json");
    let mut engine = GaydarEngine::new(Arc::clone(&client), state_path.clone());

    let started = Instant::now();
    let picks = engine.bootstrap(6).expect("bootstrap");
    let elapsed = started.elapsed();
    println!("bootstrap(6) took {elapsed:?}");
    for (index, pick) in picks.iter().enumerate() {
        println!(
            "  {}. {} - {} | {:?} | {:.3} | {}",
            index + 1,
            pick.track.artist,
            pick.track.title,
            pick.track.duration_label(),
            pick.score,
            pick.reason
        );
    }
    assert!(
        picks.len() >= 3,
        "bootstrap should return at least three tracks"
    );
    assert!(
        elapsed < Duration::from_secs(20),
        "bootstrap should stay under 20s"
    );

    let artists: HashSet<String> = picks
        .iter()
        .map(|pick| pick.track.artist.to_lowercase())
        .collect();
    println!("bootstrap distinct artists: {artists:?}");
    assert!(artists.len() >= 3, "bootstrap must be artist-diverse");

    // The first pick is verified before bootstrap returns.
    let first_stream = client
        .resolve_stream(picks[0].track.id)
        .expect("first bootstrap pick must resolve");
    println!(
        "first pick verified: {:?} {}",
        first_stream.kind,
        &first_stream.url[..first_stream.url.len().min(90)]
    );
    assert_eq!(
        client
            .stream_support(picks[0].track.id)
            .expect("support")
            .is_playable(),
        true
    );

    let play_track = picks
        .iter()
        .find(|pick| !pick.track.tags.is_empty())
        .unwrap_or(&picks[0])
        .track
        .clone();
    let skip_track = picks
        .iter()
        .find(|pick| pick.track.id != play_track.id)
        .expect("second track")
        .track
        .clone();

    let taste_before = engine.taste_summary();
    engine.record_play(&play_track);
    engine.record_complete(&play_track);
    engine.record_like(&play_track);
    engine.record_skip(&skip_track, 0.1);
    let taste_after = engine.taste_summary();
    println!("taste before: {taste_before:?}");
    println!("taste after: {taste_after:?}");
    println!("top artists after like: {:?}", engine.top_artists(3));
    if !play_track.tags.is_empty() || play_track.genre.is_some() {
        assert_ne!(
            taste_before, taste_after,
            "like/play must move the taste vector"
        );
    } else {
        println!("play track has no tags; the artist weight is the taste signal");
    }
    assert!(engine.is_liked(play_track.id));
    assert!(
        engine
            .liked_tracks()
            .iter()
            .any(|track| track.id == play_track.id)
    );
    assert!(
        engine
            .top_artists(5)
            .iter()
            .any(|artist| artist.eq_ignore_ascii_case(&play_track.artist)),
        "liked artist should top the artist weights"
    );
    assert!(engine.total_plays() >= 1);
    assert!(
        engine
            .history()
            .iter()
            .any(|track| track.id == play_track.id)
    );

    engine.set_dials(Dials {
        camp: 0.9,
        club: 0.5,
        soft: 0.2,
        chaos: 0.8,
    });
    engine.save().expect("explicit save");

    let raw = fs::read_to_string(&state_path).expect("state file");
    let state: Value = serde_json::from_str(&raw).expect("state json");
    println!(
        "state keys={:?} liked={} history={} skipped={} plays={} tag_weights={} artist_weights={}",
        state
            .as_object()
            .map(|object| object.keys().cloned().collect::<Vec<_>>())
            .unwrap_or_default(),
        state["liked"].as_array().map(Vec::len).unwrap_or(0),
        state["history"].as_array().map(Vec::len).unwrap_or(0),
        state["skipped"].as_array().map(Vec::len).unwrap_or(0),
        state["plays"].as_u64().unwrap_or(0),
        state["tag_weights"]
            .as_object()
            .map(|object| object.len())
            .unwrap_or(0),
        state["artist_weights"]
            .as_object()
            .map(|object| object.len())
            .unwrap_or(0)
    );
    assert!(
        state["liked"]
            .as_array()
            .unwrap()
            .iter()
            .any(|item| item["id"].as_u64() == Some(play_track.id))
    );
    assert!(
        state["skipped"]
            .as_array()
            .unwrap()
            .iter()
            .any(|item| item.as_u64() == Some(skip_track.id))
    );
    assert!(state["plays"].as_u64().unwrap_or(0) >= 1);

    // Round-trip through a fresh engine.
    let reloaded = GaydarEngine::new(Arc::clone(&client), state_path.clone());
    assert!(reloaded.is_liked(play_track.id), "like must survive reload");
    assert!(reloaded.total_plays() >= 1, "plays must survive reload");
    assert_eq!(reloaded.dials().chaos, 0.8);
    assert_eq!(reloaded.dials().camp, 0.9);
    assert!(
        reloaded
            .history()
            .iter()
            .any(|track| track.id == play_track.id)
    );
    println!(
        "reloaded state: liked={} history={} plays={} top_artists={:?}",
        reloaded.liked_tracks().len(),
        reloaded.history().len(),
        reloaded.total_plays(),
        reloaded.top_artists(3)
    );

    // Refill must never return a seen, skipped, or liked track.
    let mut refill_engine = reloaded;
    let refill = refill_engine.refill(4).expect("refill");
    let excluded: HashSet<u64> = refill_engine
        .history()
        .iter()
        .map(|track| track.id)
        .chain(refill_engine.liked_tracks().iter().map(|track| track.id))
        .collect();
    println!("refill:");
    for pick in &refill {
        println!(
            "  {} - {} | {}",
            pick.track.artist, pick.track.title, pick.reason
        );
    }
    assert!(!refill.is_empty(), "refill must return tracks");
    for pick in &refill {
        assert!(
            !excluded.contains(&pick.track.id),
            "refill returned an already-played or liked track: {}",
            pick.track.id
        );
    }

    // Corrupt state must not panic and must be set aside.
    let corrupt_dir = temp_dir("corrupt");
    let corrupt_path = corrupt_dir.join("state.json");
    fs::write(&corrupt_path, b"{ this is not json").expect("write corrupt state");
    let corrupt_engine = GaydarEngine::new(Arc::clone(&client), corrupt_path.clone());
    assert_eq!(corrupt_engine.total_plays(), 0);
    assert!(corrupt_engine.liked_tracks().is_empty());
    let backups: Vec<String> = fs::read_dir(&corrupt_dir)
        .expect("read corrupt dir")
        .filter_map(|entry| entry.ok())
        .map(|entry| entry.file_name().to_string_lossy().into_owned())
        .collect();
    println!("corrupt state backup files: {backups:?}");
    assert!(
        backups.iter().any(|name| name.contains("corrupt-")),
        "corrupt state should be renamed aside"
    );
}

#[test]
#[ignore = "slow live check of client id rotation handling"]
fn live_client_id_refresh_on_401() {
    let client =
        SoundCloudClient::with_client_id("aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa").expect("client");
    let started = Instant::now();
    let tracks = client
        .search_tracks("queer anthem", 10)
        .expect("search after refresh");
    println!(
        "refresh path returned {} tracks in {:?}, client_id now {}",
        tracks.len(),
        started.elapsed(),
        client.client_id()
    );
    assert_ne!(client.client_id(), "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa");
    assert!(!tracks.is_empty());
}
