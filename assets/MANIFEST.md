# GAYDAR FM — Asset Manifest

Visual identity library for **GAYDAR FM**, a queer SoundCloud radio built in Rust + BUI.
Direction: *club-kid glamour meets editorial print* — hot magenta, electric lilac, acid lime,
cyan and deep aubergine, with rainbow reserved for deliberate pride moments instead of a default
gradient. Drag-show poster, ballroom flyer, 90s queer zine, chrome hearts, disco balls, lipstick.

Every file in this directory is original vector artwork or OFL-licensed type. No third-party
imagery, no stock, no raster source art. All SVGs are hand-authored geometry plus gradients and
clip paths — **no filters, no masks, no embedded text, no external references** — so they render
identically in resvg/usvg, Blink/WebKit, and `rsvg-convert`.

---

## Palette

| Token | Hex | Role |
|---|---|---|
| Magenta | `#FF1F8F` | primary accent, CTAs, "live" states |
| Lilac | `#B98CFF` | secondary accent, metadata, hover |
| Acid lime | `#C6FF3D` | highlights, "on air", sticker badges |
| Cyan | `#2EE6FF` | tertiary accent, links, focus rings |
| Cream | `#FFF3E4` | type on dark, print paper |
| Aubergine | `#14001A` | base background, ink on light |
| Deep aubergine | `#0A000F` | cover grounds, vignettes |
| Violet | `#7B2CFF` | chrome gradients only |
| Pink | `#FF5CB8` | soft fills, gradients |
| Pride set | `#E40303 #FF8C00 #FFED00 #008026 #004DFF #750787` | rainbow stickers, pennant, collage |

Rules of thumb: cream on aubergine for body, magenta for action, lime for one thing per screen,
lilac for what is *about* to be interactive. Never build a six-stop rainbow gradient behind UI —
the rainbow only appears as an object (arc, pennant, strip).

---

## Fonts — `assets/fonts/`

All families are **SIL Open Font License 1.1**; the verbatim license ships as `OFL.txt` in each
family directory. Sources are the canonical Google Fonts repository (google/fonts, `ofl/`), fetched
2026-11-08.

| File | Family (as reported by FreeType/fontconfig) | Style | Weight | Source | Notes |
|---|---|---|---|---|---|
| `fonts/unbounded/Unbounded-Black.ttf` | Unbounded | Black | 900 | [google/fonts ofl/unbounded](https://github.com/google/fonts/tree/main/ofl/unbounded) | static instance of `Unbounded[wght].ttf` (v1.701) at wght=900, via fontTools 4.68 |
| `fonts/unbounded/Unbounded-Bold.ttf` | Unbounded | Bold | 700 | same | static instance at wght=700 |
| `fonts/instrument-serif/InstrumentSerif-Italic.ttf` | Instrument Serif | Italic | 400 | [ofl/instrumentserif](https://github.com/google/fonts/tree/main/ofl/instrumentserif) | editorial italic accents (v1.000) |
| `fonts/instrument-serif/InstrumentSerif-Regular.ttf` | Instrument Serif | Regular | 400 | same | bonus upright companion (v1.000) |
| `fonts/dm-sans/DMSans-Regular.ttf` | DM Sans | Regular | 400 | [ofl/dmsans](https://github.com/google/fonts/tree/main/ofl/dmsans) | instance of `DMSans[opsz,wght].ttf` (v4.004) at opsz=14, wght=400 |
| `fonts/dm-sans/DMSans-Medium.ttf` | DM Sans | Medium | 500 | same | opsz=14, wght=500 |
| `fonts/dm-sans/DMSans-Bold.ttf` | DM Sans | Bold | 700 | same | opsz=14, wght=700 |
| `fonts/dm-sans/DMSans-Italic.ttf` | DM Sans | Italic | 400 | same | bonus, opsz=14, wght=400 |
| `fonts/*/OFL.txt` | — | — | — | — | SIL OFL 1.1, per-family copyright line |

**Family names were normalized.** The Google Fonts DM Sans variable font reports its family as
`DM Sans 14pt` after optical-size instancing; the `name` tables of the shipped statics were
rewritten to `DM Sans` (with `Regular` / `Medium` / `Bold` / `Italic` subfamilies) so that both
fontconfig and Rust font stacks resolve `DM Sans` + weight correctly. Unbounded instances were
likewise normalized to family `Unbounded`, subfamily `Black` / `Bold`. `OS/2.usWeightClass`,
`fsSelection` and `head.macStyle` were set to match. Nothing else was altered — outlines, metrics
and kerning are the upstream bytes.

Recommended use:

* **Unbounded Black** — headlines, track titles at 24px+, the wordmark. Very wide; track it tight
  (`-0.02em`) at display sizes and never below 14px.
* **Unbounded Bold** — subheads and buttons that need a lighter step of the same voice.
* **Instrument Serif Italic** — one accent line per screen: dedications, "now playing", editorial
  captions. 20px and up; it is a display italic, not body text.
* **DM Sans 400/500/700** — everything else. 400 body ≥13px, 500 metadata/labels (pair with
  `+0.02em`), 700 for track names and small caps with tracking 0.2–0.34em.

Cap heights at 100px: Unbounded 75, Instrument Serif 72, DM Sans 70. Units per em 1000 for all.

---

## Icons — `assets/icons/` (24×24, transparent, single colour)

25 icons on a 24×24 grid with a 2px optical inset. Two consistent families:

* **Utility** — `1.8px` stroke, round caps/joins, `fill="none"`.
* **Transport** — solid shapes (`fill="currentColor"`) so playback controls read at 16px.

Every file sets `color="#000"` and paints with `currentColor`, which means: black when opened
standalone, tinted when inlined or used as an **alpha mask**, and never dependent on a font.

| File | Use |
|---|---|
| `play.svg` | start playback (solid) |
| `pause.svg` | pause (solid) |
| `next.svg` | next track (solid) |
| `prev.svg` | previous track (solid) |
| `shuffle.svg` | shuffle toggle |
| `repeat.svg` | repeat all |
| `repeat-one.svg` | repeat track |
| `heart.svg` | like (outline) |
| `heart-filled.svg` | liked |
| `plus.svg` | add to queue / new playlist |
| `search.svg` | search field |
| `home.svg` | home / discover tab |
| `gaydar.svg` | radio tab — broadcasting heart |
| `sparkles.svg` | recommendations / "for you" |
| `sliders.svg` | settings |
| `volume.svg` | volume |
| `volume-muted.svg` | muted |
| `queue.svg` | up-next list |
| `history.svg` | listening history |
| `close.svg` | dismiss |
| `chevron-down.svg` | disclosure / select |
| `external-link.svg` | open on SoundCloud |
| `drag-handle.svg` | reorder queue |
| `check.svg` | confirm / downloaded |
| `equalizer.svg` | audio / EQ affordance |

Usage (alpha mask, tint via `background-color` / `color`):

```css
.i-play {
  width: 24px; height: 24px; background: currentColor;
  -webkit-mask: url(icons/play.svg) center / 24px 24px no-repeat;
          mask: url(icons/play.svg) center / 24px 24px no-repeat;
}
```

> These are **alpha** masks. If you deliberately switch to `mask-mode: luminance`, set
> `color: #fff` first — luminance masks read the black fill as transparent.

Inline SVG works too: the `color` presentation attribute is overridden by CSS `color`, so a single
rule tints every icon.

---

## Brand marks — `assets/brand/`

| File | Dimensions | Purpose |
|---|---|---|
| `app-mark.svg` | 64×64 | app icon: cream heart at the centre of a radar scope (magenta→violet gradient tile, lime rings, sweep wedge, two blips). Legible down to 24px. |
| `wordmark.svg` | 1280×320 | primary lockup on dark: cream **GAYDAR** with magenta misregistration, lime **FM** sticker badge, lilac "QUEER RADIO · SOUNDCLOUD". Transparent background. |
| `wordmark-ink.svg` | 1280×320 | same lockup in aubergine for light/print backgrounds. |

Both wordmarks are outlined type (Unbounded Black + DM Sans Bold converted to paths by HarfBuzz
shaping), so they need no font installed and never reflow.

---

## Stickers & decoration — `assets/stickers/`

All transparent, all original vector. Gradients are ordinary `linearGradient`/`radialGradient`;
nothing here needs filters or blend modes.

| File | Size | Notes |
|---|---|---|
| `heart-chrome.svg` | 240×240 | chrome balloon heart; banded chrome gradient, specular lobes, prism rim (magenta→violet→cyan). Hero / empty-state art. |
| `sparkle-4.svg` | 160×160 | four-point sparkle, cyan→lime→white, magenta offset ghost. |
| `disco-ball.svg` | 220×220 | faceted mirror ball (computed lat/long tiles), hanger, four light rays. |
| `rainbow-arc.svg` | 320×190 | six-band pride arc, round caps. The deliberate rainbow. |
| `lips.svg` | 240×160 | glossy lips with seam and highlights. |
| `flower.svg` | 200×200 | eight-petal bloom, alternating pink/violet petals, lime disc. |
| `butterfly.svg` | 240×240 | mirrored cyan→magenta wings, ink body and antennae. |
| `pride-pennant.svg` | 280×200 | striped pennant on a cream pole. |
| `word-y2gay.svg` | 631×230 | **Y2GAY** die-cut sticker: cream keyline, white→lilac→magenta dip-dye fill, magenta offset. |
| `word-serving.svg` | 652×200 | **SERVING** pill sticker, magenta tag, cream type, lime/cyan dots. |
| `halftone-dots.svg` | 240×240 | radial halftone blob; `currentColor` ink — tint it, or use as a mask. |
| `sunburst.svg` | 400×400 | 24-ray sunburst, alternating full/half opacity, centre disc and ring. Bleeds past its box on purpose — crop it. |

---

## Covers — `assets/covers/` (1000×1000)

Fallback / mock album art. Typographic, abstract, no people, no copyrighted imagery. Each carries
a small `GAYDAR FM · 00N` print mark for series coherence.

| File | Theme | Anchor colours |
|---|---|---|
| `01-hyperpop-chrome.svg` | hyperpop chrome — giant chrome heart in a magenta burst | aubergine, chrome, magenta, lime |
| `02-ballroom-vogue.svg` | ballroom vogue — gold double-rule flyer, spotlight, oversized BALLROOM | aubergine, antique gold, cream |
| `03-sapphic-dusk.svg` | sapphic dusk — sliced sun, sea reflection, Instrument Serif italic | magenta, peach, violet, deep aubergine |
| `04-trans-joy.svg` | trans joy — trans-flag arch, TRANS JOY, confetti | trans blue/pink, cream, lime |
| `05-leather-camp.svg` | leather/camp — halftone leather, chrome studs, zipper, CAMP in acid lime | black, lime, chrome, magenta glow |
| `06-disco.svg` | disco — faceted ball, light beams, chrome DISCO | aubergine, chrome, cyan, lime |
| `07-ambient-softness.svg` | ambient softness — cream field, faint concentric rings, "soft focus" | cream, lilac, pink, aubergine ink |
| `08-chaotic-collage.svg` | chaotic collage — torn paper, tape, barcode, NO SKIPS, rainbow strip | cream, magenta, lime, cyan, ink |

---

## Patterns — `assets/patterns/` (seamless tiles)

SVG versions are `currentColor`-tintable; PNG versions are baked and can be used directly.
`@2x` PNGs are exact 2× renders for HiDPI surfaces.

| File | Tile | Notes |
|---|---|---|
| `grain-256.png` / `grain-512.png` | 256², 512² | monochrome alpha noise, ~6–14% alpha, seamless by construction. Overlay at 100% opacity. |
| `halftone-32.svg` | 32² | 45° dot screen, dots r=6.4. |
| `halftone-32.png` / `halftone-32@2x.png` | 32², 64² | ink dots on transparency. |
| `checker-32.svg` | 32² | 16px checker, ink squares on transparency (pairs with any background). |
| `checker-32.png` / `checker-32@2x.png` | 32², 64² | baked ink. |
| `hearts-96.svg` | 96² | four mini hearts in a staggered grid. |
| `hearts-96.png` / `hearts-96@2x.png` | 96², 192² | baked ink. |
| `starfield-192.svg` | 192² | 34 stars + 4 sparkles, seeded; default `currentColor` is cream for dark grounds. |
| `starfield-192.png` / `starfield-192@2x.png` | 192², 384² | baked **cream** (for aubergine backgrounds). |

---

## Contact sheet

`contact-sheet.png` — 2400×3060 documentation sheet: lockups, palette, type specimens, all 25
icons, all 12 stickers, all 8 covers, all patterns. Rendered from the real files in headless
Chromium so it reflects what the app will ship.

---

## Licensing

**Typeface software** — Unbounded, Instrument Serif and DM Sans are licensed under the SIL Open
Font License, Version 1.1. License text: `fonts/unbounded/OFL.txt`,
`fonts/instrument-serif/OFL.txt`, `fonts/dm-sans/OFL.txt`.

* Unbounded — Copyright 2022 The Unbounded Project Authors
  ([github.com/googlefonts/unbounded](https://github.com/googlefonts/unbounded)).
* Instrument Serif — Copyright 2022 The Instrument Serif Project Authors
  ([github.com/Instrument/instrument-serif](https://github.com/Instrument/instrument-serif)).
* DM Sans — Copyright 2014 The DM Sans Project Authors
  ([github.com/googlefonts/dm-fonts](https://github.com/googlefonts/dm-fonts)).

The OFL permits embedding and redistribution with software; the fonts are not sold separately and
their Reserved Font Names are unchanged. Static instances derived with fontTools remain under the
OFL.

**Artwork** — all SVG in `icons/`, `brand/`, `stickers/`, `covers/`, `patterns/` and the contact
sheet are original work created for GAYDAR FM and released with the project under
**GPL-3.0-or-later** (the project license). No third-party marks, logos, photographs or fonts are
embedded as outlines. Pride and trans flag colours are used as colour, not as third-party logos.

---

## Verification performed

| Check | Command | Result |
|---|---|---|
| XML well-formedness | `xmllint --noout` on every `.svg` | 52/52 parse |
| No fragile features | `grep -E "<filter|<mask|<image|<foreignObject|<text|<font-face|mix-blend-mode"` | no matches |
| No external deps | `grep href` | only `xmlns` |
| Fonts parse fully | `fontTools.ttLib.TTFont(...).ensureDecompiled()` | 8/8 OK |
| Fonts resolve | `fc-scan` | families/styles/weights correct (see table) |
| Icons tint | CSS `mask-image` in Chromium, served over HTTP | all test icons tint correctly from a single `background-color` |
| Type renders | `@font-face` from the shipped TTFs in Chromium | wordmark, specimens and covers' outlined type render as designed |
| Tiles seam | 3×3 manual tiling of every PNG | no seams |
| Visual review | contact sheet inspected at full size, fixes applied | see contact sheet |

Font files are regular TrueType with `glyf` outlines — loadable by FreeType without hinting
special-casing, and by `fontdb`/`ab_glyph`/`rustybuzz`-style Rust stacks.

---

## File count

75 files: 8 fonts + 3 OFL texts, 25 icons, 3 brand marks, 12 stickers, 8 covers,
14 pattern tiles, 1 contact sheet, this manifest.

`assets/seeds.json` also lives in this directory but belongs to a different workstream and is
not described here.
