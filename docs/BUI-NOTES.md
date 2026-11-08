# BUI 0.5 field notes — the GAYDAR FM Now Playing prototype

Prototype: `examples/now_playing.rs` · final frame: `docs/now-playing-prototype.png` · scratch build: `/tmp/gaydar-ui-proto`

This is a disposable, single-file visual prototype of the production **Now Playing** screen. It uses blenderui 0.5, png 0.18, CSS, two Rust canvas painters, and mock data. It never touches the network, audio, auth, or the production model. The goal was to discover what BUI does well, what it quietly does differently from a browser, and which visual ideas are worth carrying into production.

## Run / capture

```bash
# Native X11 window (1440x900)
cargo run --release --example now_playing

# Headless 1440x900 PNG; --at advances BUI's virtual clock in ms
cargo run --release --example now_playing -- --screenshot /tmp/now-playing.png
cargo run --release --example now_playing -- --screenshot /tmp/now-playing.png --at 2600
```

The reference capture is at **2600 ms** (equalizer, radar sweep, particles, and the play-ring animation mid-flight). Native launch was smoke-tested for four seconds under X11; the headless path is the deterministic one.

## What the prototype proves

- A 1440x900 three-column desktop composition (228 / flexible center / 296) with no template chrome: art-directed hero, physical album object, vinyl, ticket, radar, broadcast deck.
- Real BUI state and listeners: transport toggles pause/play, the heart toggles like and count, nav swaps the active row. Hover transitions are wired but don't show up in a still.
- Layered CSS-only art: linear/radial gradients, clip-path hearts, layered pseudo-elements, rotated type, a 12-groove vinyl, sticker artefacts, and two canvas layers (art grain/halftone + page dither).
- A deterministic headless screenshot path, so the screen can be reviewed and regression-pinned without a display.

## Layout map (production reference)

```text
app 1440x900
├─ 3px pride line
├─ full-bleed atmosphere: glows, giant ghost "GAYDAR", 26 rotated rays, orbital rules
├─ 228 left rail: radar-heart mark, wordmark, 5 nav rows, radar scanner,
│                 GUEST DIVA ticket, ALSA pill, pride flag
├─ 916 center: 62 top bar · 424 hero (372 album art, 350 vinyl, certified sticker,
│              title block, editorial phrase) · 300 deck (waveform + transport) ·
│              70 recommendation strip (quote + 4-band EQ + SERVING) · 44 ticker
└─ 296 right rail: UP NEXT (4 rows), GAYDAR CONFIDENCE meter, MOOD DIALS,
                   STREAM source block, signal/footer
```

The palette tokens live at the top of the stylesheet (`#09050d` aubergine-black, `#ff4fa3` hot pink, `#c7a6ff` lilac, `#59e6ff` cyan, `#c9ff5c` acid lime, `#fff3e9` cream). Typography is IBM Plex Sans Condensed for display, IBM Plex Serif Light Italic for the editorial voice, and IBM Plex Mono for broadcast microcopy, with BUI's embedded Inter as the fallback.

## BUI 0.5 capabilities this screen relies on

Verified working in this build:

| Area | Used here |
| --- | --- |
| Layout | flex, absolute positioning, percentage/calc widths, `gap`, fixed 1440x900 |
| CSS | custom classes, `var()`, `:nth-child`, `:before/:after` content, `box-sizing:border-box` (BUI default) |
| Paint | linear and radial gradients, multi-layer backgrounds, `box-shadow` + inset shadows, `border-radius` + `overflow:hidden`, `opacity` |
| Shape | `clip-path: polygon()` hearts, stars, triangles; rotated rectangles; dashed/double borders |
| Effects | `filter: blur()/drop-shadow()`, layered transforms, text-shadow |
| Type | `@font-face`, italic serif, tight tracking, `white-space:nowrap`, mixed inline spans |
| Motion | `@keyframes`, infinite vinyl/radar/equalizer/particle loops, `transition` on hover, animated `transform` and `opacity` |
| Canvas | two painted layers: deterministic grain/halftone at 372x372 and page dither at 1440x900 |
| Input | `on_click` + `cx.listener`; headless `simulate_click`/bounds are available for tests |

The engine also advertises (docs) solid flex/grid, every CSS 2.1 screen property, `filter`, `mask`, `mix-blend-mode`, `@media`, `@supports`, `@container`, `@keyframes`, and local `@font-face`; the notes below are the sharp edges found while using them.

## Gotchas (and the fixes used here)

### 1. Whitespace at inline-run boundaries is trimmed
`"serving " + <span>soft-girl</span> + " euphoria"` collapsed to `servingsoft-girleuphoria`. A `\u{00a0}` at the boundaries did not survive either. **Fix:** make each word an `inline-block` span with an explicit `margin-right` (`.phrase-w`). This is the reliable way to mix type styles inside one phrase.

### 2. Installed font-family names can resolve to the wrong face
`font-family: "IBM Plex Sans Condensed"` resolved to a non-condensed face in this environment. **Fix:** register the exact file with `@font-face` and use the alias:

```css
@font-face { font-family: "Gaydar Display";
  src: url("/usr/share/fonts/truetype/IBMPlexSansCondensed-Bold.otf"); font-weight: 700; }
```

BUI keys faces by **family + bold + italic**, not by numeric weight. Paths may be absolute or relative to the process working directory; faces are cached once per process. For production, ship the OTFs/TTFs inside the app and either use relative paths or call `blenderui::add_font("Brand", bold, italic, include_bytes!(...))` before the first frame. There is no per-glyph fallback across families: test special characters in the exact font.

### 3. `conic-gradient()` is not trustworthy
A conic gradient rendered as wide linear bands instead of a cone in 0.5. **Fix:** build sunbursts/fans from N absolutely positioned 1–2px divs with `transform-origin: 50% 100%` and `rotate(step)` — the 26-ray page motif and 22-ray art motif are exactly that.

### 4. Gradient `background-size` / repeat does not tile a dot field
`radial-gradient(...) 0 0 / 12px 12px repeat` produced one enlarged blurred blob, not a halftone grid. Background images/SVG are supported, but don't rely on CSS tiling for micro-patterns. **Fix:** paint the art halftone and the page dither with `canvas()` (the shipped prototype uses ~800 art dots and ~2600 page dots), or use many tiny divs for a small area.

### 5. Stacking can ignore the browser's stacking-context model
A decorative `.vinyl-edge` overlay with `z-index: 8` inside an `overflow:hidden` disc painted **over** the sibling title block with `z-index: 6` — the word "Chappell" disappeared. Removing the overlay fixed it. Treat high z-index decorative overlays over text as unsafe; keep rim/glow effects as **children with no high z-index**, or draw them with canvas.

### 6. A large border on a clipped/rotated disc may not show
The disc's 2px cream `border` was not visible against the glow. **Fix:** add a child `.vinyl-ring` at `inset: 0` with `border: 2px solid` and `border-radius: 50%`. That child ring is the visible rim in the final frame. The same trick works for thin inner rims.

### 7. Fine `repeating-radial-gradient` grooves flatten
The 1–5px repeating radial gradient intended as vinyl grooves averaged into a flat tint. **Fix:** 12 concentric absolutely positioned circles, each with a 1px `rgba(255,243,233,0.075)` border. They are crisp, cheap, and deterministic.

### 8. Wide, low-alpha radial glows band visibly
The software raster shows subtle concentric banding in large radial gradients. **Fix:** lower the alpha, and lay a low-alpha canvas dither over the page (the page grain layer). It is a small but meaningful premium-UI trick.

### 9. `backdrop-filter` looked noisy
A frosted-glass card produced visible dithering/noise in the headless raster. Solid translucency + a 1px light border reads more cleanly here. If glass is required, test both the software and `BUI_GPU=1` paths.

### 10. `-webkit-text-stroke` is not an outline tool
It is not applied. `color: transparent` plus layered `text-shadow` offsets also produced a filled-looking mark rather than a clean hollow outline. **Fix:** for outlined display type, ship a raster/SVG asset, or use layered duplicate text with a tiny offset for a misregistration print effect (the title uses a 3px pink offset shadow for depth, not a true outline).

### 11. Canvas works, but its paint runs on every paint
`canvas(|bounds, painter| ...)` paints in logical pixels and composes correctly inside rotated/clipped parents. Give it an explicit sized wrapper and `w/h`; don't rely on intrinsic sizing. The paint callback runs on each repaint, so for a heavy static texture either keep the dot count modest or bake a texture. Our page dither is 2600 dots and the art grain ~700; a frame is still fast, but it is per-frame work.

### 12. Headless screenshots need a warm-up and a clock
The first `cx.draw` can be a cold layout; draw once, advance the virtual clock, draw again, and capture the last frame:

```rust
let win = cx.open_window(opts, ...)?;
let (w, h, px) = {
    cx.draw(win.id);                              // warm layout
    cx.advance_clock(Duration::from_millis(2600)); // @keyframes mid-flight
    cx.draw(win.id);
    cx.draw(win.id)                               // capture frame
};
// px are 0x00RRGGBB, device pixels; convert to RGBA for png 0.18
```

Keep the window at 1440x900 and DPR 1 for a reproducible 1440x900 file. Animations run on the virtual clock, so infinite loops never block the capture; the 2600ms frame is the designed still. The label of the vinyl was moved outside the rotating layer so the composition is identical at any capture time.

### 13. Media and interaction edges
- No `animationend` / `transitionend` events; design state changes without relying on them.
- Transitions start only if the element had a `transition` in the previous frame; don't add `transition` only under `:hover` and expect the first hover to animate.
- 2D transforms draw, but hit testing uses the untransformed box; 3D transforms do nothing.
- Widgets draw above page content; avoid overlapping native widgets with z-indexed art.
- Inline SVG exists as a root but SVG `fill`/`stroke` properties are not in the supported set. This prototype uses no SVG: hearts/stars are clip-path polygons, icons are CSS primitives, and the texture is canvas.
- `text-align: justify`, OpenType features, and multi-column layout are not implemented; the layout avoids them.

## Reusable prototype code patterns

- **CSS-first layout**: every visual rule lives in one stylesheet string; Rust builds semantic class trees and only computes wave bar heights/colors, groove rings, mood widths, particle positions, and the canvas textures. This makes the production port a styling exercise rather than a rewrite.
- **Deterministic art**: wave bars are a deterministic sine mix; glitter positions are a constant table; the grain uses a tiny LCG. No `rand` needed, and every screenshot is identical.
- **Layered object**: album art = base gradient → glow → rays → canvas grain/halftone → orbits → shadow → heart rim → clipped chrome heart → shade → scanlines → labels. Each layer is an ordinary div, so it is inspectable.
- **One accent per job**: pink = desire/action, lilac = recommendation, cyan = source/signal, lime = live/healthy. That mapping is what keeps a very saturated screen readable.
- **Microtype as texture**: 5.5–9px mono is used for credits, metadata, and broadcast chrome; the hierarchy is carried by the 68px title, the 50px serif phrase, and the 94% numeral. In production, raise the smallest labels for accessibility while keeping the proportions.

## Production recommendations

1. Bundle fonts and register them at startup; remove absolute system paths.
2. Convert the monolith into components (`Sidebar`, `Hero`, `Transport`, `Queue`, `Confidence`, `MoodDials`) with the same class names; pass real state from the model.
3. Replace the mock queue/confidence/moods with the recommendation engine results; preserve the "why this track" line — it is the human face of the algorithm.
4. Keep the native ALSA status pill honest: read the actual device/rate from the audio layer and show a degraded state, not a fake lime dot.
5. Add keyboard focus, reduced-motion handling (hold the vinyl/particles), and responsive breakpoints below 1280px; the fixed 1440x900 composition is a design target, not a shipping constraint.
6. Re-test filters, clipping, and stacking on the GPU path (`BUI_GPU=1`) and on the target X11/Wayland setup before porting the effect-heavy art.

## Files

- `examples/now_playing.rs` — the full prototype (single file, no extra deps beyond blenderui + png).
- `docs/now-playing-prototype.png` — 1440x900 reference frame at 2600ms.
- `/tmp/gaydar-nowplaying.png` — same frame from the scratch build.
- `/tmp/gaydar-ui-proto/` — isolated Cargo crate used to iterate (blenderui 0.5, png 0.18).

## Production-port gotchas (found while shipping the real app)

These are specific to the actual GAYDAR FM build on blenderui 0.5, not the prototype:

1. **Multiple `.on_click` calls on one element are not additive.** Adding a debug listener and then the real listener to the same div resulted in only the first one firing. Keep exactly one listener per element; move debug/test behavior into that listener or into the parent.
2. **`App::element_bounds` is unavailable inside a click listener for the element being clicked.** BUI removes the frame from the window while dispatching the event, so `cx.element_bounds(window.window_handle(), id)` returns `None` inside its own click handler. Do not implement a seek/volume bar by measuring its bounds at click time. Use fixed segments with individual click handlers (the shipping app uses 48 clickable waveform bars and 14 volume segments), or capture bounds during render.
3. **Animated conic gradients corrupt the effect layer.** A `.vinyl` with a conic-gradient background plus `animation: spin` produced full-width horizontal banding across the window; the same element without animation rendered cleanly. The radar sweep is therefore a canvas painter, the vinyl is static, and the only infinite animations are safe primitive effects (equalizer scaleY, caret blink).
4. **Transitions over `box-shadow` can panic in `anim::lerp`.** Focusing the search field with `transition: border-color, box-shadow` tripped an index-out-of-bounds in blenderui 0.5 when the shadow list changed length. Transition border/background/color only; do not transition a shadow from none to a list on focus.
5. **BUI's native `input` widget ignores CSS colors and opacity.** A styled text input still painted as a white Chrome field and an `opacity:0` input still drew. The shipping search field is a custom focusable div that handles `on_key_down` itself (characters, Backspace, Enter, Escape, caret animation) and calls `window.focus_id`.
6. **`pointer-events:none` and very thin elements** were not reliable for hit testing in 0.5. Enlarge click targets with a wrapper; use explicit segment ids for tests.
7. **Asset paths.** Bundle fonts/icons/covers and resolve `GAYDAR_ASSETS_DIR` first, then `./assets`, exe-adjacent assets, then `<exe>/../share/gaydar/assets`. This is what makes the packaged launcher and a source `cargo run` both work.
8. **Headless screenshots need a warm-up draw.** Drawing an image-heavy screen immediately can show incomplete SVG/PNG decode; call `cx.draw` once, pump events/clock if needed, then draw again for the final file. The production `--screenshot` path does a single draw and is deterministic because assets are local and small; live capture waits for playback and artwork.
9. **Rendering while audio loads.** If the UI only requests frames while `Playing`, a state that starts as `Loading` will freeze the UI after the first frame. Request animation frames for `Loading | Playing | booting | toast | boot animation` in the root render.
10. **The custom waveform is a better BUI seek control than a 6px line.** It is more readable, more on-brand, and avoids the bounds-in-listener trap. Each bar owns its fraction.
