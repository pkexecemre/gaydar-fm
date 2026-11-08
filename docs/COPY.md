# GAYDAR FM — Production Copy

English only. Everything here is final copy, not placeholder. Keys are suggestions for the Rust model
and can be renamed, but the strings themselves are the product. Budgets: buttons ≤ 24 characters,
toasts ≤ 80, hero headline ≤ 60, dial description ≤ 90.

Voice check before shipping a string:
1. Is it specific (scene, record, feeling), or is it a poster?
2. Is the joke aimed at power, the algorithm, or the situation — never at the listener's body or identity?
3. Would a tired drag queen at 3am read it without rolling her eyes?

---

## 1. App identity

```
app.title              = "GAYDAR FM"
app.wordmark           = "GAYDAR"
app.wordmark.suffix    = "FM"
app.tagline            = "Your gaydar has taste."
app.subtitle           = "A very queer SoundCloud radio. No login. No closet."
app.window_title       = "GAYDAR FM — your gaydar has taste"
app.about.short        = "A very queer SoundCloud radio with taste, opinions, and no login."
app.about.long         = "GAYDAR FM plays real SoundCloud tracks, scores them with the Gaydar, and lets you move four dials: Camp, Club, Soft, Chaos. No account. No closet. No corporate pride float."
app.version_label      = "GAYDAR FM 0.1 — built with Rust + BUI"
app.unofficial_badge   = "UNOFFICIAL SOUNDCLOUD CLIENT"
```

---

## 2. Sidebar

```
nav.floor              = "The Floor"
nav.floor.sub          = "Home"
nav.radio              = "Radio"
nav.booth              = "The Booth"
nav.booth.sub          = "Search"
nav.liked              = "Liked"
nav.history            = "History"
nav.agenda             = "the Agenda"
nav.settings           = "Settings"
nav.dials              = "Dials"
nav.now_playing        = "Now playing"
nav.expand             = "Expand sidebar"
nav.collapse           = "Collapse sidebar"
sidebar.footer         = "No account. No ads. No chill."
sidebar.signal.ok      = "Signal: clear"
sidebar.signal.reading = "Signal: reading the room"
sidebar.signal.offline = "Signal: lost"
```

---

## 3. Home hero — three variants

Use one per session, rotate only on app launch (not on every navigation).

### Variant A — the signature

```
hero.a.kicker    = "THE AGENDA, LIVE"
hero.a.headline  = "Your gaydar has taste."
hero.a.subhead   = "No login. No closet. Just play."
hero.a.cta       = "Take the floor"
hero.a.secondary = "Browse the booth"
```

### Variant B — the group chat

```
hero.b.kicker    = "SIGNAL ACQUIRED"
hero.b.headline  = "The girls are online. The radio is not okay."
hero.b.subhead   = "Ballroom, drag, hyperpop, and the soft stuff after."
hero.b.cta       = "Start the radio"
hero.b.secondary = "See what the Agenda picked"
```

### Variant C — the after-hours

```
hero.c.kicker    = "NO COVER, NO CLOSET"
hero.c.headline  = "SoundCloud's gayest after-hours."
hero.c.subhead   = "Hand-picked by the Agenda. Skipped by no one."
hero.c.cta       = "Play the Agenda"
hero.c.secondary = "Move the dials"
```

Shared hero strip, under the CTA:

```
hero.strip.scenes = "Ballroom · Drag · Hyperpop · Queer punk · Trans ambient · Disco"
hero.strip.stats  = "{n} tracks read · {k} on the Agenda"
hero.strip.dials  = "Camp {camp} · Club {club} · Soft {soft} · Chaos {chaos}"
```

Rotating one-liners (launch banner, about panel, or above a hero variant):

```
hero.line.01 = "Your gaydar has taste."
hero.line.02 = "No login. No closet. Just play."
hero.line.03 = "SoundCloud's gayest after-hours, now with better lighting."
hero.line.04 = "The girls are online. The radio is not okay."
hero.line.05 = "A radio for people who have never once been chill."
hero.line.06 = "Skip the filler. Keep the drama."
hero.line.07 = "Tuned in, coming out, turning up."
hero.line.08 = "Every track hand-picked by a very opinionated homosexual."
```

---

## 4. Radio and loading

```
radio.title             = "The Floor"
radio.subtitle          = "Continuous, opinionated, and never on shuffle."
radio.loading.01        = "Reading the room..."
radio.loading.02        = "Tuning the gaydar..."
radio.loading.03        = "Consulting the Agenda..."
radio.loading.04        = "The DJ is in the bathroom..."
radio.loading.05        = "Warming up the decks..."
radio.loading.06        = "Looking for the beat that looks back..."
radio.loading.07        = "Asking the girls..."
radio.loading.08        = "Rummaging through the crates..."
radio.loading.09        = "Calibrating camp..."
radio.loading.10        = "Buffering, but make it fashion..."
radio.buffering         = "Buffering. Hold your earring."
radio.reconnecting      = "Signal dropped. Trying the aux again."
radio.recovered         = "Signal restored. Where were we?"
radio.stalled           = "The stream went quiet. Skipping before it gets awkward."
radio.empty             = "The DJ is in the bathroom. Give her a minute."
radio.empty_cta         = "Try again"
radio.refilling         = "The Agenda is reading ahead..."
radio.refill_failed     = "Couldn't refill the queue. The girls are on a break."
radio.count             = "{n} tracks on the Agenda"
radio.explicit          = "Explicit. As if you needed a warning."
radio.continuous        = "Continuous play"
radio.stop              = "Stop the radio"
radio.start             = "Start the radio"
```

---

## 5. Search

```
search.placeholder      = "Search SoundCloud. A feeling, a scene, a name you can't stop saying."
search.prompt           = "What are we looking for?"
search.prompt.sub       = "Try a scene, an artist, or the song you heard in the smoking area."
search.hint             = "Press Enter to search. Esc to leave the booth."
search.results          = "{n} tracks. The girls delivered."
search.results_for      = "Results for {q}"
search.empty            = "Nothing yet. The girls are typing."
search.no_results       = "Not a single track. Even the algorithm is gagged."
search.no_results_hint  = "Try 'ballroom vogue', 'queer punk', or the name of your first crush."
search.offline          = "Can't reach SoundCloud. Check your connection; the girls are waiting."
search.error            = "That search ghosted us. Try again in a second."
search.clear            = "Clear search"
search.recent           = "Recent searches"
search.recent.empty     = "No recent searches. Bold of you."
search.suggested        = "The Agenda suggests"
search.loading          = "Reading the room..."
search.open_on_sc       = "Open on SoundCloud"
search.artist           = "Artist"
search.track            = "Track"
search.play             = "Play"
search.add              = "Add to the Agenda"
```

Search suggestion chips (rotate, do not show all at once):

```
search.chip.01 = "ballroom vogue"
search.chip.02 = "drag queen dj set"
search.chip.03 = "trans ambient"
search.chip.04 = "queer punk"
search.chip.05 = "vogue beats"
search.chip.06 = "sapphic pop"
search.chip.07 = "queer techno"
search.chip.08 = "disco diva"
```

---

## 6. Liked

```
liked.title            = "Liked"
liked.subtitle         = "The ones that made you gasp. Kept locally."
liked.count            = "{n} liked tracks"
liked.empty            = "No likes yet. Your heart is a closed venue."
liked.empty_hint       = "Tap the heart when a track makes you gasp."
liked.empty_cta        = "Take the floor"
liked.remove           = "Remove from liked"
liked.remove_confirm   = "Remove {title} from liked?"
liked.removed          = "Removed. The venue stays open."
```

---

## 7. History

```
history.title          = "History"
history.subtitle       = "Everything you played, including the questionable bits."
history.count          = "{n} plays"
history.empty          = "No history. You blacked out. Respect."
history.empty_hint     = "Play something and the gaydar will start taking notes."
history.clear          = "Clear history"
history.clear_confirm  = "Clear the whole history? The gaydar will forget your ex."
history.cleared        = "History cleared. We saw nothing."
history.played_at      = "Played {time}"
history.today          = "Today"
history.yesterday      = "Yesterday"
```

---

## 8. Now playing

```
now_playing.label        = "Now playing"
now_playing.up_next      = "Up next"
now_playing.from_agenda  = "From the Agenda"
now_playing.playing      = "Playing"
now_playing.paused       = "Paused"
now_playing.buffering    = "Buffering"
now_playing.error        = "Stream error"
now_playing.like         = "Like"
now_playing.liked        = "Liked"
now_playing.unlike       = "Unlike"
now_playing.skip         = "Skip"
now_playing.previous     = "Previous"
now_playing.open         = "Open on SoundCloud"
now_playing.copy_link    = "Copy link"
now_playing.duration     = "Duration"
now_playing.plays        = "{n} plays"
now_playing.artist       = "Artist"
now_playing.genre        = "Genre"
now_playing.reason       = "Why this track?"
now_playing.unavailable  = "Unavailable"
now_playing.explicit     = "Explicit"
now_playing.queue        = "Queue"
now_playing.queue.empty  = "The queue is empty. Surely you have opinions."
now_playing.queue.full   = "Queue full. One banger at a time."
now_playing.volume       = "Volume"
now_playing.mute         = "Mute"
now_playing.unmute       = "Unmute"
now_playing.alsa         = "Output: {device}"
now_playing.alsa.none    = "Output: no device"
```

---

## 9. ALSA settings

```
settings.audio.title            = "Output"
settings.audio.subtitle         = "ALSA, directly. No layers, no mystery."
settings.audio.backend          = "Backend"
settings.audio.backend.alsa     = "ALSA (direct)"
settings.audio.backend.default  = "System default"
settings.audio.backend.pipewire = "PipeWire (via ALSA)"
settings.audio.backend.pulse    = "PulseAudio (via ALSA)"
settings.audio.backend.note     = "GAYDAR talks to ALSA directly. PulseAudio and PipeWire usually show up as the default device."
settings.audio.device           = "Device"
settings.audio.device.default   = "Default (follow system)"
settings.audio.device.refresh   = "Refresh devices"
settings.audio.device.count     = "{n} devices found"
settings.audio.device.empty     = "No ALSA devices found. Plug something in, darling."
settings.audio.device.busy      = "Device busy. Something else is hogging the aux."
settings.audio.device.denied    = "Permission denied. Add your user to the audio group and log back in."
settings.audio.device.open_fail = "Couldn't open that device. Check 'aplay -l' and try again."
settings.audio.volume           = "Volume"
settings.audio.volume.muted     = "Muted"
settings.audio.volume.hint      = "0 is silence. 100 is a mistake."
settings.audio.sample_rate      = "Sample rate"
settings.audio.buffer           = "Buffer"
settings.audio.buffer.hint      = "Higher buffer, fewer underruns, more latency. Life is compromise."
settings.audio.latency          = "Latency"
settings.audio.test             = "Test tone"
settings.audio.test.playing     = "Testing, one, two, camp."
settings.audio.test.done        = "That was the tone. You're welcome."
settings.audio.error.underrun   = "The buffer ran dry. Raise the buffer, or lower the chaos."
settings.audio.error.generic    = "The audio device had opinions. Try another output."
settings.audio.saved            = "Output saved."
settings.audio.unsaved          = "Unsaved changes."
```

---

## 10. Gaydar dials

```
dial.title              = "The Gaydar"
dial.subtitle           = "Four dials. No wrong answers, only revealing ones."
dial.reset              = "Reset to factory gay"
dial.reset_done         = "Reset. Back to the house style."
dial.reading            = "Reading: {summary}"
dial.value              = "{name} {value} of 1.00"
dial.a11y               = "{name} dial, {value} of 1.00, {descriptor}"
dial.changed            = "{name} at {value}. Noted."
```

```
dial.camp.name          = "Camp"
dial.camp.low           = "Understated"
dial.camp.high          = "Matinee diva"
dial.camp.description   = "Old Hollywood, new nails. How much drama do you want with your chorus?"
dial.camp.a11y.low      = "understated, nearly straight-faced"
dial.camp.a11y.high     = "full matinee diva, curtains up"

dial.club.name          = "Club"
dial.club.low           = "Living room"
dial.club.high          = "Last train home"
dial.club.description   = "BPM, sweat, and a stranger's good opinion."
dial.club.a11y.low      = "living-room tempo"
dial.club.a11y.high     = "last-train, lights-up intensity"

dial.soft.name          = "Soft"
dial.soft.low           = "Armour on"
dial.soft.high          = "Held"
dial.soft.description   = "For the tender part of the night, and the walk home after."
dial.soft.a11y.low      = "armoured up"
dial.soft.a11y.high     = "held, close, quiet"

dial.chaos.name         = "Chaos"
dial.chaos.low          = "Composed"
dial.chaos.high         = "Unwell (affectionate)"
dial.chaos.description  = "How many genres should collide before the chorus hits?"
dial.chaos.a11y.low     = "composed, one genre at a time"
dial.chaos.a11y.high    = "delightfully unwell, genres colliding"
```

Dial presets (optional, but they make the product feel alive):

```
dial.preset.matinee     = "Matinee"
dial.preset.matinee.sub = "Camp 0.95, Club 0.35, Soft 0.6, Chaos 0.2"
dial.preset.warehouse   = "Warehouse"
dial.preset.warehouse.sub = "Camp 0.5, Club 0.95, Soft 0.15, Chaos 0.55"
dial.preset.held        = "Held"
dial.preset.held.sub    = "Camp 0.35, Club 0.2, Soft 0.95, Chaos 0.2"
dial.preset.groupchat   = "Group chat at 3am"
dial.preset.groupchat.sub = "Camp 0.8, Club 0.65, Soft 0.3, Chaos 0.9"
```

---

## 11. Toasts

```
toast.liked              = "Liked. The girls are kicking their feet."
toast.unliked            = "Unliked. It's not you, it's the last track."
toast.skipped            = "Skipped. The algorithm is taking it personally (it will recover)."
toast.unavailable        = "Track unavailable. The label said no."
toast.drm                = "This one is DRM-locked. Even we can't get in."
toast.stream_error       = "Stream error. That one ghosted us; skipping."
toast.client_id_refresh  = "Refreshing the client ID. SoundCloud moved the keys again."
toast.signal_restored    = "Signal restored. We're back."
toast.no_network         = "No network. The gaydar is running on memory."
toast.copied_link        = "Link copied. Send it to the group chat."
toast.queue_full         = "Queue full. One banger at a time."
toast.volume_muted       = "Muted. A bold choice."
toast.device_changed     = "Output changed to {device}."
toast.test_tone          = "Testing, one, two, camp."
toast.rate_limited       = "SoundCloud says slow down. Same, honestly."
toast.storage_error      = "Couldn't save that. Your disk has boundaries."
```

---

## 12. Footer and disclaimers

```
footer.unofficial   = "GAYDAR FM is an unofficial SoundCloud client. Not affiliated with or endorsed by SoundCloud. All tracks and artwork belong to their creators."
footer.account      = "No account. No login. No tracking. Your likes, history, and dials stay on this machine."
footer.playback     = "Playback depends on what SoundCloud serves; some tracks are region- or DRM-locked."
footer.built        = "Built with Rust + BUI. Queerly."
footer.version      = "Version {version}"
footer.licence      = "GPL-3.0-or-later"
```

---

## 13. Ten "Gaydar reason" phrases

Shown under "Why this track?". Rotate; each must be truthful to the score that produced it.

```
reason.01 = "Because you played three ballroom mixes and never once sat down."
reason.02 = "Your Camp dial is at 0.9. This is the only logical conclusion."
reason.03 = "You liked a drag queen's DJ set at 2am. The algorithm took notes."
reason.04 = "Soft, sad, and a little bit holy: your trans ambient era is showing."
reason.05 = "Filed under: songs that make you text your ex."
reason.06 = "For the girls who kiss girls, and the girls who kiss them."
reason.07 = "You skipped the last four tracks. The gaydar recalibrated, dramatically."
reason.08 = "No one has ever described you as chill."
reason.09 = "Recommended by the Agenda, seconded by the DJ."
reason.10 = "You keep coming back to this artist. It's giving devotion."
```

Reason fallback (when there is no signal yet):

```
reason.fallback = "The gaydar has a hunch. Give it a few tracks."
```

---

## 14. Accessibility labels

```
a11y.play              = "Play"
a11y.pause             = "Pause"
a11y.like              = "Like track"
a11y.unlike            = "Unlike track"
a11y.skip              = "Skip track"
a11y.previous          = "Previous track"
a11y.volume            = "Volume, {value} percent"
a11y.mute              = "Mute audio"
a11y.unmute            = "Unmute audio"
a11y.artwork           = "Artwork for {title} by {artist}"
a11y.album_placeholder = "No artwork. The vibe is the artwork."
a11y.loading           = "Loading. {message}"
a11y.dial              = "{name} dial"
a11y.now_playing       = "Now playing: {title} by {artist}"
a11y.queue_position    = "Track {n} of {total}"
a11y.explicit          = "Explicit content"
a11y.offline           = "Offline"
```

---

## 15. Error matrix (state → copy → action)

| State | Headline | Body | Action |
| --- | --- | --- | --- |
| No network | "No signal. The gaydar needs Wi-Fi." | "Check your connection; the girls are waiting." | "Retry" |
| SoundCloud 401/403 | "SoundCloud moved the keys again." | "Refreshing the client ID. Give it a second." | "Refresh" |
| Rate limited | "SoundCloud says slow down." | "Same, honestly. Try again in a moment." | "Retry" |
| Stream dead | "That one ghosted us." | "Skipping before it gets awkward." | "Skip" |
| DRM block | "The label said no." | "This track is DRM-locked. Skipping." | "Skip" |
| ALSA missing | "No ALSA devices found." | "Plug something in, darling, then refresh." | "Refresh devices" |
| ALSA busy | "Device busy." | "Something else is hogging the aux." | "Try default" |
| Empty search | "Even the algorithm is gagged." | "Try a scene, an artist, or a crush's name." | "Clear" |

---

## 16. Do not ship

- "Celebrate diversity", "vibrant community", "everyone is welcome", "empowerment", "pride month
  campaign", "inclusive experience".
- "An error occurred", "Something went wrong", "Please try again later" (unless the matrix above
  supplies a specific reason).
- Any joke about bodies, disability, sobriety, HIV status, sex work, or someone's first Pride.
- Slurs used against people, including "faggot", "tranny", and "dyke" as an insult.
- "Guilty pleasure". Taste is not a crime.
- Fake artist names, invented scenes, or lyrics that do not exist.
