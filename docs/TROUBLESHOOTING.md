# GAYDAR FM — Troubleshooting

Everything here is about the current build. Start with the symptom, run the checks, and only then change settings. When something fails, the app prints the useful detail to stderr as well as the UI; keep that output.

Useful first commands:

~~~bash
cargo --version
ffmpeg -version
aplay -l
pkg-config --modversion alsa
curl -sS -o /dev/null -w '%{http_code}\n' https://soundcloud.com/
cargo run --release 2>&1 | tee /tmp/gaydar.log
~~~

If the app panics, rerun with <code>RUST_BACKTRACE=1</code>. There is no log file and no <code>RUST_LOG</code> integration; the two stderr prefixes are <code>[gaydar audio]</code> and <code>[gaydar radio]</code>.

---

## No audio

### 1. Is ffmpeg there?

The engine spawns <code>ffmpeg</code> by name, not a vendored binary:

~~~bash
which ffmpeg
ffmpeg -version
~~~

If the app reports <code>could not start ffmpeg (is it installed?)</code>, install ffmpeg. It must support MP3 and HLS demuxing/decoding; a normal distro build does.

### 2. Is the selected ALSA device the one you think it is?

<code>default</code> is an ALSA plugin, not "the analog output". On a laptop with HDMI, USB audio, and analog, it can point at the wrong card. Open **Settings**, look at the real PCM names, and pick one. The list is scrollable, holds every hint, and can be re-enumerated with **REFRESH**; <code>default</code> is always first.

~~~bash
aplay -l
speaker-test -D default -c 2 -t sine -l 1
speaker-test -D hw:0,0 -c 2 -t sine -l 1   # replace 0,0 with your card/device
speaker-test -D plughw:0,0 -c 2 -t sine -l 1
~~~

<code>speaker-test</code> comes from <code>alsa-utils</code>. If it works but GAYDAR FM does not, the app is on the wrong device or its format negotiation failed.

Try a launch with an explicit device:

~~~bash
GAYDAR_ALSA_DEVICE=default cargo run --release
GAYDAR_ALSA_DEVICE=pipewire cargo run --release
GAYDAR_ALSA_DEVICE=pulse cargo run --release
GAYDAR_ALSA_DEVICE=plughw:0,0 cargo run --release
GAYDAR_ALSA_DEVICE=hw:0,0 cargo run --release
~~~

- <code>default</code> — the ALSA default plugin; usually routes through PipeWire/PulseAudio if configured.
- <code>pipewire</code> / <code>pulse</code> — the sound-server PCMs, when those names exist.
- <code>plughw:CARD,DEV</code> — hardware with ALSA conversion; the safest fallback when the device refuses 48/44.1 kHz s16 stereo.
- <code>hw:CARD,DEV</code> — direct hardware, often exclusive; another player may block it.

The engine asks for **s16le, stereo, 48 kHz**, then **44.1 kHz**. If the device says no to both, you get:

<code>could not open ALSA output "…" as 16-bit stereo at 48 kHz or 44.1 kHz: …</code>

That is a device capability problem, not a decoder problem. Use <code>default</code>, <code>pipewire</code>, <code>pulse</code>, or <code>plughw:</code>.

If this happened when the app started, the audio engine may not exist at all; changing the device in Settings cannot create it. Restart with a working device selected:

~~~bash
GAYDAR_ALSA_DEVICE=plughw:0,0 cargo run --release
~~~

If the device fails while a track is playing, the engine does exist, and switching devices in Settings restarts the stream at the current position.

### 3. Is something else holding the device?

<code>hw:</code> devices are often exclusive. Close other players, browsers with audio, or anything using the same card; or switch to the sound-server PCM. A busy device usually produces an "open failed"/"device busy" message in the ALSA error and on stderr.

### 4. Is the app's volume up?

Volume is software gain applied after decoding, with a <code>volume²</code> curve. Check the player's volume bars and **Settings → Volume**. The default live volume is 0.78. The system mixer still applies separately; check <code>alsamixer</code> too.

### 5. Does the position advance?

If the waveform moves but you hear nothing, the stream is decoding and the audio is going to a different device. Change the device. If the position stays at 0:00, the stream has not produced its first PCM frame; see the timeout section.

### 6. Read the actual error

Common messages and their meaning:

| Message (or prefix) | Meaning | Fix |
| --- | --- | --- |
| <code>could not start ffmpeg (is it installed?)</code> | No executable named <code>ffmpeg</code> on <code>PATH</code>. | Install ffmpeg; launch from a shell that has it. |
| <code>could not open ALSA output … as 16-bit stereo at 48 kHz or 44.1 kHz</code> | The PCM refused both formats/rates. | Try <code>default</code>, <code>pipewire</code>, <code>pulse</code>, <code>plughw:</code>. |
| <code>ALSA playback failed on "…" and underrun recovery failed</code> | The device stopped accepting frames. | Re-select the device; check cables/USB; try <code>plughw</code>. |
| <code>ffmpeg produced no audio within 15 seconds (expired URL, dead stream, or blocked network)</code> | No first PCM byte. | Retry; check network; the signed URL may have expired. |
| <code>audio stream stalled for 25 seconds while unpaused; ffmpeg was stopped</code> | The stream went silent. | Retry; check network; the CDN URL may be stale. |
| <code>ffmpeg exited with status … (the stream may have expired)</code> | Decoder died mid-stream. | Retry; skip the track; check the URL/TTL. |
| <code>ffmpeg decoded no audio for this stream</code> | A 0-byte or audio-less response. | Retry; if it repeats for one track, it is a bad stream. |
| <code>ALSA rejected s16le access on "…"</code> | The PCM cannot take interleaved s16le. | Use <code>plughw:</code>/<code>default</code>. |

The tail of ffmpeg's stderr is appended to playback errors (up to 400 characters), so the UI sometimes shows the decoder's own sentence after the app's.

### 7. Is the app even live?

<code>--demo</code> and <code>--screenshot</code> never create an audio engine. In demo mode the transport is simulated and there is no sound by design. Run <code>cargo run --release</code> (no demo flags) for audio.

---

## No search results

### 1. Live search runs on submit, not on every keystroke

In the live app, typing only edits the query. Press **Enter** or click **SEARCH** to run it. In <code>--demo</code>, filtering happens as you type, which is why demo search can feel different. This is a common "it does nothing" report.

### 2. Empty versus error

- **The crate is empty, henny.** — the request succeeded and returned no playable tracks after filtering. This is usually the MP3/DRM filter, not a network failure.
- **The signal dropped. / Search is being dramatic.** — the request failed; the error text under it is the API or network message.
- **SoundCloud is still getting ready** — the radio worker has not finished its client ID/bootstrap phase. Wait for the boot screen to finish, then retry.

### 3. Is the query one SoundCloud can serve without DRM?

The search is filtered to uploads with a non-snipped MP3 transcoding. Label-backed catalogues often expose only AAC/ABR (DRM) and therefore return zero. Try broader scene terms before assuming a bug:

<code>queer anthem</code>, <code>ballroom</code>, <code>vogue beats</code>, <code>hyperpop</code>, <code>sapphic</code>, <code>trans joy</code>, <code>drag</code>, <code>queercore</code>, <code>queer punk</code>, <code>soft queer</code>

Indie uploads, edits, bootlegs, and live sets are much more likely to have MP3 transcodings than major-label releases.

### 4. Check the network and client ID

~~~bash
curl -sS -o /dev/null -w '%{http_code}\n' https://soundcloud.com/
cargo test --test live_soundcloud live_search_resolve_and_download -- --nocapture
~~~

The test prints how many MP3-capable results each query returned and verifies real streams. If it fails at discovery or search, go to [Client ID rejected](#client-id-rejected-401403).

### 5. Duration, policy, and the music filter

Tracks under 45 seconds or over 45 minutes are dropped, as are tracks with policy <code>BLOCK</code>/<code>SNIP</code> or <code>streamable = false</code>. A search that looks healthy on SoundCloud can legitimately return fewer results here.

The list filter also rejects obvious talk/podcast/interview/review uploads (title/description talk terms, genre <code>podcast</code>/<code>talk</code>/<code>spoken word</code>/<code>audiobook</code>/<code>comedy</code>, or talk tags) and a narrow set of slur-titled uploads. It is deliberately conservative: it does not try to judge every spoken-word or experimental track, and it can still miss things. If a release you expected is absent, check whether its title or description looks like an episode/interview.

---

## Client ID rejected (401/403)

### What is happening

The client uses a public web client ID, not an account. SoundCloud rotates or invalidates these IDs. On a 401/403 the client refreshes once automatically; if that refresh also fails, the request returns an authentication error and the UI shows it.

Typical messages:

- <code>SoundCloud rejected the client id</code>
- <code>client id rejected and refresh failed: …</code>
- <code>could not obtain a working SoundCloud client id; SoundCloud may be unreachable or blocking this network (last extraction error: …)</code>
- <code>could not reach SoundCloud: …</code>

### 1. Find the cache

| Situation | Path |
| --- | --- |
| Default | <code>$XDG_CACHE_HOME/gaydar/client_id.json</code> (usually <code>~/.cache/gaydar/client_id.json</code>) |
| <code>GAYDAR_CACHE_DIR</code> set | <code>$GAYDAR_CACHE_DIR/client_id.json</code> |

Check it:

~~~bash
ls -l ~/.cache/gaydar/client_id.json
head -c 300 ~/.cache/gaydar/client_id.json
~~~

It should be JSON with a 32-character alphanumeric <code>client_id</code>, <code>saved_at</code>, and <code>source</code>. A truncated or empty file is harmless; it will be replaced.

### 2. Move it aside and restart

Stop the app first.

~~~bash
mv ~/.cache/gaydar/client_id.json ~/.cache/gaydar/client_id.json.old
cargo run --release
~~~

The next launch performs fresh discovery: fetch <code>https://soundcloud.com/</code>, scan the page, possibly fetch up to 12 JavaScript bundles, validate candidates with a search request, and finally try the built-in public IDs. This can take roughly ten seconds on a healthy connection. The boot screen stays up while it runs; if it fails, it shows the error and a retry button.

If you use <code>GAYDAR_CACHE_DIR</code>, move the file there instead.

### 3. Check the network path

~~~bash
curl -sS -o /dev/null -w '%{http_code}\n' https://soundcloud.com/
curl -sS -o /dev/null -w '%{http_code}\n' https://api-v2.soundcloud.com/search/tracks
~~~

- The first should be 200.
- The second will be 401 without a valid <code>client_id</code>; that is expected. The important part is that DNS/TLS/connect work.
- A VPN, corporate proxy, DNS filter, or aggressive privacy extension can block <code>soundcloud.com</code> or <code>a-v2.sndcdn.com</code>. Try another network if you can.

### 4. Don't hammer it

Discovery is paced at about five requests per second and has retries. Restarting in a loop will not make SoundCloud hand out a new ID faster and may make rate limiting worse. Wait a minute, then retry once.

### 5. What you cannot do from the CLI

There is no <code>--client-id</code> or <code>--refresh-client-id</code> flag. The library exposes <code>SoundCloudClient::discover_client_id()</code> and <code>refresh_client_id()</code> for tests and developers, but the app's supported repair path is deleting/moving the cache and restarting. Do not paste a client ID into an issue unless you understand it is a public, rotating token; it is not an account credential, but it is still better not to circulate live ones.

### 6. The ignored rotation test

When changing discovery code, run:

~~~bash
cargo test --test live_soundcloud -- --ignored live_client_id_refresh_on_401 --nocapture
~~~

It constructs a client with a deliberately invalid ID and verifies that refresh recovers. It is ignored by default because it is a slow live check.

---

## No MP3 tracks

### Why this happens

SoundCloud exposes multiple transcoding families per track. This client can play only the MP3 family:

- <code>preset</code> starting with <code>mp3</code>, protocol <code>progressive</code> → playable.
- <code>preset</code> starting with <code>mp3</code>, protocol <code>hls</code>, no real encryption → playable.
- <code>aac_*</code> / <code>abr_*</code> → DRM-only; skipped.
- <code>snipped: true</code> → a preview; skipped.
- policy <code>BLOCK</code>/<code>SNIP</code>, non-streamable, outside 45 s–45 min → skipped.
- obvious talk/podcast/interview/review or a slur-titled upload → rejected by the narrow music-likelihood filter before queueing.

The app does not show a "DRM" badge on every filtered row; it simply never returns those rows from search, related, artist, or chart lists. A station tries its primary query and then one fallback. If both come up short, the radio reports <code>&lt;name&gt; is quiet right now (&lt;reason&gt;). Try another station.</code> The reason can be <code>the search results were all DRM-locked</code>, <code>no playable results for "query"</code>, or an API/network error.

### What to try

1. Broad scene queries before artist names: <code>queer anthem</code>, <code>ballroom</code>, <code>vogue beats</code>, <code>hyperpop</code>, <code>sapphic</code>, <code>trans joy</code>, <code>queercore</code>, <code>queer punk</code>.
2. Add <code>edit</code>, <code>remix</code>, <code>bootleg</code>, <code>live set</code>, <code>mix</code>, or <code>unreleased</code> to an artist search. User uploads are far more likely to be MP3.
3. Try a different station; the four presets use different queries and dials.
4. Try a different network. The filter is the same everywhere, but geography and catalog availability change what SoundCloud returns.

### What not to try

- Logging in. There is no login, and the app does not use account-gated endpoints.
- Extracting DRM keys or proxying a browser session. That is out of scope and not supported.
- Retrying the same DRM-only track forever. The client classifies it as <code>StreamUnavailable</code> before playback and drops it.

### A live data point

In a live run on 2026-11-08, searches for *Rina Sawayama*, *Chappell Roan*, and *Kim Petras* returned **zero** MP3-playable results, while *"queer anthem"* returned **13–14 across runs**. Catalog state changes; the lesson does not: label-backed uploads are often DRM-only, and scene/indie searches are where this player works best.

---

## Headless screenshot

### The command

~~~bash
mkdir -p /tmp/gaydar-shots
cargo run --release -- --screenshot /tmp/gaydar-shots/home.png --screen home
~~~

- <code>--screenshot</code> must be followed by a path. If the value is missing, the app treats it as a normal live launch and opens a window.
- The parent directory must exist; <code>File::create</code> will panic with <code>create screenshot</code> if it does not.
- The process prints <code>screenshot: &lt;path&gt; (&lt;width&gt;x&lt;height&gt;)</code> to stderr and exits 0.
- The PNG is RGBA; the default size is 1440×900. <code>--width</code> and <code>--height</code> take floats. The headless path uses the requested size; 1280×800 and 1000×600 are verified.
- <code>--screen</code> values: <code>home</code>, <code>gaydar</code>, <code>search</code>, <code>liked</code>, <code>history</code>, <code>settings</code>, <code>now</code>, <code>nowplaying</code>, <code>boot</code>. Unknown values become Home.
- <code>--queue</code> opens the queue drawer at launch. It works with a screenshot (the drawer covers the right side of the frame), with <code>--demo</code>, and in a live launch. It is not a screen name.
- <code>--live-shot PATH</code> is the live screenshot mode: it runs the real app headlessly, waits up to 45 s for playback and 5 s more for artwork, then writes the frame. It needs network, ffmpeg, and ALSA (use <code>GAYDAR_ALSA_DEVICE=null</code> for a silent run). If playback never starts, it still writes a frame and prints <code>live screenshot (not playing)</code>; inspect the boot/error state instead of treating the file as a success.

### It is blank, black, or missing assets

1. **Run from the repository root**, or set the assets directory:

   ~~~bash
   GAYDAR_ASSETS_DIR=/path/to/gaydar/assets cargo run --release -- --screenshot /tmp/home.png --screen home
   ~~~

   Without <code>assets/</code>, the wordmark and app mark do not load, icons fall back to plain boxes, and the grain layer is missing. The UI still renders, but it will not look like the gallery.
2. **Fonts missing** — BUI falls back to its embedded/Inter/DejaVu stack, which changes the type. Ensure <code>assets/fonts/</code> contains the TTFs. The app skips missing font files silently.
3. **Wrong screen** — <code>--screen boot</code> is intentionally sparse; it is the tuning screen, not a failed render. <code>--screen</code> is only used by demo/screenshot/boot; a normal live launch ignores it and starts on Home.
4. **No display** — headless mode should need no X server. If it fails with a display/X11 error, you are probably not in screenshot mode (the flag or its path is missing), or the binary is stale. Rebuild with <code>cargo build --release</code>.
5. **No network or audio** — that is by design for `--screenshot`. It uses demo data and never contacts SoundCloud or ALSA. Do not use it to diagnose playback; use `--live-shot` or the live app for that.

### The gallery

The gallery contains <code>docs/gallery/01-home-1440x900.png</code> through <code>08-boot-1440x900.png</code>, the <code>09-home-1280x800.png</code>/<code>10-now-playing-1280x800.png</code> pair, and <code>contact-sheet.png</code>. The README links the 1440×900 captures and the contact sheet. They are generated, not hand-drawn; the screenshot workflow in [DEVELOPMENT.md](DEVELOPMENT.md#5-screenshot-workflow) shows the commands. If a file is missing, regenerate it rather than committing a stale image.

---

## Build fails

### <code>failed to run custom build command for blenderui-sys</code>

BUI builds its C/C++ kit through CMake. Check:

~~~bash
cmake --version
c++ --version
cc --version
ls /usr/include/X11/Xlib.h
~~~

Install CMake and a C++ compiler. On Debian/Ubuntu: <code>build-essential cmake</code>. On Fedora: <code>gcc gcc-c++ cmake</code>. On Arch: <code>base-devel cmake</code>.

X11 headers are optional for the build but required for a native window. Without <code>X11/Xlib.h</code>, CMake reports "X11 headers not found; no native Linux windows (Tk / headless only)" and the build can still succeed headless-only. Install the X11 development package and rebuild if you want the visible app.

### <code>alsa-sys</code> / <code>pkg-config</code> errors

The ALSA crate needs <code>alsa.pc</code> and the development headers:

~~~bash
pkg-config --modversion alsa
~~~

If that fails: Debian/Ubuntu <code>libasound2-dev</code>, Fedora <code>alsa-lib-devel</code>, Arch <code>alsa-lib</code>, openSUSE <code>alsa-devel</code>. Make sure <code>pkg-config</code>/<code>pkgconf</code> is installed too; on openSUSE, the provider is usually <code>pkgconf-pkg-config</code>.

### Linker errors about <code>stdc++</code>

BUI links the C++ runtime. Install the C++ compiler package (<code>g++</code>/<code>gcc-c++</code>), not just <code>gcc</code>.

### <code>edition2024</code> / <code>rust-version</code> errors

Use Rust 1.88 or newer; BUI declares <code>rust-version = "1.88"</code> and both crates use edition 2024. <code>rustup update stable</code> is the usual fix.

### Other build diagnostics

~~~bash
cargo build --release -vv
~~~

The <code>-vv</code> output shows the CMake configure/build lines. A clean rebuild is sometimes necessary after changing the C kit:

~~~bash
cargo clean
cargo build --release
~~~

That is expensive; try it only after the toolchain checks above.

---

## The window won't open

### X11 / Wayland

The visible app uses BUI's native X11 window host. A Wayland session normally runs it through XWayland. If there is no display:

- Over SSH or on a headless box, use <code>--screenshot</code>; the live window cannot open without a display.
- Check <code>DISPLAY</code> and <code>WAYLAND_DISPLAY</code> in the shell that launches the app.
- BUI dlopen()s <code>libX11</code> and the OpenGL driver at runtime. Install the runtime libraries and a working driver; development headers are a build-time concern.

### GPU path

The app asks for <code>Application::new().gpu(true)</code>. BUI documents that it falls back to software raster where no device exists, and the <code>BUI_GPU</code> environment variable (1/0) overrides the request:

~~~bash
BUI_GPU=0 cargo run --release
~~~

If the GPU path renders badly but the software path is fine, use that as a diagnostic. It does not change the screenshot path, which is headless.

### Window size

The visible window defaults to 1440×900 and declares a minimum of 1120×720. You can request a different live size:

~~~bash
cargo run --release -- --width 1280 --height 800
~~~

Values below the minimum may be clamped by the window manager. For reproducible captures, use <code>--screenshot</code> instead.

### Build succeeded but there is no window host

If X11 headers were missing at build time, the binary is headless-only. Install the X11 development package, then:

~~~bash
cargo clean
cargo build --release
~~~

---

## Crackling, stutter, or dropped audio

- The engine writes <b>s16le stereo</b> at 48 kHz or 44.1 kHz with a period near 512 frames and a buffer of at least 2048 frames. ALSA underruns are recovered with <code>try_recover</code>; repeated recovery failures become a playback error.
- A busy or poorly behaved device is more often the cause than Rust. Try <code>default</code>, <code>pipewire</code>, or <code>plughw:</code> before <code>hw:</code>.
- Bluetooth and USB devices add latency and sometimes refuse the requested format. The error will name the device; try the sound-server PCM.
- Network stalls are reported separately (15 s first audio, 25 s mid-stream). Slow CDN responses can produce a pause before the first chunk; the loading state is real.
- The app is not bit-perfect and does not claim to be: <code>ffmpeg</code> decodes, the app applies software volume, and ALSA may resample through a plugin. If you need bit-perfect output, this is not that player.
- The 32-band meter is analysis only; it cannot cause distortion. There is no EQ in the signal path.

---

## State and settings

### Where they are

| File | Default |
| --- | --- |
| Settings | <code>~/.local/share/gaydar/settings.json</code> (or <code>$XDG_DATA_HOME/gaydar/settings.json</code>) |
| Taste state | <code>~/.local/share/gaydar/state.json</code>, plus <code>.bak</code> and <code>.corrupt-*</code> |
| Client ID | <code>~/.cache/gaydar/client_id.json</code> (or <code>$XDG_CACHE_HOME/gaydar/client_id.json</code>) |
| Artwork | <code>~/.cache/gaydar/art/&lt;track_id&gt;.jpg</code> |

With <code>GAYDAR_DATA_DIR</code> or <code>GAYDAR_CACHE_DIR</code>, the override is used directly; the app does not append <code>gaydar</code>. With <code>GAYDAR_CACHE_DIR</code>, the client ID file and artwork share that directory.

### A corrupt state file

The engine moves a corrupt <code>state.json</code> to <code>state.json.corrupt-&lt;unix&gt;</code>, tries <code>state.json.bak</code>, and starts from defaults if both fail. The app should not crash. If it does, keep the file and the stderr output; that is a bug.

A malformed <code>settings.json</code> is treated as absent. The app falls back to <code>device = "default"</code> and its live default volume.

### Reset taste or settings

Stop the app first.

~~~bash
# Reset taste, keep settings and cache
mv ~/.local/share/gaydar/state.json ~/.local/share/gaydar/state.json.old

# Reset device/volume too
mv ~/.local/share/gaydar/settings.json ~/.local/share/gaydar/settings.json.old

# Full local reset (taste, settings, client ID, artwork)
mv ~/.local/share/gaydar ~/.local/share/gaydar.old
rm -rf ~/.cache/gaydar
~~~

<code>rm -rf</code> is permanent. If you only want to free artwork, delete <code>~/.cache/gaydar/art/</code>; the app re-downloads on demand.

### State does not persist

- Check that the data directory is writable. State saves are best-effort at the call site (<code>let _ = save()</code>), so a read-only home or full disk can let the app run while learning disappears.
- With <code>GAYDAR_DATA_DIR</code>, verify the path exists or is creatable by your user.
- The app writes state after plays/likes/skips, not on every render. Killing it immediately after a click can lose the last event.
- There is no lock. Two instances sharing a directory will overwrite each other.

---

## Artwork

- Artwork is normalised to the 500 px variant, validated by magic bytes, and cached as <code>&lt;track_id&gt;.jpg</code> even if the bytes are PNG/WebP/GIF. That is expected; the extension is a cache key, not a format claim.
- A failed download leaves the gradient/monogram fallback in the UI. It is cosmetic; playback does not depend on it.
- Delete <code>~/.cache/gaydar/art/</code> to force re-download. The app ignores non-image cache files and will replace them.
- Very large images (over 32 MiB) and invalid responses are rejected.

---

## Queue is empty or refill fails

- The queue holds recommendations. It refills when fewer than four items remain, in batches of eight. Bootstrap requests twelve.
- If the pool is exhausted, refill falls back to a full bootstrap. If that fails, the boot/error path reports the reason.
- A station tries a primary query and one fallback, both at a 50-result limit. If both come up short, it reports <code>&lt;name&gt; is quiet right now (&lt;reason&gt;). Try another station.</code> The reason is <code>the search results were all DRM-locked</code> when results existed but none resolved, <code>no playable results for "query"</code> when a search was empty, or the API/network error. Pick a different station or search broader; the MP3-only catalogue is small, and this is supply, not a filter bug.
- Every played, skipped, liked, and already-served track is excluded from recommendations. That is intentional; a small candidate pool can empty quickly. Waiting for new searches or changing dials/station helps.
- Network failures do not erase the existing pool; the UI keeps what it has and reports an error.

---

## Logs and diagnostics

There is no log file. Capture stderr:

~~~bash
cargo run --release 2>&1 | tee /tmp/gaydar.log
~~~

Useful things to include in a report:

- The exact command and whether it was live, <code>--demo</code>, or <code>--screenshot</code>.
- <code>cargo --version</code>, <code>ffmpeg -version</code>, <code>aplay -l</code>.
- The stderr lines starting with <code>[gaydar audio]</code> or <code>[gaydar radio]</code>.
- Whether the same device works with <code>speaker-test</code>.
- For search issues: the query and whether the UI showed "empty" or "error".

Do not attach <code>state.json</code> or screenshots of your history unless you are comfortable sharing them; they contain track titles and listening history.

---

## Quick reset checklist

If you have changed many things and want a known-clean run:

~~~bash
# 1. Stop the app.
# 2. Keep the old state, don't delete it.
mv ~/.local/share/gaydar ~/.local/share/gaydar.old
mv ~/.cache/gaydar ~/.cache/gaydar.old

# 3. Run from the repository root.
cargo run --release

# 4. If audio still fails, try a known PCM explicitly.
GAYDAR_ALSA_DEVICE=default cargo run --release
~~~

If a clean run still fails, the problem is environmental (network, ALSA, ffmpeg, display) or a real bug. Capture the stderr output before changing more settings.
