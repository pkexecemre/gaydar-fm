#!/usr/bin/env bash
# Reproducible headless visual-QA gallery for GAYDAR FM.
#
# Renders the demo UI through the app's own screenshot path:
#   cargo run --release -- --screenshot out.png --screen home
# The release binary is built once, then invoked directly for each screen.
#
# Usage:
#   scripts/gaydar-screenshots.sh
#
# Outputs (docs/gallery):
#   01-home-1440x900.png ... 08-boot-1440x900.png
#   09-home-1280x800.png
#   10-now-playing-1280x800.png
#   11-queue-1440x900.png (extra: queue drawer)
#   contact-sheet.png (when ImageMagick is available)
#
# Requires: bash, cargo, and a POSIX-ish Linux host. Uses pngquant when
# available to keep the PNGs small; ImageMagick is optional (contact sheet).

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"
OUT="$ROOT/docs/gallery"
BIN="$ROOT/target/release/gaydar"
TMP="$(mktemp -d "${TMPDIR:-/tmp}/gaydar-gallery.XXXXXX")"
trap 'rm -rf "$TMP"' EXIT

mkdir -p "$OUT"

# name:screen:width:height  (name is also the output stem)
CAPTURES=(
  "01-home-1440x900:home:1440:900"
  "02-gaydar-1440x900:gaydar:1440:900"
  "03-search-1440x900:search:1440:900"
  "04-liked-1440x900:liked:1440:900"
  "05-history-1440x900:history:1440:900"
  "06-settings-1440x900:settings:1440:900"
  "07-now-playing-1440x900:now:1440:900"
  "08-boot-1440x900:boot:1440:900"
  "09-home-1280x800:home:1280:800"
  "10-now-playing-1280x800:now:1280:800"
)

if ! command -v cargo >/dev/null 2>&1; then
  echo "error: cargo is required" >&2
  exit 1
fi

echo "==> build release"
cd "$ROOT"
BUILD_LOG="$TMP/build.log"
if ! cargo build --release >"$BUILD_LOG" 2>&1; then
  tail -60 "$BUILD_LOG" >&2
  echo "error: release build failed; screenshots not captured" >&2
  exit 1
fi
if [[ ! -x "$BIN" ]]; then
  echo "error: missing executable $BIN" >&2
  exit 1
fi
if [[ ! -d "$ROOT/assets/fonts" ]]; then
  echo "error: missing assets/fonts; run from a complete checkout" >&2
  exit 1
fi

# Resolve assets relative to the checkout even if the caller's cwd is odd.
export GAYDAR_ASSETS_DIR="$ROOT/assets"

export_png() {
  local raw="$1" dest="$2"
  if command -v pngquant >/dev/null 2>&1; then
    local q="$TMP/quant-$(basename "$dest")"
    if pngquant --force --strip --quality=70-90 --speed 3 --output "$q" "$raw" >/dev/null 2>&1; then
      mv -f "$q" "$dest"
      return 0
    fi
  fi
  cp -f "$raw" "$dest"
}

echo "==> render"
for spec in "${CAPTURES[@]}"; do
  IFS=':' read -r name screen width height <<<"$spec"
  raw="$TMP/$name.png"
  log="$TMP/$name.log"
  "$BIN" --screenshot "$raw" --screen "$screen" --width "$width" --height "$height" >"$log" 2>&1 || {
    echo "error: $name failed" >&2
    cat "$log" >&2
    exit 1
  }
  [[ -s "$raw" ]] || { echo "error: $name produced an empty PNG" >&2; exit 1; }
  if command -v identify >/dev/null 2>&1; then
    actual="$(identify -format '%wx%h' "$raw" 2>/dev/null || true)"
    if [[ "$actual" != "${width}x${height}" ]]; then
      echo "error: $name is ${actual:-unknown}, expected ${width}x${height}" >&2
      exit 1
    fi
  fi
  export_png "$raw" "$OUT/$name.png"
  printf '  %-28s %8s  %s\n' "$name.png" "$(stat -c%s "$OUT/$name.png")" "bytes"
 done

# Build a compact wide contact sheet when ImageMagick is available.
if command -v magick >/dev/null 2>&1; then
  echo "==> contact sheet"
  montage_args=()
  for spec in "${CAPTURES[@]}"; do
    IFS=':' read -r name _ <<<"$spec"
    montage_args+=(-label "$name" "$OUT/$name.png")
  done
  if magick montage "${montage_args[@]}" \
      -tile 5x2 -geometry 420x263+12+12 \
      -background '#0b050d' -fill '#fff3e4' \
      -font DejaVu-Sans -pointsize 13 \
      "$TMP/contact-sheet.png" >/dev/null 2>&1; then
    export_png "$TMP/contact-sheet.png" "$OUT/contact-sheet.png"
    printf '  %-28s %8s  %s\n' "contact-sheet.png" "$(stat -c%s "$OUT/contact-sheet.png")" "bytes"
  else
    echo "warning: ImageMagick montage failed; contact-sheet.png not written" >&2
  fi
else
  echo "note: ImageMagick not found; skipping contact-sheet.png" >&2
fi

# Extra: the queue drawer is a production overlay, not one of the eight base
# screens. It is captured separately so the contact sheet keeps the base grid.
echo "==> extra: queue drawer"
QUEUE_RAW="$TMP/11-queue-1440x900.png"
QUEUE_LOG="$TMP/11-queue-1440x900.log"
"$BIN" --screenshot "$QUEUE_RAW" --screen home --queue --width 1440 --height 900 >"$QUEUE_LOG" 2>&1 || {
  echo "error: queue drawer capture failed" >&2
  cat "$QUEUE_LOG" >&2
  exit 1
}
[[ -s "$QUEUE_RAW" ]] || { echo "error: queue drawer produced an empty PNG" >&2; exit 1; }
if command -v identify >/dev/null 2>&1; then
  actual="$(identify -format '%wx%h' "$QUEUE_RAW" 2>/dev/null || true)"
  if [[ "$actual" != "1440x900" ]]; then
    echo "error: queue drawer is ${actual:-unknown}, expected 1440x900" >&2
    exit 1
  fi
fi
export_png "$QUEUE_RAW" "$OUT/11-queue-1440x900.png"
printf '  %-28s %8s  %s\n' "11-queue-1440x900.png" "$(stat -c%s "$OUT/11-queue-1440x900.png")" "bytes"

# README-compatible aliases (the README embeds docs/gallery/home.png, etc.).
# These are relative symlinks so there is only one copy of each PNG.
for alias in home gaydar search liked history settings now now-playing boot queue; do
  case "$alias" in
    home) target="01-home-1440x900.png" ;;
    gaydar) target="02-gaydar-1440x900.png" ;;
    search) target="03-search-1440x900.png" ;;
    liked) target="04-liked-1440x900.png" ;;
    history) target="05-history-1440x900.png" ;;
    settings) target="06-settings-1440x900.png" ;;
    now) target="07-now-playing-1440x900.png" ;;
    now-playing) target="07-now-playing-1440x900.png" ;;
    boot) target="08-boot-1440x900.png" ;;
    queue) target="11-queue-1440x900.png" ;;
  esac
  ln -sfn "$target" "$OUT/$alias.png"
done

echo "==> done"
echo "output: $OUT"
