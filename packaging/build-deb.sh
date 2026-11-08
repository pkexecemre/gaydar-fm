#!/usr/bin/env bash
# Build a self-contained Debian package for GAYDAR FM.
#
# Usage: packaging/build-deb.sh [--skip-build]
# Output: dist/gaydar-fm_<version>_amd64.deb (+ .sha256)
#
# Standard Linux tools only (ar, tar, sha256sum), so this works even where
# dpkg-deb is not installed. The binary resolves assets from
# /usr/share/gaydar/assets through its exe-adjacent fallback.
set -euo pipefail

ROOT=$(cd "$(dirname "$0")/.." && pwd -P)
cd "$ROOT"

SKIP_BUILD=0
if [[ $# -gt 0 && "$1" == "--skip-build" ]]; then SKIP_BUILD=1; fi

if [[ "$SKIP_BUILD" -eq 0 ]]; then
  echo "==> cargo build --release"
  cargo build --release
fi

BIN="$ROOT/target/release/gaydar"
[[ -x "$BIN" ]] || { echo "error: $BIN missing; build first" >&2; exit 1; }
[[ -d "$ROOT/assets/fonts" ]] || { echo "error: assets/fonts missing" >&2; exit 1; }

VERSION=$(sed -n 's/^version = "\([^"]*\)"/\1/p' Cargo.toml | head -1)
[[ -n "$VERSION" ]] || { echo "error: could not read version from Cargo.toml" >&2; exit 1; }
ARCH=amd64
PKG=gaydar-fm
OUT_DIR="$ROOT/dist"
STAGE=$(mktemp -d /tmp/gaydar-deb.XXXXXX)
trap 'rm -rf "$STAGE"' EXIT

echo "==> staging $PKG $VERSION ($ARCH)"
install -d "$STAGE/DEBIAN" \
  "$STAGE/usr/bin" \
  "$STAGE/usr/share/gaydar/assets" \
  "$STAGE/usr/share/applications" \
  "$STAGE/usr/share/icons/hicolor/scalable/apps" \
  "$STAGE/usr/share/icons/hicolor/256x256/apps" \
  "$STAGE/usr/share/doc/$PKG"

install -m 755 "$BIN" "$STAGE/usr/bin/gaydar"
cp -a "$ROOT/assets/." "$STAGE/usr/share/gaydar/assets/"
install -m 644 "$ROOT/packaging/gaydar.desktop" "$STAGE/usr/share/applications/gaydar.desktop"
install -m 644 "$ROOT/packaging/gaydar.svg" "$STAGE/usr/share/icons/hicolor/scalable/apps/gaydar.svg"
install -m 644 "$ROOT/packaging/gaydar-256.png" "$STAGE/usr/share/icons/hicolor/256x256/apps/gaydar.png"
install -m 644 "$ROOT/LICENSE" "$STAGE/usr/share/doc/$PKG/copyright"

cat > "$STAGE/DEBIAN/control" <<EOF
Package: $PKG
Version: $VERSION
Section: sound
Priority: optional
Architecture: $ARCH
Depends: ffmpeg, libasound2t64 | libasound2, libx11-6, libxext6, libgl1, libfreetype6
Maintainer: GAYDAR FM <gaydar-fm@users.noreply.github.com>
Homepage: https://github.com/pkexecemre/gaydar-fm
Description: A very queer SoundCloud radio player
 GAYDAR FM is a native Rust + BUI SoundCloud radio. It discovers a public
 SoundCloud web client id without an account, keeps only uploads that expose
 an unencrypted MP3 stream, decodes through ffmpeg, and writes PCM directly
 to ALSA. The local Gaydar algorithm learns from plays, skips and likes.
 .
 Features a four-dial mood tuner, segmented waveform seeking, queue drawer,
 ALSA device picker, and a very loud queer visual identity. No login. No
 closet. No Electron.
EOF

(
  cd "$STAGE"
  find usr -type f -print0 | sort -z | xargs -0 md5sum > DEBIAN/md5sums
)

find "$STAGE/usr" -type d -exec chmod 755 {} +
find "$STAGE/usr" -type f -exec chmod 644 {} +
chmod 755 "$STAGE/usr/bin/gaydar"
chmod 755 "$STAGE/DEBIAN"
chmod 644 "$STAGE/DEBIAN/control" "$STAGE/DEBIAN/md5sums"

(
  cd "$STAGE"
  tar --sort=name --mtime='UTC 2026-01-01' --owner=0 --group=0 --numeric-owner \
      -czf control.tar.gz -C DEBIAN .
  tar --sort=name --mtime='UTC 2026-01-01' --owner=0 --group=0 --numeric-owner \
      --exclude='./DEBIAN' --exclude='./control.tar.gz' \
      --exclude='./data.tar.gz' --exclude='./debian-binary' \
      -czf data.tar.gz .
  printf '2.0\n' > debian-binary
)

mkdir -p "$OUT_DIR"
DEB="$OUT_DIR/$PKG"_"$VERSION"_"$ARCH".deb
rm -f "$DEB"
(
  cd "$STAGE"
  ar rcs "$DEB" debian-binary control.tar.gz data.tar.gz
)

if command -v dpkg-deb >/dev/null 2>&1; then
  dpkg-deb --info "$DEB" >/dev/null
  dpkg-deb --contents "$DEB" >/dev/null
  echo "==> dpkg-deb validation OK"
fi

SHA=$(sha256sum "$DEB" | awk '{print $1}')
printf '%s  %s\n' "$SHA" "$(basename "$DEB")" > "$DEB.sha256"
echo "==> $DEB"
echo "    sha256 $SHA"
