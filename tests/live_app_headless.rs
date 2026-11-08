//! Live end-to-end smoke test: real SoundCloud -> Gaydar -> ffmpeg -> ALSA null.
//!
//! Ignored by default because it needs network access, ffmpeg, ALSA and ~20 s.
//! Run with:
//!   GAYDAR_ALSA_DEVICE=null GAYDAR_DATA_DIR=/tmp/gaydar-live/data \
//!   GAYDAR_CACHE_DIR=/tmp/gaydar-live/cache \
//!   cargo test --test live_app_headless -- --ignored --nocapture

use blenderui::prelude::*;
use gaydar::app::{GaydarApp, STATIONS, Screen};
use gaydar::audio::PlaybackState;
use std::time::{Duration, Instant};

#[test]
#[ignore]
fn starts_playing_from_soundcloud_and_advances() {
    Application::headless().run(|cx| {
        let win = cx
            .open_window(
                WindowOptions {
                    window_bounds: Some(WindowBounds::Windowed(Bounds {
                        origin: point(px(0.0), px(0.0)),
                        size: size(px(1440.0), px(900.0)),
                    })),
                    ..Default::default()
                },
                |_, cx| cx.new(|_| GaydarApp::live()),
            )
            .expect("window");
        win.update(cx, |app, _, cx| app.start_live(cx));

        let deadline = Instant::now() + Duration::from_secs(35);
        let mut first_track = None;
        while Instant::now() < deadline {
            std::thread::sleep(Duration::from_millis(120));
            cx.run_pending();
            win.update(cx, |app, _, cx| app.pump(cx));
            let app = win.root().read(cx);
            if let Some(err) = &app.error {
                eprintln!("boot error: {err}");
                break;
            }
            if app.playback.state == PlaybackState::Playing {
                first_track = app.current_id();
                eprintln!(
                    "PLAYING: {} — {} at {}ms / {}ms (device {})",
                    app.current
                        .as_ref()
                        .map(|t| t.title.as_str())
                        .unwrap_or("?"),
                    app.current
                        .as_ref()
                        .map(|t| t.artist.as_str())
                        .unwrap_or("?"),
                    app.playback.position_ms,
                    app.playback.duration_ms,
                    app.playback.device
                );
                break;
            }
        }
        assert!(first_track.is_some(), "live app never reached Playing");

        // Let the position move, then seek and verify.
        std::thread::sleep(Duration::from_millis(1200));
        cx.run_pending();
        win.update(cx, |app, _, cx| app.pump(cx));
        let pos = win.root().read(cx).playback.position_ms;
        assert!(pos > 100, "playback position did not advance: {pos}ms");

        win.update(cx, |app, _, cx| {
            app.seek_fraction(0.5);
            app.pump(cx);
        });
        std::thread::sleep(Duration::from_millis(900));
        cx.run_pending();
        win.update(cx, |app, _, cx| app.pump(cx));
        let app = win.root().read(cx);
        let ratio = app.playback.position_ms as f32 / app.playback.duration_ms.max(1) as f32;
        assert!(
            (ratio - 0.5).abs() < 0.12,
            "seek ratio after live seek: {ratio}"
        );
        eprintln!(
            "after seek: {}ms ({:.0}%)",
            app.playback.position_ms,
            ratio * 100.0
        );

        // Advance to the next queued track.
        let previous = app.current_id();
        let _ = app;
        win.update(cx, |app, _, cx| {
            app.next_track();
            app.pump(cx);
        });
        let deadline = Instant::now() + Duration::from_secs(25);
        let mut advanced = false;
        while Instant::now() < deadline {
            std::thread::sleep(Duration::from_millis(150));
            cx.run_pending();
            win.update(cx, |app, _, cx| app.pump(cx));
            let app = win.root().read(cx);
            if app.current_id().is_some()
                && app.current_id() != previous
                && app.playback.state == PlaybackState::Playing
            {
                eprintln!(
                    "NEXT: {} — {} at {}ms",
                    app.current
                        .as_ref()
                        .map(|t| t.title.as_str())
                        .unwrap_or("?"),
                    app.current
                        .as_ref()
                        .map(|t| t.artist.as_str())
                        .unwrap_or("?"),
                    app.playback.position_ms
                );
                advanced = true;
                break;
            }
        }
        assert!(advanced, "queue did not advance to a new playing track");

        // Like the current track and wait for the engine to persist it.
        let liked_id = win.root().read(cx).current_id().expect("current");
        win.update(cx, |app, _, cx| {
            if let Some(track) = app.current.clone() {
                app.toggle_like(&track);
            }
            app.pump(cx);
        });
        let deadline = Instant::now() + Duration::from_secs(8);
        while Instant::now() < deadline {
            std::thread::sleep(Duration::from_millis(120));
            cx.run_pending();
            win.update(cx, |app, _, cx| app.pump(cx));
            if win.root().read(cx).liked.contains(&liked_id) {
                break;
            }
        }
        assert!(
            win.root().read(cx).liked.contains(&liked_id),
            "like did not stick"
        );

        // Search for a validated queer query.
        win.update(cx, |app, _, cx| {
            app.set_search_query("ballroom vogue".into());
            app.submit_search();
            app.pump(cx);
        });
        let deadline = Instant::now() + Duration::from_secs(25);
        while Instant::now() < deadline {
            std::thread::sleep(Duration::from_millis(150));
            cx.run_pending();
            win.update(cx, |app, _, cx| app.pump(cx));
            let app = win.root().read(cx);
            if !app.searching && !app.search_results.is_empty() {
                eprintln!(
                    "SEARCH: {} results, first: {}",
                    app.search_results.len(),
                    app.search_results[0].title
                );
                break;
            }
        }
        assert!(
            !win.root().read(cx).search_results.is_empty(),
            "live search returned no results"
        );

        // Tune a station through the real worker and wait for playback.
        let before_station = win.root().read(cx).current_id();
        win.update(cx, |app, _, cx| {
            app.apply_station(&STATIONS[1]);
            app.pump(cx);
        });
        let deadline = Instant::now() + Duration::from_secs(30);
        let mut station_playing = false;
        while Instant::now() < deadline {
            std::thread::sleep(Duration::from_millis(150));
            cx.run_pending();
            win.update(cx, |app, _, cx| app.pump(cx));
            let app = win.root().read(cx);
            if app.current_id() != before_station && app.playback.state == PlaybackState::Playing {
                eprintln!(
                    "STATION: {} — {}",
                    app.current
                        .as_ref()
                        .map(|t| t.title.as_str())
                        .unwrap_or("?"),
                    app.current
                        .as_ref()
                        .map(|t| t.artist.as_str())
                        .unwrap_or("?")
                );
                station_playing = true;
                break;
            }
        }
        assert!(station_playing, "station did not start a new track");

        // Seek near the end and verify the service advances on natural finish.
        let station_track = win.root().read(cx).current_id();
        win.update(cx, |app, _, cx| {
            app.seek_fraction(0.985);
            app.pump(cx);
        });
        let deadline = Instant::now() + Duration::from_secs(25);
        let mut finished_advanced = false;
        while Instant::now() < deadline {
            std::thread::sleep(Duration::from_millis(150));
            cx.run_pending();
            win.update(cx, |app, _, cx| app.pump(cx));
            let app = win.root().read(cx);
            if app.current_id() != station_track && app.playback.state == PlaybackState::Playing {
                eprintln!(
                    "AFTER FINISH: {} — {}",
                    app.current
                        .as_ref()
                        .map(|t| t.title.as_str())
                        .unwrap_or("?"),
                    app.current
                        .as_ref()
                        .map(|t| t.artist.as_str())
                        .unwrap_or("?")
                );
                finished_advanced = true;
                break;
            }
        }
        assert!(
            finished_advanced,
            "natural finish did not advance the queue"
        );

        // Ensure the state file is written and contains a play.
        let state =
            std::fs::read_to_string(std::env::var("GAYDAR_DATA_DIR").unwrap() + "/state.json")
                .expect("state.json");
        assert!(state.contains("history"), "state.json has no history");
        win.update(cx, |app, _, _| app.goto(Screen::Home));
    });
}
