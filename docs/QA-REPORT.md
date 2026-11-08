# GAYDAR FM — live end-to-end QA report

**Date:** 2026-11-08, 21:00–22:45 +03 (Europe/Istanbul)
**Host:** CachyOS, Linux 7.3.1-5-cachyos-bore-v4 x86_64
**Rust:** rustc/cargo 1.98.1
**ffmpeg:** 9.0.2
**BUI:** blenderui 0.5 (Cargo.lock)
**Artifact:** `target/release/gaydar`, 18,605,720 bytes (built 22:40:32)
**SHA-256:** `06a07d24e383643b821f4e48927f8b70c2c1a5a82f3449060ea8262f252cbe2c`
**Audio:** direct ALSA to `null` via `GAYDAR_ALSA_DEVICE=null`
**Display:** Xvfb `:103` at 1440x900 for external runs; BUI headless windows at 1440x900 for scripted interaction

This report covers live startup, real SoundCloud catalogue access, real ffmpeg decoding, real ALSA playback to the null device, UI interaction, persistence, restart and process cleanup. No source files were modified by QA.

---

## 1. Result summary

| Check | Command | Result |
|---|---|---|
| Library unit tests | `cargo test --lib` | **4 passed, 0 failed** (0.00 s) |
| Existing headless UI interaction test | `cargo test --test ui_interactions` | **1 passed, 0 failed** (9.22 s) |
| New live E2E file, default run | `cargo test --test live_app_e2e` | **1 passed, 0 failed, 2 ignored** (ALSA enumeration test) |
| New live E2E file, live tests | `GAYDAR_ALSA_DEVICE=null cargo test --test live_app_e2e -- --ignored --test-threads=1 --nocapture` | **2 passed, 0 failed** (55.25 s) |
| New live screenshot mode | `./target/release/gaydar --live-shot ... --screen home` | **exit 0, playing=true, real track + art in PNG** |
| Release build | `cargo build --release` | **success** (one binary, 18 MB) |
| External app under Xvfb | release binary + `DISPLAY=:103` | **boot → live home → Playing → clean exit 0; no app-owned ffmpeg left** |

Final source state used for the last full run: `src/app.rs` 22:34:21; every other source file except `src/audio.rs`, `src/lib.rs`, `src/model.rs` and `src/gaydar.rs` carries the 22:18:43 tree timestamp; `src/gaydar.rs` is 21:39:36. The release binary was built at 22:40:32 after those changes. A transient in-progress source edit (21:24–21:28) made the tree uncompilable; it was resolved by another workstream before final validation. QA did not edit src.

### Requirement-by-requirement

| # | Requirement | Verdict | Evidence |
|---|---|---|---|
| 1 | Live startup reaches boot screenshot, then home with a real track and album art | **PASS** | External `boot.png` + `home1.png`; headless `01-boot.png`, `02-home-playing.png`; 12–20 cached artwork files |
| 2 | Audio position advances, state becomes Playing, ffmpeg child exists and is reaped on exit | **PASS** | Headless asserts `Playing` + position delta; external `ps` showed one ffmpeg child; after exit none |
| 3 | Next advances to a different track; seek changes position; play/pause works | **PASS** | Real X11 clicks: next changed track `Mueres Por Mi` → `Please Go / Don't Go`; seek jumped to 2:11; pause froze the clock (pixel diff 0), resume advanced |
| 4 | Search returns real results for `ballroom vogue` and results are playable | **PASS (UI search); play via service call** | 6 real filtered results, first `Richeyrite – Vogue Like *Ballroom Remix*` streamed and reached `Playing` |
| 5 | Like updates persisted `state.json` and UI canon; Ballroom 4AM loads a playable queue | **PASS after station fix** | `liked` contains track 1424768506; restart log says `liked=1`; station produced 5 queue entries and a playing current track |
| 6 | Settings lists real ALSA devices; volume persists to `settings.json` | **PASS** | 27 ALSA hints including `null`, `pipewire`, USB `dongle`, HDMI and ALC897; `settings.json` = `{"device":"null","volume":1.0}` |
| 7 | No orphan ffmpeg after quitting; state survives restart | **PASS** | Exit 0 with no app-owned ffmpeg; restart restored the liked track, persisted plays, volume 1.0, device null and artwork cache |

---

## 2. What was actually run

### 2.1 External app (real X11 window)

```bash
Xvfb :103 -screen 0 1440x900x24 -nolisten tcp
mkdir -p /tmp/gaydar-qa-final6/{data,cache,shots}
cd /home/emre/Projeler/gaydar
DISPLAY=:103 \
GAYDAR_DATA_DIR=/tmp/gaydar-qa-final6/data \
GAYDAR_CACHE_DIR=/tmp/gaydar-qa-final6/cache \
GAYDAR_ALSA_DEVICE=null \
./target/release/gaydar
```

Interaction was driven with a small Python XTest helper at `/tmp/gaydar-qa/x11.py` (motion/button/key events and a synthetic `WM_DELETE_WINDOW`), because `xdotool` is not installed on the host. Screenshots were captured with ImageMagick `import -window root`.

Observed live-session evidence:

- Boot frame at `/tmp/gaydar-qa/shots/boot.png` — "Tuning the gaydar / hold still, gorgeous", 01 CLIENT / 02 STREAM / 03 SERVING, no track.
- Home frame at `/tmp/gaydar-qa/shots/home1.png` — real track **Papa Topo – Mueres Por Mi**, real cover, paused state 0:35, ALSA `null` status.
- Next frame at `/tmp/gaydar-qa/shots/next.png` — **SYNESTHEMA – Please Go / Don't Go (Instrumental)**, new artwork, position 0:06; `state.json` showed `plays=2, skipped=1, history=2`.
- Seek: click at ~86% on the waveform restarted ffmpeg (`-ss`), position jumped to **2:11**; crop at `/tmp/gaydar-qa2/shots/seekcrop.png`.
- Station: click Ballroom 4AM loaded **Richeyrite – Vogue Like *Ballroom Remix***, dials `club=1.0, soft=0.18, chaos=0.62`; frame at `/tmp/gaydar-qa2/shots/station.png`.
- The final Xvfb run on SHA-256 `06a07d2…` reached home with **SYNESTHEMA – Please Go / Don't Go (Instrumental)**, position 0:47, playing, and 12 cached covers: `/tmp/gaydar-qa-final6/shots/home.png`. One synthetic `WM_DELETE` then produced exit 0 and no app-owned ffmpeg.
- The final binary (SHA-256 `06a07d2…`) was verified with the new live screenshot mode: `--live-shot /tmp/gaydar-live-shot3/home.png --screen home` printed `playing: true`, wrote a 1440x900 PNG with a real current track and artwork, exited 0, and left no ffmpeg child. Image: `/tmp/gaydar-live-shot3/home.png`.

Process checks used `ps -eo pid,ppid,etime,cmd`. A SoundCloud ffmpeg child was present while playing:

```text
./target/release/gaydar
  ffmpeg -nostdin -hide_banner -loglevel error -user_agent Mozilla/5.0 ...
         -headers Referer: https://soundcloud.com/ ... -i https://cf-media.sndcdn.com/...128.mp3 ...
         -vn -f s16le -ac 2 -ar 48000 -
```

After quitting, both the app and its ffmpeg child were gone. One external run exited 0 after `WM_DELETE`; in another run the first raw `XSendEvent` close was not actioned and a second was required (see Limitations).

### 2.2 Headless live harness

`tests/live_app_e2e.rs` uses `Application::headless()`, the real `GaydarApp::live()` state and `GaydarApp::start_live(cx)`. Because BUI's headless `Application::run` invokes the closure once and then shuts down, the harness explicitly pumps each frame:

```rust
cx.run_pending();
cx.draw(win.id);   // Render::render() calls app.pump()
```

It waits on real state transitions and asserts externally observable facts. It also saves 1440x900 PNGs from `cx.draw()` for every phase.

Test 1, `live_startup_playback_controls_and_shutdown` (part of the final 55.25 s combined run):

1. Boot state: `booting=true`, `current=None`, audio engine exists, device `null`.
2. Wait for bootstrap; validates real non-empty title/artist, duration >10 s, real track id, artwork URL. Final first track **Papa Topo – Actriz De Terror Italiana**.
3. Wait for `PlaybackState::Playing` and `position_ms > 0`; find exactly one ffmpeg child in `/proc/self/task/*/children`; assert the command line contains `-f s16le`, `-ar 48000` and a SoundCloud CDN URL.
4. Wait for `position_ms` to advance ≥500 ms; wait for artwork to download; validate image magic and size.
5. Click `play-toggle`; assert `Paused`; sleep 1.2 s; assert position moved ≤120 ms. Click again; assert `Playing`.
6. Click the next button by geometry (no element id): play-toggle right edge + 12 px gap + 12 px half-width of the 24 px control, y = play-toggle center. Assert current id changes, `Playing`, source present; wait for both old and new ids in history.
7. Click `seek-bar-40` (40.5/48 = 84.4%); assert position/duration within 0.12 and `Playing`; assert ffmpeg restarted and carries `-ss`.
8. After the headless app shuts down, poll for zero ffmpeg children. Final log: first ffmpeg pid 443523; after seek 443598; clean shutdown.

Test 2, `live_search_like_station_settings_and_restart` (part of the final 55.25 s combined run):

1. Boot to a real home track.
2. Navigate to Search via `nav-search`; focus `search-field`; type `ballroom vogue` through `simulate_key` (including the space); press Return. Wait for non-searching state. Final result: **6 tracks**, first **Richeyrite – Vogue Like *Ballroom Remix***, all with title/artist, at least one with artwork.
3. Play the first result through `app.play_track()` — the same public call used by the row's click listener at `src/ui.rs:415-490`; wait for current id, `Playing` and a resolved source. (The row itself has no element id, so a raw row click cannot be addressed through `element_bounds`.)
4. Navigate to Liked; press `l` (root shortcut, `src/ui.rs:950`). Assert `liked` and `liked_tracks` contain the current id. Poll `state.json` until the track id appears in `liked`; assert the same through the app state.
5. Click `nav-home`, then `station-ballroom`. Wait for Home + `dials.club == 1.0` + `radio_status` containing BALLROOM + `Playing` + source + non-empty queue. Final: **Richeyrite – Vogue Like *Ballroom Remix***, queue=5.
6. Navigate to Settings through the public `app.goto(Screen::Settings)` (gear has no id). Assert:
   - `default` is first and `is_default`;
   - at least one non-default ALSA hint;
   - names/descriptions non-empty;
   - `null` is enumerated;
   - `app.device == "null"`.
   Final device list included `default`, `null`, `sysdefault`, `speexrate`, `pipewire`, `upmix`, USB `sysdefault:CARD=dongle`, several `surround*`, `iec958`, four HDMI outputs and ALC897 analog outputs (27 hints).
7. Click `settings-volume-17`; assert volume 1.0; wait for `settings.json`; assert `{"device":"null","volume":1.0}`.
8. Shut down app/audio/worker, assert zero ffmpeg children.
9. Restart: run a second headless application with the same data dir; wait for a fresh live bootstrap. Assert liked id is restored, `liked_tracks` has it, plays ≥3, volume 1.0, device null, artwork cache non-empty. Final restart log at bootstrap: track **Richeyrite – Vogue Like**, liked=1, plays=3, volume=1, device=null. The restarted app then recorded its own new autoplay, so the final state file on disk reached 4 plays.

The non-ignored test `alsa_null_device_is_enumerable_and_openable` checks the enumerator without network and opens `AudioEngine::new(Some("null"), 0.2)`.

---

## 3. Persistence evidence

Final `state.json` (from the last full run) contained:

```json
{
  "version": 1,
  "plays": 4,
  "completions": 0,
  "liked": [{"id": 1424768506, "artist": "Richeyrite", "title": "Vogue Like *Ballroom Remix*"}],
  "history": [
    {"id": 1162346398, "title": "Actriz De Terror Italiana", "outcome": "started"},
    {"id": 1424768506, "title": "Vogue Like *Ballroom Remix*", "outcome": "started"},
    {"id": 1424768506, "title": "Vogue Like *Ballroom Remix*", "outcome": "started"},
    {"id": 1541548450, "title": "Please Go / Don't Go (instrumental)", "outcome": "started"}
  ],
  "dials": {"camp": 0.82, "club": 1.0, "soft": 0.18, "chaos": 0.62}
}
```

`settings.json`:

```json
{"device":"null","volume":1.0}
```

After restart, the new app loaded the liked id, history/plays, dials, volume and device from disk; the test did not re-like the track.

---

## 4. Bugs and findings (prioritized)

### P1 — FIXED DURING QA: Ballroom 4AM could not load a queue because station search was too shallow

**Symptom (reproduced twice):** clicking Ballroom 4AM on a live session produced only a toast:
`BALLROOM 4AM is quiet right now (no playable results for "queer rave"). Try another station.`
The previous track kept playing, `radio_status` stayed `SOUNDCLOUD CONNECTED · 39AVoi`, and the dials changed to Ballroom's values anyway.

**Root cause:** station search used `client.search_tracks(&query, 24)`; the MP3 filter is intentionally strict. For query `ballroom vogue`, the first true-MP3 result was at **rank 27** (Richeyrite – Vogue Like *Ballroom Remix*); for fallback `queer rave`, rank 22 was 60 min (over the 45 min cap), rank 24 was policy `SNIP`, and the first usable result was rank 29. A depth of 24 therefore returned zero. UI search with limit 30 found exactly one; the old `ballroom vogue beat` query only worked because its single MP3 result happened to be rank 22.

**Fix already in tree before final validation:** station commands now carry a primary + fallback query, station search limit is 50 (`src/radio.rs:331`), UI search limit is 50, and `resolve_first` tries more candidates. The final test loaded Ballroom with 5 queue entries and streamed Richeyrite. This finding is retained because the failure mode is real and was observed, not because the current tree is failing.

**Residual risk (open):** station queries are still hard-coded strings whose usable yield depends on SoundCloud ranking. A depth of 50 is better but not a guarantee. Deeper paging or falling back to the recommendation engine's existing queue would be more robust.

### P2 — OPEN (medium): a failed station leaves the app in a half-applied station state

**Code:** `GaydarApp::apply_station` (`src/app.rs:715-748`) still sets `self.dials = station.dials` before the worker answers. It no longer marks the transport as loading (the older tree did, which magnified the symptom). `RadioEvent::Error` (`src/app.rs:1045-1059`) only logs and shows a toast; it does not restore the previous dials or radio status.

**Observed symptom before the station fix:** after the failure, the home screen showed Ballroom's dials, the old track kept playing, the radio chip still said "ON AIR"/"SOUNDCLOUD CONNECTED", and the only feedback was a 3.2 s toast. A user can easily miss it and believe the station loaded. On the current tree only the dials/status part of that state remains.

**Reproduction:** force a station command whose queries return no usable tracks (or observe the pre-fix tree with limit 24), then click Ballroom. No current live query reproduced this after the depth fix.

**Suggested fix:** apply dials only when `RadioEvent::Station` succeeds; on failure restore the previous dials and keep a visible error until dismissed.

### P3 — OPEN (low, testability): several interactive controls have no element id

Headless `element_bounds` and BUI `simulate_click` can only address `.id(...)` elements. Missing ids:

- next/previous transport buttons: `src/ui.rs:748-757` (play has `play-toggle`, next has none)
- track rows: `src/ui.rs:415-490`
- settings device rows: `src/views.rs:548-566`
- settings gear: `src/ui.rs:236-239` (Settings is not in the rail; `nav-settings` is only a fallback string in `nav_item`, no item uses it)

QA compensated by clicking next by geometry (right edge + 12 px gap + 12 px half-width), calling `app.play_track()` for the first result, and calling `app.goto(Screen::Settings)`. These are the same handlers the UI runs, but the compensation is less faithful than a real click and would silently pass if the click wiring broke. Adding ids would make future E2E tests substantially stronger.

### P4 — OPEN (low, data-dependent): `is_mp3` ignores MIME when a non-mp3 preset is present

`Transcoding::is_mp3` (`src/soundcloud.rs:1521-1531`) returns early on any non-empty preset: if the preset does not start with `mp3`, it returns false even when `format.mime_type` contains `mpeg`. SoundCloud's `abr_sq` transcodings have `preset: "abr_sq"` and `mime_type: "audio/mpegurl"`; they are therefore treated as non-MP3/DRM and dropped. This is why `ballroom vogue` has so few app-visible results (23 raw mpeg-MIME candidates vs. 1 true `mp3_1_0` candidate in the first 30).

I fetched one `abr_sq` URL for track 2007928131 with `client_id` + `track_authorization`; it returned HTTP 404, so the exclusion may be correct and this is **not confirmed as a user-facing bug**. It is a fragile heuristic: if SoundCloud starts serving playable audio only through `abr_sq`, the app will hide those tracks. If `abr_sq` is deliberately unsupported, a comment/test should say so.

### P5 — OBSERVATION (low, harness): one synthetic WM_DELETE was ignored

In one external run, a single raw `WM_DELETE_WINDOW` ClientMessage sent with `XSendEvent` (no window manager) left the app running for 6+ s; a second message produced exit 0 and killed ffmpeg. In the final artifact run the first message worked (exit 0, no ffmpeg), and the earlier successful run also closed on the first message. The headless Drop path always cleaned up, and X-server death also left no app-owned ffmpeg. Because the host has no window manager and the close is synthetic, this is recorded as harness flakiness rather than a confirmed product defect. A native WM close test on a real desktop session would settle it.

---

## 5. Limitations and things not verified

1. **No OCR/automated pixel assertions of text.** Screenshots were inspected visually. State assertions come from the app's public fields and disk files; external state (Playing/Paused) is inferred from the app's own audio status plus process/pixel evidence in the headless test.
2. **Search result activation** was exercised through `app.play_track()`, not a click on the result row, because `track_row` has no id. The row's listener calls exactly that method. Real search typing and Return were exercised.
3. **Settings navigation** used `app.goto(Screen::Settings)` because the gear has no id and the rail has no Settings entry. Volume segment clicks and device enumeration were real UI state.
4. **Volume audibility** cannot be measured on the `null` ALSA device. The test verifies the app volume field, the audio engine status and `settings.json`; it does not measure acoustic output or switching to a physical device.
5. **Device switching** was not exercised against real hardware (only enumeration and the `null` selection). Switching to a physical device while playing would restart the stream and is a reasonable follow-up on real hardware.
6. **Live tests are ignored** and depend on SoundCloud's catalogue/ranking and network. They are deterministic in structure (wait for invariants, bounded timeouts) but not in which track is served. Run serially: the test sets process-global `GAYDAR_*` variables and a shared BUI/audio process environment.
7. **BUI native close** is synthetic (XSendEvent, no WM). See P5.
8. **Xvfb `:103` died several times** after a few minutes of external running (exit 0, app then exits 1 with "X connection to :103 broken"). This is environmental; after X death the app's Drop path still killed ffmpeg and no orphan remained. All final headless tests are independent of Xvfb.
9. The source tree changed while QA ran (other agents finishing). Final validation was rerun after the last observed change (22:34:21, binary built 22:40:32). If source changes again, rerun the commands below.

---

## 6. How to reproduce

### Fast checks (no network, no audio)

```bash
cd /home/emre/Projeler/gaydar
cargo test --lib
cargo test --test ui_interactions
cargo test --test live_app_e2e          # 1 ALSA test passes; 2 live tests ignored
```

### Live E2E (needs SoundCloud network + ffmpeg + ALSA null)

```bash
cd /home/emre/Projeler/gaydar
GAYDAR_ALSA_DEVICE=null \
  cargo test --test live_app_e2e -- \
  --ignored --test-threads=1 --nocapture
```

The harness creates its own `/tmp/gaydar-live-e2e/<tag>-.../{data,cache,shots}` per test, sets `GAYDAR_DATA_DIR`, `GAYDAR_CACHE_DIR` and `GAYDAR_ALSA_DEVICE=null`, and never touches the developer's real GAYDAR state. Artwork and screenshots remain after the run for inspection.

### External release run

```bash
Xvfb :103 -screen 0 1440x900x24 -nolisten tcp &
mkdir -p /tmp/gaydar-qa/data /tmp/gaydar-qa/cache
cd /home/emre/Projeler/gaydar
DISPLAY=:103 \
GAYDAR_DATA_DIR=/tmp/gaydar-qa/data \
GAYDAR_CACHE_DIR=/tmp/gaydar-qa/cache \
GAYDAR_ALSA_DEVICE=null \
./target/release/gaydar
# capture: DISPLAY=:103 import -window root /tmp/gaydar-qa/home.png
# process: ps -eo pid,ppid,cmd | grep -E 'gaydar|ffmpeg'
```

Close via the window manager in a normal desktop session. The QA harness used XTest clicks and a synthetic `WM_DELETE_WINDOW`; helper source is at `/tmp/gaydar-qa/x11.py`.

---

## 7. Evidence index

Ephemeral scratch under `/tmp`:

- External boot + home + next: `/tmp/gaydar-qa/shots/{boot,home1,next}.png`
- External pause/seek/station: `/tmp/gaydar-qa/shots/{paused1,paused2,playing}.png`, `/tmp/gaydar-qa2/shots/{seekcrop,station}.png`
- Final release external home: `/tmp/gaydar-qa-final6/shots/home.png`
- Final release live-shot: `/tmp/gaydar-live-shot3/home.png`
- Final live test 1 (startup/controls/shutdown): `/tmp/gaydar-live-e2e/startup-442894-1-1794166910439341931/shots/`
- Final live test 2 (search/like/station/settings/restart): `/tmp/gaydar-live-e2e/social-442894-0-1794166880270059513/shots/`
- Persisted data for test 2: `/tmp/gaydar-live-e2e/social-442894-0-1794166880270059513/data/{state.json,settings.json}`
- Test source: `tests/live_app_e2e.rs`

The final test logs are also visible with `--nocapture`: search returned 6 results, Ballroom queue=5, settings wrote `{"device":"null","volume":1.0}`, restart restored liked=1/plays=3/volume=1/device=null.

---

## 8. Bottom line

The app builds, starts live against SoundCloud, reaches the boot and home screens with real tracks and artwork, decodes through ffmpeg to ALSA null, advances/seeks/pauses/resumes, changes tracks via the next control, searches real catalogue results, likes and persists canon, loads the Ballroom station after the station-depth fix, enumerates real ALSA hints, persists volume, restarts from disk and leaves no app-owned ffmpeg on a real quit. The one substantive product bug found (station under-fetch causing Ballroom to fail) was fixed in-tree and re-verified. The main open risks are the half-applied station state on a future failure, missing element ids for deterministic UI automation, and the brittle preset-vs-MIME MP3 heuristic.
