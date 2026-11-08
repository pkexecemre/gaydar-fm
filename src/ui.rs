//! The GAYDAR FM interface: a queer club-poster radio, rendered with BUI.

use crate::app::{
    GaydarApp, STATIONS, Screen, Station, ToastKind, demo_cover_class, demo_reasons, demo_tracks,
};
use crate::audio::PlaybackState;
use crate::extra::EXTRA_CSS;
use crate::model::Track;
use crate::theme;
use crate::views::{
    boot_screen, gaydar_screen, liked_screen, now_playing, search_screen, settings_screen,
};
use blenderui::prelude::*;
use std::path::Path;

pub(crate) fn icon(name: &str, size: f32, color: &str) -> Div {
    let path = theme::icon_path(name);
    if Path::new(&path).exists() {
        div()
            .class("ui-icon")
            .style(format!(
                "width:{size}px;height:{size}px;flex:0 0 {size}px;background:{color};-webkit-mask-image:url('{path}');mask-image:url('{path}');-webkit-mask-size:contain;mask-size:contain;-webkit-mask-repeat:no-repeat;mask-repeat:no-repeat;-webkit-mask-position:center;mask-position:center;"
            ))
    } else {
        div().style(format!(
            "width:{size}px;height:{size}px;flex:0 0 {size}px;border:1.4px solid {color};border-radius:4px;opacity:.55;"
        ))
    }
}

pub(crate) fn fallback_cover(track: &Track) -> String {
    let by_id = match track.id {
        1 => Some("06-disco.svg"),
        2 => Some("01-hyperpop-chrome.svg"),
        3 => Some("03-sapphic-dusk.svg"),
        4 => Some("02-ballroom-vogue.svg"),
        5 => Some("06-disco.svg"),
        6 => Some("01-hyperpop-chrome.svg"),
        7 => Some("05-leather-camp.svg"),
        8 => Some("05-leather-camp.svg"),
        9 => Some("07-ambient-softness.svg"),
        10 => Some("06-disco.svg"),
        11 => Some("08-chaotic-collage.svg"),
        12 => Some("05-leather-camp.svg"),
        _ => None,
    };
    let by_genre = match track.genre.as_deref().unwrap_or("").to_lowercase().as_str() {
        "hyper" | "hyperpop" | "glitch" => "01-hyperpop-chrome.svg",
        "ballroom" | "vogue" => "02-ballroom-vogue.svg",
        "soft" | "sapphic" | "ambient" => "03-sapphic-dusk.svg",
        "trans" => "04-trans-joy.svg",
        "camp" | "drag" => "05-leather-camp.svg",
        "club" | "disco" | "house" => "06-disco.svg",
        "chaos" | "experimental" => "08-chaotic-collage.svg",
        _ => "",
    };
    let file = by_id.unwrap_or_else(|| {
        if !by_genre.is_empty() {
            by_genre
        } else {
            match track.id as usize % 8 {
                0 => "01-hyperpop-chrome.svg",
                1 => "02-ballroom-vogue.svg",
                2 => "03-sapphic-dusk.svg",
                3 => "04-trans-joy.svg",
                4 => "05-leather-camp.svg",
                5 => "06-disco.svg",
                6 => "07-ambient-softness.svg",
                _ => "08-chaotic-collage.svg",
            }
        }
    });
    theme::asset(&format!("covers/{file}"))
}

pub(crate) fn track_thumb(app: &GaydarApp, track: &Track, class: &str) -> Div {
    let art = app.art_path(track).unwrap_or_else(|| fallback_cover(track));
    div()
        .class(format!("{class} {}", demo_cover_class(track.id)))
        .child(
            img()
                .src(art)
                .style("width:100%;height:100%;object-fit:cover;display:block;"),
        )
}

fn cover_art_large(app: &GaydarApp, track: &Track) -> Div {
    let art = app.art_path(track).unwrap_or_else(|| fallback_cover(track));
    div().class("cover-art").style(format!(
        "background-image:url('{art}');background-size:cover;background-position:center;"
    ))
}

fn nav_item(
    app: &GaydarApp,
    screen: Screen,
    icon_name: &str,
    label: &str,
    cx: &mut Context<GaydarApp>,
) -> Div {
    let active =
        app.screen == screen || (screen == Screen::Home && app.screen == Screen::NowPlaying);
    let color = if active { "#160812" } else { "#b9a7bd" };
    let nav_id = match screen {
        Screen::Home => "nav-home",
        Screen::Gaydar => "nav-gaydar",
        Screen::Search => "nav-search",
        Screen::Liked => "nav-liked",
        Screen::History => "nav-history",
        _ => "nav-settings",
    };
    let mut el = div()
        .class(if active {
            "nav-item active"
        } else {
            "nav-item"
        })
        .id(nav_id)
        .child(icon(icon_name, 15.0, color))
        .child(div().child(label.to_string()))
        .child(
            div()
                .class("nav-kbd")
                .child(if screen == Screen::Search { "/" } else { "" }),
        )
        .on_click(cx.listener(move |app, _: &ClickEvent, _, cx| {
            app.goto(screen);
            cx.notify();
        }));
    if active {
        el = el.child(div().class("nav-active-mark"));
    }
    el
}

fn rail(app: &GaydarApp, cx: &mut Context<GaydarApp>) -> AnyElement {
    let mut nav = div().class("nav");
    nav = nav
        .child(nav_item(app, Screen::Home, "home", "For You", cx))
        .child(nav_item(app, Screen::Gaydar, "gaydar", "Gaydar", cx))
        .child(nav_item(app, Screen::Search, "search", "Search", cx))
        .child(nav_item(app, Screen::Liked, "heart", "Liked", cx))
        .child(nav_item(app, Screen::History, "history", "History", cx));

    div()
        .class("rail")
        .child(
            div()
                .class("brand")
                .child(
                    img()
                        .src(theme::asset("brand/app-mark.svg"))
                        .style("width:40px;height:40px;flex:0 0 40px;display:block;"),
                )
                .child(
                    img()
                        .src(theme::asset("brand/wordmark.svg"))
                        .style("width:164px;height:41px;display:block;object-fit:contain;"),
                ),
        )
        .child(
            div()
                .class("guest-card")
                .child(div().class("guest-kicker").child("GUEST DIVA"))
                .child(div().class("guest-title").child("No login. No closet."))
                .child(div().class("guest-note").child(
                    "Just open it and let the gaydar drive. Your taste stays on this device.",
                )),
        )
        .child(div().class("nav-label").child("LISTEN"))
        .child(nav)
        .child(div().class("rail-spacer"))
        .child(
            div()
                .class("rail-mini")
                .child(div().class("rail-mini-label").child("OUTPUT · ALSA"))
                .child(
                    div()
                        .class("rail-mini-row")
                        .child(div().class("alsa-dot"))
                        .child(div().class("alsa-name").child(app.device.clone())),
                )
                .child(div().class("alsa-meta").child(format!(
                    "{} · {} kHz · stereo",
                    if app.playback.state == PlaybackState::Playing {
                        "playing"
                    } else {
                        "ready"
                    },
                    app.playback.sample_rate / 1000
                ))),
        )
        .child(
            div()
                .class("rail-foot")
                .child(div().class("heart-rainbow").child("♥ ♥ ♥ ♥ ♥"))
                .child(div().child("Made for the girls, gays & theys."))
                .child(div().child("Unofficial SoundCloud client.")),
        )
        .into_any_element()
}

fn search_trigger(cx: &mut Context<GaydarApp>) -> Div {
    div()
        .class("search-trigger")
        .child(div().class("search-trigger-icon"))
        .child(div().child("Search the queer underground…"))
        .child(div().class("search-key").child("/"))
        .on_click(cx.listener(|app, _: &ClickEvent, _, cx| {
            app.goto(Screen::Search);
            cx.notify();
        }))
}

fn topbar(app: &GaydarApp, cx: &mut Context<GaydarApp>) -> AnyElement {
    div()
        .class("topbar")
        .child(
            div()
                .class("topbar-context")
                .child(div().class("topbar-eyebrow").child(app.screen.eyebrow()))
                .child(div().class("topbar-screen").child(app.screen.label())),
        )
        .child(search_trigger(cx))
        .child(
            div()
                .class("topbar-right")
                .child(
                    div()
                        .class("live-chip")
                        .child(div().class("live-dot"))
                        .child(div().child(if app.booting { "TUNING IN" } else { "ON AIR" })),
                )
                .child(
                    div()
                        .class("icon-btn")
                        .child(div().class("gear"))
                        .on_click(cx.listener(|app, _: &ClickEvent, _, cx| {
                            app.goto(Screen::Settings);
                            cx.notify();
                        })),
                ),
        )
        .into_any_element()
}

fn section_head(
    title: &str,
    sub: &str,
    link: &str,
    target: Screen,
    cx: &mut Context<GaydarApp>,
) -> Div {
    div()
        .class("section-head")
        .child(
            div()
                .class("section-title")
                .child(title.to_string())
                .child(div().class("section-sub").child(sub.to_string())),
        )
        .child(
            div()
                .class("section-link")
                .child(link.to_string())
                .on_click(cx.listener(move |app, _: &ClickEvent, _, cx| {
                    app.goto(target);
                    cx.notify();
                })),
        )
}

fn hero(app: &GaydarApp, cx: &mut Context<GaydarApp>) -> AnyElement {
    let track = app
        .current
        .clone()
        .unwrap_or_else(|| demo_tracks()[0].clone());
    let playing = app.playback.state == PlaybackState::Playing;
    let greeting = {
        use chrono::Timelike;
        match chrono::Local::now().hour() {
            5..=11 => "Good morning,",
            12..=17 => "Good afternoon,",
            _ => "Good evening,",
        }
    };
    let mut eq = div().class("hero-eq");
    for i in 0..5 {
        let h = [8.0, 14.0, 20.0, 12.0, 17.0][i];
        eq = eq.child(div().style(format!(
            "width:3px;height:{h}px;border-radius:2px;background:{};",
            ["#ffcf5c", "#ff4fa3", "#c3a6ff", "#7ef9ff", "#d6ff3f"][i]
        )));
    }

    div()
        .class("hero")
        .child(
            div()
                .class("hero-copy")
                .child(
                    div()
                        .class("kicker")
                        .child(div().class("kicker-line"))
                        .child(div().child("THE GAY AGENDA · LIVE FROM THE CLOUD")),
                )
                .child(
                    div()
                        .class("hero-title")
                        .child(greeting.to_string())
                        .child(div().class("hero-serif").child("gorgeous.")),
                )
                .child(
                    div()
                        .class("lede")
                        .child("No login. No closet. Your gaydar picks the first track, learns what makes you replay it, and never lets the aux die."),
                )
                .child(
                    div()
                        .class("hero-actions")
                        .child(
                            button()
                                .class("btn-primary")
                                .child(icon(if playing { "pause" } else { "play" }, 12.0, "#170912"))
                                .child(if playing { "PAUSE" } else { "START THE RADIO" })
                                .on_click(cx.listener(|app, _: &ClickEvent, _, cx| {
                                    if app.demo { app.toggle_play(); } else { app.start_radio(); }
                                    cx.notify();
                                })),
                        )
                        .child(
                            button()
                                .class("btn-ghost")
                                .child("TUNE THE GAYDAR")
                                .on_click(cx.listener(|app, _: &ClickEvent, _, cx| {
                                    app.goto(Screen::Gaydar);
                                    cx.notify();
                                })),
                        ),
                )
                .child(
                    div()
                        .class("hero-meta")
                        .child(eq)
                        .child(div().child(if playing { "NOW SERVING" } else { "ON STANDBY" }))
                        .child(div().class("hero-meta-sep"))
                        .child(div().child(track.title.clone()))
                        .child(div().class("hero-meta-sep"))
                        .child(div().child(track.artist.clone())),
                ),
        )
        .child(
            div()
                .class("hero-art")
                .child(div().class("vinyl"))
                .child(
                    div()
                        .class("cover-card")
                        .child(cover_art_large(app, &track)),
                )
                .child(
                    div()
                        .class("sticker")
                        .child("GAYDAR CERTIFIED")
                        .child(div().class("sticker-note").child("100% CUNT-APPROVED")),
                )
                .child(
                    div()
                        .class("onair")
                        .child(div().class("onair-dot"))
                        .child(if playing { "ON AIR" } else { "STANDBY" }),
                ),
        )
        .into_any_element()
}

fn station_card(
    app: &GaydarApp,
    station: Station,
    index: usize,
    cx: &mut Context<GaydarApp>,
) -> Div {
    let _ = app;
    let mut word = div().class("station-artword");
    for part in station.name.split_whitespace() {
        word = word.child(div().child(part.to_string()));
    }
    div()
        .class(format!("station {}", station.id))
        .id(format!("station-{}", station.id))
        .child(
            div()
                .class("station-inner")
                .child(
                    div()
                        .class("station-top")
                        .child(div().class("station-tag").child(station.tag))
                        .child(div().class("station-num").child(format!("0{}", index + 1))),
                )
                .child(word)
                .child(div().class("station-info").child(station.blurb))
                .child(
                    div()
                        .class("station-play")
                        .child(icon("play", 12.0, "#1a0914")),
                ),
        )
        .child(div().class("station-artdeco"))
        .on_click(cx.listener(move |app, _: &ClickEvent, _, cx| {
            app.apply_station(&station);
            cx.notify();
        }))
}

pub(crate) fn track_row(
    app: &GaydarApp,
    track: &Track,
    index: usize,
    reason: Option<String>,
    cx: &mut Context<GaydarApp>,
) -> Div {
    let active = app.current_id() == Some(track.id);
    let liked = app.is_liked(track.id);
    let t = track.clone();
    let t_play = track.clone();
    let reason_text = reason.unwrap_or_else(|| {
        track
            .genre
            .clone()
            .map(|g| format!("{} · soundcloud", g.to_lowercase()))
            .unwrap_or_else(|| "soundcloud".into())
    });
    div()
        .class(if active {
            "track-row active"
        } else {
            "track-row"
        })
        .child(div().class("track-index").child(if active {
            "▶".to_string()
        } else {
            format!("{:02}", index + 1)
        }))
        .child(track_thumb(app, track, "track-art"))
        .child(
            div()
                .class("track-info")
                .child(div().class("track-title").child(track.title.clone()))
                .child(div().class("track-artist").child(format!(
                    "{} · {}",
                    track.artist,
                    track.genre.clone().unwrap_or_else(|| "soundcloud".into())
                ))),
        )
        .child(div().class("track-reason").child(reason_text.clone()))
        .child(div().class("track-dur").child(track.duration_label()))
        .child(
            div()
                .class(if liked {
                    "track-heart on"
                } else {
                    "track-heart"
                })
                .child(icon(
                    if liked { "heart-filled" } else { "heart" },
                    13.0,
                    if liked { "#ff4fa3" } else { "#806d88" },
                ))
                .on_click(cx.listener(move |app, _: &ClickEvent, _, cx| {
                    cx.stop_propagation();
                    app.toggle_like(&t);
                    cx.notify();
                })),
        )
        .on_click(cx.listener(move |app, _: &ClickEvent, _, cx| {
            app.play_track(t_play.clone(), reason_text.clone());
            cx.notify();
        }))
}

fn home(app: &GaydarApp, cx: &mut Context<GaydarApp>) -> AnyElement {
    let recs: Vec<_> = app.queue.iter().take(5).cloned().collect();
    let fresh: Vec<_> = demo_tracks().into_iter().skip(3).take(4).collect();
    let mut content = div()
        .class("content")
        .child(hero(app, cx))
        .child(section_head(
            "STATIONS FOR THE GIRLS, GAYS & THEYS",
            "PICK A MOOD, HENNY",
            "VIEW ALL",
            Screen::Gaydar,
            cx,
        ));
    let mut grid = div().class("station-grid");
    for (i, station) in STATIONS.iter().enumerate() {
        grid = grid.child(station_card(app, *station, i, cx));
    }
    content = content.child(grid);
    content = content
        .child(section_head(
            "THE GAYDAR SAYS",
            "RECOMMENDED BY YOUR TASTE",
            "TUNE IT",
            Screen::Gaydar,
            cx,
        ))
        .child(if recs.is_empty() {
            div()
                .class("empty-inline")
                .child("The gaydar is listening. Start the radio to feed it.")
        } else {
            let mut list = div().class("tracklist");
            for (i, rec) in recs.iter().enumerate() {
                list = list.child(track_row(app, &rec.track, i, Some(rec.reason.clone()), cx));
            }
            list
        });
    content = content
        .child(section_head(
            "FRESHLY OUT THE CLOSET",
            "NEW QUEER RELEASES",
            "SCOPE",
            Screen::Search,
            cx,
        ))
        .child({
            let mut row = div().class("mini-grid");
            for (i, t) in fresh.iter().enumerate() {
                let tt = t.clone();
                let app_art = track_thumb(app, t, "mini-art");
                row = row.child(
                    div()
                        .class("mini-card")
                        .child(app_art)
                        .child(
                            div()
                                .class("mini-info")
                                .child(div().class("mini-title").child(t.title.clone()))
                                .child(div().class("mini-artist").child(t.artist.clone())),
                        )
                        .on_click(cx.listener(move |app, _: &ClickEvent, _, cx| {
                            app.play_track(
                                tt.clone(),
                                app_reasons()[i % app_reasons().len()].to_string(),
                            );
                            cx.notify();
                        })),
                );
            }
            row
        });
    content.into_any_element()
}

fn app_reasons() -> [&'static str; 8] {
    demo_reasons()
}

pub(crate) fn wave_seek(
    app: &GaydarApp,
    id_prefix: &'static str,
    segments: usize,
    cx: &mut Context<GaydarApp>,
) -> Div {
    let progress = app.progress();
    let seed = app.current_id().unwrap_or(7);
    let mut row = div().class("wave-seek");
    for i in 0..segments {
        let played = (i as f32 + 0.5) / segments as f32 <= progress;
        let wave = (((i as u64 * 37 + seed * 13) % 19) as f32) / 19.0;
        let height = 4.0 + wave * 16.0;
        let color = if played {
            let t = i as f32 / segments as f32;
            if t < 0.28 {
                "#ff4fa3"
            } else if t < 0.52 {
                "#ffcf5c"
            } else if t < 0.76 {
                "#7ef9ff"
            } else {
                "#c3a6ff"
            }
        } else {
            "rgba(255,255,255,.13)"
        };
        row = row.child(
            div()
                .class(if played {
                    "wave-bar played"
                } else {
                    "wave-bar"
                })
                .id(format!("{id_prefix}-{i}"))
                .style(format!("height:{height:.0}px;background:{color};"))
                .on_click(cx.listener(move |app, _: &ClickEvent, _, cx| {
                    app.seek_fraction((i as f32 + 0.5) / segments as f32);
                    cx.notify();
                })),
        );
    }
    row
}

pub(crate) fn volume_bars(
    app: &GaydarApp,
    id_prefix: &'static str,
    segments: usize,
    cx: &mut Context<GaydarApp>,
) -> Div {
    let mut row = div().class("volume-bars");
    let lit = (app.volume * segments as f32).round() as usize;
    for i in 0..segments {
        let on = i < lit;
        row = row.child(
            div()
                .class(if on { "volume-seg on" } else { "volume-seg" })
                .id(format!("{id_prefix}-{i}"))
                .on_click(cx.listener(move |app, _: &ClickEvent, _, cx| {
                    app.set_volume((i as f32 + 1.0) / segments as f32);
                    cx.notify();
                })),
        );
    }
    row
}

fn player(app: &GaydarApp, cx: &mut Context<GaydarApp>) -> AnyElement {
    let Some(track) = app.current.clone() else {
        return div()
            .class("player")
            .child(
                div()
                    .class("empty-player")
                    .child(div().class("empty-title").child("The aux is free."))
                    .child(
                        div()
                            .class("empty-note")
                            .child("Press play and let the gaydar choose."),
                    ),
            )
            .into_any_element();
    };
    let playing = app.playback.state == PlaybackState::Playing;
    let liked = app.is_liked(track.id);
    let mut viz = div().class("mini-viz");
    for i in 0..20 {
        let level = app.playback.levels[(i * 32 / 20).min(31)].max(0.08);
        let color = match i % 4 {
            0 => "#ff4fa3",
            1 => "#ffcf5c",
            2 => "#c3a6ff",
            _ => "#7ef9ff",
        };
        viz = viz.child(div().style(format!(
            "width:2px;height:{:.0}px;border-radius:2px;background:{};opacity:.85;",
            3.0 + level * 19.0,
            color
        )));
    }
    let seek_id = "seek-bar";
    let vol_id = "volume-bar";
    div()
        .class("player")
        .id("player-bar")
        .child(
            div()
                .class("player-track")
                .child(
                    div()
                        .class("player-art")
                        .id("player-art")
                        .child(
                            img()
                                .src(
                                    app.art_path(&track)
                                        .unwrap_or_else(|| fallback_cover(&track)),
                                )
                                .style("width:100%;height:100%;object-fit:cover;display:block;"),
                        )
                        .on_click(cx.listener(|app, _: &ClickEvent, _, cx| {
                            app.toggle_now_playing();
                            cx.notify();
                        })),
                )
                .child(
                    div()
                        .class("player-meta")
                        .child(div().class("player-title").child(track.title.clone()))
                        .child(div().class("player-artist").child(track.artist.clone())),
                )
                .child(
                    div()
                        .class(if liked {
                            "player-heart on"
                        } else {
                            "player-heart"
                        })
                        .child(icon(
                            if liked { "heart-filled" } else { "heart" },
                            14.0,
                            if liked { "#ff4fa3" } else { "#8f7a97" },
                        ))
                        .on_click(cx.listener(move |app, _: &ClickEvent, _, cx| {
                            app.toggle_like(&track);
                            cx.notify();
                        })),
                ),
        )
        .child(
            div()
                .class("player-center")
                .child(
                    div()
                        .class("transport")
                        .child(
                            div()
                                .class("small-control")
                                .child(icon("shuffle", 13.0, "#a08ba8")),
                        )
                        .child(
                            div()
                                .class("small-control")
                                .child(icon("prev", 13.0, "#a08ba8"))
                                .on_click(cx.listener(|app, _: &ClickEvent, _, cx| {
                                    app.prev_track();
                                    cx.notify();
                                })),
                        )
                        .child(
                            div()
                                .class(if playing {
                                    "play-btn"
                                } else {
                                    "play-btn paused"
                                })
                                .id("play-toggle")
                                .child(icon(
                                    if playing { "pause" } else { "play" },
                                    14.0,
                                    "#1b0912",
                                ))
                                .on_click(cx.listener(|app, _: &ClickEvent, _, cx| {
                                    app.toggle_play();
                                    cx.notify();
                                })),
                        )
                        .child(
                            div()
                                .class("small-control")
                                .child(icon("next", 13.0, "#a08ba8"))
                                .on_click(cx.listener(|app, _: &ClickEvent, _, cx| {
                                    app.next_track();
                                    cx.notify();
                                })),
                        )
                        .child(
                            div()
                                .class("small-control active")
                                .child(icon("repeat", 13.0, "#d6ff3f")),
                        ),
                )
                .child(
                    div()
                        .class("seek-row")
                        .child(div().class("time").child(app.position_label()))
                        .child(wave_seek(app, seek_id, 48, cx))
                        .child(div().class("time right").child(app.duration_label())),
                ),
        )
        .child(
            div()
                .class("player-right")
                .child(viz)
                .child(
                    div()
                        .class("volume-wrap")
                        .child(icon("volume", 13.0, "#a08ba8"))
                        .child(volume_bars(app, vol_id, 14, cx)),
                )
                .child(
                    div()
                        .class("queue-btn")
                        .id("queue-toggle")
                        .child(icon("queue", 13.0, "#a08ba8"))
                        .on_click(cx.listener(|app, _: &ClickEvent, _, cx| {
                            app.queue_open = !app.queue_open;
                            cx.notify();
                        })),
                )
                .child(
                    div()
                        .class("queue-count")
                        .child(app.queue.len().to_string()),
                ),
        )
        .into_any_element()
}

fn queue_drawer(app: &GaydarApp, cx: &mut Context<GaydarApp>) -> AnyElement {
    let mut rows = div().class("qd-list");
    if app.queue.is_empty() {
        rows = rows.child(
            div()
                .class("empty-state")
                .child(div().class("empty-kicker").child("QUEUE DRY"))
                .child(div().class("empty-title").child("She's fetching."))
                .child(
                    div()
                        .class("empty-note")
                        .child("The gaydar will drop more tracks in a second."),
                ),
        );
    } else {
        for (i, rec) in app.queue.iter().enumerate() {
            let t = rec.track.clone();
            let reason = rec.reason.clone();
            rows = rows.child(
                div()
                    .class("qd-row")
                    .child(div().class("qd-index").child(format!("{:02}", i + 1)))
                    .child(track_thumb(app, &t, "qd-art"))
                    .child(
                        div()
                            .class("qd-info")
                            .child(div().class("qd-title").child(t.title.clone()))
                            .child(div().class("qd-artist").child(t.artist.clone()))
                            .child(div().class("qd-reason").child(reason.clone())),
                    )
                    .child(div().class("qd-dur").child(t.duration_label()))
                    .on_click(cx.listener(move |app, _: &ClickEvent, _, cx| {
                        app.play_track(t.clone(), reason.clone());
                        app.queue_open = false;
                        cx.notify();
                    })),
            );
        }
    }
    let queue_count = app.queue.len();
    div()
        .class("queue-scrim")
        .on_click(cx.listener(|app, _: &ClickEvent, _, cx| {
            app.queue_open = false;
            cx.notify();
        }))
        .child(
            div()
                .class("queue-drawer")
                .id("queue-drawer")
                .on_click(cx.listener(|_, _: &ClickEvent, _, cx| cx.stop_propagation()))
                .child(
                    div()
                        .class("qd-head")
                        .child(
                            div()
                                .child(div().class("qd-kicker").child("THE GAYDAR'S NEXT"))
                                .child(div().class("qd-title-big").child("UP NEXT")),
                        )
                        .child(
                            div()
                                .class("qd-close")
                                .id("queue-close")
                                .child(icon("close", 13.0, "#cbbcd0"))
                                .on_click(cx.listener(|app, _: &ClickEvent, _, cx| {
                                    app.queue_open = false;
                                    cx.notify();
                                })),
                        ),
                )
                .child(
                    div()
                        .class("qd-count")
                        .child(format!("{queue_count} TRACKS · AUTO-DJ ON")),
                )
                .child(rows)
                .child(
                    div()
                        .class("qd-foot")
                        .child("The queue learns from every like, skip and replay."),
                ),
        )
        .into_any_element()
}

fn error_banner(app: &GaydarApp, cx: &mut Context<GaydarApp>) -> AnyElement {
    div()
        .class("error-banner")
        .child(div().class("error-tag").child("SIGNAL TROUBLE"))
        .child(
            div()
                .class("error-text")
                .child(app.error.clone().unwrap_or_default()),
        )
        .child(
            div()
                .class("error-dismiss")
                .child("DISMISS")
                .on_click(cx.listener(|app, _: &ClickEvent, _, cx| {
                    app.dismiss_error();
                    cx.notify();
                })),
        )
        .into_any_element()
}

fn toast(app: &GaydarApp) -> Option<AnyElement> {
    let t = app.toast.as_ref()?;
    if !t.alive() {
        return None;
    }
    let cls = match t.kind {
        ToastKind::Info => "toast",
        ToastKind::Good => "toast good",
        ToastKind::Bad => "toast bad",
    };
    Some(
        div()
            .class(cls)
            .child(div().class("toast-dot"))
            .child(t.text.clone())
            .into_any_element(),
    )
}

pub fn root(app: &mut GaydarApp, _window: &mut Window, cx: &mut Context<GaydarApp>) -> AnyElement {
    if app.booting && !app.demo {
        return html()
            .child(
                head()
                    .child(style().child(theme::CSS))
                    .child(style().child(EXTRA_CSS)),
            )
            .child(body().child(boot_screen(app, cx)))
            .into_any_element();
    }
    let mut app_shell = div().class("app").id("app-root").on_key_down(cx.listener(
        |app, e: &KeyDownEvent, window, cx| {
            if window.focused_id() == Some("search-field") {
                return;
            }
            let key = e.keystroke.key.as_str();
            match key {
                "space" | " " => app.toggle_play(),
                "Right" | "RightArrow" => app.next_track(),
                "Left" | "LeftArrow" => app.prev_track(),
                "q" | "Q" => {
                    if app.screen != Screen::NowPlaying {
                        app.queue_open = !app.queue_open;
                    }
                }
                "l" | "L" => {
                    if let Some(track) = app.current.clone() {
                        app.toggle_like(&track);
                    }
                }
                "Escape" => {
                    if app.queue_open {
                        app.queue_open = false;
                    } else if app.screen == Screen::NowPlaying {
                        app.toggle_now_playing();
                    } else if app.error.is_some() {
                        app.dismiss_error();
                    }
                }
                "slash" | "/" => {
                    app.goto(Screen::Search);
                }
                _ => {}
            }
            cx.notify();
        },
    ));
    app_shell = app_shell.child(div().class("pride-bar"));
    let page = if app.screen == Screen::NowPlaying {
        now_playing(app, cx)
    } else {
        let mut main = div().class("main").child(topbar(app, cx));
        if app.error.is_some() {
            main = main.child(error_banner(app, cx));
        }
        main = main.child(match app.screen {
            Screen::Home => home(app, cx),
            Screen::Gaydar => gaydar_screen(app, cx),
            Screen::Search => search_screen(app, cx),
            Screen::Liked => liked_screen(app, cx, false),
            Screen::History => liked_screen(app, cx, true),
            Screen::Settings => settings_screen(app, cx),
            Screen::NowPlaying => unreachable!(),
        });
        div()
            .class("shell")
            .child(rail(app, cx))
            .child(main)
            .into_any_element()
    };
    if app.screen == Screen::NowPlaying {
        app_shell = app_shell.child(page);
    } else {
        app_shell = app_shell.child(page).child(player(app, cx));
        if app.queue_open {
            app_shell = app_shell.child(queue_drawer(app, cx));
        }
    }
    app_shell = app_shell.child(div().class("grain-layer").style(format!(
        "background-image:url('{}');",
        theme::asset("patterns/grain-256.png")
    )));
    if let Some(t) = toast(app) {
        app_shell = app_shell.child(t);
    }
    html()
        .child(
            head()
                .child(style().child(theme::CSS))
                .child(style().child(EXTRA_CSS)),
        )
        .child(body().child(app_shell))
        .into_any_element()
}

impl Render for GaydarApp {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        self.pump(cx);
        if self.screen == Screen::Gaydar || self.booting {
            self.radar_phase = (self.radar_phase + 0.022) % std::f32::consts::TAU;
            window.request_animation_frame();
        }
        if self.playback.state == PlaybackState::Playing
            || self.playback.state == PlaybackState::Loading
            || self.booting
            || self.loading_track
            || self.toast.as_ref().is_some_and(|t| t.alive())
        {
            window.request_animation_frame();
        }
        if self.toast.as_ref().is_some_and(|t| !t.alive()) {
            self.toast = None;
        }
        root(self, window, cx)
    }
}
