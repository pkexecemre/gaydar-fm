#!/usr/bin/env bash
# Build a portable AppImage. Needs appimagetool on PATH (it can be an AppImage
# run via --appimage-extract-and-run on hosts without FUSE).
# Output: dist/GAYDAR-FM-<version>-x86_64.AppImage
set -euo pipefail
ROOT=$(cd "$(dirname "$0")/.." && pwd -P)
cd "$ROOT"
BIN="$ROOT/target/release/gaydar"
[[ -x "$BIN" ]] || { echo "error: build target/release/gaydar first" >&2; exit 1; }
VERSION=$(sed -n 's/^version = "\([^"]*\)"/\1/p' Cargo.toml | head -1)
APPDIR="$ROOT/build/AppDir"
rm -rf "$APPDIR"
mkdir -p "$APPDIR/usr/bin" "$APPDIR/usr/share/gaydar/assets" \
         "$APPDIR/usr/share/applications" \
         "$APPDIR/usr/share/icons/hicolor/scalable/apps" \
         "$APPDIR/usr/share/icons/hicolor/256x256/apps"
install -m 755 "$BIN" "$APPDIR/usr/bin/gaydar"
cp -a assets/. "$APPDIR/usr/share/gaydar/assets/"
chmod -R a+rX "$APPDIR/usr/share/gaydar/assets"
install -m 644 packaging/gaydar.desktop "$APPDIR/usr/share/applications/gaydar.desktop"
install -m 644 packaging/gaydar.svg "$APPDIR/usr/share/icons/hicolor/scalable/apps/gaydar.svg"
install -m 644 packaging/gaydar-256.png "$APPDIR/usr/share/icons/hicolor/256x256/apps/gaydar.png"
install -m 644 packaging/gaydar.desktop "$APPDIR/gaydar.desktop"
install -m 644 packaging/gaydar-256.png "$APPDIR/gaydar.png"
ln -sf gaydar.png "$APPDIR/.DirIcon"
cat > "$APPDIR/AppRun" <<'EOF'
#!/bin/sh
HERE=$(dirname "$(readlink -f "$0")")
export PATH="$HERE/usr/bin:$PATH"
export GAYDAR_ASSETS_DIR="$HERE/usr/share/gaydar/assets"
exec "$HERE/usr/bin/gaydar" "$@"
EOF
chmod 755 "$APPDIR/AppRun"
mkdir -p dist
# Avoid appimagetool's own runtime download (it can stall on CI/network).
RUNTIME_FILE=/tmp/gaydar-tools/runtime-x86_64
if [ ! -f "$RUNTIME_FILE" ]; then RUNTIME_FILE=/tmp/appimage-runtime-x86_64; fi
if [ -f "$RUNTIME_FILE" ]; then
  ARCH=x86_64 VERSION="$VERSION" appimagetool --appimage-extract-and-run \
    --runtime-file "$RUNTIME_FILE" "$APPDIR" "dist/GAYDAR-FM-$VERSION-x86_64.AppImage"
else
  ARCH=x86_64 VERSION="$VERSION" appimagetool --appimage-extract-and-run \
    "$APPDIR" "dist/GAYDAR-FM-$VERSION-x86_64.AppImage"
fi
chmod 755 "dist/GAYDAR-FM-$VERSION-x86_64.AppImage"
ls -lh "dist/GAYDAR-FM-$VERSION-x86_64.AppImage"
sha256sum "dist/GAYDAR-FM-$VERSION-x86_64.AppImage" | tee "dist/GAYDAR-FM-$VERSION-x86_64.AppImage.sha256"
