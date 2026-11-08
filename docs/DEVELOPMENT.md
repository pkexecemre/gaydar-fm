# GAYDAR FM — Development

This is the working guide for changing GAYDAR FM without breaking the things that make it work: a non-blocking UI, a playable-only SoundCloud path, and an audio engine that owns ALSA on its own thread.

Read [ARCHITECTURE.md](ARCHITECTURE.md) for the design and [TROUBLESHOOTING.md](TROUBLESHOOTING.md) for symptoms. Brand and copy rules live in [BRAND.md](BRAND.md) and [COPY.md](COPY.md); BUI-specific findings live in [BUI-NOTES.md](BUI-NOTES.md).

---

## 1. Repository map

| Path | Responsibility |
| --- | --- |
| [src/main.rs](../src/main.rs) | CLI parsing, BUI application, demo/live selection, headless PNG writer. |
| [src/app.rs](../src/app.rs) | <code>GaydarApp</code>, screens, demo data, app actions, settings persistence, audio/radio orchestration. |
| [src/ui.rs](../src/ui.rs) | App shell, rail, top bar, Home, player, queue drawer, toasts. |
| [src/views.rs](../src/views.rs) | Gaydar, Search, Liked/History, Settings, Now Serving, Boot. |
| [src/theme.rs](../src/theme.rs) | Palette constants, asset lookup, font loading, main stylesheet. |
| [src/extra.rs](../src/extra.rs) | Additional CSS for the secondary screens, player, Now Playing, boot, radar. |
| [src/model.rs](../src/model.rs) | <code>Track</code>, <code>Dials</code>, <code>StreamSource</code>, <code>Recommendation</code>. |
| [src/soundcloud.rs](../src/soundcloud.rs) | Public API client, client ID discovery/cache/refresh, normalisation, MP3 filtering, stream resolution, artwork. |
| [src/gaydar.rs](../src/gaydar.rs) | Recommendation engine, candidate pool, scoring, reasons, learning, state file. |
| [src/radio.rs](../src/radio.rs) | Radio worker thread, command/event bridge, background fetch/resolve. |
| [src/audio.rs](../src/audio.rs) | ALSA engine thread, ffmpeg process, PCM output, seek/pause/device switch, level meter. |
| [tests/ui_interactions.rs](../tests/ui_interactions.rs) | Headless BUI interaction test. |
| [tests/live_soundcloud.rs](../tests/live_soundcloud.rs) | Opt-in live network/stream/engine tests. |
| [tests/live_app_headless.rs](../tests/live_app_headless.rs) | Opt-in live end-to-end app test: playback, seek, next, search, station, natural finish. |
| [tests/live_app_e2e.rs](../tests/live_app_e2e.rs) | Broader ignored app scenarios (controls, settings, restart, null device). |
| [tests/algorithm_audit.rs](../tests/algorithm_audit.rs) | Ignored live audit harness for catalogue, dials, stations and feedback. |
| [examples/now_playing.rs](../examples/now_playing.rs) | BUI visual prototype; not the production screen. |
| [assets/](../assets) | Fonts, icons, brand marks, stickers, covers, patterns, seed reference data. |
| [packaging/](../packaging) | Installer, uninstaller, freedesktop entry, icons, packaging README. |
| [docs/](../docs) | Brand, copy, BUI notes, architecture, troubleshooting, this file. |

The UI is built from BUI elements and CSS classes; there is no HTML file, no JavaScript, and no web server. `GaydarApp` implements `Render`; the tree is rebuilt from state on each paint.

---

## 2. Setup

Requirements and distro package names are in the [README](../README.md#requirements). In short: Linux, Rust 1.88+, CMake, a C/C++ compiler, <code>pkg-config</code>, ALSA development headers, X11 development headers, and <code>ffmpeg</code> on <code>PATH</code> at runtime.

~~~bash
cargo build --release
cargo run --release
~~~

The first build compiles BUI's C kit via CMake. The binary is <code>target/release/gaydar</code>. Run from the repository root so <code>assets/</code> resolves, or set <code>GAYDAR_ASSETS_DIR</code>.

For a desktop install, use the packaging layer (it builds first unless <code>--no-build</code>):

~~~bash
./packaging/install.sh                         # $HOME/.local
./packaging/install.sh --prefix=/opt/gaydar
DESTDIR=/tmp/stage ./packaging/install.sh --prefix=/usr
./packaging/uninstall.sh --dry-run
~~~

The installer writes a <code>bin/gaydar</code> launcher (sets <code>GAYDAR_ASSETS_DIR</code>), <code>bin/gaydar.bin</code>, the asset tree, the desktop entry and icons. It never calls <code>sudo</code> and does not touch the XDG data/cache directories. Details: [packaging/README.md](../packaging/README.md).

For a scratch profile that cannot pollute your real likes/history:

~~~bash
mkdir -p /tmp/gaydar-dev/data /tmp/gaydar-dev/cache
GAYDAR_DATA_DIR=/tmp/gaydar-dev/data \
GAYDAR_CACHE_DIR=/tmp/gaydar-dev/cache \
cargo run --release
~~~

With those overrides, <code>state.json</code>, <code>settings.json</code>, <code>client_id.json</code>, and artwork all live under <code>/tmp/gaydar-dev</code>. Delete the directory for a clean slate.

---

## 3. Run modes

| Mode | Command | Network | Audio | Display |
| --- | --- | --- | --- | --- |
| Live app | <code>cargo run --release</code> | Yes | Yes | Yes |
| Demo window | <code>cargo run --release -- --demo --screen gaydar</code> | No | No | Yes |
| Headless screenshot | <code>cargo run --release -- --screenshot /tmp/home.png --screen home</code> | No | No | No |
| Headless live screenshot | <code>GAYDAR_ALSA_DEVICE=null cargo run --release -- --live-shot /tmp/live.png --screen now</code> | Yes | Yes | No |

The demo screenshot path uses `Application::headless()`, renders one frame, and writes a PNG from `GaydarApp::demo(screen)`, so the content is fixed. `--live-shot` uses the same headless window but a live `GaydarApp`, waits up to 45 s for playback, and optionally captures Now Serving with `--screen now`. Available screen names: <code>home</code>, <code>gaydar</code>, <code>search</code>, <code>liked</code>, <code>history</code>, <code>settings</code>, <code>now</code> (or <code>nowplaying</code>), <code>boot</code>. Unknown names fall back to Home.

`--queue` opens the queue drawer at launch and combines with a screenshot or with `--demo`; in a live launch it opens once the window exists. `--demo` still opens a window and therefore needs a display, but it never creates the radio or audio services. It is the right mode for layout work without hitting SoundCloud.

Live window size can be set with <code>--width</code>/<code>--height</code> too; the declared minimum is 1120×720. Screenshot dimensions are independent of that minimum.

---

## 4. Tests

### 4.1 Unit tests

~~~bash
cargo test --lib
~~~

Four tests, all in `soundcloud.rs`:

- quoted/comma/space tag-list parsing;
- artwork URL normalisation;
- MP3/HLS/DRM/snipped transcoding classification;
- image magic validation.

There are no unit tests for the recommender, audio engine, or radio worker. That is a current gap, not a design statement.

### 4.2 Headless UI test

~~~bash
cargo test --test ui_interactions
~~~

It opens a headless 1440×900 window, clicks <code>search-field</code>, types <code>ballroom</code>, presses Return, navigates Home, toggles play/pause, opens/closes the queue with <code>queue-toggle</code>/<code>queue-close</code>, verifies the global Space shortcut, clicks a waveform segment (<code>seek-bar-24</code>), applies <code>station-ballroom</code>, and opens <code>player-art</code>. If you rename those ids, update the test.

### 4.3 Live SoundCloud tests

~~~bash
cargo test --test live_soundcloud -- --nocapture
~~~

This target needs internet access and valid SoundCloud responses. It covers:

- client ID discovery and validation;
- cached client ID startup (second construction under 2 s);
- search, MP3 support, stream resolution, a real media range download, and artwork;
- a DRM-only track failing fast with a typed <code>StreamUnavailable</code>;
- engine bootstrap, artist diversity, first-pick verification, taste changes, likes/history/skips, state round-trip, corrupted-state recovery, and refill exclusions.

One test is ignored:

~~~bash
cargo test --test live_soundcloud -- --ignored live_client_id_refresh_on_401 --nocapture
~~~

It forces an invalid client ID and verifies the 401 refresh path. Run it when changing discovery, caching, or request auth.

### 4.4 Live headless end-to-end

~~~bash
E2E_DIR=$(mktemp -d)
GAYDAR_ALSA_DEVICE=null \
GAYDAR_DATA_DIR="$E2E_DIR/data" \
GAYDAR_CACHE_DIR="$E2E_DIR/cache" \
  cargo test --test live_app_headless -- --ignored --nocapture
~~~

It boots the real app with the real SoundCloud client, plays a resolved track through ALSA's `null` device, verifies the position advances, seeks to 50%, advances to the next track, likes it, searches, starts a station, seeks near the end, and verifies the natural finish advances the queue and writes `state.json`. It needs network, ffmpeg, ALSA and about half a minute. The sibling `tests/live_app_e2e.rs` target contains broader ignored scenarios for controls, settings, restart and the null device; read its header before running it.

### 4.5 Full test target

<code>cargo test</code> runs every non-ignored target: the unit tests, the UI test, the live SoundCloud tests, the non-ignored ALSA null check, and the other integration targets. The expensive live app scenarios are <code>#[ignore]</code>d. Because the live target needs the network, this is not a good default on a train. Use the three commands above deliberately.

### 4.6 Verified status

On 2026-11-08, on this checkout:

| Check | Result |
| --- | --- |
| <code>cargo build --release</code> | OK |
| <code>cargo test --lib</code> | 4 passed |
| <code>cargo test --test ui_interactions</code> | 1 passed |
| <code>cargo test --test live_soundcloud -- --nocapture</code> | 5 passed, 1 ignored |
| Ignored client ID rotation test | 1 passed when run explicitly |
| Ignored live headless E2E | 1 passed when run explicitly |
| Live screenshot (`--live-shot --screen now`) | 1 PNG, `playing: true` |
| Eight demo screenshot runs | 8 PNGs, 1440×900 |
| Packaging install / uninstall dry-run | staged install OK, dry-run lists the same files |

There is no CI configuration. If you add one, keep live tests out of the required path and make the headless build/test job deterministic.

---

## 5. Screenshot workflow

The current gallery uses the numbered 1440×900 and 1280×800 captures below; regenerate it intentionally with the exact names:

~~~bash
mkdir -p docs/gallery
cargo run --release -- --screenshot docs/gallery/01-home-1440x900.png --screen home
cargo run --release -- --screenshot docs/gallery/02-gaydar-1440x900.png --screen gaydar
cargo run --release -- --screenshot docs/gallery/03-search-1440x900.png --screen search
cargo run --release -- --screenshot docs/gallery/04-liked-1440x900.png --screen liked
cargo run --release -- --screenshot docs/gallery/05-history-1440x900.png --screen history
cargo run --release -- --screenshot docs/gallery/06-settings-1440x900.png --screen settings
cargo run --release -- --screenshot docs/gallery/07-now-playing-1440x900.png --screen now
cargo run --release -- --screenshot docs/gallery/08-boot-1440x900.png --screen boot
cargo run --release -- --screenshot docs/gallery/09-home-1280x800.png --screen home --width 1280 --height 800
cargo run --release -- --screenshot docs/gallery/10-now-playing-1280x800.png --screen now --width 1280 --height 800
# Optional check, outside the gallery: the queue drawer over Home
cargo run --release -- --screenshot /tmp/gaydar-queue.png --screen home --queue
~~~

That writes inside the repository. Only do it when the UI change is meant to update the gallery, and review the PNGs before committing. For a quick look without touching the repo:

~~~bash
cargo run --release -- --screenshot /tmp/gaydar-home.png --screen home
~~~

The renderer is deterministic for demo data, but font availability and asset discovery still affect the output. Run from the repository root or set <code>GAYDAR_ASSETS_DIR</code>. Demo screenshot mode never proves network, ALSA, or ffmpeg behaviour.

For a live, network-backed capture (headless but real):

~~~bash
GAYDAR_ALSA_DEVICE=null cargo run --release -- --live-shot /tmp/gaydar-live.png --screen now
~~~

It waits up to 45 s for playback and up to 5 s more for artwork, then writes the frame. A non-playing frame still exits 0 and is labelled on stderr; it is not a playback success.

The full Now Playing prototype is [examples/now_playing.rs](../examples/now_playing.rs); its run/capture commands and BUI findings are documented in [BUI-NOTES.md](BUI-NOTES.md). It is a design reference, not the production view.

---

## 6. Code style and checks

The tree is **not currently rustfmt-clean**, and Clippy reports warnings. Do not pretend otherwise in a pull request.

- Format the files you touch:

  ~~~bash
  rustfmt --edition 2024 src/your_file.rs tests/your_test.rs
  ~~~

  A bare <code>cargo fmt</code> reformats the whole tree and will bury your diff. If you do run it, separate the formatting change from the behaviour change.

- Clippy is useful as advice, not as a gate:

  ~~~bash
  cargo clippy --all-targets
  ~~~

  The current tree has existing warnings; <code>-D warnings</code> will fail for reasons unrelated to your change.

- Avoid <code>unwrap()</code>/<code>expect()</code> on network, audio, or user data. <code>expect</code> is acceptable for invariants that truly cannot fail (for example, a regex built from escaped literals).
- Keep public state and methods on <code>GaydarApp</code> coherent; the UI and tests both read fields directly.
- New CSS belongs in <code>theme::CSS</code> or <code>extra::EXTRA_CSS</code>. Inline styles are already used for computed values (wave bars, gradients); use classes for everything static.
- The UI is rebuilt from state every frame. Do not put expensive work in <code>render</code>; use the worker, an audio command, or a cached value.
- Global keys are handled by the <code>app-root</code> element in `ui.rs`; the handler returns early when <code>search-field</code> owns focus. If you add a shortcut, add it there and update the README keyboard table and the Settings card if it belongs to the transport/search subset.

---

## 7. Invariants to preserve

These are the load-bearing rules. A change that breaks one is a regression even if it compiles.

1. **The UI thread never blocks on I/O.** No <code>reqwest</code>, no ALSA, no <code>ffmpeg</code>, no long sleeps in a click handler or render.
2. **Only MP3 transcodings reach the audio engine.** No AAC/ABR, no snipped previews, no DRM workarounds, no login. The filter is in `soundcloud.rs`; the queue must not bypass it. List results also pass <code>looks_like_music</code>, which drops obvious talk/podcast/interview/review uploads and a narrow slur blocklist.
3. **A media URL is verified before `ffmpeg` sees it.** Do not remove the range probe or the HLS key check.
4. **The audio thread owns ALSA and the ffmpeg child.** Do not create a second audio path, do not call ALSA from the UI, and do not add <code>rodio</code>/<code>cpal</code>.
5. **The radio worker owns the client and engine.** Do not call <code>SoundCloudClient</code> or <code>GaydarEngine</code> from a UI callback.
6. **State writes are atomic.** Temp file + rename; keep the <code>.bak</code> behaviour.
7. **The playable filter is not a popularity filter.** Recommendations may prefer queer/indie scenes, but they must still obey the duration/policy/MP3 rules.
8. **No telemetry.** There is no analytics, crash reporting, or update check; keep it that way unless the product decision changes explicitly.

---

## 8. Common changes

### 8.1 Add or change a screen

1. Add a variant to <code>Screen</code> in `app.rs` and fill in <code>label()</code> and <code>eyebrow()</code>.
2. Map the CLI name in <code>parse_screen</code> in `main.rs`. Unknown names deliberately fall back to Home.
3. Add the match arm in <code>root</code> in `ui.rs`. `NowPlaying` is special: it replaces the shell and player.
4. Implement the view in `views.rs` (or `ui.rs` for shell-level views), taking <code>&GaydarApp</code> and <code>&mut Context&lt;GaydarApp&gt;</code>.
5. Add a nav item if it is user-facing; give it a stable id (<code>nav-...</code>) and update the UI test if the navigation flow changes.
6. Add the screen to the screenshot table in the README and to the screenshot loop in this document if it should be in the gallery.
7. Check both demo and live states. Demo data lives in <code>demo()</code>, <code>demo_tracks()</code>, <code>demo_reasons()</code>, and <code>demo_levels()</code> in `app.rs`.

### 8.2 Add a station

<code>STATIONS</code> is a fixed array of four in `app.rs`. A station has an <code>id</code> (used as a CSS class and an HTML id), a display name, a volume tag, a blurb, a <code>query</code>, a <code>fallback_query</code>, and a <code>Dials</code> value.

- Add the CSS for <code>.station.&lt;id&gt;</code> in the stylesheet. Without it, the card has no background.
- Demo mode picks a demo track with a tag that occurs in the station query, or falls back to the first track. If you add a concept that demo tracks do not cover, add a matching demo track.
- Live mode applies the dials, then tries the primary and fallback queries at limit 50. For each query it maps up to 12 picks and resolves the first up-to-four for autoplay; it stops at the first query with a resolvable pick, so a query that returns only DRM-locked results falls through to the fallback. If both fail, the UI reports the station as quiet with the last reason.
- Keep the four-station layout in mind; the grid is <code>repeat(4, minmax(0, 1fr))</code>.

### 8.3 Change the recommender

The relevant code is in `gaydar.rs`:

- <code>score_candidate</code> — the numeric blend.
- <code>reason_for</code> — the explanation strings.
- <code>select</code> — sorting, artist cap, fallback fill.
- <code>record_play</code>/<code>record_complete</code>/<code>record_skip</code>/<code>record_like</code>/<code>record_unlike</code> — learning deltas.
- <code>bootstrap_queries</code>, <code>refill_jobs</code>, <code>seed_pool</code> — discovery.
- The seed constants at the top of the file.

If you change weights or thresholds, update [ARCHITECTURE.md](ARCHITECTURE.md#43-scoring) and the README's algorithm section. Then run the live engine test at least once; it verifies that bootstrap returns at least three tracks, that distinct artists appear, and that refill excludes seen/liked tracks. If you touch feedback handling, remember that complete/skip/like/unlike invalidate <code>last_refill</code>; play does not.

<code>assets/seeds.json</code> is **not read by the engine**. It is brand/reference data for seed queries, artist labels, and copy. Editing it does not change recommendations; editing the constants in `gaydar.rs` does.

### 8.4 Change audio

Read [ARCHITECTURE.md](ARCHITECTURE.md#5-the-audio-engine) first. <code>audio.rs</code> is self-contained:

- <code>AudioEngine</code> is the UI-facing handle (commands + status).
- <code>ThreadEngine</code> runs the loop and owns <code>Playback</code>.
- <code>spawn_ffmpeg</code> builds the child and the reader/stderr threads.
- <code>open_pcm_with_fallback</code> negotiates ALSA.
- <code>LevelMeter</code> computes the 32 display bands.

For an end-to-end audio path without the visible UI, use the live headless test in section 4.4 (or `GAYDAR_ALSA_DEVICE=null` for a silent run). There is no standalone audio-probe example in the current tree; `tests/live_app_headless.rs` is the supported end-to-end path.

When changing the audio path:

- Keep the API as <code>s16le</code>, stereo; the PCM code assumes 2 channels and 2 bytes per sample.
- Keep the bounded chunk channel; it is the backpressure mechanism.
- Kill and reap the child on every path (stop, seek, device switch, drop). A leaked <code>ffmpeg</code> is a bug.
- Test sample-rate fallback with a device that only supports 44.1 kHz if you can.
- Do not add a second decoder or a second ALSA writer.
- Do not claim bit-perfect output; software volume and ALSA plugins are in the path.

### 8.5 Change the SoundCloud client

`soundcloud.rs` owns all HTTP. If you add an endpoint:

- Use <code>api_get</code> so pacing, retries, client ID refresh, and error typing stay consistent.
- Add a normaliser to <code>Track</code> only if the field is needed; otherwise keep the raw JSON local.
- Respect the playable filter: <code>is_playable</code>, <code>looks_like_music</code>, and an MP3 transcoding check. List endpoints use <code>collection_tracks</code>; do not create a second path that skips them.
- Return typed <code>StreamUnavailable</code> for DRM-only/withdrawn tracks so the queue can drop them without retrying.
- Do not add account endpoints, OAuth, or private APIs.
- Test both a progressive MP3 and an HLS MP3 if you touch resolution. A DRM-only track must still fail fast.

Client ID changes are especially sensitive: run the ignored rotation test, and clear/observe the cache during manual testing. The ID is a public token but very ephemeral; do not commit one or add it to documentation.

### 8.6 Change persisted state

`gaydar.rs` has <code>STATE_VERSION</code>, <code>PersistedState</code>, <code>sanitize_state</code>, <code>load_state</code>, and <code>save</code>. If you change the schema:

- Keep serde defaults so older files load.
- Update <code>STATE_VERSION</code> if the meaning changes; the loader currently treats unknown versions as the current model after sanitising, so an explicit migration is better than silent reinterpretation.
- Clamp/normalise anything that can come from disk.
- Keep the atomic write and <code>.bak</code> copy.
- Do not write secrets or account data; there are none today.

Settings live in `app.rs` (<code>StoredSettings</code>) and are intentionally tiny: device and volume. If you add a setting, update <code>save_settings</code>/<code>start_live</code>, the Settings UI, and the README data table.

---

## 9. Debugging

### 9.1 Environment variables

| Variable | Effect |
| --- | --- |
| <code>GAYDAR_DATA_DIR</code> | Data directory for <code>state.json</code> and <code>settings.json</code> (used as given). |
| <code>GAYDAR_CACHE_DIR</code> | Cache directory for <code>client_id.json</code> and artwork (used as given). |
| <code>GAYDAR_ASSETS_DIR</code> | Asset root, if <code>./assets</code> is not the right one. |
| <code>GAYDAR_ALSA_DEVICE</code> | ALSA PCM name applied at startup, overriding the stored device for that run. |
| <code>BUI_GPU</code> | BUI's GPU switch (1/0); overrides the app's requested renderer. Useful for rendering diagnosis. |
| <code>RUST_BACKTRACE</code> | Standard Rust panic backtrace. |

There is no <code>GAYDAR_LOG</code> or tracing filter. The app writes explicit <code>[gaydar audio]</code> / <code>[gaydar radio]</code> lines to stderr.

### 9.2 Useful commands

~~~bash
cargo run --release 2>&1 | tee /tmp/gaydar.log
GAYDAR_ALSA_DEVICE=default cargo run --release
cargo test --test live_soundcloud live_client_id_discovery -- --nocapture
cargo test --test live_soundcloud live_search_resolve_and_download -- --nocapture
cargo test --test live_soundcloud live_drm_only_track_errors_without_hanging -- --nocapture
cargo build --release -vv
~~~

The live test filters print useful facts: discovered client ID and timing, playable counts per query, stream kinds and content types, artwork bytes, and the DRM error.

### 9.3 Isolate the problem

- **UI only?** Run <code>--demo</code> or a screenshot; no network/audio.
- **Network only?** Run the live client/search tests.
- **Audio only?** Run the live headless test with a known-good PCM, or verify the device with <code>speaker-test</code>.
- **State only?** Use fresh <code>GAYDAR_DATA_DIR</code>/<code>GAYDAR_CACHE_DIR</code> directories.
- **Rendering only?** Try <code>BUI_GPU=0</code> and compare with the default GPU path.

### 9.4 Inspect state safely

~~~bash
head -c 500 ~/.local/share/gaydar/state.json
head -c 300 ~/.cache/gaydar/client_id.json
~~~

Do not edit a live state file while the app is running; it is read once and then overwritten on the next save. Move it aside and restart instead.

---

## 10. Known gaps

These are true in the current build and are documented so they do not get mistaken for regressions:

- **Decorative player icons.** The shuffle and repeat icons are not controls; there is no shuffle/repeat implementation.
- **Not rustfmt-clean; Clippy warnings exist.** Do not add a formatting-only mega-diff to a behaviour PR.
- **Thin unit coverage.** Only <code>soundcloud.rs</code> has unit tests; the engine/audio/radio paths rely on the live integration tests.
- **Bootstrap verification fallback.** If every probed pick is <code>StreamUnavailable</code> and the batch has three or fewer items, <code>verify_bootstrap_picks</code> returns the pre-verification clone. The pool filter usually prevents DRM here, but the fallback can re-admit it.
- **State schema is brittle.** <code>Track</code> fields have no per-field serde defaults; one missing non-Option field in a persisted track makes the whole <code>state.json</code> unparseable and the file is moved aside as corrupt. See [ARCHITECTURE.md](ARCHITECTURE.md#74-taste-state).
- **Station supply is still catalogue-limited.** Each station tries one primary query and one fallback at a 50-result limit; both can return zero or very few MP3-playable tracks. The radio reports a quiet station rather than padding the queue.
- **No CI.** Tests are local/manual.
- **`assets/seeds.json` is not wired.** The engine uses constants in `gaydar.rs`.
- **Decorative/unused assets.** The app uses brand marks, icons, grain, and fonts; the covers and stickers are available but not referenced by the production UI. `assets/covers` and `assets/stickers` are part of the brand library, not dead code to delete.
- **No queue/current-track persistence.** Restarting bootstraps a new queue.
- **No media keys/MPRIS/tray/desktop notifications.**
- **Linux only.** No portability layer for CoreAudio/WASAPI.

If you fix one of these, update this list, the README's "Known rough edges", and the relevant architecture note.

---

## 11. Contributor workflow

1. **Read the design first.** [ARCHITECTURE.md](ARCHITECTURE.md) explains the threading and the playable filter. [BRAND.md](BRAND.md) and [COPY.md](COPY.md) govern UI language.
2. **Work in a branch.** Keep behaviour changes and formatting separate.
3. **Build and test deliberately:**

   ~~~bash
   cargo build --release
   cargo test --lib
   cargo test --test ui_interactions
   ~~~

   If you touched network, streaming, resolution, or the engine, run the live target as well. If you touched client ID handling, run the ignored rotation test.
4. **Format touched files** with <code>rustfmt --edition 2024 &lt;files&gt;</code>. Do not reformat the world.
5. **Screenshot UI changes** with an explicit screen and size; review the PNG. Add to <code>docs/gallery/</code> only when the change is meant to update the gallery.
6. **Keep the invariants.** No blocking UI, no DRM bypass, no account endpoints, no second audio path, no telemetry.
7. **Update docs** when behaviour changes: README commands/controls/limits, ARCHITECTURE weights/flow, TROUBLESHOOTING symptoms, this file's gaps.
8. **Do not commit** <code>target/</code>, <code>state.json</code>, <code>settings.json</code>, <code>client_id.json</code>, cached artwork, <code>/tmp</code> artifacts, or personal screenshots.
9. **License:** contributions are under GPL-3.0-or-later. BUI is GPL-3.0-or-later; new dependencies must be license-compatible. Fonts remain OFL; do not relicense them.

### Pull request checklist

- [ ] <code>cargo build --release</code> succeeds.
- [ ] <code>cargo test --lib</code> and <code>cargo test --test ui_interactions</code> pass.
- [ ] Live tests pass if the change touches SoundCloud/audio/engine.
- [ ] MP3/DRM filter and media verification are intact.
- [ ] No UI-thread network/ALSA calls added.
- [ ] No secrets, client IDs, or personal state committed.
- [ ] Docs updated for new flags, settings, paths, or behaviour.
- [ ] New UI copy follows [BRAND.md](BRAND.md) and [COPY.md](COPY.md).
- [ ] Screenshot included for visual changes (or a reason not to).

---

## 12. Where to look next

- [README](../README.md) — user-facing setup, caveats, controls, data paths.
- [ARCHITECTURE.md](ARCHITECTURE.md) — threads, Mermaid diagrams, scoring formula, audio state machine, JSON schemas.
- [TROUBLESHOOTING.md](TROUBLESHOOTING.md) — no audio, no results, client ID, no MP3, screenshots, builds.
- [BRAND.md](BRAND.md) — voice, palette, naming, accessibility.
- [COPY.md](COPY.md) — production strings and budgets.
- [BUI-NOTES.md](BUI-NOTES.md) — what BUI 0.5 does well and where it differs from a browser.
- [assets/MANIFEST.md](../assets/MANIFEST.md) — fonts, icons, marks, stickers, covers, patterns, licensing.
