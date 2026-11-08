# GAYDAR FM — Algorithm & Music Curation Audit

**Date:** 2026-11-08, ~21:00–21:35 Europe/Istanbul
**Scope:** live SoundCloud API + the real **GaydarEngine** (no mocks), isolated state under **/tmp**.
**Source read:** **src/gaydar.rs**, **src/soundcloud.rs**, **src/app.rs** STATIONS, **assets/seeds.json**,
**docs/BRAND.md**, **docs/COPY.md**. Also **src/radio.rs** station path for behaviour confirmation.
**Source changed:** nothing under **src/** or **Cargo.toml**. Added **tests/algorithm_audit.rs** (ignored by default) and this document.
**Client id:** discovered live with **SoundCloudClient::discover_client_id()** (4.0 s, 32 chars).
It resolved to the same id already in the on-disk cache (39AVoi…), so this is one client-id view.
**Rate limit:** the client's process-wide pacer (5 req/s) was used for every API call; raw first-page
search calls added a 260 ms sleep. No 429s were observed.

---

## TL;DR

1. **All four shipped stations are effectively broken today.** **camp classics drag diva** returns
   **0 results**; the other three return **1, 3 and 1** mp3-playable tracks when fetched the way the
   app fetches them (limit 24). None can fill an 8-track station.
2. **The cold pool is tiny.** **bootstrap(8)** is fast (2.0–3.0 s), artist-diverse (8/8) and every
   pick played (24/24), but the pool behind it is only ~14–20 tracks. A third **refill(8)** can
   return **6**, then **4**, then fail with *"could not start the Gaydar radio: SoundCloud returned
   no usable tracks"* after **18 unique tracks**. The app refills below a 4-track queue, so this is
   a minutes-not-hours failure.
3. **Same-session refill ignores fresh feedback.** **refill(8)** 49 ms after like + complete + skip
   returned **0 tracks by the liked artist, 0 tracks sharing its tags, and 2 tracks by the artist
   just skipped**. The 90-second **recently_filled** gate suppresses the very fetch that would use
   the new taste. After a restart (no gate), the same state produced 2 liked-artist tracks and 5/8
   tag-overlap tracks.
4. **The Camp dial does not make the radio campier.** Cold-start camp-max and default both had mean
   camp category score **0.104** and 6/8 of the same tracks. Club, Soft and Chaos extremes did move
   their categories (+73%, +128%, +73%); all-min barely moved anything (7/8 same as default). The
   cause is visible in **seed_pool()**: 49 artist seeds carry more acquisition weight than the 10
   per-dial family seeds, **entry.max()** lets core seeds override a zeroed dial, and cold bootstrap
   always includes a random artist plus **"queer anthem"**.
5. **The MP3 filter itself is trustworthy; the catalogue is the problem.** Across **240
   resolve_stream calls covering 171 unique tracks, there were 0 failures**. Every MP3-tagged
   track we sent to the resolver played. The binding constraint is how few
   tracks expose one, not whether the resolver works.

---

## 1. Live catalogue baseline

A first-page search returns 50 items. Every raw first-page response in this audit had
**media.transcodings** on every item, so the client's detail-lookup fallback was never needed.
The MP3 share is nevertheless small and scene-dependent.

Across the 21 first-page queries measured (4 stations + 17 candidates):

| Slice | Typical value |
|---|---|
| Raw items returned | 12–51 (usually 50) |
| MP3 progressive transcoding | 1–19 |
| MP3 HLS transcoding | **0 in every query** |
| DRM-only (AAC/ABR) | 3–51 |
| No usable transcoding (incl. snipped-only) | 0–33 |
| Eligible after policy/duration | 0–17 |
| Verified to resolve | all checked, 0 failures |

Two consequences that matter for the rest of the report:

* **Do not equate "streamable" with "playable".** The BRAND seed note's first claim (18+ streamable)
  can be true while the second claim (every query has an MP3 yield) is false. Spot checks of
  **assets/seeds.json** queries today: Trixie Mattel **0 MP3 / 50**, COUCOU CHLOE **0 / 50**,
  vogue beats **2 / 50**, queer techno **1 MP3 but 0 eligible / 50**. These are raw first-page
  counts; they were not all resolve-tested.
* **search_tracks filtering is correct.** For every query where the raw and filtered limits
  matched (50 vs 50), **search_tracks** returned exactly the count of raw items that were
  MP3-capable, policy-allowed, streamable and 45 s–45 min. The station discrepancy below is caused
  by the app's lower limit, not by the filter.

---

## 2. bootstrap(8)

Three cold starts, fresh state directory each time, default dials.

| Metric | Result |
|---|---|
| Latency | **2.25 s, 2.42 s, 2.23 s** (range over all 10 cold starts in this audit: 1.97–3.04 s) |
| Returned | 8, 8, 8 |
| Distinct artists | **8, 8, 8** |
| Playable after resolve | **24/24** (100%) |
| Reason invariant violations | 0/24 checked |
| Cross-run unique tracks | **13 in 24 slots** |
| Tracks appearing in all 3 runs | 5 |

**Artist diversity inside a batch is genuinely good.** The same cannot be said for diversity
across cold starts: the same five tracks appear in every run because **bootstrap_queries()** always
includes **"queer anthem"** plus a random artist seed, and that query's accessible MP3 set is only
six or seven tracks. The top of the cold-start catalogue is effectively a fixed short list.

Real picks observed (artist — title, resolved stream):

* Cassandra Complex — *Queer Punk Anthem* (1:16, progressive)
* The Really Really Ghost — *Queer Punk Anthem* (1:21, progressive)
* SYNESTHEMA — *Please Go / Don't Go (instrumental)* (3:52, progressive)
* Papa Topo — *Mueres Por Mi* / *Mal Herido* / *Actriz De Terror Italiana* (synth disco/copla)
* The IZM. — *Issit Innit (Haus Of IZM Ballroom Edit)* (4:05, progressive)
* KID@ART — *Pretty Ayvonne {A Queer Anthem}* (2:43, progressive)
* Lazarus Lynch — *Black Queer Anthem* (2:19, progressive)
* lynn-asberg — *Bronski Beat - Smalltown Boy (instrumental cover)* (5:05, progressive)

### Reason quality

Reason distribution over the 24 cold picks:

| Reason | Count | Share |
|---|---:|---:|
| **the gaydar says yes** | 16 | 66.7% |
| **the gaydar says explore** | 4 | 16.7% |
| **camp canon** | 3 | 12.5% |
| **certified ballroom** | 1 | 4.2% |

The reasons are structurally truthful (0 invariant violations: no **certified ballroom** without a
ballroom/vogue tag, no tag-affinity reason citing a missing tag, no replay reason on a cold start),
but **two thirds are the generic "says yes" fallback**. This is an ordering bug, not a scoring bug:
**reason_for()** checks the generic **queer >= 0.35** and **fit >= 0.5** before the source-specific
reasons, so a seed-search candidate with any queer signal never reaches
**"dug out of the {query} crate"** (**src/gaydar.rs:1111–1124**). The specific source reason exists
but is almost unreachable for exactly the tracks that need an explanation.

Two concrete false positives in the cold output:

* **KevMoney2Beats — *ASAP Rocky feat. Kid Cudi Type Beat - Ballroom Floor*** (4:41). A trap type
  beat. The word "ballroom" in the title raises **queer_signal** above 0.5, so it gets
  **the gaydar says yes**. It is not ballroom music.
* **poulain vole — *The Unholy review – Satan continues to inspire the very worst films Lil Nas X's
  catchy que…*** (3:53). A review podcast, not music, selected because its tag/title text contains
  **queer**, **anthem**, **lil nas x**. It played fine, which is the problem: playability is not
  curation.

### Reason wording issues

* **soft sapphic detour** fires for **non-sapphic** tracks. Examples: *Candy Says (Velvet Underground
  cover)* (tags: bluegrass/banjo/synths/cover), KID@ART's *Pretty Ayvonne*, SYNESTHEMA's *gay pride
  synthpop* instrumental. The condition is **soft >= 0.4 && queer >= 0.2**; it does not check for
  sapphic/WLW/lesbian signal.
* **camp canon** fires on Papa Topo because "synth disco" hits the **disco** camp term. Defensible
  as camp-adjacent, but the reason overstates a canon claim.
* **the gaydar says explore** is a random 6–24% bonus (**0.06 + 0.18 * chaos**). It is honest, but
  it is not a reason; it is a coin flip.

---

## 3. refill(8) after like / complete / skip

Test: bootstrap(8), then **record_play** + **record_complete** + **record_like** on a tagged rock
track, **record_skip(0.05)** on a different artist (Papa Topo), then refill.

### Same session

| Metric | Result |
|---|---|
| Latency | **49 ms** (no network fetch) |
| Returned | 8 |
| Distinct artists | 7 |
| Repeats of bootstrap picks | 0 (served set works) |
| Tracks by the liked artist | **0** |
| Tracks sharing a tag with the liked track | **0** |
| Tracks by the just-skipped artist | **2** |

The skipped track was **Papa Topo — Mal Herido**. The next refill returned **Papa Topo — Mueres Por
Mi** and **Papa Topo — Actriz De Terror Italiana**. The negative artist weight (-1.4) and negative
tag spread lowered their scores to 0.411 and 0.404, but they still made the eight because the pool
was small and the second-pass artist fallback admits them. **excluded_ids()** excludes the **track
id** that was skipped, not the artist (**src/gaydar.rs:1278–1289**). A skip is a track-level signal
in the exclusion set but an artist-level signal only in the soft score.

Why no taste? **refill()** only fetches when **!(enough && recently_filled)**. **last_refill** was
set by the bootstrap seconds earlier, so **recently_filled** is true and **available_count() >= 8**;
the job list (related seeds, artist uploads, tag searches) never runs
(**src/gaydar.rs:650–671**). The new like and skip change scores, but not the candidate pool.

### After a restart (same state file, pool and served set gone)

| Metric | Result |
|---|---|
| Latency | **1.22 s** |
| Returned | 8 |
| Distinct artists | 5 |
| Repeats of played/liked tracks | 0 |
| Tracks by the liked artist | **2** |
| Tracks sharing a tag with the liked track | **5/8** |
| Reasons | 2 × **because you keep replaying …**, 6 × **more like …** |
| Playable after resolve | 8/8 |

This is the system working as designed: related-track and artist-upload jobs immediately reflect the
like. The cost is diversity — two tracks each from three artists. That is acceptable for a
"more like this" refill, but it means the 90-second gate is throwing away the engine's best signal
during the session where the listener actually gave it.

### Exhaustion behaviour

A separate engine consumed the pool with repeated **refill(8)**:

| Call | Latency | Returned | New | Cumulative served |
|---|---:|---:|---:|---:|
| bootstrap | 2.3 s | 8 | 8 | 8 |
| refill 1 | 0.98 s | **6** | 6 | 14 |
| refill 2 | 3.37 s | **4** | 4 | 18 |
| refill 3 | 3.33 s | **error** | — | 18 |
| retry after error | — | **8** | — | — |

The error was **could not start the Gaydar radio: SoundCloud returned no usable tracks**. That
message is wrong twice: the radio was already running, and SoundCloud did return tracks; they were
either already served or not MP3-playable. The immediate retry succeeded, so this is not a hard
infinite loop, but it is an unstable, non-actionable failure that the UI can surface after a short
session. A **refill** that can only find 6 or 4 returns them silently instead of topping up or
reporting exhaustion.

---

## 4. Dial sensitivity

Seven cold starts (fresh pool each), one per setting. All 7 × 8 picks resolved (56/56).
Category means are the same **1 - exp(-0.9 * hits)** formula used by **dial_fit()**, computed over
the eight picks. "Overlap" is with the default run.

| Setting | Camp mean | Club mean | Soft mean | Chaos mean | Same tracks as default | Dominant reasons |
|---|---:|---:|---:|---:|---:|---|
| default | 0.104 | 0.272 | 0.221 | 0.312 | — | 6 queer, 1 camp, 1 explore |
| camp = 1 | **0.104** | 0.198 | 0.221 | 0.312 | 6/8 | 6 queer, 1 camp, 1 explore |
| club = 1 | 0.104 | **0.471** | 0.191 | 0.312 | 6/8 | 6 queer, 2 explore |
| soft = 1 | 0.104 | 0.272 | **0.504** | 0.312 | 4/8 | 3 soft, 3 queer, 2 dial |
| chaos = 1 | 0.226 | 0.406 | 0.221 | **0.540** | **3/8** | 5 chaos, 3 explore |
| all = 1 | 0.104 | 0.272 | 0.295 | 0.312 | 5/8 | 3 soft, 2 chaos, 1 camp, 2 explore |
| all = 0.05 | 0.104 | 0.346 | 0.295 | 0.312 | **7/8** | 7 queer, 1 explore |

What actually changes:

* **Club, Soft and Chaos work** as coarse steering: their category means rise by 73%, 128% and 73%,
  and chaos produces the most different set (Jaccard 0.231 vs default).
* **Camp does not work.** Camp-max is 0.104, identical to default, and 6/8 tracks are the same. It
  mostly swaps one Papa Topo track for another. The accessible pool contains very little that its
  own camp lexicon can see, and acquisition does not change enough to find more.
* **All-min barely does anything.** 7/8 of the default tracks return, all with generic queer
  reasons. Zeroing the dials does not produce a "no dials" radio; the base score and queer signal
  dominate.

### Why the dials are weak

Three mechanisms, all visible in **src/gaydar.rs**:

1. **dial_fit()** is only **0.20** of the score and is centred at 0.35:
   **0.20 * (fit - 0.35)**, i.e. roughly -0.07 to +0.13 (**1039–1052**, **1657–1666**). Base 0.42,
   tag affinity 0.30, artist affinity 0.24, queer signal 0.20 and source bonus 0.12 all outrank it.
2. **seed_pool()** gives **49 artist names** weight **0.45 + 0.35 * max(camp, club)** each, while
   each family seed gets **0.25 + dial**. Artist weight alone is ~37–53 total; the ten camp seeds at
   camp=1 are 12.5. **entry.max()** also lets a core seed (0.7) override a zeroed family seed (0.25)
   for overlapping words like **camp**, **ballroom**, **hyperpop**, **sapphic**, **trans joy** and
   **queer pop** (**1538–1554**). A zeroed dial is therefore not zero.
3. Cold bootstrap always includes a random **ARTIST_SEEDS** name and **"queer anthem"** regardless
   of dials (**877–913**). That is half the query set before the weighted pool is consulted.

Dial settings persist, so the next session starts with them. That part works.

---

## 5. Station queries

The station path is **radio.rs:239–259**: set dials, **client.search_tracks(&query, 24)**, map the
results, resolve the first up to four, and error if none resolve. The limit is **24**, not 50.
That matters for HYPERPOP.

### 5.1 Shipped stations, measured

| Station | Query | Raw (50) | MP3 | Eligible | Returned at 24 | Verified | What the station actually gets |
|---|---|---:|---:|---:|---:|---:|---|
| CAMP CLASSICS | **camp classics drag diva** | **0** | 0 | 0 | **0** | 0 | Error: *"CAMP CLASSICS is quiet right now. Try again."* |
| BALLROOM 4AM | **ballroom vogue beat** | 50 | 1 | 1 | 1 | 1 | A one-track station |
| SOFT SERVE | **sapphic soft queer** | 12 | 3 | 3 | 3 | 3 | 1 song + 2 spoken-word/talk pieces |
| HYPERPOP COMMUNION | **hyperpop queer chaos** | 41 | 7 | 6 | **1** | 1 | A one-track station; 5 more eligible MP3 tracks exist beyond position 24 |

Concrete results:

* **CAMP:** the raw response is empty (**total_results: 0**). This is not a filter issue; SoundCloud
  finds nothing for the three-word phrase. The station is dead on arrival.
* **BALLROOM:** **Richeyrite — Vogue Like *Ballroom Remix*** (2:00), progressive, verified.
* **SOFT:**
  * **Soft_T — queer rebellion** (0:55) — the only actual song.
  * **20riley.annmarie — Here And Queer- Sapphic Relationships In T.V.** (22:16) — a TV discussion.
  * **West Yorkshire Queer Stories — Paula Smith - On Being A Soft Butch** (3:14) — an oral history.
* **HYPER:** **Trap King Sam — Lavender Chaos [Hyperpop] [Energy] [Happy]** (3:12), progressive,
  verified. At limit 50 the query has 6 eligible tracks; the 24-item cut exposes only 1.

None of these four strings appears in **assets/seeds.json** **seed_queries**. The nearest entries are
partial word overlaps (**ballroom vogue**, **vogue beats**, **sapphic pop**, **queer rave**), and
several of those are themselves weak today.

### 5.2 Replacement queries, resolve-tested

Raw is the first page of 50. "Verified" means **resolve_stream** succeeded. Where more than 12
tracks were eligible, only the first 12 were resolved; the checked count is shown.

| Station | Query | Raw | MP3 | Eligible | Verified | Honest assessment |
|---|---|---|---:|---:|---:|---|
| camp | **drag queen dj set** | 50 | 9 | 6 | **6/6** | Best semantic fit; contains duplicate uploads and long mixes |
| camp | **camp pop** | 50 | 13 | 12 | **12/12** | High yield, low precision: summer-camp choirs/rap, not queer camp |
| camp | **camp classics** | 48 | 18 | 13 | **12/13 checked** | Worse precision: Brant Lake Camp, Mr Oizo bootleg, summer camp |
| camp | **gay anthem** | 50 | 9 | 9 | **9/9** | Relevant, but two titles contain a racial slur; needs a blocklist |
| camp | **Jinkx Monsoon** | 50 | 19 | 17 | **12/17 checked** | Relevant drag content, but at least 3/12 checked are interviews |
| ballroom | **ballroom vogue** | 50 | 6 | 6 | **6/6** | Best ballroom precision; includes 9:48 / 24:27 / 30:31 mixes |
| ballroom | **ballroom beats** | 50 | 3 | 3 | **3/3** | Small but clean: The IZM edit, BEATS BY MATIX, a 19:12 mix |
| ballroom | **ha dance** | 50 | 6 | 6 | **6/6** | Only 2/6 are actually "ha" tracks; rest are fuzzy-search noise |
| ballroom | **runway beat** | 50 | 11 | 11 | **11/11** | Sampled results are mostly Playboi Carti / Gunna type beats, not ballroom |
| soft | **Beverly Glenn-Copeland** | 50 | 19 | 16 | **12/16 checked** | Best soft/trans yield and precision; one 36:05 podcast |
| soft | **sad queer** | 19 | 16 | 13 | **12/13 checked** | 2 podcasts/talks + a 35:36 mix in the checked 12; real songs exist |
| soft | **trans ambient** | 50 | 15 | 8 | **8/8** | At least 3/8 are "Trans" substring false positives (Domi Trans, TransVerse, FifthsTransMinThirds) |
| soft | **sapphic pop** | 19 | 5 | 5 | **5/5** | All five are Bop Alloy jazz tracks featuring "Steph the Sapphic Songstress" |
| soft | **Perfume Genius** | 50 | 12 | 12 | **12/12** | All covers/remixes, one 30:54 interview; not the artist's catalogue |
| hyper | **queer breakcore** | 21 | 9 | 6 | **6/6** | Best hyper/chaos fit: ZETI, That1Queer, Snow Queen, Soda sсum |
| hyper | **queercore** | 50 | 5 | 4 | **4/4** | CRUSH FUND, Middle-Aged Queers, Heticide, one 21:50 podcast |
| hyper | **hyperpop edit** | 51 | 3 | 3 | **3/3** | PCPETE, Playboi Carti edit, Mohit R; small but on-genre |

Ten of these are existing **seeds.json** **seed_queries**: **drag queen dj set**, **Jinkx Monsoon**,
**ballroom vogue**, **Beverly Glenn-Copeland**, **trans ambient**, **sapphic pop**, **Perfume
Genius**, **queer breakcore**, **hyperpop edit** and **queercore**. **camp pop**, **gay anthem**,
**ballroom beats**, **ha dance**, **runway beat** and **sad queer** are adjacent terms tested because
the seed file has no strong camp/ballroom/soft replacement.

These are upper bounds before dedupe and semantic filtering. The sum of eligible counts is 57 camp,
26 ballroom, 54 soft, 13 hyper; the actual unique, usable music is much lower.

Real tracks worth keeping in the replacement pool:

* **Camp:** **Sebastian Setien — Bob The Drag Queen DJ Mitch Ferrino - Purse First Audio** (3:49),
  **Cuntalicious — DJ R3hab (drag queen mega mix)** (4:25), **FJD — Camp Pop** (2:59),
  **Jugend Klub — Ten Camp Classics (Part 1)** (26:39), **Mek Krek — gay anthem** (1:56).
* **Ballroom:** **Richeyrite — Vogue Like Ballroom Remix** (2:00),
  **The IZM. — Issit Innit (Haus Of IZM Ballroom Edit)** (4:05),
  **BEATS BY MATIX — BCR (eternal ballroom)** (3:34),
  **GoorillaDJ — The Ha Dance** (1:14),
  **Bumbaraz — Vogue Ballroom Mix Sessions** (24:27).
* **Soft:** **Beverly Glenn-Copeland — Heartache** (5:26),
  **Beverly Glenn-Copeland — Return** (5:35),
  **Exister. — Transambient** (2:51),
  **sydney is a softie — trans rights (dark ambient version)** (4:17),
  **Coco Pistol — Sad n queer** (2:42),
  **Queer Panic Attack — A Sad Kind of Gay** (1:52).
* **Hyper:** **ZETI — make breakcore queer again** (4:28),
  **That1Queer — Mystery Island Breakcore [rainy, night]** (3:07),
  **6ØL6Ø7H 75UN4M1 — The Snow Queen Loves Breakcore** (2:58),
  **PCPETE — Deceptacon (Hyperpop Edit)** (1:52),
  **CRUSH FUND — Seeing Stars** (1:56),
  **Middle-Aged Queers — Queerdo** (2:29).

### 5.3 The hard curation reality

The MP3 long tail is not a clean queer-music catalogue. It is keyword-stuffed small uploads, DJ
mixes, covers, type beats, podcasts and oral histories. That is why queries with high yield often
have low precision:

* **Talks/podcasts pass every existing filter.** No genre, duration or title check excludes them.
  **is_playable()** allows 45 s–45 min, and the only long-form penalty is a 0.12 score hit above 15
  minutes (**1025–1029**). Examples: the Lil Nas X review podcast, a 36:40 "I am NOT a young, sad,
  white, queer girl" talk, a 35:22 Jinkx Monsoon interview, a 14:42 "Trans Healthcare" talk.
* **Slurs are not filtered.** **gay anthem** returned two tracks whose titles contain a racial slur
  (ids 1859056491 and 1258352977). The engine displays titles verbatim; this contradicts BRAND.md's
  "words we do not use" rule. This is a curation blocklist gap, not a search bug.
* **"Trans" is a substring magnet.** **trans ambient** returns **Trans Am-bient**, **TransVerse**,
  **Domi Trans**, **FifthsTransMinThirds** and a "Trans woman makes ambient noise track" upload.
  Only a minority are actually trans ambient.
* **Search itself is fuzzy.** **ha dance** returned a Vietnamese "Dance All Night", a Hungarian
  disco remix and "Shomer Ha Chomot". **camp classics** returned summer-camp choirs and a Mr Oizo
  bootleg. The engine trusts first-page search rank; there is no rerank for scene authenticity
  beyond the tag/dial lexicon.

---

## 6. Cleanup, empty pool, errors, state

### Unplayable likes/skips

**record_like()** and **record_skip()** do not call **is_playable()**. A synthetic track with
**streamable=false**, **policy=BLOCK**, **duration_ms=1000** was accepted: **liked_tracks()**
contained it, **is_liked()** returned true, and **record_skip()** added it to history. The UI
currently only calls these on the playing track, so this is not an observed user-facing bug, but it
means a liked track that is later taken down or DRM-locked stays in the likes list and keeps
reinforcing its tags and artist. There is no cleanup pass on load.

Track id 0 is correctly ignored by play/complete/skip/like (no plays, likes or history added).

### State sanitising

A fully-shaped dirty state file is handled correctly. On load, the engine:

* clamps dials into 0..=1 (4.0 → 1.0, -3.0 → 0.0, 2.0 → 1.0),
* clamps tag/artist weights to ±10 and drops empty keys,
* removes id-0 and duplicate likes, removes id-0 history entries,
* clamps history fractions, dedupes and removes id-0 skipped ids.

Observed after save: **{"camp":1.0,"club":0.0,"soft":1.0,"chaos":0.5}**, weights
**{"ballroom":-10.0,"queer":10.0}**, skipped **[2]**, one liked, one history entry.

A **partial** track object in state is a different story: because **Track** fields are not
**#[serde(default)]**, one missing non-Option field makes the whole state file unparseable. The
engine renames it **.corrupt-*** and starts from default, discarding all likes, history, weights and
dials. That is safe (no panic, file preserved) but blunt; a tolerant per-field loader would salvage
the rest. This was reproduced deliberately with a partial JSON object.

Corrupt JSON (**{ not json**) is moved aside without panic. Good.

### Empty pool / no network

With a dead proxy (connection refused), **bootstrap(3)** failed after **5.8 s** with:

> could not start the Gaydar radio: SoundCloud search for "queer anthem" failed: network error:
> error sending request for url (https://api-v2.soundcloud.com/search/tracks?q=queer+anthem&limit=50&client_id=…)

**refill(8)** on a fresh state failed after **8.2 s** with:

> could not start the Gaydar radio: SoundCloud search for "COBRAH" failed: network error: …

Problems:

* A refill on a running radio should not say "could not start the Gaydar radio"; the radio layer
  then wraps it again as **gaydar refill failed: …** (**radio.rs:235**).
* The raw reqwest URL and query are leaked into user-facing text. The client id is public, but the
  message is noise. The brand error matrix already has better copy ("No signal. The gaydar needs
  Wi-Fi.").
* **refill()** silently ignores job errors (**if let Ok(tracks)** at **gaydar.rs:666**), so the error
  the user eventually sees comes from the bootstrap fallback, not from the refill that actually
  failed.
* There is no infinite loop in the engine: **refill → select empty → bootstrap → bail once**. The
  immediate retry after the exhaustion error succeeded with 8 tracks, so the failure is transient.
  Whether the UI re-arms auto-refill after an Error event was outside this audit's reading scope;
  the engine itself does not spin.

---

## 7. Bugs and risks, ranked

### P0 — breaks the product

1. **All four station queries are invalid or supply-starved.**
   **camp classics drag diva** = 0 results; the other three produce 1/3/1 playable tracks at the
   app's limit of 24. None of the four is in the validated seed file.
   (**src/app.rs:83–116**, **src/radio.rs:239–259**.)
2. **The cold pool cannot sustain the queue.** In the exhaustion run, only 18 unique tracks could be served before the
   first error; the initial pool is roughly 14–20 tracks. **refill(8)** returns 6, then
   4, then errors. The player refills when the queue drops below four
   (**src/app.rs:991–994**), so this is reachable in a short listening session.
3. **Same-session refill ignores the like/skip that just happened.** The 90-second gate suppresses
   taste-derived jobs; the skip leaks the artist back into the next eight.
   (**src/gaydar.rs:650–671**, **717–750**, **1278–1289**.)

### P1 — visible quality failures

4. **Reasons are generic and sometimes culturally false.** 67% **the gaydar says yes**;
   source-specific reasons are ordered below the generic queer/fit checks; **soft sapphic detour**
   fires on non-sapphic tracks. (**src/gaydar.rs:1067–1128**.)
5. **No talk/podcast/slur filter.** Podcasts, interviews, oral histories and slur-titled tracks are
   queued as music. The MP3 resolver plays them, so nothing downstream catches it.
6. **assets/seeds.json is dead data.** No reference in **src/** or **Cargo.toml**; the engine uses
   hardcoded **CORE_SEEDS**/**ARTIST_SEEDS**. The validated **seed_queries** and cultural
   **artist_seeds** never reach the runtime, and the BRAND seed-validation note is stale.
7. **Station limit 24 truncates HYPERPOP.** The query has 6 eligible MP3 tracks at limit 50 but only
   1 in the first 24. Raising the station limit to 50 would triple that station's supply.
8. **Short refills and wrong errors.** **refill(8)** returns 6/4 without warning; exhaustion reports
   "could not start the Gaydar radio: SoundCloud returned no usable tracks", which is both false and
   unactionable.

### P2 — worth fixing, lower blast radius

9. **Camp dial is inert; all-min is nearly inert.** Artist-seed weight and the mandatory
   **"queer anthem"** query drown the family seeds; **entry.max()** lets core seeds override zeroed
   dials. Camp-max mean camp score is identical to default.
   (**src/gaydar.rs:877–913**, **1538–1554**.)
10. **Unplayable likes/skips are accepted.** No **is_playable()** guard in
    **record_like**/**record_skip**; no pruning when a liked track becomes unavailable.
11. **State schema is brittle.** Missing non-Option **Track** fields discard the entire persisted
    taste/history/dials state rather than salvaging it.
12. **verify_bootstrap_picks can re-admit DRM picks.** If every probed pick is **StreamUnavailable**
    and the batch has ≤3 picks, **verified.is_empty()** returns the original **fallback** clone
    (**src/gaydar.rs:930**, **975–978**). The pool filter usually prevents this, but the safety net
    contradicts its own comment.
13. **Duplicate uploads and long mixes.** **dedup_tracks** dedupes by track id only, so the same
    "Purse First" upload appears twice under different ids; several station candidates are 9–30
    minute mixes. Not necessarily wrong for a radio, but worth a curation decision.

---

## 8. Recommendations, prioritized

### Do first (curation, no algorithm redesign)

1. **Replace the four station queries.** Suggested, based on the verified counts above:
   * **CAMP:** primary **drag queen dj set** (6/6), secondary **camp pop** (12/12) or
     **gay anthem** (9/9). Add a slur blocklist before shipping **gay anthem**.
   * **BALLROOM:** **ballroom vogue** (6/6) + **ballroom beats** (3/3) + **ha dance** (6/6, accept
     the noise) or **runway beat** (11/11, but it is type-beat-heavy). Merge and dedupe; expect
     ~10–15 usable tracks, not 24.
   * **SOFT:** **Beverly Glenn-Copeland** (12/16 checked) + **sad queer** (12/13) +
     **trans ambient** (8/8). Filter talks and long-form; the raw yields are the best of the four
     stations.
   * **HYPER:** **queer breakcore** (6/6) + **queercore** (4/4) + **hyperpop edit** (3/3). This is
     the honest ceiling for MP3-only hyperpop right now; the tested SOPHIE, 100 gecs and PC Music
     queries returned 0 MP3 in the first 50.
2. **Let a station own multiple queries.** The current **Station** has one **query: &str**. A
   **queries: &[&str]** field, fetched and merged in **radio.rs::station**, is the smallest change
   that makes the replacement queries useful. The current three-word camp query returned 0, so test any combined
   query rather than concatenating terms.
3. **Raise the station fetch limit from 24 to 50** and dedupe by id. This alone turns HYPERPOP from
   1 track into 6.
4. **Add a music-likelihood filter.** Exclude (or heavily penalise) genres/tags containing
   **podcast**, **interview**, **talk**, **entertainment**, **news**, **review**, **spoken**, and
   titles with **episode**, **interview**, **podcast**, **review**, **mix series** when the duration
   is > 15 min. This one filter removes the worst false positives from every station and from
   bootstrap.
5. **Add a slur/title blocklist.** At minimum, do not queue tracks whose titles contain racial or
   homophobic slurs. The brand explicitly forbids displaying them; search will keep finding them.

### Do next (engine behaviour)

6. **Invalidate last_refill on feedback.** In **record_like**, **record_skip**, and
   **record_complete**, set **last_refill = None** (or a **dirty** flag) so the next refill runs at
   least the artist/related jobs. The restart experiment proves the jobs work; the gate is what
   hides them. This is the highest-leverage one-line algorithm fix.
7. **Make skips artist-aware for a window.** Exclude the skipped artist from the next 1–2 refills,
   or apply the negative artist weight to all of that artist's candidates before the second-pass
   artist fallback. Today **excluded_ids** is track-id-only and the fallback admits the skipped
   artist's other uploads.
8. **Top up to the requested count.** After **select()**, if **picks.len() < count** and the pool
   has candidates, continue selecting; if the pool is genuinely empty, return a typed
   **CatalogExhausted** instead of calling **bootstrap** and reporting "could not start". At
   minimum, do not return 6 or 4 silently when the caller asked for 8.
9. **Fix bootstrap's minimum.** The last-resort top-up uses **count.min(3)**; a bootstrap(8) that
   finds 4 tracks will not fetch more even though it could. Top up to **count** before selecting.
10. **Make reasons specific.** Move the source-specific reasons (seed crate, related, artist deep
    cut, tag) above the generic **queer >= 0.35** branch, or reserve **the gaydar says yes** for
    tracks with no better explanation. Require sapphic/WLW/lesbian signal for
    **soft sapphic detour**, or rename it **soft queer detour**.
11. **Rebalance the dials.** Lower the artist-seed weight in **seed_pool** (or remove artist names
    from the random query pool and use them only for artist jobs), remove the hard-coded
    **"queer anthem"** from every bootstrap, and raise the fit weight from 0.20. Also apply the
    zeroed dial correctly: **entry.max()** currently lets core seeds keep a disabled family alive.
12. **Use assets/seeds.json at runtime.** It already has weights, dials, labels and a cultural
    artist list. Load it with **include_str!** or a build script, revalidate the queries against the
    live MP3 check, and delete the duplicated hardcoded lists. This also fixes the BRAND.md claim
    by making the validated data the actual source of truth.

### Do later (robustness)

13. **Guard record_like/record_skip with is_playable** (or at least reject id 0, empty title,
    BLOCK/SNIP, non-streamable, out-of-window), and prune liked tracks that are no longer playable
    on load.
14. **Add #[serde(default)] to Track/HistoryEntry fields** or use a tolerant intermediate struct,
    so a schema addition does not wipe the listener's entire taste state.
15. **Fix the verify_bootstrap_picks fallback** so an all-DRM probe set returns an empty list (and
    the caller fetches more) rather than the pre-verification clone.
16. **Improve error text.** Use the brand matrix at the radio layer and keep raw URLs/reqwest errors
    in logs. Distinguish **Network**, **Auth**, **RateLimited**, **CatalogExhausted**, **DrmOnly**.
17. **Consider a per-source page cursor.** **search_tracks** only reads the first page; **next_href**
    is ignored. Pagination is the only way to grow the 14–20 track pool without adding more query
    strings. This is a larger **soundcloud.rs** change and should be coordinated.

---

## 9. Reproduction

The audit lives in **tests/algorithm_audit.rs** and is **#[ignore]**d so ordinary **cargo test** is
unaffected. All state goes to fresh **/tmp/gaydar-audit-*** directories. Use an isolated client-id
cache to exercise discovery:

~~~bash
rm -rf /tmp/gaydar-audit-cache
GAYDAR_CACHE_DIR=/tmp/gaydar-audit-cache \
  cargo test --test algorithm_audit -- --ignored --test-threads=1 --nocapture
~~~

Individual runs:

~~~bash
GAYDAR_CACHE_DIR=/tmp/gaydar-audit-cache \
  cargo test --test algorithm_audit audit_bootstrap_eight -- --ignored --exact --nocapture
GAYDAR_CACHE_DIR=/tmp/gaydar-audit-cache \
  cargo test --test algorithm_audit audit_refill_feedback -- --ignored --exact --nocapture
GAYDAR_CACHE_DIR=/tmp/gaydar-audit-cache \
  cargo test --test algorithm_audit audit_dial_sensitivity -- --ignored --exact --nocapture
GAYDAR_CACHE_DIR=/tmp/gaydar-audit-cache \
  cargo test --test algorithm_audit audit_station_queries -- --ignored --exact --nocapture
GAYDAR_CACHE_DIR=/tmp/gaydar-audit-cache \
  cargo test --test algorithm_audit audit_extra_candidates -- --ignored --exact --nocapture
GAYDAR_CACHE_DIR=/tmp/gaydar-audit-cache \
  cargo test --test algorithm_audit audit_pool_exhaustion -- --ignored --exact --nocapture
GAYDAR_CACHE_DIR=/tmp/gaydar-audit-cache \
  cargo test --test algorithm_audit audit_cleanup -- --ignored --exact --nocapture
~~~

Offline path:

~~~bash
GAYDAR_CACHE_DIR=/tmp/gaydar-audit-cache GAYDAR_AUDIT_OFFLINE=1 \
HTTPS_PROXY=http://127.0.0.1:9 HTTP_PROXY=http://127.0.0.1:9 ALL_PROXY=http://127.0.0.1:9 NO_PROXY= \
  cargo test --test algorithm_audit audit_offline_errors -- --ignored --exact --nocapture
~~~

The harness prints machine-greppable lines (**BOOT**, **FEEDBACK**, **DIAL**, **STATION**,
**CANDIDATE**, **EXHAUST**, **CLEANUP**, **OFFLINE**). It caps candidate resolutions at 12 and
station resolutions at the number returned. It does not touch the app, the audio engine or ALSA.

---

## 10. Limitations

* **One client id, one network, one region.** Discovery returned the same cached id (39AVoi…) that
  the fallback list carries. Different web client ids can expose different transcodings; this audit
  cannot rule that out.
* **Point-in-time snapshot.** SoundCloud's catalogue, transcodings and search rank change. The
  counts are exact for the stated window, not permanent.
* **First page only.** All raw counts are the first 50 search results (stations: the app's 24).
  **next_href** was not followed. The true number of playable tracks for a query may be higher.
* **Resolve checks are point-in-time.** 240 calls / 171 unique ids / 0 failures over ~35 minutes.
  A URL that resolved during the audit can expire later; the client probes at resolve time and
  retries, but playback is not a guarantee.
* **Dial runs are one cold start per setting.** Random seed choice produces a few tracks of
  variance between runs (observed across the three bootstrap runs). The direction of the club,
  soft and chaos effects is large enough to survive that; the camp result (zero change) is also
  consistent with the code path, but a multi-run average would be stronger.
* **Semantic labels are manual.** "Talk vs music" and "true ballroom vs type beat" judgments come
  from title, genre, tags and duration. Track ids and titles are included so they can be checked.
* **No UI/ALSA testing.** This audit does not exercise playback, queue presentation or the station
  screen; it measures what the engine and station fetch hand to them.