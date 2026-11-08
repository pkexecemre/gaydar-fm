//! GAYDAR FM — a SoundCloud radio for the girls, gays and theys.
//!
//! Run it:            cargo run --release
//! Screenshot mode:   cargo run -- --screenshot out.png --screen home
//! Screens:           home, gaydar, search, liked, history, settings, now

use blenderui::prelude::*;
use gaydar::app::{GaydarApp, Screen};
use gaydar::theme;

fn parse_screen(name: &str) -> Screen {
    match name {
        "gaydar" => Screen::Gaydar,
        "search" => Screen::Search,
        "liked" => Screen::Liked,
        "history" => Screen::History,
        "settings" => Screen::Settings,
        "now" | "nowplaying" => Screen::NowPlaying,
        _ => Screen::Home,
    }
}

fn save_png(path: &str, w: usize, h: usize, pixels: &[u32]) {
    let rgba: Vec<u8> = pixels
        .iter()
        .flat_map(|p| [(p >> 16) as u8, (p >> 8) as u8, *p as u8, 255])
        .collect();
    let file = std::fs::File::create(path).expect("create screenshot");
    let mut enc = png::Encoder::new(std::io::BufWriter::new(file), w as u32, h as u32);
    enc.set_color(png::ColorType::Rgba);
    enc.set_depth(png::BitDepth::Eight);
    enc.write_header()
        .expect("png header")
        .write_image_data(&rgba)
        .expect("png data");
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let screenshot = args
        .iter()
        .position(|a| a == "--screenshot")
        .and_then(|i| args.get(i + 1))
        .cloned();
    let live_shot = args
        .iter()
        .position(|a| a == "--live-shot")
        .and_then(|i| args.get(i + 1))
        .cloned();
    let screen_arg = args
        .iter()
        .position(|a| a == "--screen")
        .and_then(|i| args.get(i + 1))
        .cloned()
        .unwrap_or_else(|| "home".into());
    let boot_shot = screen_arg == "boot";
    let screen = parse_screen(&screen_arg);
    let demo = args.iter().any(|a| a == "--demo");
    let show_queue = args.iter().any(|a| a == "--queue");
    let width: f32 = args
        .iter()
        .position(|a| a == "--width")
        .and_then(|i| args.get(i + 1))
        .and_then(|s| s.parse().ok())
        .unwrap_or(1440.0);
    let height: f32 = args
        .iter()
        .position(|a| a == "--height")
        .and_then(|i| args.get(i + 1))
        .and_then(|s| s.parse().ok())
        .unwrap_or(900.0);

    let app = if screenshot.is_some() || live_shot.is_some() {
        Application::headless()
    } else {
        Application::new().gpu(true)
    };
    app.run(move |cx| {
        theme::load_fonts();
        let mut state = if (screenshot.is_some() && live_shot.is_none()) || demo || boot_shot {
            GaydarApp::demo(screen)
        } else {
            GaydarApp::live()
        };
        if boot_shot {
            state.demo = false;
            state.booting = true;
            state.current = None;
            state.current_source = None;
            state.client_ready = false;
            state.error = None;
        }
        let live = !state.demo;
        let opts = WindowOptions {
            window_bounds: Some(WindowBounds::Windowed(Bounds {
                origin: point(px(0.0), px(0.0)),
                size: size(px(width), px(height)),
            })),
            window_min_size: Some(size(px(1120.0), px(720.0))),
            titlebar: Some(TitlebarOptions {
                title: Some("GAYDAR FM".into()),
            }),
            ..Default::default()
        };
        let win = cx
            .open_window(opts, |_, cx| cx.new(|_| state))
            .expect("open GAYDAR window");
        win.update(cx, |app, window, cx| {
            window.set_window_title("GAYDAR FM — no login, no closet");
            if live {
                app.start_live(cx);
            } else {
                app.playback.levels = gaydar::app::demo_levels();
            }
            if show_queue {
                app.queue_open = true;
            }
        });
        if let Some(path) = &live_shot {
            let deadline = std::time::Instant::now() + std::time::Duration::from_secs(45);
            let mut playing = false;
            while std::time::Instant::now() < deadline {
                std::thread::sleep(std::time::Duration::from_millis(120));
                cx.run_pending();
                win.update(cx, |app, _, cx| app.pump(cx));
                let app = win.root().read(cx);
                if app.playback.state == gaydar::audio::PlaybackState::Playing {
                    playing = true;
                    break;
                }
                if app.error.is_some() && !app.booting {
                    break;
                }
            }
            if playing {
                // Give artwork and the first equalizer frames a moment to land.
                let art_deadline = std::time::Instant::now() + std::time::Duration::from_secs(5);
                while std::time::Instant::now() < art_deadline {
                    std::thread::sleep(std::time::Duration::from_millis(150));
                    cx.run_pending();
                    win.update(cx, |app, _, cx| app.pump(cx));
                    let ready = {
                        let app = win.root().read(cx);
                        app.current
                            .as_ref()
                            .is_some_and(|t| app.art_path(t).is_some())
                    };
                    if ready {
                        break;
                    }
                }
                if screen == Screen::NowPlaying {
                    win.update(cx, |app, _, _| app.screen = Screen::NowPlaying);
                }
                let (w, h, pixels) = cx.draw(win.id);
                save_png(path, w, h, &pixels);
                eprintln!("live screenshot: {path} ({w}x{h}), playing: {playing}");
            } else {
                let (w, h, pixels) = cx.draw(win.id);
                save_png(path, w, h, &pixels);
                eprintln!("live screenshot (not playing): {path} ({w}x{h})");
            }
        } else if let Some(path) = &screenshot {
            let (w, h, pixels) = cx.draw(win.id);
            save_png(path, w, h, &pixels);
            eprintln!("screenshot: {path} ({w}x{h})");
        }
    });
}
