//! Live end-to-end checks for GAYDAR FM.
//!
//! These tests drive the real application state with the real SoundCloud API,
//! a real ffmpeg process and the real ALSA backend (the \`null\` device). They
//! need network access and the system \`ffmpeg\`; they are ignored by default.
//!
//! Run them serially (they share the process environment):
//!
//! \`\`\`text
//! GAYDAR_ALSA_DEVICE=null cargo test --test live_app_e2e -- --ignored --test-threads=1 --nocapture
//! \`\`\`
//!
//! Each test creates its own temporary GAYDAR_DATA_DIR/GAYDAR_CACHE_DIR under
//! \`/tmp\`, so a run never touches the developer's real GAYDAR state. Artwork,
//! screenshots and state are kept for inspection after the run.
//!
//! The tests use BUI's headless \`Application\` + \`simulate_*\` automation.
//! Where a control has no \`.id()\` (the next button, track rows, device rows),
//! the test either clicks it by geometry relative to an identifiable sibling
//! or calls the exact public method its click listener calls. Those cases are
//! marked \`LIMITATION\` at the call site and in docs/QA-REPORT.md.

use blenderui::prelude::*;
use gaydar::app::{GaydarApp, Screen, STATIONS};
use gaydar::audio::{AlsaDevice, AudioEngine, PlaybackState};
use gaydar::theme;
use serde_json::Value;
use std::collections::BTreeSet;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Mutex;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

const WIDTH: f32 = 1440.0;
const HEIGHT: f32 = 900.0;

/// BUI/audio/env setup is process-global; run one live session at a time even
/// when cargo's default parallel test runner is used.
static LIVE_LOCK: Mutex<()> = Mutex::new(());
static DIR_SEQ: AtomicU64 = AtomicU64::new(0);

fn scratch_root() -> PathBuf {
    std::env::temp_dir().join("gaydar-live-e2e")
}

fn temp_dir(tag: &str) -> PathBuf {
    let seq = DIR_SEQ.fetch_add(1, Ordering::Relaxed);
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_nanos())
        .unwrap_or(0);
    let dir = scratch_root().join(format!("{tag}-{}-{seq}-{nanos}", std::process::id()));
    fs::create_dir_all(dir.join("data")).expect("create data dir");
    fs::create_dir_all(dir.join("cache")).expect("create cache dir");
    fs::create_dir_all(dir.join("shots")).expect("create shots dir");
    dir
}

/// A single SoundCloud client-id cache shared by the tests in this process.
/// Discovery is the slowest part of startup; the cache still lives in /tmp.
fn shared_cache() -> PathBuf {
    let dir = scratch_root().join("shared-cache");
    fs::create_dir_all(&dir).expect("create shared cache");
    dir
}

fn configure_env(data: &Path, cache: &Path) {
    // SAFETY: LIVE_LOCK serialises every test that touches these variables.
    unsafe {
        std::env::set_var("GAYDAR_DATA_DIR", data);
        std::env::set_var("GAYDAR_CACHE_DIR", cache);
        std::env::set_var("GAYDAR_ALSA_DEVICE", "null");
    }
}

fn open_live(cx: &mut App) -> WindowHandle<GaydarApp> {
    theme::load_fonts();
    let win = cx
        .open_window(
            WindowOptions {
                window_bounds: Some(WindowBounds::Windowed(Bounds {
                    origin: point(px(0.0), px(0.0)),
                    size: size(px(WIDTH), px(HEIGHT)),
                })),
                window_min_size: Some(size(px(1120.0), px(720.0))),
                titlebar: Some(TitlebarOptions {
                    title: Some("GAYDAR FM — live QA".into()),
                }),
                ..Default::default()
            },
            |_, cx| cx.new(|_| GaydarApp::live()),
        )
        .expect("open live GAYDAR window");
    win.update(cx, |app, _window, cx| app.start_live(cx))
        .expect("start_live must run on the window entity");
    cx.draw(win.id);
    win
}

fn snapshot(app: &GaydarApp) -> String {
    format!(
        "screen={:?} booting={} loading={} state={:?} pos={}ms dur={}ms track={:?} source={} queue={} dials={:?} radio='{}' error={:?} search='{}' results={} search_error={:?}",
        app.screen,
        app.booting,
        app.loading_track,
        app.playback.state,
        app.playback.position_ms,
        app.playback.duration_ms,
        app.current.as_ref().map(|t| format!("{} - {}", t.artist, t.title)),
        app.current_source.is_some(),
        app.queue.len(),
        app.dials,
        app.radio_status,
        app.error,
        app.search_query,
        app.search_results.len(),
        app.search_error,
    )
}

/// Draw (which pumps radio+audio events), then test the predicate. BUI's
/// headless run closure executes once; drawing is what drives the app.
fn wait_for(
    cx: &mut App,
    win: &WindowHandle<GaydarApp>,
    timeout: Duration,
    label: &str,
    mut predicate: impl FnMut(&GaydarApp) -> bool,
) -> bool {
    let deadline = Instant::now() + timeout;
    loop {
        cx.run_pending();
        cx.draw(win.id);
        let passed = {
            let app = win.root().read(cx);
            predicate(app)
        };
        if passed {
            return true;
        }
        if Instant::now() >= deadline {
            let app = win.root().read(cx);
            eprintln!("TIMEOUT waiting for {label}: {}", snapshot(app));
            return false;
        }
        std::thread::sleep(Duration::from_millis(80));
    }
}

fn assert_wait(
    cx: &mut App,
    win: &WindowHandle<GaydarApp>,
    timeout: Duration,
    label: &str,
    predicate: impl FnMut(&GaydarApp) -> bool,
) {
    assert!(
        wait_for(cx, win, timeout, label, predicate),
        "timed out waiting for {label}; see TIMEOUT line above"
    );
}

fn click_id(cx: &mut App, win: &WindowHandle<GaydarApp>, id: &str) {
    let bounds = cx
        .element_bounds(win.id, id)
        .unwrap_or_else(|| panic!("missing element {id}"));
    cx.simulate_click(
        win.id,
        bounds.origin.x + bounds.size.width / 2.0,
        bounds.origin.y + bounds.size.height / 2.0,
    );
    cx.draw(win.id);
}

fn type_text(cx: &mut App, win: &WindowHandle<GaydarApp>, text: &str) {
    for ch in text.chars() {
        let text = ch.to_string();
        let key = if ch == ' ' {
            "space".to_string()
        } else if ch == '\'' {
            "apostrophe".to_string()
        } else {
            text.clone()
        };
        cx.simulate_key(win.id, &text, &key);
        cx.draw(win.id);
    }
}

fn save_png(path: &Path, width: usize, height: usize, pixels: &[u32]) {
    let file = fs::File::create(path).expect("create screenshot");
    let mut encoder = png::Encoder::new(std::io::BufWriter::new(file), width as u32, height as u32);
    encoder.set_color(png::ColorType::Rgba);
    encoder.set_depth(png::BitDepth::Eight);
    let rgba: Vec<u8> = pixels
        .iter()
        .flat_map(|p| [(p >> 16) as u8, (p >> 8) as u8, *p as u8, 255])
        .collect();
    encoder
        .write_header()
        .expect("png header")
        .write_image_data(&rgba)
        .expect("png data");
}

fn screenshot(cx: &mut App, win: &WindowHandle<GaydarApp>, path: &Path) {
    let (w, h, pixels) = cx.draw(win.id);
    save_png(path, w, h, &pixels);
    eprintln!("screenshot: {} ({w}x{h})", path.display());
}

// -- process inspection ------------------------------------------------------

fn child_pids() -> BTreeSet<u32> {
    let mut pids = BTreeSet::new();
    let task_dir = PathBuf::from("/proc/self/task");
    let Ok(tasks) = fs::read_dir(&task_dir) else {
        return pids;
    };
    for task in tasks.flatten() {
        let path = task.path().join("children");
        let Ok(children) = fs::read_to_string(&path) else {
            continue;
        };
        for pid in children.split_whitespace().filter_map(|p| p.parse().ok()) {
            pids.insert(pid);
        }
    }
    pids
}

fn cmdline(pid: u32) -> String {
    fs::read(format!("/proc/{pid}/cmdline"))
        .ok()
        .map(|bytes| {
            String::from_utf8_lossy(&bytes)
                .split('\0')
                .filter(|s| !s.is_empty())
                .collect::<Vec<_>>()
                .join(" ")
        })
        .unwrap_or_default()
}

fn ffmpeg_children() -> Vec<(u32, String)> {
    child_pids()
        .into_iter()
        .filter_map(|pid| {
            let cmd = cmdline(pid);
            if cmd.starts_with("ffmpeg ") {
                Some((pid, cmd))
            } else {
                None
            }
        })
        .collect()
}

fn wait_for_ffmpeg(timeout: Duration) -> Vec<(u32, String)> {
    let deadline = Instant::now() + timeout;
    loop {
        let children = ffmpeg_children();
        if !children.is_empty() || Instant::now() >= deadline {
            return children;
        }
        std::thread::sleep(Duration::from_millis(50));
    }
}

fn wait_for_no_ffmpeg(timeout: Duration) -> Vec<(u32, String)> {
    let deadline = Instant::now() + timeout;
    loop {
        let children = ffmpeg_children();
        if children.is_empty() || Instant::now() >= deadline {
            return children;
        }
        std::thread::sleep(Duration::from_millis(50));
    }
}

fn read_json(path: &Path) -> Value {
    let bytes = fs::read(path).unwrap_or_else(|e| panic!("read {}: {e}", path.display()));
    serde_json::from_slice(&bytes).unwrap_or_else(|e| panic!("parse {}: {e}", path.display()))
}

fn wait_for_file(path: &Path, timeout: Duration) -> bool {
    let deadline = Instant::now() + timeout;
    while !path.is_file() {
        if Instant::now() >= deadline {
            return false;
        }
        std::thread::sleep(Duration::from_millis(50));
    }
    true
}

fn wait_for_liked(path: &Path, track_id: u64, timeout: Duration) -> bool {
    let deadline = Instant::now() + timeout;
    loop {
        if path.is_file()
            && read_json(path)["liked"]
                .as_array()
                .is_some_and(|liked| liked.iter().any(|t| t["id"].as_u64() == Some(track_id)))
        {
            return true;
        }
        if Instant::now() >= deadline {
            return false;
        }
        std::thread::sleep(Duration::from_millis(50));
    }
}

// -- tests -------------------------------------------------------------------

/// Startup, real track + artwork, Playing + advancing position + ffmpeg child,
/// pause/resume, next by clicking, waveform seek, and clean shutdown with no
/// orphan ffmpeg.
#[test]
#[ignore = "live network + audio + ffmpeg"]
fn live_startup_playback_controls_and_shutdown() {
    let _guard = LIVE_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let dir = temp_dir("startup");
    let data = dir.join("data");
    configure_env(&data, &shared_cache());

    let mut first_track = None;
    let mut first_ffmpeg = None;

    Application::headless().run(|cx| {
        let win = open_live(cx);

        // 1. Boot state is visible, then the real home screen arrives.
        {
            let app = win.root().read(cx);
            assert!(app.booting, "reviewed app should start on the boot screen");
            assert!(app.current.is_none(), "boot state must not show a fake track");
            assert!(app.audio.is_some(), "AudioEngine must be constructed at startup");
            assert_eq!(app.device, "null", "GAYDAR_ALSA_DEVICE=null must be honored");
        }
        screenshot(cx, &win, &dir.join("shots/01-boot.png"));

        assert_wait(
            cx,
            &win,
            Duration::from_secs(75),
            "live bootstrap to finish with a real track",
            |app| !app.booting && app.current.is_some(),
        );
        {
            let app = win.root().read(cx);
            assert!(!app.booting, "still booting: {}", snapshot(app));
            assert!(app.error.is_none(), "bootstrap error: {:?}", app.error);
            let track = app.current.clone().expect("bootstrap returned no track");
            assert!(!track.title.trim().is_empty(), "placeholder track title");
            assert!(!track.artist.trim().is_empty(), "placeholder track artist");
            assert!(track.duration_ms > 10_000, "implausible duration {}", track.duration_ms);
            assert!(track.id > 0, "track id must be a real SoundCloud id");
            assert!(
                track.artwork_url.is_some(),
                "first live track has no artwork url: {} - {}",
                track.artist,
                track.title
            );
            first_track = Some(track);
        }

        assert_wait(
            cx,
            &win,
            Duration::from_secs(30),
            "first stream to become Playing",
            |app| app.playback.state == PlaybackState::Playing && app.playback.position_ms > 0,
        );
        screenshot(cx, &win, &dir.join("shots/02-home-playing.png"));

        // 2. The decoder is a real ffmpeg child, not an in-process fake.
        let children = wait_for_ffmpeg(Duration::from_secs(10));
        assert!(
            !children.is_empty(),
            "Playing state but no ffmpeg child process; children={:?}",
            child_pids()
        );
        assert_eq!(children.len(), 1, "expected exactly one ffmpeg child, got {children:?}");
        let (ffmpeg_pid, ffmpeg_cmd) = children[0].clone();
        assert!(
            ffmpeg_cmd.contains("-f s16le") && ffmpeg_cmd.contains("-ar 48000"),
            "ffmpeg command does not write raw stereo PCM: {ffmpeg_cmd}"
        );
        assert!(
            ffmpeg_cmd.contains("sndcdn.com") || ffmpeg_cmd.contains("soundcloud"),
            "ffmpeg is not decoding a SoundCloud stream: {ffmpeg_cmd}"
        );
        first_ffmpeg = Some(ffmpeg_pid);

        // Position advances.
        let before = {
            let app = win.root().read(cx);
            app.playback.position_ms
        };
        assert_wait(
            cx,
            &win,
            Duration::from_secs(8),
            "position to advance at least 500 ms",
            |app| app.playback.position_ms >= before + 500,
        );
        screenshot(cx, &win, &dir.join("shots/03-position-advanced.png"));

        // 3. Real artwork is cached and valid.
        let track_id = {
            let app = win.root().read(cx);
            app.current_id().expect("current track")
        };
        assert_wait(
            cx,
            &win,
            Duration::from_secs(25),
            "artwork download for the current track",
            |app| app.art_cache.contains_key(&track_id),
        );
        let art_path = {
            let app = win.root().read(cx);
            app.art_cache.get(&track_id).cloned().expect("art cache entry")
        };
        let art = fs::read(&art_path).unwrap_or_else(|e| panic!("read artwork {art_path}: {e}"));
        assert!(art.len() > 1_000, "artwork is suspiciously small ({} bytes)", art.len());
        assert!(
            art.starts_with(&[0xFF, 0xD8, 0xFF])
                || art.starts_with(&[0x89, b'P', b'N', b'G'])
                || art.starts_with(b"GIF8")
                || (art.len() > 12 && &art[0..4] == b"RIFF" && &art[8..12] == b"WEBP"),
            "artwork has no image magic: {art_path}"
        );

        // 4. Play/pause through the actual button; pause freezes the clock.
        click_id(cx, &win, "play-toggle");
        assert_wait(
            cx,
            &win,
            Duration::from_secs(4),
            "Paused after clicking play-toggle",
            |app| app.playback.state == PlaybackState::Paused,
        );
        let paused_at = {
            let app = win.root().read(cx);
            app.playback.position_ms
        };
        std::thread::sleep(Duration::from_millis(1_200));
        cx.draw(win.id);
        let still_at = {
            let app = win.root().read(cx);
            app.playback.position_ms
        };
        assert!(
            still_at.abs_diff(paused_at) <= 120,
            "paused position moved {paused_at} -> {still_at}"
        );
        click_id(cx, &win, "play-toggle");
        assert_wait(
            cx,
            &win,
            Duration::from_secs(6),
            "Playing after clicking play-toggle again",
            |app| app.playback.state == PlaybackState::Playing,
        );

        // 5. Next: the control has no .id(); click its real geometry
        //    (transport gap 12px, next control 24px wide, play button 38px).
        //    LIMITATION: geometry is derived from src/theme.rs:312-317.
        let play_bounds = cx.element_bounds(win.id, "play-toggle").expect("play-toggle bounds");
        let next_x = play_bounds.origin.x + play_bounds.size.width + 12.0 + 12.0;
        let next_y = play_bounds.origin.y + play_bounds.size.height / 2.0;
        let old_id = {
            let app = win.root().read(cx);
            app.current_id().expect("current before next")
        };
        cx.simulate_click(win.id, next_x, next_y);
        cx.draw(win.id);
        assert_wait(
            cx,
            &win,
            Duration::from_secs(45),
            "next track to resolve and play",
            |app| {
                app.current_id().is_some_and(|id| id != old_id)
                    && app.playback.state == PlaybackState::Playing
                    && app.current_source.is_some()
            },
        );
        screenshot(cx, &win, &dir.join("shots/04-after-next.png"));
        let new_id = {
            let app = win.root().read(cx);
            app.current_id().expect("current after next")
        };
        assert_ne!(new_id, old_id, "next did not change the track");
        assert_wait(
            cx,
            &win,
            Duration::from_secs(10),
            "the skipped track and the new play to be persisted",
            |app| {
                app.history.iter().any(|t| t.id == old_id)
                    && app.history.iter().any(|t| t.id == new_id)
            },
        );

        // 6. Seek by clicking a real waveform segment (id seek-bar-<n>).
        click_id(cx, &win, "seek-bar-40");
        let expected_fraction = 40.5 / 48.0;
        assert_wait(
            cx,
            &win,
            Duration::from_secs(8),
            "seek position to land near the clicked segment",
            |app| {
                let duration = app.current_duration().max(1) as f32;
                let fraction = app.playback.position_ms as f32 / duration;
                (fraction - expected_fraction).abs() < 0.12
                    && app.playback.state == PlaybackState::Playing
            },
        );
        screenshot(cx, &win, &dir.join("shots/05-after-seek.png"));
        let seek_ffmpeg = wait_for_ffmpeg(Duration::from_secs(8));
        assert!(
            !seek_ffmpeg.is_empty(),
            "seek left no ffmpeg child; children={:?}",
            child_pids()
        );
        assert!(
            seek_ffmpeg.iter().all(|(_, cmd)| cmd.contains("-ss ")),
            "seek did not restart ffmpeg with -ss: {seek_ffmpeg:?}"
        );
        eprintln!(
            "first ffmpeg pid {first_ffmpeg:?}; after seek: {:?}",
            seek_ffmpeg.iter().map(|(pid, _)| *pid).collect::<Vec<_>>()
        );

        // Drop closes the window and the app; BUI shutdown runs AudioEngine::drop.
    });

    let leftovers = wait_for_no_ffmpeg(Duration::from_secs(5));
    assert!(
        leftovers.is_empty(),
        "orphan ffmpeg after clean shutdown: {leftovers:?}"
    );
    eprintln!(
        "clean shutdown: first track {:?}, state dir {}",
        first_track.as_ref().map(|t| format!("{} - {}", t.artist, t.title)),
        data.display()
    );
}

/// Search, play a real result, like it (persisted canon), load a station queue,
/// use the settings screen with real ALSA hints + persisted volume, and prove
/// that a fresh app process reconstructs taste/volume/state from disk.
#[test]
#[ignore = "live network + audio + ffmpeg"]
fn live_search_like_station_settings_and_restart() {
    let _guard = LIVE_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let dir = temp_dir("social");
    let data = dir.join("data");
    let cache = dir.join("cache");
    configure_env(&data, &cache);
    let state_path = data.join("state.json");
    let settings_path = data.join("settings.json");

    let mut liked_id = 0u64;
    let mut liked_title = String::new();
    let mut station_track = 0u64;
    let mut expected_plays_after = 0u64;
    let mut expected_volume = 0f32;

    Application::headless().run(|cx| {
        let win = open_live(cx);
        assert_wait(
            cx,
            &win,
            Duration::from_secs(75),
            "live bootstrap (social run)",
            |app| !app.booting && app.current.is_some(),
        );

        // 4. Search a validated query and wait for real filtered results.
        click_id(cx, &win, "nav-search");
        assert!(
            win.root().read(cx).screen == Screen::Search,
            "nav-search did not navigate"
        );
        click_id(cx, &win, "search-field");
        {
            let focused = cx
                .window(win.id)
                .and_then(|w| w.focused_id().map(str::to_string));
            assert_eq!(
                focused.as_deref(),
                Some("search-field"),
                "search field did not take focus"
            );
        }
        type_text(cx, &win, "ballroom vogue");
        {
            let app = win.root().read(cx);
            assert_eq!(
                app.search_query, "ballroom vogue",
                "search field did not receive the typed query (got {:?})",
                app.search_query
            );
        }
        cx.simulate_key(win.id, "\r", "Return");
        cx.draw(win.id);
        assert_wait(
            cx,
            &win,
            Duration::from_secs(45),
            "search results for 'ballroom vogue'",
            |app| !app.searching && (app.search_error.is_some() || !app.search_results.is_empty()),
        );
        {
            let app = win.root().read(cx);
            assert!(app.search_error.is_none(), "live search error: {:?}", app.search_error);
            assert!(
                !app.search_results.is_empty(),
                "live search for 'ballroom vogue' returned no mp3-capable tracks"
            );
            assert!(
                app.search_results.iter().all(|t| !t.title.trim().is_empty() && !t.artist.trim().is_empty()),
                "search returned a track without title/artist"
            );
            assert!(
                app.search_results.iter().any(|t| t.artwork_url.is_some()),
                "no search result had artwork"
            );
            eprintln!(
                "search 'ballroom vogue' -> {} results; first: {} - {}",
                app.search_results.len(),
                app.search_results[0].artist,
                app.search_results[0].title
            );
        }
        screenshot(cx, &win, &dir.join("shots/01-search-results.png"));

        // Play the first result. LIMITATION: .track-row has no element id, so
        // its listener cannot be addressed through element_bounds. The click
        // listener at src/ui.rs:418 calls exactly app.play_track(track, reason),
        // which is what this invokes.
        let target = win
            .update(cx, |app, _, cx| {
                let track = app.search_results[0].clone();
                app.play_track(track.clone(), "live search result");
                cx.notify();
                track.id
            })
            .expect("update app for search result play");
        assert_wait(
            cx,
            &win,
            Duration::from_secs(45),
            "the first search result to stream and play",
            |app| {
                app.current_id() == Some(target)
                    && app.playback.state == PlaybackState::Playing
                    && app.current_source.is_some()
            },
        );
        screenshot(cx, &win, &dir.join("shots/02-search-result-playing.png"));

        // 5. Like via the real keyboard shortcut (root handler, src/ui.rs:775).
        click_id(cx, &win, "nav-liked");
        {
            let app = win.root().read(cx);
            assert_eq!(app.screen, Screen::Liked);
            liked_id = app.current_id().expect("current before like");
            liked_title = app.current.as_ref().map(|t| t.title.clone()).unwrap_or_default();
        }
        cx.simulate_key(win.id, "l", "l");
        cx.draw(win.id);
        assert_wait(
            cx,
            &win,
            Duration::from_secs(8),
            "like shortcut to update the UI canon",
            |app| app.liked.contains(&liked_id) && app.is_liked(liked_id),
        );
        // The engine writes state.json from its worker thread, so poll disk.
        assert!(
            wait_for_liked(&state_path, liked_id, Duration::from_secs(15)),
            "state.json did not persist liked track {liked_id}; contents={}",
            fs::read_to_string(&state_path).unwrap_or_default()
        );
        let state = read_json(&state_path);
        assert!(
            state["liked"].as_array().is_some_and(|liked| liked
                .iter()
                .any(|t| t["id"].as_u64() == Some(liked_id))),
            "state.json liked array missing {liked_id}"
        );
        assert!(
            win.root().read(cx).liked_tracks.iter().any(|t| t.id == liked_id),
            "liked_tracks canon missing {liked_id}"
        );
        screenshot(cx, &win, &dir.join("shots/03-liked.png"));

        // 5b. Station click loads a real, playing Ballroom queue.
        click_id(cx, &win, "nav-home");
        click_id(cx, &win, "station-ballroom");
        assert_wait(
            cx,
            &win,
            Duration::from_secs(50),
            "Ballroom 4AM station to load and stream",
            |app| {
                app.screen == Screen::Home
                    && app.dials.club >= 0.99
                    && app.radio_status.to_uppercase().contains("BALLROOM")
                    && app.playback.state == PlaybackState::Playing
                    && app.current_source.is_some()
                    && !app.queue.is_empty()
            },
        );
        {
            let app = win.root().read(cx);
            let station = STATIONS.iter().find(|s| s.id == "ballroom").unwrap();
            assert_eq!(app.dials.club, station.dials.club);
            assert_eq!(app.dials.soft, station.dials.soft);
            assert!(app.radio_status.to_uppercase().contains("BALLROOM"), "radio_status={:?}", app.radio_status);
            station_track = app.current_id().expect("station current");
            assert_ne!(station_track, 0);
            assert!(
                app.queue.iter().all(|rec| rec.track.id != 0 && rec.track.duration_ms > 10_000),
                "station queue contains placeholder tracks"
            );
            eprintln!(
                "Ballroom station: {} - {}; queue={}",
                app.current.as_ref().map(|t| t.artist.clone()).unwrap_or_default(),
                app.current.as_ref().map(|t| t.title.clone()).unwrap_or_default(),
                app.queue.len()
            );
        }
        screenshot(cx, &win, &dir.join("shots/04-station.png"));

        // 6. Settings: real ALSA hints and a persisted software volume.
        // LIMITATION: Settings is only reachable through the unlabelled topbar
        // gear, which has no .id(); the rail has no Settings nav item. Invoke
        // the same public goto() that the gear listener calls (src/ui.rs:202).
        win.update(cx, |app, _, cx| {
            app.goto(Screen::Settings);
            cx.notify();
        })
        .expect("navigate to settings");
        assert_eq!(win.root().read(cx).screen, Screen::Settings);
        {
            let app = win.root().read(cx);
            let devices: &Vec<AlsaDevice> = &app.devices;
            assert!(
                devices.len() >= 2,
                "settings has fewer than two ALSA hints: {devices:?}"
            );
            assert!(
                devices.iter().any(|d| d.is_default && d.name == "default"),
                "list_devices must always include default first: {devices:?}"
            );
            assert!(
                devices.iter().any(|d| !d.is_default),
                "no real non-default ALSA hint was enumerated: {devices:?}"
            );
            assert!(
                devices.iter().all(|d| !d.name.trim().is_empty() && !d.description.trim().is_empty()),
                "ALSA hint with empty name/description: {devices:?}"
            );
            assert!(
                devices.iter().any(|d| d.name == "null"),
                "the null ALSA device was not enumerated: {devices:?}"
            );
            assert_eq!(app.device, "null");
            eprintln!("ALSA devices: {devices:?}");
        }
        click_id(cx, &win, "settings-volume-17");
        expected_volume = 1.0;
        {
            let app = win.root().read(cx);
            assert!(
                (app.volume - expected_volume).abs() < 0.001,
                "volume click set {} instead of {expected_volume}",
                app.volume
            );
        }
        assert!(
            wait_for_file(&settings_path, Duration::from_secs(5)),
            "settings.json was not written"
        );
        let settings = read_json(&settings_path);
        assert_eq!(settings["device"].as_str(), Some("null"));
        let persisted_volume = settings["volume"].as_f64().unwrap_or(-1.0) as f32;
        assert!(
            (persisted_volume - expected_volume).abs() < 0.001,
            "settings.json volume is {persisted_volume}, expected {expected_volume}"
        );
        eprintln!("settings.json: {settings}");
        screenshot(cx, &win, &dir.join("shots/05-settings.png"));

        expected_plays_after = win.root().read(cx).total_plays;
        assert!(expected_plays_after >= 2, "expected several live plays by now");
    });

    // Audio thread and worker are dropped with the app; no orphans.
    let leftovers = wait_for_no_ffmpeg(Duration::from_secs(5));
    assert!(leftovers.is_empty(), "orphan ffmpeg after shutdown: {leftovers:?}");

    // 7. Restart: a fresh app in a new headless application must reconstruct
    //    state.json and settings.json without replaying the like.
    configure_env(&data, &shared_cache());
    Application::headless().run(|cx| {
        let win = open_live(cx);
        assert_wait(
            cx,
            &win,
            Duration::from_secs(75),
            "restart bootstrap",
            |app| !app.booting && app.current.is_some(),
        );
        let app = win.root().read(cx);
        assert!(
            app.liked.contains(&liked_id),
            "restart lost liked track {liked_id} ({liked_title}); liked={:?}",
            app.liked
        );
        assert!(
            app.liked_tracks.iter().any(|t| t.id == liked_id),
            "restart lost the liked_tracks canon entry"
        );
        assert!(
            app.total_plays >= expected_plays_after,
            "restart lost plays: {} < {}",
            app.total_plays,
            expected_plays_after
        );
        assert!(
            (app.volume - expected_volume).abs() < 0.001,
            "restart did not load volume {} (got {})",
            expected_volume,
            app.volume
        );
        assert_eq!(app.device, "null", "restart did not keep the ALSA device");
        assert!(
            !app.art_cache.is_empty(),
            "restart has no artwork cache entries"
        );
        eprintln!(
            "restart: track={:?} liked={} plays={} volume={} device={}",
            app.current.as_ref().map(|t| format!("{} - {}", t.artist, t.title)),
            app.liked.len(),
            app.total_plays,
            app.volume,
            app.device
        );
        screenshot(cx, &win, &dir.join("shots/06-restart.png"));
    });

    let leftovers = wait_for_no_ffmpeg(Duration::from_secs(5));
    assert!(leftovers.is_empty(), "orphan ffmpeg after restart shutdown: {leftovers:?}");
    // Keep the evidence path visible in --nocapture output.
    eprintln!("live QA evidence: {}", dir.display());
}

/// Lower-level, non-UI sanity check that is cheap to run alongside the live
/// tests: the ALSA enumerator sees the null plugin and reports stable fields.
/// This one is not ignored because it needs no network.
#[test]
fn alsa_null_device_is_enumerable_and_openable() {
    let devices = AudioEngine::list_devices();
    assert!(
        devices.iter().any(|d| d.name == "default" && d.is_default),
        "default must be first/present: {devices:?}"
    );
    assert!(
        devices.iter().any(|d| d.name == "null"),
        "ALSA null device missing from hints: {devices:?}"
    );
    assert!(
        devices.iter().all(|d| !d.name.is_empty() && !d.description.is_empty()),
        "ALSA hint fields must be populated: {devices:?}"
    );
    let engine = AudioEngine::new(Some("null"), 0.2).expect("open ALSA null");
    let status = engine.status();
    assert_eq!(status.device, "null");
    assert!((status.volume - 0.2).abs() < 0.001);
}

