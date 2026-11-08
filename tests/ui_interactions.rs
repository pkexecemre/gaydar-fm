//! Headless interaction checks for the full GAYDAR UI.

use blenderui::prelude::*;
use gaydar::app::{GaydarApp, Screen};
use gaydar::audio::PlaybackState;

fn click_id(cx: &mut App, win: WindowId, id: &str) {
    let bounds = cx
        .element_bounds(win, id)
        .unwrap_or_else(|| panic!("missing element {id}"));
    cx.simulate_click(
        win,
        bounds.origin.x + bounds.size.width / 2.0,
        bounds.origin.y + bounds.size.height / 2.0,
    );
    cx.draw(win);
}

#[test]
fn search_play_seek_station_and_now_playing_work() {
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
                |_, cx| cx.new(|_| GaydarApp::demo(Screen::Search)),
            )
            .expect("window");
        cx.draw(win.id);
        assert_eq!(win.root().read(cx).screen, Screen::Search);

        // Focus the custom search field and type through the keyboard.
        click_id(cx, win.id, "search-field");
        assert_eq!(
            cx.window(win.id)
                .and_then(|w| w.focused_id().map(str::to_string))
                .as_deref(),
            Some("search-field")
        );
        for (text, key) in [
            ("b", "b"),
            ("a", "a"),
            ("l", "l"),
            ("l", "l"),
            ("r", "r"),
            ("o", "o"),
            ("o", "o"),
            ("m", "m"),
        ] {
            cx.simulate_key(win.id, text, key);
        }
        cx.draw(win.id);
        let app = win.root().read(cx);
        assert_eq!(
            app.search_query, "ballroom",
            "custom search field did not receive keys"
        );
        let _ = app;
        cx.simulate_key(win.id, "\r", "Return");
        cx.draw(win.id);
        assert!(!win.root().read(cx).search_results.is_empty());

        // Navigate home and toggle playback.
        click_id(cx, win.id, "nav-home");
        assert_eq!(win.root().read(cx).screen, Screen::Home);
        let before = win.root().read(cx).playback.state;
        assert_eq!(before, PlaybackState::Playing);
        click_id(cx, win.id, "play-toggle");
        assert_eq!(win.root().read(cx).playback.state, PlaybackState::Paused);
        click_id(cx, win.id, "play-toggle");
        assert_eq!(win.root().read(cx).playback.state, PlaybackState::Playing);

        // Queue drawer toggles from the player.
        click_id(cx, win.id, "queue-toggle");
        assert!(win.root().read(cx).queue_open, "queue did not open");
        click_id(cx, win.id, "queue-close");
        assert!(!win.root().read(cx).queue_open, "queue did not close");

        // Global keyboard shortcut while a stable element is focused.
        click_id(cx, win.id, "nav-home");
        cx.simulate_key(win.id, " ", "space");
        cx.draw(win.id);
        assert_eq!(
            win.root().read(cx).playback.state,
            PlaybackState::Paused,
            "space did not pause"
        );
        cx.simulate_key(win.id, " ", "space");
        cx.draw(win.id);
        assert_eq!(
            win.root().read(cx).playback.state,
            PlaybackState::Playing,
            "space did not resume"
        );

        // Seek by clicking a waveform segment (no element bounds needed).
        click_id(cx, win.id, "seek-bar-24");
        let app = win.root().read(cx);
        let ratio = app.playback.position_ms as f32 / app.current_duration().max(1) as f32;
        assert!((ratio - 0.51).abs() < 0.05, "seek ratio was {ratio}");
        let _ = app;

        // Station click applies its dials and starts a demo mix.
        click_id(cx, win.id, "station-ballroom");
        let app = win.root().read(cx);
        assert_eq!(app.dials.club, 1.0);
        assert_eq!(app.screen, Screen::Home);

        // Open the immersive view from the player artwork.
        click_id(cx, win.id, "player-art");
        assert_eq!(win.root().read(cx).screen, Screen::NowPlaying);
        cx.draw(win.id);
    });
}
