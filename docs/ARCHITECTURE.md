# GAYDAR FM — Architecture

This document describes the build that ships in this repository: a native Rust application on top of BUI 0.5, with a background SoundCloud worker and a dedicated ALSA audio thread. It is deliberately specific about what is implemented and what is not.

- Entry point: [src/main.rs](../src/main.rs)
- Application state and actions: [src/app.rs](../src/app.rs)
- UI: [src/ui.rs](../src/ui.rs), [src/views.rs](../src/views.rs), [src/theme.rs](../src/theme.rs), [src/extra.rs](../src/extra.rs)
- Network client: [src/soundcloud.rs](../src/soundcloud.rs)
- Recommender: [src/gaydar.rs](../src/gaydar.rs)
- Worker bridge: [src/radio.rs](../src/radio.rs)
- Audio engine: [src/audio.rs](../src/audio.rs)
- Domain types: [src/model.rs](../src/model.rs)

---

## 1. The shape of the program

GAYDAR FM is four execution contexts in one process:

| Context | Created by | Owns | Talks to the UI through |
| --- | --- | --- | --- |
| **UI thread** | BUI's event loop | <code>GaydarApp</code>, the window, assets, fonts | reads a shared event queue and <code>AudioStatus</code> every render |
| **Radio worker** | <code>RadioService::new</code> (<code>std::thread</code>, named <code>gaydar-radio</code>) | <code>SoundCloudClient</code>, <code>GaydarEngine</code> | pushes <code>RadioEvent</code>s and calls BUI's <code>UiSender</code> |
| **Audio worker** | <code>AudioEngine::new</code> (named <code>gaydar-audio</code>) | ALSA PCM handle, <code>ffmpeg</code> child, <code>LevelMeter</code> | shared <code>AudioStatus</code> plus <code>AudioEvent</code> |
| **ffmpeg child** | <code>spawn_ffmpeg</code> on the audio thread | decoding and HTTP/HLS fetching | stdout pipe into a bounded chunk channel |

The UI thread is never allowed to block on the network, ALSA, or a child process. That is the central constraint; almost every design decision below follows from it.

~~~mermaid
flowchart TD
  L["Listener"] --> UI["GaydarApp<br/>UI thread"]
  UI --> VIEWS["ui.rs / views.rs<br/>BUI elements"]
  UI --> THEME["theme.rs / extra.rs<br/>CSS, fonts, icons"]

  subgraph worker["Radio worker: gaydar-radio"]
    R["Worker"] --> SC["SoundCloudClient"]
    R --> GE["GaydarEngine"]
  end

  subgraph audio["Audio worker: gaydar-audio"]
    AE["ThreadEngine"] --> PB["Playback"]
    PB --> PCM["ALSA PCM"]
    PB --> FF["ffmpeg child"]
    FF -->|stdout, 4 KiB chunks| Q["bounded sync_channel<br/>capacity 8"]
    Q --> PB
  end

  UI -->|"RadioCommand (mpsc)"| R
  R -->|"RadioEvent queue + UiSender"| UI
  UI -->|"AudioCommand (mpsc)"| AE
  AE -->|"AudioStatus / AudioEvent"| UI

  SC -->|"HTTPS"| NET["soundcloud.com<br/>api-v2.soundcloud.com<br/>a-v2.sndcdn.com<br/>media + artwork CDNs"]
  GE -->|"atomic JSON"| STATE["state.json"]
  SC -->|"atomic write"| ART["artwork cache"]
  SC -->|"atomic write"| CID["client_id.json"]
~~~

### Commands and events

**RadioCommand** (UI → radio): <code>Bootstrap</code>, <code>Refill</code>, <code>Station</code>, <code>Search</code>, <code>Play</code>, <code>Played</code>, <code>Complete</code>, <code>Skip</code>, <code>Like</code>, <code>Unlike</code>, <code>SetDials</code>, <code>Artwork</code>, <code>Retry</code>, <code>Shutdown</code>.

**RadioEvent** (radio → UI): <code>Ready</code>, <code>Bootstrap</code>, <code>Refill</code>, <code>Station</code>, <code>Search</code>, <code>StreamReady</code>, <code>StreamError</code>, <code>Artwork</code>, <code>ArtworkFailed</code>, <code>Taste</code>, <code>Error</code>, <code>Info</code>.

**AudioCommand** (UI → audio): <code>Play</code>, <code>Stop</code>, <code>Pause</code>, <code>Resume</code>, <code>Toggle</code>, <code>Seek</code>, <code>SetVolume</code>, <code>SetDevice</code>, <code>Shutdown</code>.

**AudioEvent** (audio → UI): <code>Finished(track_id)</code>, <code>Error(message)</code>. The continuous state is <code>AudioStatus</code>: playback state, track id, position, duration, volume, device, sample rate, channels, error, and 32 analyser bands.

The radio events live in an <code>Arc&lt;Mutex&lt;VecDeque&lt;RadioEvent&gt;&gt;&gt;</code>; the worker pushes one and calls <code>UiSender::send(|cx| cx.notify_all())</code>. <code>GaydarApp::pump</code> drains that queue at the start of every render, then reads the audio status snapshot and drains audio events. No UI callback waits for I/O.

---

## 2. Startup and data flow

~~~mermaid
sequenceDiagram
  autonumber
  actor L as Listener
  participant UI as GaydarApp
  participant R as Radio worker
  participant SC as SoundCloud
  participant A as Audio worker
  participant F as ffmpeg
  participant ALSA as ALSA PCM

  L->>UI: click a track
  UI->>R: RadioCommand::Play with token, track, reason
  R->>SC: GET /tracks/id
  SC-->>R: track JSON with media.transcodings
  R->>SC: transcoding URL + track_authorization
  SC-->>R: signed media URL, or StreamUnavailable
  R->>SC: Range probe bytes=0-2047
  alt playable
    R-->>UI: RadioEvent::StreamReady with token and source
    UI->>A: AudioCommand::Play
    A->>F: spawn ffmpeg ... -f s16le -ar 48000 -ac 2 -
    F-->>A: PCM chunks in a bounded channel
    A->>ALSA: write s16le stereo frames
    A-->>UI: AudioStatus with position and levels
  else DRM-only or probe failed
    R-->>UI: RadioEvent::StreamError with unavailable flag
    UI->>UI: skip if unavailable, otherwise show the error
  end
~~~

### 2.1 Launch

1. <code>main</code> parses the tiny CLI (<code>--screenshot</code>, <code>--screen</code>, <code>--width</code>, <code>--height</code>, <code>--demo</code>). With <code>--screenshot</code> it opens <code>Application::headless()</code>; otherwise <code>Application::new().gpu(true)</code>, which BUI may fall back to software raster.
2. It loads fonts with <code>theme::load_fonts()</code> before creating state. Missing font files are skipped; the CSS font stack falls back.
3. If a screenshot, <code>--demo</code>, or <code>--screen boot</code> is in play, the state is <code>GaydarApp::demo(screen)</code>; otherwise <code>GaydarApp::live()</code>. Screenshot mode never calls <code>start_live</code>.
4. For live state, <code>start_live</code> resolves the data and cache directories, creates them, loads <code>settings.json</code>, applies <code>GAYDAR_ALSA_DEVICE</code> if present, enumerates ALSA hints, creates the audio engine, creates the radio service, and sends <code>Bootstrap { count: 12 }</code>.
5. On the radio thread, <code>initialize</code> creates <code>SoundCloudClient::new()</code> (cache or discovery) and <code>GaydarEngine::new(client, data_dir/state.json)</code>, then emits <code>Ready { client_id }</code>. A failure emits <code>Error</code> and the boot screen keeps its retry button; the app does not fake a home screen.

### 2.2 First queue

<code>GaydarEngine::bootstrap(count)</code>:

1. Prune the pool.
2. If there are fewer candidates than requested, run four seed queries in parallel: a strong artist (weight above 1.2), a strong tag (above 0.8), a random built-in artist, and <code>queer anthem</code>, filled from the weighted scene seeds. The whole bootstrap has an 18 s budget; each fetch batch is capped at 10 s.
3. If fewer than <code>min(count, 3)</code> candidates are available, try the trending chart, then one unused fallback query from <code>pride / queer / queer punk / drag queen / sapphic</code>.
4. Score and select <code>count</code> tracks (see section 4).
5. Verify up to the first three picks by calling <code>resolve_stream</code> in parallel with at most 8 s. Confirmed picks move to the front; <code>StreamUnavailable</code> (DRM-only or withdrawn) is dropped; transient failures stay.
6. Return <code>Recommendation</code>s with reason and score. <code>radio.rs</code> then tries to resolve up to the first four picks for autoplay, and emits <code>Bootstrap</code> with the queue, autoplay, liked, history, taste, top artists, play count, and dials.

The UI shows the boot screen until that event arrives. On success it replaces the queue, asks the radio for artwork for queued tracks, and either starts the verified autoplay or requests the first queue item. On failure it leaves the boot screen with the error and a retry.

Known flaw: if every probed pick is <code>StreamUnavailable</code> and the batch has three or fewer items, <code>verify_bootstrap_picks</code> returns the original pre-verification batch as a fallback. The pool filter normally prevents DRM-only tracks from reaching this point, but this fallback can re-admit one. It is listed in [DEVELOPMENT.md](DEVELOPMENT.md#10-known-gaps).

### 2.3 Playing and learning

- <code>GaydarApp::request_play</code> increments <code>playback_token</code>, stops the current audio, and sends <code>Play { token, track, reason }</code>. The token makes stale <code>StreamReady</code>/<code>StreamError</code> events harmless.
- The worker resolves the stream; <code>StreamReady</code> is the only path that hands a URL to the audio engine. On success the UI calls <code>AudioEngine::play</code>, sends <code>Played(track)</code>, and asks for more queue if fewer than four items remain.
- The worker's <code>Played</code> handler calls <code>record_play</code> and emits <code>Taste</code>; <code>Complete</code> and <code>Skip</code> are sent by <code>advance_live(true/false)</code> and similarly update state.
- When the audio thread reaches EOF it emits <code>Finished(track_id)</code>. The UI only advances if that id is still current and it is not in a loading transition, then sends <code>Complete</code> and requests/plays the next item.
- A <code>StreamError</code> with <code>unavailable = true</code> (typed <code>StreamUnavailable</code>) auto-skips up to three consecutive failures; after that, or for transient errors, the error is shown and playback stops.
- An ALSA/ffmpeg error (<code>AudioEvent::Error</code>) is shown as a banner/toast and logged to stderr; it does not silently advance.

### 2.4 Stations

A station is a **primary query plus one fallback** (<code>Station::query</code> and <code>Station::fallback_query</code>). <code>GaydarApp::apply_station</code> applies the station's dials immediately and sends <code>RadioCommand::Station { name, queries: vec![primary, fallback], dials }</code>. The radio worker calls <code>search_tracks(query, 50)</code> for the primary and then the fallback. For each query it maps up to 12 picks and resolves the first up to four; it stops at the first query that yields a resolvable autoplay. If neither query yields one, it emits an error of the form <code>&lt;name&gt; is quiet right now (&lt;reason&gt;). Try another station.</code>

A station is not a different recommender: it is a curated search plus the same dials, and every result still passes the MP3/policy/duration filters.

### 2.5 Artwork

<code>ensure_art</code> checks the in-memory <code>art_cache</code>, the pending set, and <code>cache_dir/&lt;track_id&gt;.jpg</code>; if absent, it sends <code>Artwork { track_id, url, dest }</code>. The radio worker calls <code>fetch_artwork</code>, which normalises the URL to <code>-t500x500</code>, downloads at most 32 MiB, validates JPEG/PNG/WebP/GIF magic, writes atomically, and returns the path. The UI stores it in <code>art_cache</code>; BUI's <code>img</code> then loads it from disk.

### 2.6 Screenshot data flow

<code>--screenshot</code> bypasses all of the live path. <code>main</code> opens a headless BUI window, builds <code>GaydarApp::demo(screen)</code>, draws once, converts BUI's <code>0x00RRGGBB</code> pixels to RGBA, and writes a PNG with <code>png 0.18</code>. There is no network request, no ALSA device, and no audio engine. The demo state contains twelve tracks, a five-item queue, likes, history, taste labels, fake levels, and fixed playback state. This is why screenshots are deterministic but not evidence of the live service.

<code>--live-shot</code> is the headless live path. It builds <code>GaydarApp::live()</code>, calls <code>start_live</code>, pumps the app until <code>PlaybackState::Playing</code> or a 45 s deadline, waits up to 5 s more for artwork, optionally switches to <code>NowPlaying</code>, then draws one frame. It still has no visible window or GPU, but it does use the network, <code>ffmpeg</code>, and ALSA; an error or a silent stream produces a non-playing frame and a stderr note rather than a failed exit.

---

## 3. The SoundCloud client

Source: [src/soundcloud.rs](../src/soundcloud.rs).

### 3.1 Public endpoints used

All API requests go to <code>https://api-v2.soundcloud.com</code> with <code>client_id</code> appended. The endpoints are read-only:

| Endpoint | Used for |
| --- | --- |
| <code>GET /search/tracks?q=…&amp;limit=…</code> | Search, seed queries, tag searches, fallback. |
| <code>GET /tracks/{id}</code> | Track details, media/transcodings, policy. |
| <code>GET /tracks/{id}/related?limit=…</code> | Related candidates; 404 is treated as an empty list. |
| <code>GET /users/{id}/tracks?limit=…</code> | Artist uploads (also a related-track fallback). |
| <code>GET /charts?kind=trending&amp;limit=…</code> | Cold-start fallback only; the <code>top</code> chart kind is not public and is not used. |
| <code>GET &lt;transcoding URL&gt;</code> | Signed media URL; passes <code>track_authorization</code> when the track object has one. |
| <code>GET https://soundcloud.com/</code> | Client ID discovery. |
| <code>GET a-v2.sndcdn.com/assets/*.js</code> | Client ID discovery in bundles. |
| <code>GET i1.sndcdn.com/…</code> | Artwork. |

The client sends <code>Origin: https://soundcloud.com</code>, <code>Referer: https://soundcloud.com/</code>, and a Chrome user agent. It does not use cookies, OAuth, or an account token.

### 3.2 Client ID discovery and refresh

1. Read <code>client_id.json</code> if it parses and normalises; trust it without a network round trip.
2. Otherwise <code>discover_with</code>:
   - Fetch <code>https://soundcloud.com/</code> (4 MiB cap).
   - Regex for <code>client_id:"&lt;32 alphanumeric&gt;"</code>, <code>client_id=…</code>, and URL-encoded variants.
   - Validate each candidate with <code>GET /search/tracks?q=pride&amp;limit=1</code>. A 200 accepts; 401/403 rejects; other statuses fail the validation.
   - If the page has no ID, collect up to 12 <code>a-v2.sndcdn.com/assets/*.js</code> URLs, fetch them in parallel (8 MiB each), scan and validate. The scan has a 12 s deadline and stops at the first good ID.
3. If extraction fails, try three built-in public IDs, each validated the same way.
4. Cache the accepted ID with <code>source</code> (<code>extracted</code>, <code>fallback</code>, or <code>cache</code>) and <code>saved_at</code>.

On 401/403, <code>api_get</code> calls <code>refresh_after_auth_failure</code> once per request. A generation counter prevents parallel requests from refreshing repeatedly after one of them has already rotated the ID. If the refresh fails, the request returns <code>ApiError::Auth</code> and the UI surfaces it.

Normalisation requires 8–128 characters, ASCII alphanumeric plus <code>-</code>/<code>_</code>. The cache write is atomic.

### 3.3 Pacing, retries, timeouts

- A process-wide <code>RateLimiter</code> spaces requests at **5 per second** (one slot every 200 ms), shared by all clients and discovery threads.
- Request timeout **15 s**, connect timeout **10 s**.
- Up to **2 retries** for transport errors, timeouts, 429, and 5xx. Backoff is 250 ms × 2^attempt, capped by <code>attempt.min(4)</code>. <code>Retry-After</code> is honoured up to 3 s.
- A JSON decode error is <code>ApiError::Decode</code>; a 404 is <code>ApiError::NotFound</code>; other statuses are <code>ApiError::Status</code> with a 240-character body excerpt.

### 3.4 Normalisation and the playable filter

<code>parse_track</code> maps SoundCloud JSON to <code>Track</code>:

- <code>id</code> must be non-zero; <code>title</code> is trimmed; a missing artist becomes <code>Unknown Artist</code>.
- <code>duration</code> falls back to <code>full_duration</code>.
- Artwork/avatar URLs are normalised from any of <code>-large.</code>, <code>-t500x500.</code>, <code>-t300x300.</code>, <code>-t200x200.</code>, <code>-t120x120.</code>, <code>-t60x60.</code>, <code>-small.</code>, <code>-tiny.</code>, <code>-mini.</code>, <code>-badge.</code> to <code>-t500x500.</code>.
- <code>tag_list</code> is parsed tolerantly: quoted runs are one tag; the unquoted remainder splits on commas if commas exist, otherwise on whitespace. Tags are lowercased, deduplicated, each tag is truncated to 48 characters, and the list is capped at 12.
- <code>policy</code> defaults to <code>ALLOW</code>; <code>streamable</code> defaults to true; playback/like/follower counts default to 0.

<code>is_playable</code> enforces:

- <code>id != 0</code> and a non-empty title;
- <code>streamable</code>;
- policy not containing <code>BLOCK</code> or <code>SNIP</code> (case-insensitive);
- duration in <code>45_000..=2_700_000</code> ms (45 s–45 min).

List endpoints (<code>search_tracks</code>, <code>related_tracks</code>, <code>user_tracks</code>, <code>trending_tracks</code>) additionally keep only tracks with a playable MP3 transcoding. If an item has no <code>media.transcodings</code>, up to eight detail lookups are performed for that list; beyond that the item is dropped. Results are deduplicated by id.

Before the transcoding check, <code>collection_tracks</code> also calls <code>looks_like_music</code>. That narrow filter rejects obvious talk/podcast uploads when the title or description contains a talk term (<code>podcast</code>, <code>interview</code>, <code>oral history</code>, <code>talk show</code>, <code>panel discussion</code>, <code>lecture</code>, <code>audiobook</code>, <code>radio show</code>, <code>episode</code>, <code>ep. </code>, <code>discussion</code>, <code>review</code>), when the genre contains <code>podcast</code>/<code>talk</code>/<code>spoken word</code>/<code>audiobook</code>/<code>comedy</code>, when the tags contain <code>podcast</code>/<code>interview</code>/<code>talk show</code>, or when title/description/tags contain one of six blocked slurs. It is deliberately conservative and is not applied to a direct track lookup.

### 3.5 Transcoding classification

<code>Transcoding::is_mp3</code>:

- If a non-empty <code>preset</code> exists, it is MP3 only when the preset starts with <code>mp3</code>.
- Otherwise the <code>mime_type</code> must contain <code>mpeg</code>.
- <code>snipped == true</code> is never usable.
- <code>is_snipped</code> defaults to false.

<code>stream_support_from_value</code> classifies:

| Condition | <code>StreamSupport</code> |
| --- | --- |
| No non-snipped transcodings | <code>Unavailable</code> |
| Non-snipped transcodings exist, none is MP3 | <code>DrmOnly</code> |
| At least one MP3 progressive | <code>Mp3Progressive</code> |
| Only MP3 HLS | <code>Mp3Hls</code> |
| Otherwise | <code>DrmOnly</code> |

### 3.6 Stream resolution

<code>resolve_stream(track_id)</code>:

1. Fetch the track (memory cache, 10 min TTL, 256 entries).
2. Reject policy-blocked or non-streamable tracks with a string error.
3. Classify support; non-playable support returns a typed <code>StreamUnavailable { track_id, title, support }</code>. This is the error the worker downcasts to decide "skip, never retry".
4. Collect <code>is_mp3() &amp;&amp; !is_snipped()</code> transcodings, sort by <code>(snipped_rank, protocol_rank, codec_rank)</code> — progressive before HLS, MP3 before other codecs.
5. For each transcoding, call its URL with <code>track_authorization</code> and read the returned <code>url</code>.
6. Probe that URL with <code>Range: bytes=0-2047</code>:
   - Progressive: at least 128 bytes and starts with an MPEG frame sync (0xFF) or <code>ID3</code>.
   - HLS: starts with <code>#EXTM3U</code> and does not contain a real <code>#EXT-X-KEY</code>/<code>#EXT-X-SESSION-KEY</code> (a line with <code>METHOD=NONE</code> is tolerated).
7. Return <code>StreamSource { url, kind }</code> for the first URL that passes. Otherwise fail with the collected reasons.

This is why a URL is never handed to <code>ffmpeg</code> unverified. The probe is a 2 KiB range request, not a full download.

### 3.7 Artwork

<code>fetch_artwork(url, dest)</code>:

- Reuses <code>dest</code> if it already contains a supported image.
- Requires an absolute HTTP(S) URL.
- Downloads with up to two retries, a 32 MiB cap, and a 15 s request timeout.
- Validates JPEG (FF D8 FF), PNG, GIF87a/89a, or RIFF/WEBP magic.
- Creates parent directories and writes <code>.&lt;name&gt;.&lt;pid&gt;.&lt;nanos&gt;.tmp</code> atomically, then renames.
- On failure, removes the temporary file and reports the last error.

### 3.8 In-memory caches

- Client ID: persisted JSON; not revalidated at startup.
- Track objects: <code>HashMap&lt;u64, CachedTrackValue&gt;</code>, TTL 10 min, max 256; purged or cleared when full.
- No audio cache. Every play streams from the CDN; artwork is the only cached media.

---

## 4. The recommendation engine

Source: [src/gaydar.rs](../src/gaydar.rs). The engine is synchronous and called only from the radio worker. Network work is bounded by deadlines; no UI thread ever calls it.

### 4.1 Persisted taste model

<code>state.json</code> (v1) holds:

- <code>liked</code>: newest last, max 500.
- <code>history</code>: <code>{ track, played_at, fraction, outcome }</code>, max 300. <code>outcome</code> is <code>started</code>, <code>completed</code>, or <code>skipped</code>.
- <code>tag_weights</code>, <code>artist_weights</code>: <code>BTreeMap&lt;String, f32&gt;</code>, clamped to ±10.
- <code>skipped</code>: up to 3000 ids.
- <code>dials</code>: Camp/Club/Soft/Chaos, each 0–1.
- <code>plays</code>, <code>completions</code>, <code>version</code>, <code>last_updated</code>.

<code>load_state</code> degrades safely: unreadable/corrupt JSON is moved to <code>state.json.corrupt-&lt;unix&gt;</code>, the <code>.bak</code> is tried, and a missing model becomes a fresh default. <code>sanitize_state</code> clamps dials and weights, drops empty/invalid keys, deduplicates ids, caps lists, clamps fractions, and applies decay. <code>save</code> writes a temp file, syncs, copies the previous state to <code>.bak</code>, and renames.

### 4.2 Seed and candidate sources

Built-in seed groups (hard-coded; <code>assets/seeds.json</code> is not read by the engine):

- **Core:** <code>queer anthem</code>, <code>queer</code>, <code>lgbtq</code>, <code>pride</code>, <code>drag queen</code>, <code>ballroom</code>, <code>hyperpop</code>, <code>sapphic</code>, <code>wlw</code>, <code>gay club</code>, <code>trans joy</code>, <code>queercore</code>, <code>vogue</code>, <code>camp</code>, <code>queer pop</code>, <code>queer electronic</code>.
- **Camp:** camp, drag queen, pride anthem, queer pop, diva pop, hyperpop, drag race, Eurovision queer, bubblegum pop.
- **Club:** gay club, ballroom, vogue, queer house/techno, ballroom beats, pride, Honey Dijon, Jodie Harsh, Gorgon City.
- **Soft:** sapphic, wlw, lesbian indie, queer folk, soft queer, trans joy, queer R&amp;B, sad queer, Kelela, Perfume Genius.
- **Chaos:** hyperpop, queercore, queer punk, experimental queer, SOPHIE, Arca, 100 gecs, deconstructed club, glitch queer.
- **Artists:** 49 anchors from Chappell Roan, Rina Sawayama, MUNA, Troye Sivan, SOPHIE, Kim Petras, Slayyyter, Pabllo Vittar, Arca, Kelela, Shygirl, COBRAH, Jodie Harsh, Honey Dijon, The Blessed Madonna, Romy, Perfume Genius, Anohni, Christine and the Queens, Tegan and Sara, Beth Ditto, Gossip, Syd, Frank Ocean, Steve Lacy, Lady Gaga, Madonna, Charli XCX, Caroline Polachek, Shura, Mabel, and others. These are both search seeds and a strong queer signal.

Candidates are attached to a <code>CandidateSource</code> that carries a score bonus:

| Source | Bonus |
| --- | --- |
| Seed search | 0.00 |
| Related | +0.12 |
| Artist uploads | +0.06 |
| Tag search | +0.05 |
| Trending chart | +0.03 |

The pool is capped at 400, pruned when older than 6 hours, and filtered through <code>is_playable</code>, the exclusion set (history, skipped, liked, served), and the MP3-capable list rules at fetch time.

### 4.3 Scoring

~~~text
score = 0.42
      + 0.30 * tag_affinity
      + 0.24 * artist_affinity
      + 0.20 * queer
      + 0.07 * popularity
      + 0.10 * freshness
      + 0.20 * (fit - 0.35)
      + source_bonus
      + unseen_artist_bonus
      + exploration_bonus
      + jitter
      - long_form_penalty
      - 0.055 * recent_artist_count
score = clamp(score, 0.0, 3.0)
~~~

With:

- <code>tag_affinity = tanh(sum(weights for the track's tag_keys) / (4 * sqrt(count)))</code>, where <code>tag_keys</code> is tags plus genre, and <code>count</code> is the number of tags that have a weight. No weights means 0.
- <code>artist_affinity = tanh(artist_weight / 4)</code>; artist lookup falls back to a case-insensitive match.
- <code>queer = clamp(1 - exp(-(0.75 * strong_hits + 0.25 * mild_hits)) + 0.55 * (artist in ARTIST_SEEDS), 0, 1)</code>. Hits are whole-word regex matches over title, artist, genre, tags, and description.
- <code>popularity = 0.7 * log_norm(playback_count, 6.5) + 0.2 * log_norm(likes_count, 5.0) + 0.1 * log_norm(user_followers, 6.0)</code>, with <code>log_norm(v, ceiling) = clamp(log10(1 + v) / ceiling, 0, 1)</code>.
- <code>freshness = exp(-age_days / 540)</code>, where age comes from <code>created_at</code> parsed as RFC 3339 and measured against UTC now; unparsable dates give 0.
- <code>fit = (sum dial_i * category_i) / (sum dial_i + 0.001)</code>, with category scores from the Camp/Club/Soft/Chaos keyword regexes. Each category score is <code>1 - exp(-0.9 * hits)</code>, clamped.
- <code>unseen_artist_bonus = 0.05</code> when the artist has zero learned weight and never appears in history.
- <code>explore</code> is true with probability <code>0.06 + 0.18 * chaos</code>; then <code>exploration_bonus = uniform(0.02..0.14)</code>, otherwise 0.
- <code>jitter = uniform(-0.025..0.025)</code>.
- <code>long_form_penalty = 0.12</code> above 15 minutes.
- <code>recent_artist_count</code> = occurrences of the artist in the last 12 history entries.

### 4.4 Selection

1. Score every non-excluded, playable candidate.
2. Sort descending.
3. Compute distinct artists in the scored set. <code>max_per_artist = 1</code> when there are at least <code>count</code> distinct artists; otherwise <code>max(2, ceil(count / distinct_artists))</code>.
4. Walk the sorted list, skipping ids already picked and artists at their cap.
5. If fewer than <code>count</code> picks remain, walk again without the artist cap.
6. Record all picks in <code>served</code>, remove them from the pool, and reset <code>served</code> if it exceeds 5000.

The function can return fewer than <code>count</code> when the pool is exhausted; it does not pad the result with DRM or unplayable tracks.

The exclusion set is deliberately broad: any track in history, any skipped track, any liked track, and any track already served this session. A skipped track can return only after it falls out of the 3000-id cap or a new state; a liked track is excluded from recommendations and is available from the Liked screen.

### 4.5 Reasons

<code>reason_for</code> picks the first applicable line:

1. <code>certified ballroom</code> — ballroom/vogue/house ball tags and queer at least 0.3.
2. <code>because you keep replaying &lt;artist&gt;</code> — artist weight at least 2.0 or the artist is in liked tracks.
3. <code>more &lt;tag&gt; in your veins</code> — best tag weight at least 1.4.
4. <code>more like &lt;artist&gt;</code> — source is Related.
5. <code>camp canon</code> — Camp at least 0.55 and camp score at least 0.4.
6. <code>soft sapphic detour</code> — Soft at least 0.5, soft score at least 0.4, queer at least 0.2.
7. <code>chaos dial, no notes</code> — Chaos at least 0.45 and chaos score at least 0.4.
8. <code>fresh out the group chat</code> — freshness at least 0.55.
9. <code>the gaydar says explore</code> — exploration draw.
10. <code>the gaydar says yes</code> — queer at least 0.35.
11. <code>right on the dial</code> — fit at least 0.5.
12. Source-specific: <code>deep cut from &lt;artist&gt;</code>, <code>tagged &lt;tag&gt;, kept it</code>, <code>dug out of the &lt;query&gt; crate</code>.
13. <code>&lt;genre&gt; with a queer accent</code>, or <code>queer radio staple</code>.

The reason is computed from the same signals as the score, so it is an explanation, not a decorative tagline.

### 4.6 Learning and decay

<code>reinforce(track, tag_delta, artist_delta)</code> spreads the tag delta over <code>sqrt(tag_count)</code> and adds the artist delta to the artist key. Deltas:

| Event | Tag delta | Artist delta |
| --- | --- | --- |
| Play starts | +0.18 | +0.12 |
| Complete | +1.4 | +1.2 |
| Skip under 35% | −1.8 | −1.4 |
| Skip at 35% or later | −0.15 | −0.1 |
| Like | +2.6 | +2.2 |
| Unlike | −1.4 | −1.2 |

<code>adjust_weight</code> clamps each weight to ±10. <code>prune_weights</code> keeps the 350 strongest when tags exceed 500, and the 300 strongest when artists exceed 400. On load, if more than a day has passed, weights are multiplied by <code>0.99^(min(days / 7, 52))</code>; <code>last_updated</code> is then set to now.

### 4.7 Bootstrap verification and refill

- Bootstrap count is clamped to 1–50; the app requests 12, retries request 10.
- The first 3 picks are verified in parallel; the verify budget is the remaining bootstrap budget capped at 8 s.
- Refill clamps to 1–50, uses a 10 s budget, and skips network work when the pool already has enough and the last refill was under 90 s ago. <code>record_complete</code>, <code>record_skip</code>, <code>record_like</code>, and <code>record_unlike</code> clear <code>last_refill</code>, so the next refill fetches taste-derived jobs; <code>record_play</code> does not.
- Refill jobs: up to 2 related seeds, up to 2 artist-upload jobs, 1 top tag, 1 random top tag from the top 6, and 1 weighted exploration search, capped at 6. If fewer than two jobs exist before the exploration search, up to 2 bootstrap queries are added.
- If selection is empty after refill, refill falls back to a full bootstrap.
- Network failures never erase an existing pool; they are reported as <code>Error</code> events but the UI keeps what it has.

---

## 5. The audio engine

Source: [src/audio.rs](../src/audio.rs). This is the only module that touches ALSA or spawns <code>ffmpeg</code>.

### 5.1 Thread loop

<code>ThreadEngine::run</code>:

- Drain all pending commands.
- If no playback: wait up to 100 ms for a command.
- If paused: wait up to 40 ms for a command; on timeout, decay levels and publish the frozen position.
- If playing: take the <code>Playback</code>, call <code>pump</code>, and put it back on <code>Continue</code>; on <code>Eof</code>, publish Idle and send <code>Finished(track_id)</code>; on <code>Error</code>, publish Error and send <code>Error(message)</code>.
- On shutdown or channel disconnect, stop playback and return.

<code>pump</code> waits up to 12 ms for a 4 KiB chunk. A timeout is normal while <code>ffmpeg</code> buffers; it publishes position/levels and paces. Commands are still drained between pumps, so pause/seek/stop are responsive.

### 5.2 Opening ALSA

<code>Playback::start</code> calls <code>open_pcm_with_fallback(device)</code>, which tries 48 kHz then 44.1 kHz:

- <code>PCM::new(device, Playback, false)</code>
- <code>HwParams::any</code>
- interleaved access, <code>Format::s16()</code>, 2 channels
- requested rate with <code>ValueOr::Nearest</code>; rejects the rate if the negotiated rate differs
- period near 512 frames, buffer at least max(2048 frames, 4× the period)
- <code>hw_params</code>, then <code>prepare</code>

If both rates fail, the error includes both attempts. This is the message users see when a device cannot do 16-bit stereo at 48/44.1 kHz; <code>default</code>, <code>pipewire</code>, <code>pulse</code>, or <code>plughw:</code> usually can.

### 5.3 ffmpeg invocation

For network URLs the argument list is:

~~~text
ffmpeg -nostdin -hide_banner -loglevel error
       -user_agent <Chrome UA>
       -headers <Referer and Origin, CRLF-terminated>
       [-ss <seconds>]
       -i <url>
       -vn -f s16le -ac 2 -ar <48000|44100> -
~~~

For non-HTTP URLs (including <code>file://</code>) the HTTP options are omitted (the file demuxer rejects them). <code>-ss</code> is placed before <code>-i</code> for fast seeking. stdin is null, stdout is a pipe, stderr is a pipe.

Two threads are spawned per playback:

- <code>gaydar-ffmpeg-out</code>: reads stdout in 4 KiB blocks into a <code>sync_channel(8)</code>; if the audio thread stops draining, the reader blocks, which backpressures <code>ffmpeg</code>.
- <code>gaydar-ffmpeg-err</code>: reads up to 16 KiB of stderr, normalises whitespace, and keeps up to 400 characters for the UI error.

The child is killed and reaped on stop, seek, device switch, playback drop, and engine drop. <code>reap_child</code> waits up to 2 s after EOF before killing a child that closed stdout but stayed alive.

### 5.4 Position, pacing, pause, seek, device switch

- **Position** = <code>base_ms + 1000 × (frames_written − pcm.delay) / sample_rate</code>, clamped to duration. At natural EOF, the final position uses all written frames because ALSA has drained.
- **Pacing** compares audio position to wall time since the last reset. If audio is more than 40 ms ahead, it sleeps by the excess capped at 40 ms. This keeps a fast sink from racing through a track after the reported duration.
- **Pause** freezes the position, calls <code>pcm.pause(true)</code> when supported, and decays the meter. **Resume** unpauses or re-prepares on XRun/Suspended, then resets pacing.
- **Seek** clamps the target, terminates the current <code>ffmpeg</code>, drops and re-prepares the PCM, sets <code>base_ms</code> to the target, clears buffers/meter, and spawns a new <code>ffmpeg</code> with <code>-ss</code>. The position is immediate; audio starts when the first chunk arrives.
- **Device switch** stores the new device, stops the old playback, and starts a new one at the previous position with the previous pause state. A failure is reported through <code>AudioEvent::Error</code>.

### 5.5 Error detection

| Condition | Result |
| --- | --- |
| No first PCM byte within 15 s | Error: "ffmpeg produced no audio within 15 seconds…" and child killed. |
| 25 s without a chunk after audio started | Error: "audio stream stalled for 25 seconds while unpaused; ffmpeg was stopped". |
| <code>ffmpeg</code> exits non-zero | Error includes exit status and stderr tail. |
| <code>ffmpeg</code> exits 0 but wrote no audio at the start | Error: "ffmpeg decoded no audio for this stream". |
| ALSA write fails and recovery fails | Error includes the device and recovery failure. |
| ALSA accepts zero frames repeatedly | Error after 8 retries. |

A 250 ms gap resets pacing; it is not treated as a stall.

### 5.6 The level meter

<code>LevelMeter</code> is a display analyser, not a filter:

- 32 log-spaced bands from 45 Hz to 16 kHz, ratio <code>(16000/45)^(1/32)</code>.
- Hann window of 1024 mono samples, hop 512.
- Three Goertzel probes per band at logarithmically spaced frequencies; the strongest probe's power is used.
- Amplitude is normalised for the window's coherent gain; dB is tilted by up to +14 dB across the bands.
- The display range is −66 dB to −5 dB; attack smoothing is 0.55, release 0.10; pause decay is 0.88.
- The meter runs on the audio thread from the same samples that go to ALSA. It never writes back to the PCM.

Software volume is applied per interleaved frame with a linear ramp within each block, and the gain target is <code>volume^2</code>. This is a perceptual curve, not a calibrated dB control.

### 5.7 ALSA device enumeration

<code>AudioEngine::list_devices</code> walks ALSA playback hints, skips capture-direction hints, cleans duplicate whitespace in descriptions, de-duplicates by PCM name, and always prepends <code>default</code> with the default hint's description (or "Default ALSA output"). The Settings UI renders the whole list in a scrollable panel; <code>GaydarApp::refresh_devices</code> re-enumerates it from the REFRESH action.

---

## 6. UI and application state

Source: [src/app.rs](../src/app.rs), [src/ui.rs](../src/ui.rs), [src/views.rs](../src/views.rs), [src/theme.rs](../src/theme.rs), [src/extra.rs](../src/extra.rs).

- <code>Screen</code> is <code>Home</code>, <code>Gaydar</code>, <code>Search</code>, <code>Liked</code>, <code>History</code>, <code>Settings</code>, <code>NowPlaying</code>. <code>root</code> renders the app shell plus the player for all except <code>NowPlaying</code>, which replaces the shell with the immersive view. If <code>booting &amp;&amp; !demo</code>, the boot screen replaces everything.
- <code>GaydarApp</code> owns all UI state, including <code>current</code>, <code>queue</code>, <code>history</code>, <code>liked</code>, <code>taste</code>, <code>top_artists</code>, dials, playback status, ALSA devices, search state, toasts, and the demo flag.
- Demo and live state share the same rendering path. <code>demo()</code> supplies twelve tracks, a queue, likes, history, and fake levels; <code>live()</code> starts from an empty shell and <code>start_live</code> brings up the real services.
- <code>theme.rs</code> has the palette tokens and the main stylesheet; <code>extra.rs</code> adds the second-half CSS (screens, player, Now Playing, boot, radar, small fixes). Fonts are registered from <code>assets/fonts</code> only when the files exist. Icons are SVG masks; missing icons degrade to a small outlined square.
- The window requests animation frames while playing, loading, booting, showing a toast, or on the Gaydar screen (radar sweep). It does not animate continuously when idle.
- UI test hooks: <code>search-field</code>, <code>nav-home</code>, <code>play-toggle</code>, <code>queue-toggle</code>, <code>queue-close</code>, <code>seek-bar-N</code>, <code>station-ballroom</code>, <code>player-art</code>. The headless test in [tests/ui_interactions.rs](../tests/ui_interactions.rs) drives those ids and the global Space shortcut.

The normal app shell (the boot screen is separate) installs a global <code>on_key_down</code> handler: Space toggles play/pause, Right/Left go next/previous, <code>/</code> opens Search, <code>q</code> toggles the queue drawer (except on Now Serving), <code>l</code> likes/unlikes the current track, and Escape closes the queue, leaves Now Serving, or dismisses an error. When the search field owns focus, the global handler returns early and the field handles typing, Backspace, Enter, and Escape itself. The Settings shortcuts card lists the transport and search subset; <code>q</code>, <code>l</code>, and Escape also work but are not shown there.

---

## 7. Persistence and file formats

### 7.1 Locations

| Purpose | Default | Override |
| --- | --- | --- |
| Settings | <code>$XDG_DATA_HOME/gaydar/settings.json</code> | <code>GAYDAR_DATA_DIR/settings.json</code> |
| Taste | <code>$XDG_DATA_HOME/gaydar/state.json</code> (+ <code>.bak</code>, <code>.corrupt-*</code>) | <code>GAYDAR_DATA_DIR/state.json</code> |
| Client ID | <code>$XDG_CACHE_HOME/gaydar/client_id.json</code> | <code>GAYDAR_CACHE_DIR/client_id.json</code> |
| Artwork | <code>$XDG_CACHE_HOME/gaydar/art/&lt;track_id&gt;.jpg</code> | <code>GAYDAR_CACHE_DIR/&lt;track_id&gt;.jpg</code> |

<code>dirs</code> supplies the XDG defaults. If the override is set it is used as-is; there is no extra subdirectory. With <code>GAYDAR_CACHE_DIR</code>, artwork and the client ID share one directory.

### 7.2 Settings

~~~json
{
  "device": "default",
  "volume": 0.78
}
~~~

Both fields are optional on read. The device is the ALSA PCM name, not the human description. Settings are written when the device or volume changes, not on every frame.

### 7.3 Client ID

~~~json
{
  "client_id": "…32 alphanumeric characters…",
  "saved_at": 1794160000,
  "source": "extracted"
}
~~~

<code>source</code> is one of <code>extracted</code>, <code>fallback</code>, or <code>cache</code> (the last is only used in memory; the written source reflects how it was acquired).

### 7.4 Taste state

~~~json
{
  "version": 1,
  "liked": [],
  "history": [
    {
      "track": { "id": 1, "title": "…", "artist": "…" },
      "played_at": 1794160000,
      "fraction": 1.0,
      "outcome": "completed"
    }
  ],
  "tag_weights": { "queer": 2.4 },
  "artist_weights": { "MUNA": 1.2 },
  "skipped": [],
  "dials": { "camp": 0.82, "club": 0.74, "soft": 0.46, "chaos": 0.38 },
  "plays": 1,
  "completions": 1,
  "last_updated": 1794160000
}
~~~

The <code>track</code> objects use the same fields as <code>Track</code> in [src/model.rs](../src/model.rs). Top-level state fields have serde defaults, and <code>sanitize_state</code> clamps values, but <code>Track</code> fields are not individually defaulted: a partially shaped track object makes the whole state file unparseable, so it is moved aside as corrupt and the backup/default is used. That is a known limitation, not a tolerant migration.

### 7.5 Atomicity and concurrency

- State, client ID, and artwork use temp-file + <code>rename</code> writes.
- The state save copies the old file to <code>.bak</code> before the rename.
- There is no cross-process lock. Two instances sharing a data directory can overwrite each other's state; run one.
- The state file is loaded once at startup. Changes made by another process are not merged.
- There is no queue persistence and no "resume last track" on restart; only taste, likes, history, dials, and settings survive.

---

## 8. Failure semantics

| Failure | Behaviour |
| --- | --- |
| Client ID extraction fails | Boot error with the extraction detail; the boot retry button calls <code>Retry</code>. |
| Client ID rejected mid-request | Refresh once; if refresh fails, <code>ApiError::Auth</code> and an error event. |
| Search/related/user/chart request fails | The radio reports a search/refill error; the UI keeps existing results/queue. |
| DRM-only or non-streamable track | Typed <code>StreamUnavailable</code>; bootstrap normally drops it and the queue skips up to 3 consecutive, then stops with an error. The all-probes-DRM bootstrap fallback can re-admit the original batch (known flaw). |
| Media URL probe fails | <code>resolve_stream</code> tries the next transcoding, then returns a string error; no bad URL reaches <code>ffmpeg</code>. |
| <code>ffmpeg</code> missing | <code>spawn_ffmpeg</code> returns "could not start ffmpeg (is it installed?)"; playback error. |
| ALSA open fails | Error status and toast; the rest of the app stays usable. If the engine was created, switching devices restarts playback; if <code>AudioEngine::new</code> itself failed at startup, restart with a working <code>GAYDAR_ALSA_DEVICE</code>. |
| Playback stalls | 15 s first-audio or 25 s mid-stream timeouts kill the child and report an error. |
| State corrupt | Renamed aside; <code>.bak</code> tried; default state if both fail. |
| Artwork invalid/too large | Not cached; the UI keeps the gradient/monogram fallback. |
| Queue empty after refill | Refill falls back to bootstrap; if that fails, the boot/error path. |

---

## 9. Limits and deliberate non-goals

- **Linux/ALSA only.** There is no CoreAudio/WASAPI path, no <code>rodio</code>, no <code>cpal</code>.
- **No DRM workaround.** DRM-only tracks are classified and dropped; no keys, no login, no browser emulation.
- **No offline audio.** Artwork is cached; audio is always streamed.
- **No full catalogue.** Only MP3-transcoded, public, policy-allowed uploads in the duration window.
- **No queue persistence.** Current track and queue are session state; taste is not.
- **No gapless/crossfade/normalisation.** Tracks are independent <code>ffmpeg</code> processes.
- **No equaliser.** The 32 bands are a meter.
- **No shuffle/repeat implementation.** The player icons are decorative in this build.
- **No global hotkeys, MPRIS, tray, or media keys.**
- **Single window, single instance assumption.** No inter-process lock.
- **No download/export of audio.**

These are product choices and current limits, not hidden behaviours.

---

## 10. Tests

- <code>cargo test --lib</code>: four unit tests in <code>soundcloud.rs</code> covering tag-list parsing, artwork URL normalisation, transcoding classification, and image magic.
- <code>cargo test --test ui_interactions</code>: headless BUI interaction test: focus/type in the search field, submit, navigate Home, toggle play/pause, open/close the queue, verify the global Space shortcut, seek by segment, apply the Ballroom station, and open Now Serving.
- <code>cargo test --test live_soundcloud -- --nocapture</code>: live client ID discovery and cache startup, search/resolve/download, DRM-only fast failure, engine bootstrap/taste/state/refill/corruption. One test, <code>live_client_id_refresh_on_401</code>, is <code>#[ignore]</code> and can be run explicitly.
- <code>E2E_DIR=$(mktemp -d); GAYDAR_ALSA_DEVICE=null GAYDAR_DATA_DIR="$E2E_DIR/data" GAYDAR_CACHE_DIR="$E2E_DIR/cache" cargo test --test live_app_headless -- --ignored --nocapture</code>: a live headless app run through playback, seek, next, search, station, and a natural finish. It needs network, ffmpeg, and ALSA, and the fresh directories keep it out of your real state.

Tests are documented with commands and expected behaviour in [DEVELOPMENT.md](DEVELOPMENT.md#4-tests). They require network access for the live target and are not run in CI because there is no CI configuration in the repository.

