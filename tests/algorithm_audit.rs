//! Live algorithm + curation audit for GAYDAR FM.
//!
//! These tests are ignored by default because they talk to SoundCloud and take
//! minutes. Run one at a time, e.g.:
//!
//!     GAYDAR_CACHE_DIR=/tmp/gaydar-audit-cache \
//!     cargo test --test algorithm_audit audit_bootstrap_eight -- --ignored --exact --nocapture
//!
//! State always lives under a fresh /tmp directory. The client's process-wide
//! pacer keeps SoundCloud API calls near 5 req/s; raw search calls add a sleep.

use gaydar::gaydar::GaydarEngine;
use gaydar::model::{Dials, Recommendation, Track};
use gaydar::soundcloud::{
    SoundCloudClient, StreamSupport, is_playable, stream_support_from_value,
};
use regex::Regex;
use serde_json::Value;
use std::collections::{BTreeSet, HashSet};
use std::fs;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

const USER_AGENT: &str = "Mozilla/5.0 (X11; Linux x86_64) AppleWebKit/537.36 \
                          (KHTML, like Gecko) Chrome/124.0.0.0 Safari/537.36";
const RAW_LIMIT: usize = 50;

fn now_nanos() -> u128 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_nanos())
        .unwrap_or(0)
}

fn temp_dir(tag: &str) -> PathBuf {
    let directory = std::env::temp_dir().join(format!(
        "gaydar-audit-{tag}-{}-{}",
        std::process::id(),
        now_nanos()
    ));
    fs::create_dir_all(&directory).expect("create temp dir");
    directory
}

fn live_client() -> SoundCloudClient {
    SoundCloudClient::new().expect("SoundCloud client (cache or discovery)")
}

fn raw_http() -> reqwest::blocking::Client {
    reqwest::blocking::Client::builder()
        .user_agent(USER_AGENT)
        .timeout(Duration::from_secs(25))
        .build()
        .expect("raw HTTP client")
}

fn raw_search(client_id: &str, query: &str, limit: usize) -> Value {
    let response = raw_http()
        .get("https://api-v2.soundcloud.com/search/tracks")
        .query(&[
            ("q", query.to_string()),
            ("limit", limit.to_string()),
            ("client_id", client_id.to_string()),
        ])
        .header("Origin", "https://soundcloud.com")
        .header("Referer", "https://soundcloud.com/")
        .send()
        .expect("raw search send")
        .error_for_status()
        .expect("raw search status")
        .json()
        .expect("raw search json");
    std::thread::sleep(Duration::from_millis(260));
    response
}

fn policy_blocked(policy: &str) -> bool {
    let policy = policy.trim().to_ascii_uppercase();
    policy.contains("BLOCK") || policy.contains("SNIP")
}

#[derive(Default, Clone)]
struct RawStats {
    total: usize,
    with_media: usize,
    mp3_progressive: usize,
    mp3_hls: usize,
    drm_only: usize,
    unavailable: usize,
    eligible: usize,
    blocked: usize,
    out_of_window: usize,
}

fn classify_raw(value: &Value) -> RawStats {
    let mut stats = RawStats::default();
    let Some(items) = value.get("collection").and_then(Value::as_array) else {
        return stats;
    };
    stats.total = items.len();
    for item in items {
        let track_value = item.get("track").unwrap_or(item);
        let has_media = track_value
            .get("media")
            .and_then(|media| media.get("transcodings"))
            .and_then(Value::as_array)
            .map(|items| !items.is_empty())
            .unwrap_or(false);
        if has_media {
            stats.with_media += 1;
        }
        let support = stream_support_from_value(track_value);
        let policy = track_value
            .get("policy")
            .and_then(Value::as_str)
            .unwrap_or("ALLOW");
        let streamable = track_value
            .get("streamable")
            .and_then(Value::as_bool)
            .unwrap_or(true);
        let duration = track_value
            .get("duration")
            .and_then(Value::as_u64)
            .or_else(|| {
                track_value
                    .get("full_duration")
                    .and_then(Value::as_u64)
            })
            .unwrap_or(0);
        let in_window = (45_000..=2_700_000).contains(&duration);
        if policy_blocked(policy) {
            stats.blocked += 1;
        } else if !in_window {
            stats.out_of_window += 1;
        }
        match support {
            StreamSupport::Mp3Progressive => stats.mp3_progressive += 1,
            StreamSupport::Mp3Hls => stats.mp3_hls += 1,
            StreamSupport::DrmOnly => stats.drm_only += 1,
            StreamSupport::Unavailable => stats.unavailable += 1,
        }
        if support.is_playable() && streamable && !policy_blocked(policy) && in_window {
            stats.eligible += 1;
        }
    }
    stats
}

fn short(text: &str, max: usize) -> String {
    let mut out: String = text.chars().take(max).collect();
    if text.chars().count() > max {
        out.push('…');
    }
    out
}

fn display(track: &Track) -> String {
    format!(
        "{} - {} [{}]",
        track.artist,
        short(&track.title, 64),
        track.duration_label()
    )
}

fn resolve_tracks(
    client: &SoundCloudClient,
    tracks: &[Track],
    cap: usize,
    prefix: &str,
) -> (usize, usize) {
    let mut ok = 0usize;
    let mut failed = 0usize;
    for track in tracks.iter().take(cap) {
        let started = Instant::now();
        match client.resolve_stream(track.id) {
            Ok(stream) => {
                ok += 1;
                println!(
                    "{prefix} RESOLVED id={} {:?} kind={:?} ms={} {}",
                    track.id,
                    display(track),
                    stream.kind,
                    started.elapsed().as_millis(),
                    stream.url.chars().take(80).collect::<String>()
                );
            }
            Err(error) => {
                failed += 1;
                println!(
                    "{prefix} RESOLVE_FAIL id={} {:?} ms={} error={}",
                    track.id,
                    display(track),
                    started.elapsed().as_millis(),
                    short(&error.to_string(), 240)
                );
            }
        }
    }
    (ok, failed)
}

// Lexicons copied from src/gaydar.rs. They are duplicated here on purpose:
// the audit measures the same category formulas without changing source.
const CAMP_TERMS: &[&str] = &[
    "camp", "kitsch", "diva", "drag", "disco", "musical", "theatre", "theater",
    "parody", "bubblegum", "hyperpop", "eurovision", "abba", "girl group",
    "pop princess", "campy", "cabaret", "burlesque", "drag race", "slay", "pink",
];
const CLUB_TERMS: &[&str] = &[
    "club", "house", "techno", "dance", "rave", "edm", "bass", "garage",
    "drum and bass", "dnb", "jersey club", "ballroom", "vogue", "voguing",
    "tech house", "deep house", "disco house", "ghetto house", "acid", "trance",
    "electro", "party", "bootleg", "bpm", "mix",
];
const SOFT_TERMS: &[&str] = &[
    "soft", "ambient", "acoustic", "ballad", "piano", "dreamy", "dream pop",
    "r&b", "soul", "lo-fi", "lofi", "sad", "indie", "singer-songwriter", "folk",
    "gentle", "slow", "intimate", "bedroom pop", "shoegaze", "ethereal", "hushed",
];
const CHAOS_TERMS: &[&str] = &[
    "experimental", "noise", "glitch", "breakcore", "hyperpop", "industrial",
    "hardcore", "speedcore", "deconstructed", "avant", "pc music", "digicore",
    "hexd", "scene", "screamo", "metal", "punk", "distorted", "chaotic", "weird",
];

fn terms_regex(terms: &[&str]) -> Regex {
    let alternatives: Vec<String> = terms.iter().map(|term| regex::escape(term)).collect();
    Regex::new(&format!(r"(?i)\b(?:{})\b", alternatives.join("|"))).expect("valid regex")
}

fn category_score(regex: &Regex, text: &str) -> f32 {
    let hits = regex.find_iter(text).count() as f32;
    (1.0 - (-0.9 * hits).exp()).clamp(0.0, 1.0)
}

fn track_text(track: &Track) -> String {
    let mut text = track.seed_text().to_lowercase();
    if let Some(description) = &track.description {
        text.push(' ');
        text.push_str(&description.to_lowercase());
    }
    text
}

#[derive(Default, Clone, Copy)]
struct FamilyScores {
    camp: f32,
    club: f32,
    soft: f32,
    chaos: f32,
}

fn family_scores(track: &Track) -> FamilyScores {
    let text = track_text(track);
    FamilyScores {
        camp: category_score(&terms_regex(CAMP_TERMS), &text),
        club: category_score(&terms_regex(CLUB_TERMS), &text),
        soft: category_score(&terms_regex(SOFT_TERMS), &text),
        chaos: category_score(&terms_regex(CHAOS_TERMS), &text),
    }
}

fn mean_family(picks: &[Recommendation]) -> FamilyScores {
    let mut total = FamilyScores::default();
    for pick in picks {
        let scores = family_scores(&pick.track);
        total.camp += scores.camp;
        total.club += scores.club;
        total.soft += scores.soft;
        total.chaos += scores.chaos;
    }
    let n = picks.len().max(1) as f32;
    FamilyScores {
        camp: total.camp / n,
        club: total.club / n,
        soft: total.soft / n,
        chaos: total.chaos / n,
    }
}

fn distinct_artists(picks: &[Recommendation]) -> usize {
    picks
        .iter()
        .map(|pick| pick.track.artist.trim().to_lowercase())
        .collect::<HashSet<_>>()
        .len()
}

fn print_picks(prefix: &str, picks: &[Recommendation]) {
    for (index, pick) in picks.iter().enumerate() {
        println!(
            "{prefix} PICK n={} id={} artist={:?} title={:?} duration={} genre={:?} tags={:?} score={:.3} reason={:?}",
            index + 1,
            pick.track.id,
            pick.track.artist,
            short(&pick.track.title, 90),
            pick.track.duration_label(),
            pick.track.genre,
            pick.track.tags,
            pick.score,
            pick.reason
        );
    }
}

fn reason_class(reason: &str) -> &'static str {
    if reason.starts_with("because you keep replaying") {
        "artist-replay"
    } else if reason.starts_with("more like ") {
        "related"
    } else if reason.starts_with("more ") && reason.ends_with(" in your veins") {
        "tag-affinity"
    } else if reason == "certified ballroom" {
        "ballroom"
    } else if reason == "camp canon" {
        "camp"
    } else if reason == "soft sapphic detour" {
        "soft"
    } else if reason == "chaos dial, no notes" {
        "chaos"
    } else if reason == "fresh out the group chat" {
        "fresh"
    } else if reason == "the gaydar says explore" {
        "explore"
    } else if reason == "the gaydar says yes" {
        "queer"
    } else if reason == "right on the dial" {
        "dial"
    } else if reason.starts_with("deep cut from ") {
        "artist-deep-cut"
    } else if reason.starts_with("tagged ") {
        "tag-source"
    } else if reason.starts_with("dug out of the ") {
        "seed-crate"
    } else if reason.ends_with(" with a queer accent") {
        "genre"
    } else if reason == "queer radio staple" {
        "staple"
    } else {
        "other"
    }
}

fn reason_histogram(picks: &[Recommendation]) -> Vec<(String, usize)> {
    let mut counts: std::collections::BTreeMap<String, usize> = Default::default();
    for pick in picks {
        *counts.entry(reason_class(&pick.reason).to_string()).or_insert(0) += 1;
    }
    counts.into_iter().collect()
}

fn contains_any(track: &Track, needles: &[&str]) -> bool {
    let mut parts: Vec<String> = track.tags.clone();
    if let Some(genre) = &track.genre {
        parts.push(genre.to_lowercase());
    }
    parts.iter().any(|part| {
        needles
            .iter()
            .any(|needle| part.to_lowercase().contains(needle))
    })
}

fn tag_present(track: &Track, tag: &str) -> bool {
    let tag = tag.to_lowercase();
    track
        .tags
        .iter()
        .chain(track.genre.as_ref().into_iter())
        .any(|part| part.to_lowercase() == tag)
}

fn tag_overlap(left: &Track, right: &Track) -> usize {
    let left: HashSet<String> = left
        .tags
        .iter()
        .chain(left.genre.as_ref().into_iter())
        .map(|tag| tag.to_lowercase())
        .collect();
    right
        .tags
        .iter()
        .chain(right.genre.as_ref().into_iter())
        .filter(|tag| left.contains(&tag.to_lowercase()))
        .count()
}

fn overlap(base: &[Recommendation], other: &[Recommendation]) -> (usize, f32) {
    let base_ids: BTreeSet<u64> = base.iter().map(|pick| pick.track.id).collect();
    let other_ids: BTreeSet<u64> = other.iter().map(|pick| pick.track.id).collect();
    let common = base_ids.intersection(&other_ids).count();
    let union = base_ids.union(&other_ids).count().max(1);
    (common, common as f32 / union as f32)
}

fn check_reasons(picks: &[Recommendation], stage: &str) -> usize {
    let mut violations = 0usize;
    for pick in picks {
        let reason = &pick.reason;
        let mut bad = None;
        if reason == "certified ballroom" && !contains_any(&pick.track, &["ballroom", "vogue", "voguing"]) {
            bad = Some("certified ballroom without ballroom/vogue tag");
        } else if reason.starts_with("more ") && reason.ends_with(" in your veins") {
            let tag = reason
                .trim_start_matches("more ")
                .trim_end_matches(" in your veins");
            if !tag_present(&pick.track, tag) {
                bad = Some("tag-affinity reason cites a tag the track does not carry");
            }
        } else if stage == "bootstrap" && reason.starts_with("because you keep replaying") {
            bad = Some("artist-replay reason on a cold bootstrap");
        } else if stage == "bootstrap" && reason.starts_with("more like ") {
            bad = Some("related reason on a cold bootstrap");
        } else if reason.trim().is_empty() {
            bad = Some("empty reason");
        }
        if let Some(message) = bad {
            violations += 1;
            println!("REASON_INVARIANT stage={stage} id={} reason={reason:?} problem={message}", pick.track.id);
        }
    }
    violations
}

// ---------------------------------------------------------------------------
// 1. bootstrap(8)
// ---------------------------------------------------------------------------

#[test]
#[ignore = "live SoundCloud algorithm audit"]
fn audit_bootstrap_eight() {
    let client = Arc::new(live_client());
    let mut total_resolved = 0usize;
    let mut total_picks = 0usize;
    for run in 1..=3 {
        let directory = temp_dir(&format!("boot-{run}"));
        let mut engine = GaydarEngine::new(Arc::clone(&client), directory.join("state.json"));
        let started = Instant::now();
        let picks = engine.bootstrap(8).expect("bootstrap(8)");
        let elapsed = started.elapsed();
        println!(
            "BOOT run={run} ms={} count={} distinct_artists={} histogram={:?}",
            elapsed.as_millis(),
            picks.len(),
            distinct_artists(&picks),
            reason_histogram(&picks)
        );
        print_picks("BOOT", &picks);
        let violations = check_reasons(&picks, "bootstrap");
        println!("BOOT run={run} reason_violations={violations}");
        let (ok, failed) = resolve_tracks(&client, &picks.iter().map(|p| p.track.clone()).collect::<Vec<_>>(), 8, "BOOT");
        println!(
            "BOOT run={run} resolve_ok={ok} resolve_failed={failed} playable_rate={:.3}",
            ok as f32 / picks.len().max(1) as f32
        );
        total_resolved += ok;
        total_picks += picks.len();
        println!(
            "BOOT run={run} unplayable_in_output={}",
            picks.iter().filter(|p| !is_playable(&p.track)).count()
        );
    }
    println!(
        "BOOT aggregate picks={total_picks} resolved={total_resolved} rate={:.3}",
        total_resolved as f32 / total_picks.max(1) as f32
    );
}

// ---------------------------------------------------------------------------
// 2. refill(8) after like / complete / skip
// ---------------------------------------------------------------------------

#[test]
#[ignore = "live SoundCloud algorithm audit"]
fn audit_refill_feedback() {
    let client = Arc::new(live_client());
    let directory = temp_dir("refill");
    let state_path = directory.join("state.json");
    let mut engine = GaydarEngine::new(Arc::clone(&client), state_path.clone());
    let boot = engine.bootstrap(8).expect("bootstrap(8)");
    println!("FEEDBACK bootstrap count={} distinct={}", boot.len(), distinct_artists(&boot));
    print_picks("FEEDBACK-BOOT", &boot);

    let like_track = boot
        .iter()
        .max_by_key(|pick| pick.track.tags.len())
        .expect("bootstrap picks")
        .track
        .clone();
    let skip_track = boot
        .iter()
        .find(|pick| {
            !pick.track.artist.eq_ignore_ascii_case(&like_track.artist)
        })
        .map(|pick| pick.track.clone())
        .expect("second artist");

    println!(
        "FEEDBACK like={:?} tags={:?} genre={:?}",
        display(&like_track),
        like_track.tags,
        like_track.genre
    );
    println!("FEEDBACK skip={:?} tags={:?}", display(&skip_track), skip_track.tags);
    engine.record_play(&like_track);
    engine.record_complete(&like_track);
    engine.record_like(&like_track);
    engine.record_skip(&skip_track, 0.05);
    println!("FEEDBACK taste={:?}", engine.taste_summary());
    println!("FEEDBACK top_artists={:?}", engine.top_artists(5));
    println!("FEEDBACK plays={}", engine.total_plays());

    let started = Instant::now();
    let same_session = engine.refill(8).expect("same-session refill");
    println!(
        "FEEDBACK same_session ms={} count={} distinct={} histogram={:?}",
        started.elapsed().as_millis(),
        same_session.len(),
        distinct_artists(&same_session),
        reason_histogram(&same_session)
    );
    print_picks("FEEDBACK-SAME", &same_session);
    let boot_ids: BTreeSet<u64> = boot.iter().map(|pick| pick.track.id).collect();
    let repeats = same_session.iter().filter(|pick| boot_ids.contains(&pick.track.id)).count();
    let liked_artist_hits = same_session
        .iter()
        .filter(|pick| pick.track.artist.eq_ignore_ascii_case(&like_track.artist))
        .count();
    let tag_hits = same_session
        .iter()
        .filter(|pick| tag_overlap(&like_track, &pick.track) > 0)
        .count();
    println!(
        "FEEDBACK same_session repeats_boot={repeats} liked_artist_hits={liked_artist_hits} tag_overlap_hits={tag_hits} violations={}",
        check_reasons(&same_session, "refill")
    );
    let (ok, failed) = resolve_tracks(&client, &same_session.iter().map(|p| p.track.clone()).collect::<Vec<_>>(), 8, "FEEDBACK-SAME");
    println!("FEEDBACK same_session resolve_ok={ok} resolve_failed={failed}");
    drop(engine);

    // Simulate an app restart: the pool and served set are gone, but taste,
    // history, likes, skipped ids, and dials survive. This forces refill_jobs
    // to run instead of reusing the in-memory pool.
    let mut reloaded = GaydarEngine::new(Arc::clone(&client), state_path.clone());
    let started = Instant::now();
    let restarted = reloaded.refill(8).expect("restart refill");
    println!(
        "FEEDBACK restart ms={} count={} distinct={} histogram={:?}",
        started.elapsed().as_millis(),
        restarted.len(),
        distinct_artists(&restarted),
        reason_histogram(&restarted)
    );
    print_picks("FEEDBACK-RESTART", &restarted);
    let history_ids: BTreeSet<u64> = reloaded.history().iter().map(|track| track.id).collect();
    let liked_ids: BTreeSet<u64> = reloaded.liked_tracks().iter().map(|track| track.id).collect();
    let repeats_boot = restarted.iter().filter(|pick| boot_ids.contains(&pick.track.id)).count();
    let repeats_excluded = restarted
        .iter()
        .filter(|pick| history_ids.contains(&pick.track.id) || liked_ids.contains(&pick.track.id))
        .count();
    let liked_artist_hits = restarted
        .iter()
        .filter(|pick| pick.track.artist.eq_ignore_ascii_case(&like_track.artist))
        .count();
    let tag_hits = restarted
        .iter()
        .filter(|pick| tag_overlap(&like_track, &pick.track) > 0)
        .count();
    println!(
        "FEEDBACK restart repeats_boot_unplayed={repeats_boot} repeats_played_or_liked={repeats_excluded} liked_artist_hits={liked_artist_hits} tag_overlap_hits={tag_hits} violations={}",
        check_reasons(&restarted, "refill")
    );
    let (ok, failed) = resolve_tracks(&client, &restarted.iter().map(|p| p.track.clone()).collect::<Vec<_>>(), 8, "FEEDBACK-RESTART");
    println!("FEEDBACK restart resolve_ok={ok} resolve_failed={failed}");
}

// ---------------------------------------------------------------------------
// 3. dial sensitivity
// ---------------------------------------------------------------------------

#[test]
#[ignore = "live SoundCloud algorithm audit"]
fn audit_dial_sensitivity() {
    let client = Arc::new(live_client());

    // Each extreme gets a fresh cold start. A shared pool is unfair here: the
    // pool holds only about two refills in this environment, so later dials
    // would be selecting from the leftovers of earlier dials.
    let cases: Vec<(&str, Dials)> = vec![
        ("default", Dials::default()),
        ("camp-max", Dials { camp: 1.0, club: 0.0, soft: 0.0, chaos: 0.0 }),
        ("club-max", Dials { camp: 0.0, club: 1.0, soft: 0.0, chaos: 0.0 }),
        ("soft-max", Dials { camp: 0.0, club: 0.0, soft: 1.0, chaos: 0.0 }),
        ("chaos-max", Dials { camp: 0.0, club: 0.0, soft: 0.0, chaos: 1.0 }),
        ("all-max", Dials { camp: 1.0, club: 1.0, soft: 1.0, chaos: 1.0 }),
        ("all-min", Dials { camp: 0.05, club: 0.05, soft: 0.05, chaos: 0.05 }),
    ];
    let mut base: Vec<Recommendation> = Vec::new();
    for (name, dials) in cases {
        let directory = temp_dir(&format!("dial-{name}"));
        let mut engine = GaydarEngine::new(Arc::clone(&client), directory.join("state.json"));
        engine.set_dials(dials);
        let started = Instant::now();
        let picks = match engine.bootstrap(8) {
            Ok(picks) => picks,
            Err(error) => {
                println!("DIAL name={name} ERROR ms={} error={error}", started.elapsed().as_millis());
                continue;
            }
        };
        if name == "default" {
            base = picks.clone();
        }
        let means = mean_family(&picks);
        let (common, jaccard) = if base.is_empty() || name == "default" {
            (picks.len(), 1.0)
        } else {
            overlap(&base, &picks)
        };
        println!(
            "DIAL name={name} dials={:?} ms={} count={} distinct={} common_with_default={common} jaccard={jaccard:.3} mean_camp={:.3} mean_club={:.3} mean_soft={:.3} mean_chaos={:.3} histogram={:?}",
            dials,
            started.elapsed().as_millis(),
            picks.len(),
            distinct_artists(&picks),
            means.camp,
            means.club,
            means.soft,
            means.chaos,
            reason_histogram(&picks)
        );
        print_picks(&format!("DIAL-{name}"), &picks);
        let (ok, failed) = resolve_tracks(&client, &picks.iter().map(|p| p.track.clone()).collect::<Vec<_>>(), 8, &format!("DIAL-{name}"));
        println!("DIAL name={name} resolve_ok={ok} resolve_failed={failed}");
    }
}

// ---------------------------------------------------------------------------
// 3b. pool exhaustion and the empty-pool loop
// ---------------------------------------------------------------------------

#[test]
#[ignore = "live SoundCloud algorithm audit"]
fn audit_pool_exhaustion() {
    let client = Arc::new(live_client());
    let directory = temp_dir("exhaust");
    let mut engine = GaydarEngine::new(Arc::clone(&client), directory.join("state.json"));
    let boot = match engine.bootstrap(8) {
        Ok(picks) => picks,
        Err(error) => {
            println!("EXHAUST bootstrap_error={error}");
            return;
        }
    };
    println!("EXHAUST bootstrap count={} distinct={}", boot.len(), distinct_artists(&boot));
    let mut served: BTreeSet<u64> = boot.iter().map(|p| p.track.id).collect();
    for round in 1..=6 {
        let started = Instant::now();
        match engine.refill(8) {
            Ok(picks) => {
                let new = picks.iter().filter(|p| !served.contains(&p.track.id)).count();
                println!(
                    "EXHAUST refill round={round} ms={} count={} new={} distinct={} total_served_before={}",
                    started.elapsed().as_millis(),
                    picks.len(),
                    new,
                    distinct_artists(&picks),
                    served.len()
                );
                if picks.is_empty() {
                    println!("EXHAUST round={round} EMPTY_WITHOUT_ERROR");
                    break;
                }
                for pick in &picks {
                    served.insert(pick.track.id);
                }
            }
            Err(error) => {
                println!(
                    "EXHAUST refill round={round} ms={} ERROR={error}",
                    started.elapsed().as_millis()
                );
                // A second call proves the failure is not a one-shot fluke and
                // does not spin forever.
                let again = engine.refill(8);
                println!(
                    "EXHAUST refill round={round} retry_after_error={:?}",
                    again.as_ref().map(|picks| picks.len()).map_err(|error| error.to_string())
                );
                break;
            }
        }
    }
    println!("EXHAUST total_unique_served={}", served.len());
}

// ---------------------------------------------------------------------------
// 4. station queries and seeds.json validation
// ---------------------------------------------------------------------------

#[test]
#[ignore = "live SoundCloud algorithm audit"]
fn audit_station_queries() {
    let client = live_client();
    let client_id = client.client_id();
    let stations = [
        ("camp", "camp classics drag diva"),
        ("ballroom", "ballroom vogue beat"),
        ("soft", "sapphic soft queer"),
        ("hyper", "hyperpop queer chaos"),
    ];
    for (name, query) in stations {
        audit_query(&client, &client_id, "STATION", name, query, 24, true);
    }

    // Validated alternatives from assets/seeds.json plus adjacent scene terms.
    let candidates = [
        ("camp", "camp pop"),
        ("camp", "camp classics"),
        ("camp", "gay anthem"),
        ("ballroom", "ballroom vogue"),
        ("ballroom", "ha dance"),
        ("ballroom", "runway beat"),
        ("soft", "trans ambient"),
        ("soft", "sapphic pop"),
        ("soft", "sad queer"),
        ("hyper", "queer breakcore"),
        ("hyper", "hyperpop edit"),
        ("hyper", "queercore"),
    ];
    for (name, query) in candidates {
        audit_query(&client, &client_id, "CANDIDATE", name, query, 50, false);
    }
}

fn audit_query(
    client: &SoundCloudClient,
    client_id: &str,
    kind: &str,
    name: &str,
    query: &str,
    filter_limit: usize,
    resolve_all: bool,
) {
    let raw = raw_search(client_id, query, RAW_LIMIT);
    let stats = classify_raw(&raw);
    let started = Instant::now();
    let tracks = match client.search_tracks(query, filter_limit) {
        Ok(tracks) => tracks,
        Err(error) => {
            println!(
                "{kind} name={name} query={query:?} SEARCH_ERROR ms={} error={}",
                started.elapsed().as_millis(),
                error
            );
            return;
        }
    };
    let filter_ms = started.elapsed().as_millis();
    println!(
        "{kind} name={name} query={query:?} raw_total={} with_media={} mp3_prog={} mp3_hls={} drm={} unavailable={} raw_eligible={} filtered={} filter_ms={filter_ms}",
        stats.total,
        stats.with_media,
        stats.mp3_progressive,
        stats.mp3_hls,
        stats.drm_only,
        stats.unavailable,
        stats.eligible,
        tracks.len()
    );
    for track in tracks.iter().take(6) {
        println!(
            "{kind} name={name} SAMPLE id={} {:?} genre={:?} tags={:?}",
            track.id,
            display(track),
            track.genre,
            track.tags
        );
    }
    let cap = if resolve_all { tracks.len() } else { 12 };
    let (ok, failed) = resolve_tracks(client, &tracks, cap, &format!("{kind}-{name}"));
    println!(
        "{kind} name={name} resolve_checked={} resolve_ok={ok} resolve_failed={failed} verified_playable={ok}",
        tracks.len().min(cap)
    );
}

// ---------------------------------------------------------------------------
// 5. cleanup and state sanitising
// ---------------------------------------------------------------------------

#[test]
#[ignore = "live SoundCloud algorithm audit"]
fn audit_cleanup() {
    let client = Arc::new(SoundCloudClient::new().expect("client"));
    let directory = temp_dir("cleanup");
    let mut engine = GaydarEngine::new(Arc::clone(&client), directory.join("state.json"));

    let mut unplayable = Track::default();
    unplayable.id = 999_000_001;
    unplayable.title = "Unplayable test tone".to_string();
    unplayable.artist = "Test Artist".to_string();
    unplayable.duration_ms = 1000;
    unplayable.streamable = false;
    unplayable.policy = "BLOCK".to_string();
    unplayable.tags = vec!["queer".to_string(), "ballroom".to_string()];
    println!(
        "CLEANUP unplayable is_playable={} id={}",
        is_playable(&unplayable),
        unplayable.id
    );
    engine.record_like(&unplayable);
    engine.record_skip(&unplayable, 0.0);
    println!(
        "CLEANUP after_like_skip liked={} is_liked={} liked_tracks={} history={}",
        engine.liked_tracks().len(),
        engine.is_liked(unplayable.id),
        engine.liked_tracks().iter().map(|t| t.title.clone()).collect::<Vec<_>>().join("; "),
        engine.history().len()
    );

    let mut zero = Track::default();
    zero.title = "zero".to_string();
    let before = engine.total_plays();
    engine.record_play(&zero);
    engine.record_complete(&zero);
    engine.record_skip(&zero, 0.0);
    engine.record_like(&zero);
    println!(
        "CLEANUP zero_id plays_delta={} liked={} history={}",
        engine.total_plays() - before,
        engine.liked_tracks().len(),
        engine.history().len()
    );

    // A dirty state file must be sanitised on load, not trusted.
    let dirty_path = directory.join("dirty.json");
    let track = |id: u64, title: &str, artist: &str| serde_json::json!({
        "id": id,
        "title": title,
        "artist": artist,
        "artist_id": 7,
        "artist_avatar_url": null,
        "artwork_url": null,
        "duration_ms": 120000,
        "genre": "Pop",
        "tags": ["queer"],
        "description": null,
        "playback_count": 10,
        "likes_count": 2,
        "user_followers": 3,
        "created_at": null,
        "permalink_url": "https://soundcloud.com/test",
        "streamable": true,
        "policy": "ALLOW"
    });
    let dirty = serde_json::json!({
        "version": 99,
        "liked": [
            track(1, "keep", "A"),
            track(1, "duplicate", "A"),
            track(0, "zero", "A")
        ],
        "history": [
            {"track": track(2, "played", "B"), "fraction": 4.0, "outcome": "completed"},
            {"track": track(0, "bad", "B"), "fraction": -1.0}
        ],
        "tag_weights": {"queer": 99.0, "": 1.0, "ballroom": -99.0},
        "artist_weights": {"A": 99.0, "": 1.0},
        "skipped": [0, 2, 2],
        "dials": {"camp": 4.0, "club": -3.0, "soft": 2.0, "chaos": 0.5},
        "plays": 7
    });
    fs::write(&dirty_path, serde_json::to_vec(&dirty).expect("dirty json")).expect("write dirty");
    let dirty_engine = GaydarEngine::new(Arc::clone(&client), dirty_path.clone());
    let dials = dirty_engine.dials();
    println!(
        "CLEANUP dirty dials={dials:?} liked={} history={} plays={} taste={:?} top_artists={:?}",
        dirty_engine.liked_tracks().len(),
        dirty_engine.history().len(),
        dirty_engine.total_plays(),
        dirty_engine.taste_summary(),
        dirty_engine.top_artists(3)
    );
    dirty_engine.save().expect("save sanitised");
    let saved: Value = serde_json::from_slice(&fs::read(&dirty_path).expect("read saved")).expect("saved json");
    println!(
        "CLEANUP saved dials={:?} tag_weights={:?} skipped={:?} liked={} history={}",
        saved["dials"],
        saved["tag_weights"],
        saved["skipped"],
        saved["liked"].as_array().map(Vec::len).unwrap_or(0),
        saved["history"].as_array().map(Vec::len).unwrap_or(0)
    );

    // Corrupt state must be moved aside, not crash.
    let corrupt_dir = temp_dir("corrupt");
    let corrupt_path = corrupt_dir.join("state.json");
    fs::write(&corrupt_path, b"{ not json").expect("write corrupt");
    let corrupt = GaydarEngine::new(Arc::clone(&client), corrupt_path.clone());
    println!(
        "CLEANUP corrupt plays={} liked={} files={:?}",
        corrupt.total_plays(),
        corrupt.liked_tracks().len(),
        fs::read_dir(&corrupt_dir)
            .expect("read dir")
            .filter_map(Result::ok)
            .map(|entry| entry.file_name().to_string_lossy().into_owned())
            .collect::<Vec<_>>()
    );
}

// ---------------------------------------------------------------------------
// 6. offline / empty-pool behaviour (run with a dead proxy)
// ---------------------------------------------------------------------------

#[test]
#[ignore = "live SoundCloud algorithm audit"]
fn audit_offline_errors() {
    if std::env::var("GAYDAR_AUDIT_OFFLINE").as_deref() != Ok("1") {
        println!("OFFLINE skipped (set GAYDAR_AUDIT_OFFLINE=1 and a dead HTTPS_PROXY)");
        return;
    }
    let client = Arc::new(
        SoundCloudClient::with_client_id("39AVoiZVrdvJEMFLH8T0c5WGshQaAPRJ").expect("client"),
    );
    let directory = temp_dir("offline");
    let mut engine = GaydarEngine::new(Arc::clone(&client), directory.join("state.json"));
    let started = Instant::now();
    let result = engine.bootstrap(3);
    match result {
        Ok(picks) => println!(
            "OFFLINE bootstrap_unexpectedly_ok ms={} count={}",
            started.elapsed().as_millis(),
            picks.len()
        ),
        Err(error) => println!(
            "OFFLINE bootstrap_error ms={} error={}",
            started.elapsed().as_millis(),
            error
        ),
    }

    let mut refill_engine = GaydarEngine::new(Arc::clone(&client), directory.join("refill.json"));
    let started = Instant::now();
    let result = refill_engine.refill(8);
    match result {
        Ok(picks) => println!(
            "OFFLINE refill_unexpectedly_ok ms={} count={}",
            started.elapsed().as_millis(),
            picks.len()
        ),
        Err(error) => println!(
            "OFFLINE refill_error ms={} error={}",
            started.elapsed().as_millis(),
            error
        ),
    }
}

// ---------------------------------------------------------------------------
// 7. discovery latency
// ---------------------------------------------------------------------------

#[test]
#[ignore = "live SoundCloud algorithm audit"]
fn audit_client_discovery() {
    let started = Instant::now();
    match SoundCloudClient::discover_client_id() {
        Ok(client_id) => {
            println!(
                "DISCOVERY ok ms={} len={} prefix={}",
                started.elapsed().as_millis(),
                client_id.len(),
                &client_id[..client_id.len().min(6)]
            );
            let client = SoundCloudClient::with_client_id(client_id).expect("client");
            let search_started = Instant::now();
            match client.search_tracks("queer anthem", 5) {
                Ok(tracks) => println!(
                    "DISCOVERY search ok ms={} tracks={}",
                    search_started.elapsed().as_millis(),
                    tracks.len()
                ),
                Err(error) => println!("DISCOVERY search error={error}"),
            }
        }
        Err(error) => println!("DISCOVERY error ms={} error={}", started.elapsed().as_millis(), error),
    }
}

// ---------------------------------------------------------------------------
// 4b. extra candidates for the station replacement table
// ---------------------------------------------------------------------------

#[test]
#[ignore = "live SoundCloud algorithm audit"]
fn audit_extra_candidates() {
    let client = live_client();
    let client_id = client.client_id();
    for (name, query) in [
        ("camp", "drag queen dj set"),
        ("camp", "Jinkx Monsoon"),
        ("ballroom", "ballroom beats"),
        ("soft", "Beverly Glenn-Copeland"),
        ("soft", "Perfume Genius"),
    ] {
        audit_query(&client, &client_id, "EXTRA", name, query, 50, false);
    }
}
