//! Secondary screens: Gaydar, search, library, settings, now playing and boot.

use crate::app::{GaydarApp, ToastKind, demo_tracks};
use crate::audio::PlaybackState;
use crate::theme;
use crate::ui::{fallback_cover, icon, track_row, volume_bars, wave_seek};
use blenderui::prelude::*;

// ---------------------------------------------------------------------------
// A real canvas radar (no animated conic gradients: BUI bands those layers)

fn radar_canvas(phase: f32, size_px: f32) -> impl IntoElement {
    canvas(move |b: Bounds<f32>, p: &mut Painter| {
        let side = b.size.width.min(b.size.height);
        let cx = b.origin.x + b.size.width / 2.0;
        let cy = b.origin.y + b.size.height / 2.0;
        let radius = side / 2.0 - 3.0;
        let circle = |r: f32| Bounds {
            origin: point(cx - r, cy - r),
            size: size(r * 2.0, r * 2.0),
        };
        for factor in [0.28f32, 0.53, 0.77, 1.0] {
            p.oval(
                circle(radius * factor),
                rgba(0x00000000),
                rgba(0xd6ff3f22),
                1.0,
            );
        }
        p.line(
            &[(cx - radius, cy), (cx + radius, cy)],
            rgba(0xd6ff3f12),
            1.0,
        );
        p.line(
            &[(cx, cy - radius), (cx, cy + radius)],
            rgba(0xd6ff3f12),
            1.0,
        );
        for i in 0..34 {
            let a = phase - i as f32 * 0.045;
            let alpha = ((1.0 - i as f32 / 34.0).powf(1.8) * 70.0) as u32;
            p.arc(b, a.to_degrees() - 6.0, 6.0, rgba(0xd6ff3f00 | alpha));
        }
        p.oval(
            circle(radius * 0.06),
            rgba(0xd6ff3f18),
            rgba(0x00000000),
            0.0,
        );
        let blips = [
            (0.34f32, 0.28f32, 0xff4fa3u32),
            (0.67, 0.18, 0xd6ff3f),
            (0.55, 0.52, 0xd6ff3f),
            (0.76, 0.63, 0xff4fa3),
            (0.28, 0.68, 0x7ef9ff),
            (0.19, 0.55, 0x7ef9ff),
            (0.82, 0.38, 0xd6ff3f),
        ];
        for (nx, ny, color) in blips {
            let x = b.origin.x + b.size.width * nx;
            let y = b.origin.y + b.size.height * ny;
            p.oval(
                Bounds {
                    origin: point(x - 7.0, y - 7.0),
                    size: size(14.0, 14.0),
                },
                rgba(color & 0xffffff00 | 0x1a),
                rgba(0x00000000),
                0.0,
            );
            p.oval(
                Bounds {
                    origin: point(x - 2.5, y - 2.5),
                    size: size(5.0, 5.0),
                },
                rgba(color & 0xffffff00 | 0xff),
                rgba(0x00000000),
                0.0,
            );
        }
    })
    .style(format!(
        "position:absolute;left:0;top:0;width:{size_px}px;height:{size_px}px;"
    ))
}

// ---------------------------------------------------------------------------
// The Gaydar

pub fn gaydar_screen(app: &GaydarApp, cx: &mut Context<GaydarApp>) -> AnyElement {
    let specs = [
        (
            "CAMP",
            app.dials.camp,
            "more drama, more feathers",
            "#ff4fa3",
        ),
        (
            "CLUB",
            app.dials.club,
            "floor-filling, sweat-friendly",
            "#ffcf5c",
        ),
        (
            "SOFT",
            app.dials.soft,
            "tender, sapphic, slightly damp",
            "#c3a6ff",
        ),
        (
            "CHAOS",
            app.dials.chaos,
            "unhinged in a good way",
            "#d6ff3f",
        ),
    ];
    let mut dials = div()
        .class("dials-panel")
        .child(div().class("dials-title").child("MOOD DIALS"))
        .child(div().class("dials-note").child("Turn the four knobs that decide what the gaydar serves next. It updates as you listen."));
    for (idx, (name, value, desc, color)) in specs.iter().enumerate() {
        let mut segs = div().class("dial-segs");
        for i in 0..18 {
            let on = (i as f32 + 1.0) / 18.0 <= *value;
            let seg_color = if on { *color } else { "rgba(255,255,255,.075)" };
            segs = segs.child(
                div()
                    .class("dial-seg")
                    .style(format!("background:{seg_color};"))
                    .on_click(cx.listener(move |app, _: &ClickEvent, _, cx| {
                        app.set_dial(idx, (i as f32 + 1.0) / 18.0);
                        cx.notify();
                    })),
            );
        }
        dials = dials.child(
            div()
                .class("dial")
                .child(
                    div()
                        .class("dial-top")
                        .child(div().class("dial-name").child(*name))
                        .child(
                            div()
                                .class("dial-value")
                                .child(format!("{:.0}", value * 100.0)),
                        ),
                )
                .child(div().class("dial-desc").child(*desc))
                .child(segs),
        );
    }
    dials =
        dials
            .child(
                div()
                    .class("dial-actions")
                    .child(div().class("mini-btn solid").child("SURPRISE ME").on_click(
                        cx.listener(|app, _: &ClickEvent, _, cx| {
                            app.set_dials_all(crate::model::Dials {
                                camp: 0.92,
                                club: 0.81,
                                soft: 0.44,
                                chaos: 0.73,
                            });
                            app.notify("new dials, new you", ToastKind::Good);
                            cx.notify();
                        }),
                    ))
                    .child(
                        div()
                            .class("mini-btn")
                            .child("CAMP IT UP")
                            .on_click(cx.listener(|app, _: &ClickEvent, _, cx| {
                                app.set_dials_all(crate::model::Dials {
                                    camp: 1.0,
                                    club: 0.62,
                                    soft: 0.2,
                                    chaos: 0.5,
                                });
                                cx.notify();
                            })),
                    ),
            )
            .child(
                div()
                    .class("dial-whisper")
                    .child(div().class("dial-whisper-kicker").child("CURRENT READ"))
                    .child(
                        div()
                            .class("dial-whisper-copy")
                            .child(if app.current.is_some() {
                                format!(
                                    "The girls are getting {} with a side of {}.",
                                    top_word(&app.taste, 0),
                                    top_word(&app.taste, 1)
                                )
                            } else {
                                "Nothing on the aux yet. Start the radio and let her cook."
                                    .to_string()
                            }),
                    ),
            );

    let mut taste_tags = div().class("taste-cloud");
    for (tag, weight) in app.taste.iter() {
        taste_tags = taste_tags.child(
            div()
                .class("taste-tag")
                .child(tag.clone())
                .child(format!(" {:.0}", weight * 100.0)),
        );
    }

    div()
        .class("content content-wide")
        .child(
            div()
                .class("screen-head")
                .child(
                    div()
                        .child(div().class("screen-title").child("THE GAYDAR"))
                        .child(div().class("screen-lede").child("An algorithm with a personality disorder, in the best way. It learns from replays, skips and the little heart button, then serves something queer enough to matter.")),
                )
                .child(div().class("chip on").child("LIVE · LEARNING")),
        )
        .child(
            div()
                .class("gaydar-layout")
                .child(
                    div()
                        .class("radar-card")
                        .child(
                            div()
                                .class("radar-head")
                                .child(div().class("radar-label").child("QUEER SIGNAL · 2.4 KM"))
                                .child(div().class("radar-live").child(div().class("live-dot")).child("SCANNING")),
                        )
                        .child(
                            div()
                                .class("radar-plot")
                                .child(radar_canvas(app.radar_phase, 300.0))
                                .child(div().class("radar-center").child(icon("heart-filled", 20.0, "#180913"))),
                        )
                        .child(
                            div()
                                .class("radar-note")
                                .child(div().child("◉ 7 QUEER SIGNALS IN RANGE"))
                                .child(div().child("MODEL: GAYDAR v2.6")),
                        ),
                )
                .child(dials),
        )
        .child(
            div()
                .class("gaydar-lower")
                .child(
                    div()
                        .class("reason-card")
                        .child(div().class("reason-eyebrow").child("WHY THIS TRACK?"))
                        .child(div().class("reason-text").child(app.current_reason.clone()))
                        .child(div().class("reason-track").child(format!(
                            "{} · {}",
                            app.current.as_ref().map(|t| t.title.clone()).unwrap_or_else(|| "nothing yet".into()),
                            app.current.as_ref().map(|t| t.artist.clone()).unwrap_or_default()
                        )))
                        .child(taste_tags),
                )
                .child(
                    div()
                        .class("stat-row")
                        .child(div().class("stat-card").child(div().class("stat-num").child(app.total_plays.to_string())).child(div().class("stat-label").child("PLAYS SERVED")))
                        .child(div().class("stat-card").child(div().class("stat-num").child(app.liked_tracks.len().to_string())).child(div().class("stat-label").child("IN THE CANON")))
                        .child(div().class("stat-card").child(div().class("stat-num artist").child(app.top_artists.first().cloned().unwrap_or_else(|| "—".into()))).child(div().class("stat-label").child("TOP ARTIST"))),
                ),
        )
        .into_any_element()
}

fn reading_bar(label: &str, value: f32, color: &str) -> Div {
    div()
        .class("np-read-item")
        .child(div().class("np-read-label").child(label.to_string()))
        .child(
            div()
                .class("np-read-track")
                .child(div().class("np-read-fill").style(format!(
                    "width:{:.0}%;background:{};",
                    value * 100.0,
                    color
                ))),
        )
        .child(
            div()
                .class("np-read-value")
                .child(format!("{:.0}", value * 100.0)),
        )
}

fn top_word(taste: &[(String, f32)], index: usize) -> String {
    taste
        .get(index)
        .map(|(tag, _)| tag.clone())
        .unwrap_or_else(|| "camp".into())
}

// ---------------------------------------------------------------------------
// Search

fn search_field(app: &GaydarApp, cx: &mut Context<GaydarApp>) -> Div {
    let text = if app.search_query.is_empty() {
        div()
            .class("fancy-search-placeholder")
            .child("Try “sapphic hyperpop” or “ballroom”…")
            .into_any_element()
    } else {
        div()
            .class("fancy-search-value")
            .child(app.search_query.clone())
            .into_any_element()
    };
    let clear = if app.search_query.is_empty() {
        div().into_any_element()
    } else {
        div()
            .class("fancy-search-clear")
            .child(icon("close", 12.0, "#8b7a92"))
            .on_click(cx.listener(|app, _: &ClickEvent, _, cx| {
                app.search_clear();
                cx.notify();
            }))
            .into_any_element()
    };
    div()
        .class("fancy-search")
        .id("search-field")
        .child(icon("search", 16.0, "#ff4fa3"))
        .child(text)
        .child(div().class("search-caret"))
        .child(clear)
        .on_click(cx.listener(|_app, _: &ClickEvent, window, _cx| {
            window.focus_id("search-field");
        }))
        .on_key_down(cx.listener(
            |app, e: &KeyDownEvent, _, cx| match e.keystroke.key.as_str() {
                "Backspace" => {
                    app.search_backspace();
                    cx.notify();
                }
                "Return" | "KP_Enter" => {
                    app.submit_search();
                    cx.notify();
                }
                "Escape" => {
                    app.search_clear();
                    cx.notify();
                }
                _ => {
                    if !e.keystroke.modifiers.control && !e.keystroke.modifiers.alt {
                        if let Some(ch) = &e.keystroke.key_char {
                            if !ch.is_empty() && !ch.chars().any(|c| c.is_control()) {
                                app.search_push(ch);
                                cx.notify();
                            }
                        }
                    }
                }
            },
        ))
}

pub fn search_screen(app: &GaydarApp, cx: &mut Context<GaydarApp>) -> AnyElement {
    let chips = [
        "hyperpop",
        "ballroom",
        "sapphic",
        "trans joy",
        "drag",
        "queercore",
        "camp",
        "pride",
    ];
    let mut chip_row = div().class("chip-row");
    for chip in chips {
        let q = chip.to_string();
        chip_row = chip_row.child(div().class("chip").child(chip).on_click(cx.listener(
            move |app, _: &ClickEvent, _, cx| {
                app.set_search_query(q.clone());
                app.submit_search();
                cx.notify();
            },
        )));
    }
    let mut results = div();
    if app.searching {
        results = results.child(
            div()
                .class("searching")
                .child(div().class("search-pulse"))
                .child(
                    div()
                        .class("searching-text")
                        .child("digging through the queer underground…"),
                ),
        );
    } else if let Some(error) = &app.search_error {
        results = results.child(
            div()
                .class("empty-state")
                .child(div().class("empty-kicker").child("THE SIGNAL DROPPED"))
                .child(
                    div()
                        .class("empty-title")
                        .child("Search is being dramatic."),
                )
                .child(div().class("empty-note").child(error.clone())),
        );
    } else if app.search_results.is_empty() {
        results = results.child(
            div()
                .class("empty-state")
                .child(div().class("empty-kicker").child("NOTHING BUT DUST"))
                .child(div().class("empty-title").child("The crate is empty, henny."))
                .child(div().class("empty-note").child("Try a different mood. The gaydar suggests hyperpop, ballroom, or full-blown diva.")),
        );
    } else {
        let mut list = div().class("tracklist");
        for (i, t) in app.search_results.iter().enumerate() {
            list = list.child(track_row(
                app,
                t,
                i,
                Some(t.genre.clone().unwrap_or_else(|| "search result".into())),
                cx,
            ));
        }
        results = results.child(list);
    }

    div()
        .class("content content-wide")
        .child(
            div()
                .class("search-hero")
                .child(div().class("screen-title").child("DIG THROUGH THE CRATE"))
                .child(div().class("screen-lede").child("SoundCloud's public catalogue, no account required. Search a feeling, an artist, or a very specific era of gay."))
                .child(
                    div()
                        .class("search-big")
                        .child(search_field(app, cx))
                        .child(
                            div()
                                .class("search-btn")
                                .child("SEARCH")
                                .on_click(cx.listener(|app, _: &ClickEvent, _, cx| {
                                    app.submit_search();
                                    cx.notify();
                                })),
                        ),
                )
                .child(div().class("search-hint").child("No login. No follow. No algorithmic middleman with a diversity hire."))
                .child(chip_row),
        )
        .child(
            div()
                .class("search-results-head")
                .child(div().class("section-title").child("RESULTS"))
                .child(div().class("result-count").child(format!("{} TRACKS · SOUNDCLOUD", app.search_results.len()))),
        )
        .child(results)
        .into_any_element()
}

// ---------------------------------------------------------------------------
// Liked / History

pub fn liked_screen(app: &GaydarApp, cx: &mut Context<GaydarApp>, history: bool) -> AnyElement {
    let tracks = if history {
        app.history.clone()
    } else {
        app.liked_tracks.clone()
    };
    let title = if history {
        "ALREADY SERVED"
    } else {
        "THE CANON"
    };
    let serif = if history {
        "look back, but make it fashion."
    } else {
        "the tracks that made you gay(er)."
    };
    let mut list = div().class("tracklist");
    for (i, t) in tracks.iter().enumerate() {
        list = list.child(track_row(app, t, i, None, cx));
    }
    let body = if tracks.is_empty() {
        div()
            .class("empty-state")
            .child(
                div()
                    .child(div().class("empty-kicker").child(if history { "A FRESH START" } else { "THE CANON IS EMPTY" }))
                    .child(div().class("empty-title").child(if history { "No history. Suspicious." } else { "No likes yet, virgin." }))
                    .child(div().class("empty-note").child(if history {
                        "Play something. The gaydar remembers everything, including that one questionable remix."
                    } else {
                        "Hit the little heart on something you love. We'll keep it sacred."
                    })),
            )
            .into_any_element()
    } else {
        list.into_any_element()
    };
    div()
        .class("content content-wide")
        .child(
            div()
                .class("screen-head")
                .child(
                    div()
                        .child(div().class("screen-title").child(title))
                        .child(div().class("screen-lede").child(serif)),
                )
                .child(
                    div()
                        .class("chip on")
                        .child(format!("{} TRACKS", tracks.len())),
                ),
        )
        .child(body)
        .into_any_element()
}

// ---------------------------------------------------------------------------
// Settings

pub fn settings_screen(app: &GaydarApp, cx: &mut Context<GaydarApp>) -> AnyElement {
    let mut devices = div().class("device-list");
    if app.devices.is_empty() {
        devices = devices.child(
            div()
                .class("setting-note")
                .child("No ALSA devices found yet. Plug something in, then refresh."),
        );
    } else {
        for d in app.devices.iter() {
            let selected = app.device == d.name;
            let name = d.name.clone();
            devices = devices.child(
                div()
                    .class(if selected {
                        "device-row on"
                    } else {
                        "device-row"
                    })
                    .child(div().class("device-dot"))
                    .child(div().class("device-name").child(name.clone()))
                    .child(div().class("device-desc").child(d.description.clone()))
                    .on_click(cx.listener(move |app, _: &ClickEvent, _, cx| {
                        app.set_device(name.clone());
                        cx.notify();
                    })),
            );
        }
    }
    let volume_id = "settings-volume";
    div()
        .class("content content-wide")
        .child(
            div()
                .class("screen-head")
                .child(
                    div()
                        .child(div().class("screen-title").child("TECHNICAL, HENNY"))
                        .child(div().class("screen-lede").child("Native ALSA output, direct from Rust. Pick a device, set the volume, and let the girls sing through your speakers.")),
                )
                .child(div().class("chip on").child("ALSA · DIRECT")),
        )
        .child(
            div()
                .class("settings-grid")
                .child(
                    div()
                        .class("settings-main")
                        .child(
                            div()
                                .class("setting-card")
                                .child(
                                    div()
                                        .class("setting-head-row")
                                        .child(div().class("setting-title").child("OUTPUT DEVICE"))
                                        .child(
                                            div()
                                                .class("setting-action")
                                                .child("REFRESH")
                                                .on_click(cx.listener(|app, _: &ClickEvent, _, cx| {
                                                    app.refresh_devices();
                                                    cx.notify();
                                                })),
                                        ),
                                )
                                .child(div().class("setting-note").child("These are real ALSA PCM hints, not a fake dropdown. The player writes s16le stereo straight to the selected device."))
                                .child(devices),
                        )
                        .child(
                            div()
                                .class("setting-card")
                                .child(div().class("setting-title").child("VOLUME"))
                                .child(div().class("setting-note").child("Software gain applied to the PCM stream before it hits ALSA. Turn it up, mother."))
                                .child(
                                    div()
                                        .class("volume-demo")
                                        .child(div().class("time").child(format!("{:.0}", app.volume * 100.0)))
                                        .child(volume_bars(app, volume_id, 18, cx))
                                        .child(icon("volume", 14.0, "#ff4fa3")),
                                ),
                        )
                        .child(
                            div()
                                .class("setting-card")
                                .child(div().class("setting-title").child("PLAYBACK STATE"))
                                .child(div().class("kv").child(span().child("backend")).child(span().child("ALSA · snd_pcm")))
                                .child(div().class("kv").child(span().child("state")).child(span().child(crate::app::status_label(app.playback.state))))
                                .child(div().class("kv").child(span().child("format")).child(span().child(format!("{} Hz · {} ch · s16le", app.playback.sample_rate, app.playback.channels))))
                                .child(div().class("kv").child(span().child("device")).child(span().child(app.device.clone())))
                                .child(div().class("kv").child(span().child("position")).child(span().child(format!("{} / {}", app.position_label(), app.duration_label())))),
                        ),
                )
                .child(
                    div()
                        .class("settings-side")
                        .child(
                            div()
                                .class("setting-card")
                                .child(div().class("setting-title").child("THE FINE PRINT"))
                                .child(div().class("about-line").child("GAYDAR FM is an ").child(b().child("unofficial")).child(" SoundCloud client. It streams public tracks through SoundCloud's own public web API. No account, no OAuth, no closet."))
                                .child(div().class("about-line").style("margin-top:9px;").child("Your likes, history and taste profile live on this machine only. The algorithm is local; the music is SoundCloud's."))
                                .child(div().class("about-line").style("margin-top:9px;").child("Built with Rust + BUI. Audio via ALSA. Artwork via SoundCloud.")),
                        )
                        .child(
                            div()
                                .class("setting-card")
                                .child(div().class("setting-title").child("SHORTCUTS"))
                                .child(div().class("kv").child(span().child("play / pause")).child(span().child("space")))
                                .child(div().class("kv").child(span().child("next")).child(span().child("→")))
                                .child(div().class("kv").child(span().child("previous")).child(span().child("←")))
                                .child(div().class("kv").child(span().child("search")).child(span().child("/"))),
                        ),
                ),
        )
        .into_any_element()
}

// ---------------------------------------------------------------------------
// Immersive Now Playing

pub fn now_playing(app: &GaydarApp, cx: &mut Context<GaydarApp>) -> AnyElement {
    let track = app
        .current
        .clone()
        .unwrap_or_else(|| demo_tracks()[0].clone());
    let playing = app.playback.state == PlaybackState::Playing;
    let liked = app.is_liked(track.id);
    let reason = if app.current_reason.is_empty() {
        "the gaydar is feeling something".to_string()
    } else {
        app.current_reason.clone()
    };
    let mut tags = div().class("np-tags");
    for tag in track.tags.iter().take(4) {
        tags = tags.child(div().class("np-tag").child(tag.clone()));
    }
    let mut queue = div().class("np-queue-list");
    for rec in app.queue.iter().take(7) {
        let t = rec.track.clone();
        queue = queue.child(
            div()
                .class("np-queue-row")
                .child(crate::ui::track_thumb(app, &t, "q-art"))
                .child(
                    div()
                        .class("q-info")
                        .child(div().class("q-title").child(t.title.clone()))
                        .child(div().class("q-artist").child(t.artist.clone())),
                )
                .child(div().class("q-dur").child(t.duration_label()))
                .on_click(cx.listener(move |app, _: &ClickEvent, _, cx| {
                    app.play_track(t.clone(), "picked from the queue, bossy");
                    cx.notify();
                })),
        );
    }
    let art = app
        .art_path(&track)
        .unwrap_or_else(|| fallback_cover(&track));
    let art_inner = div()
        .style(format!("width:100%;height:100%;background-image:url('{art}');background-size:cover;background-position:center;"))
        .into_any_element();
    let mut eq = div().class("np-viz");
    for i in 0..48 {
        let level = app.playback.levels[(i * 32 / 48).min(31)].max(0.06);
        let color = match i % 5 {
            0 => "#ff4fa3",
            1 => "#ffcf5c",
            2 => "#d6ff3f",
            3 => "#7ef9ff",
            _ => "#c3a6ff",
        };
        eq = eq.child(div().style(format!(
            "flex:1;height:{:.0}px;border-radius:2px;background:{};opacity:.9;",
            4.0 + level * 42.0,
            color
        )));
    }
    div()
        .class("np")
        .child(div().class("np-bg"))
        .child(
            div()
                .class("np-top")
                .child(
                    div()
                        .class("np-back")
                        .child("← BACK TO THE AGENDA")
                        .on_click(cx.listener(|app, _: &ClickEvent, _, cx| {
                            app.toggle_now_playing();
                            cx.notify();
                        })),
                )
                .child(
                    img()
                        .src(theme::asset("brand/wordmark.svg"))
                        .style("width:118px;height:30px;display:block;object-fit:contain;"),
                )
                .child(
                    div()
                        .class("np-top-right")
                        .child("NO LOGIN · NO CLOSET · ALL SERVING"),
                ),
        )
        .child(
            div()
                .class("np-body")
                .child(
                    div().class("np-left").child(
                        div()
                            .class("np-art-wrap")
                            .child(div().class("np-vinyl"))
                            .child(div().class("np-art").child(art_inner)),
                    ),
                )
                .child(
                    div()
                        .class("np-info")
                        .child(
                            div()
                                .class("np-eyebrow")
                                .child("NOW SERVING")
                                .child(div().style("width:8px;"))
                                .child(div().class("np-cert").child("GAYDAR CERTIFIED")),
                        )
                        .child(div().class("np-big").child(track.title.clone()))
                        .child(div().class("np-serif").child(reason))
                        .child(div().class("np-track-artist").child(format!(
                            "{} · {}",
                            track.artist,
                            track.genre.clone().unwrap_or_else(|| "SoundCloud".into())
                        )))
                        .child(
                            div()
                                .class("np-actions")
                                .child(
                                    div()
                                        .class(if liked { "np-heart on" } else { "np-heart" })
                                        .child(icon(
                                            if liked { "heart-filled" } else { "heart" },
                                            15.0,
                                            if liked { "#ff4fa3" } else { "#b39fba" },
                                        ))
                                        .on_click(cx.listener(
                                            move |app, _: &ClickEvent, _, cx| {
                                                app.toggle_like(&track);
                                                cx.notify();
                                            },
                                        )),
                                )
                                .child(
                                    div()
                                        .class("np-play")
                                        .child(icon(
                                            if playing { "pause" } else { "play" },
                                            18.0,
                                            "#1a0913",
                                        ))
                                        .on_click(cx.listener(|app, _: &ClickEvent, _, cx| {
                                            app.toggle_play();
                                            cx.notify();
                                        })),
                                )
                                .child(
                                    div()
                                        .class("np-heart")
                                        .child(icon("next", 15.0, "#b39fba"))
                                        .on_click(cx.listener(|app, _: &ClickEvent, _, cx| {
                                            app.next_track();
                                            cx.notify();
                                        })),
                                ),
                        )
                        .child(
                            div()
                                .class("np-seek")
                                .child(div().class("time").child(app.position_label()))
                                .child(wave_seek(app, "np-seek-bar", 56, cx))
                                .child(div().class("time right").child(app.duration_label())),
                        )
                        .child(eq)
                        .child(
                            div()
                                .class("np-reading")
                                .child(div().class("np-reading-title").child("THE READING"))
                                .child(
                                    div()
                                        .class("np-reading-grid")
                                        .child(reading_bar("CAMP", app.dials.camp, "#ff4fa3"))
                                        .child(reading_bar("CLUB", app.dials.club, "#ffcf5c"))
                                        .child(reading_bar("SOFT", app.dials.soft, "#c3a6ff"))
                                        .child(reading_bar("CHAOS", app.dials.chaos, "#d6ff3f")),
                                ),
                        ),
                )
                .child(
                    div()
                        .class("np-queue")
                        .child(div().class("np-queue-title").child("UP NEXT"))
                        .child(div().class("np-queue-sub").child("CURATED BY YOUR GAYDAR"))
                        .child(queue),
                ),
        )
        .child(
            div().class("np-foot").child(tags).child(
                div()
                    .class("np-foot-right")
                    .child(div().class("np-vol-label").child("VOL"))
                    .child(volume_bars(app, "np-volume", 12, cx))
                    .child(div().class("np-top-right").child("ALSA · DIRECT OUT")),
            ),
        )
        .into_any_element()
}

// ---------------------------------------------------------------------------
// Boot

pub fn boot_screen(app: &GaydarApp, cx: &mut Context<GaydarApp>) -> AnyElement {
    let failed = app.error.clone();
    let loading = failed.is_none();
    let title = if loading {
        "Tuning the gaydar"
    } else {
        "The signal dropped."
    };
    let serif = if loading {
        "hold still, gorgeous."
    } else {
        "don't panic, she's dramatic."
    };
    let copy = if loading {
        "No login. No closet. Finding something queer enough to start the night.".to_string()
    } else {
        format!(
            "SoundCloud wouldn't give up the aux. {}",
            failed.unwrap_or_else(|| "Try again in a second.".into())
        )
    };
    let mut center = div()
        .class("boot-center")
        .child(
            div()
                .class("boot-brand")
                .child(
                    img()
                        .src(theme::asset("brand/app-mark.svg"))
                        .style("width:50px;height:50px;display:block;"),
                )
                .child(
                    img()
                        .src(theme::asset("brand/wordmark.svg"))
                        .style("width:196px;height:49px;display:block;object-fit:contain;"),
                ),
        )
        .child(div().class("boot-kicker").child(if loading {
            "SOUNDCLOUD · NO LOGIN · NO CLOSET"
        } else {
            "CONNECTION GAGGED"
        }))
        .child(div().class("boot-big").child(title))
        .child(div().class("boot-serif").child(serif))
        .child(div().class("boot-copy").child(copy))
        .child(div().class("boot-bar").child(div().class(if loading {
            "boot-bar-fill"
        } else {
            "boot-bar-fill failed"
        })))
        .child(
            div()
                .class("boot-steps")
                .child(div().class("boot-step on").child("01 CLIENT"))
                .child(
                    div()
                        .class(if loading {
                            "boot-step dim"
                        } else {
                            "boot-step bad"
                        })
                        .child("02 STREAM"),
                )
                .child(div().class("boot-step dim").child("03 SERVING")),
        );
    if !loading {
        center = center.child(
            div()
                .class("boot-retry")
                .child("TRY THE GAYDAR AGAIN")
                .on_click(cx.listener(|app, _: &ClickEvent, _, cx| {
                    app.retry_boot();
                    cx.notify();
                })),
        );
    }
    div()
        .class("boot")
        .child(
            div()
                .class("boot-orbit")
                .child(radar_canvas(app.radar_phase, 560.0))
                .child(div().class("radar-center boot-center-heart").child(icon(
                    "heart-filled",
                    22.0,
                    "#180913",
                ))),
        )
        .child(center)
        .child(
            div()
                .class("boot-foot")
                .child("RUST + BUI · ALSA DIRECT · YOUR TASTE STAYS LOCAL"),
        )
        .into_any_element()
}
