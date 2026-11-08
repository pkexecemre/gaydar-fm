#!/bin/sh
# GAYDAR FM installer.
#
# Installs a release build into a prefix (default: $HOME/.local), following the
# usual Unix layout:
#
#   PREFIX/bin/gaydar                      launcher (sets GAYDAR_ASSETS_DIR)
#   PREFIX/bin/gaydar.bin                  the compiled ELF
#   PREFIX/share/gaydar/assets/            bundled fonts, art, icons, seeds
#   PREFIX/share/icons/hicolor/...         app icon (SVG + 256px PNG)
#   PREFIX/share/applications/gaydar.desktop
#
# DESTDIR is honoured for staged/package installs. Installation writes only
# under DESTDIR+PREFIX (the cargo build writes to its target directory), and the
# script never calls sudo. It is safe to run repeatedly.
set -eu
umask 022

SCRIPT_DIR=$(CDPATH= cd -- "$(dirname -- "$0")" && pwd -P)
REPO_ROOT=$(CDPATH= cd -- "$SCRIPT_DIR/.." && pwd -P)

PREFIX=${HOME:-/nonexistent}/.local
DESTDIR=${DESTDIR:-}
DO_BUILD=1
PROG=gaydar-install

usage() {
  cat <<'EOF'
Usage: packaging/install.sh [options]

Install GAYDAR FM into a prefix. Defaults to $HOME/.local.

Options:
  --prefix PATH     install prefix (absolute; default: $HOME/.local)
  --destdir PATH    staging root prepended to the prefix (or set DESTDIR)
  --no-build        use an existing target/release/gaydar instead of rebuilding
  -h, --help        show this help

Environment:
  DESTDIR           staging root, same as --destdir
  CARGO_TARGET_DIR  cargo target directory, if you use a custom one
  CARGO_BUILD_TARGET
                    cargo target triple, if set

Examples:
  ./packaging/install.sh
  ./packaging/install.sh --prefix=/usr/local
  DESTDIR=/tmp/gaydar-stage ./packaging/install.sh --prefix=/usr
  ./packaging/install.sh --prefix=/opt/gaydar --no-build

The installer never runs sudo. It does not touch your GAYDAR data or cache
directories; those live under XDG_DATA_HOME and XDG_CACHE_HOME at runtime.
EOF
}

say() { printf '%s\n' "$*"; }
warn() { printf '%s: %s\n' "$PROG" "$*" >&2; }
fail() { printf '%s: %s\n' "$PROG" "$*" >&2; exit 1; }

while [ "$#" -gt 0 ]; do
  case $1 in
    --prefix)
      [ "$#" -ge 2 ] || fail "--prefix requires a path"
      PREFIX=$2
      shift 2
      ;;
    --prefix=*)
      PREFIX=${1#*=}
      shift
      ;;
    --destdir)
      [ "$#" -ge 2 ] || fail "--destdir requires a path"
      DESTDIR=$2
      shift 2
      ;;
    --destdir=*)
      DESTDIR=${1#*=}
      shift
      ;;
    --no-build)
      DO_BUILD=0
      shift
      ;;
    -h|--help)
      usage
      exit 0
      ;;
    *)
      fail "unknown option: $1 (try --help)"
      ;;
  esac
done

[ -n "$PREFIX" ] || fail "--prefix must not be empty"
case $PREFIX in
  /*) ;;
  *) fail "--prefix must be an absolute path, got: $PREFIX" ;;
esac
PREFIX=${PREFIX%/}
[ -n "$PREFIX" ] || PREFIX=/

DESTDIR=${DESTDIR%/}
if [ "$PREFIX" = / ] && [ -z "$DESTDIR" ]; then
  fail "refusing to install directly into /; use a prefix such as /usr/local or a DESTDIR"
fi

if [ "$PREFIX" = / ]; then
  INSTALL_ROOT=$DESTDIR
else
  INSTALL_ROOT=$DESTDIR$PREFIX
fi
BIN_DIR=$INSTALL_ROOT/bin
SHARE_DIR=$INSTALL_ROOT/share
DATA_DIR=$SHARE_DIR/gaydar
ASSETS_DEST=$DATA_DIR/assets
APPS_DIR=$SHARE_DIR/applications
ICON_SCALABLE_DIR=$SHARE_DIR/icons/hicolor/scalable/apps
ICON_256_DIR=$SHARE_DIR/icons/hicolor/256x256/apps

DESKTOP_SRC=$SCRIPT_DIR/gaydar.desktop
ICON_SVG_SRC=$SCRIPT_DIR/gaydar.svg
ICON_PNG_SRC=$SCRIPT_DIR/gaydar-256.png
ASSETS_SRC=$REPO_ROOT/assets

[ -f "$DESKTOP_SRC" ] || fail "missing packaging file: $DESKTOP_SRC"
[ -f "$ICON_SVG_SRC" ] || fail "missing packaging file: $ICON_SVG_SRC"
[ -f "$ICON_PNG_SRC" ] || fail "missing packaging file: $ICON_PNG_SRC"
[ -d "$ASSETS_SRC" ] || fail "missing asset directory: $ASSETS_SRC"

TARGET_DIR=${CARGO_TARGET_DIR:-"$REPO_ROOT/target"}
case $TARGET_DIR in
  /*) ;;
  *) TARGET_DIR=$REPO_ROOT/$TARGET_DIR ;;
esac
TARGET_PROFILE_DIR=$TARGET_DIR/release
if [ -n "${CARGO_BUILD_TARGET:-}" ]; then
  TARGET_PROFILE_DIR=$TARGET_DIR/$CARGO_BUILD_TARGET/release
fi
SRC_BIN=$TARGET_PROFILE_DIR/gaydar

if [ "$DO_BUILD" -eq 1 ]; then
  command -v cargo >/dev/null 2>&1 || fail "cargo not found; install Rust or use --no-build"
  say "Building GAYDAR FM (cargo build --release)..."
  ( cd "$REPO_ROOT" && cargo build --release )
else
  say "Skipping cargo build (--no-build)."
fi
[ -x "$SRC_BIN" ] || fail "release binary not found at $SRC_BIN; run without --no-build"

# Temp paths are created next to their final locations so mv is atomic on the
# same filesystem. The trap removes any leftovers after an error or signal.
BIN_TMP=
LAUNCHER_TMP=
DESKTOP_TMP=
ICON_SVG_TMP=
ICON_PNG_TMP=
ASSETS_TMP=
cleanup() {
  [ -z "$BIN_TMP" ] || rm -f -- "$BIN_TMP"
  [ -z "$LAUNCHER_TMP" ] || rm -f -- "$LAUNCHER_TMP"
  [ -z "$DESKTOP_TMP" ] || rm -f -- "$DESKTOP_TMP"
  [ -z "$ICON_SVG_TMP" ] || rm -f -- "$ICON_SVG_TMP"
  [ -z "$ICON_PNG_TMP" ] || rm -f -- "$ICON_PNG_TMP"
  [ -z "$ASSETS_TMP" ] || rm -rf -- "$ASSETS_TMP"
}
trap cleanup EXIT

say "Installing into $INSTALL_ROOT (prefix: $PREFIX)"

install -d -m 0755 "$BIN_DIR" "$DATA_DIR" "$APPS_DIR" "$ICON_SCALABLE_DIR" "$ICON_256_DIR"

# 1. The real binary. Keep the launcher at the conventional "gaydar" path.
BIN_TMP=$BIN_DIR/.gaydar.bin.tmp.$$
install -m 0755 "$SRC_BIN" "$BIN_TMP"
mv -f -- "$BIN_TMP" "$BIN_DIR/gaydar.bin"
BIN_TMP=

# 2. A relocatable launcher that pins the installed asset tree. Resolving the
#    prefix from the launcher's own location keeps DESTDIR staging and prefix
#    moves working, and it prevents a stray ./assets in the caller's cwd from
#    shadowing the installed assets.
LAUNCHER_TMP=$BIN_DIR/.gaydar.launcher.tmp.$$
cat >"$LAUNCHER_TMP" <<'GAYDAR_LAUNCHER'
#!/bin/sh
# GAYDAR FM launcher, installed by packaging/install.sh.
# Resolves its own install prefix so bundled fonts and art are found even when
# the desktop environment starts it from an unrelated working directory.
set -eu

self=$0
case $self in
  */*) ;;
  *)
    self=$(command -v "$self") || {
      echo "gaydar: launcher not found on PATH" >&2
      exit 127
    }
    ;;
esac

# Follow symlinks to the real launcher. Bounded so a symlink loop cannot hang.
links=0
while [ -L "$self" ]; do
  links=$((links + 1))
  if [ "$links" -gt 40 ]; then
    echo "gaydar: too many levels of symbolic links resolving $0" >&2
    exit 127
  fi
  target=$(readlink "$self") || exit 127
  case $target in
    /*) self=$target ;;
    *) self=$(dirname "$self")/$target ;;
  esac
done

bindir=$(CDPATH= cd -- "$(dirname -- "$self")" && pwd -P) || {
  echo "gaydar: cannot resolve launcher directory" >&2
  exit 127
}
prefix=$(CDPATH= cd -- "$bindir/.." && pwd -P) || {
  echo "gaydar: cannot resolve install prefix" >&2
  exit 127
}

if [ -z "${GAYDAR_ASSETS_DIR:-}" ] && [ -d "$prefix/share/gaydar/assets" ]; then
  GAYDAR_ASSETS_DIR=$prefix/share/gaydar/assets
  export GAYDAR_ASSETS_DIR
fi

exec "$bindir/gaydar.bin" "$@"
GAYDAR_LAUNCHER
chmod 0755 "$LAUNCHER_TMP"
mv -f -- "$LAUNCHER_TMP" "$BIN_DIR/gaydar"
LAUNCHER_TMP=

# 3. Desktop entry, with Exec rewritten to the absolute installed launcher.
#    Quote it per the Desktop Entry spec so prefixes containing spaces work.
DESKTOP_EXEC=${PREFIX%/}/bin/gaydar
DESKTOP_EXEC_ESCAPED=$(printf '%s' "$DESKTOP_EXEC" | sed -e 's/\\/\\\\/g' -e 's/"/\\"/g' -e 's/`/\\`/g' -e 's/\$/\\$/g' -e 's/%/%%/g')
DESKTOP_EXEC_FIELD="\"$DESKTOP_EXEC_ESCAPED\""
DESKTOP_TMP=$APPS_DIR/.gaydar.desktop.tmp.$$
while IFS= read -r line || [ -n "$line" ]; do
  case $line in
    Exec=*) printf 'Exec=%s\n' "$DESKTOP_EXEC_FIELD" ;;
    *) printf '%s\n' "$line" ;;
  esac
done <"$DESKTOP_SRC" >"$DESKTOP_TMP"
chmod 0644 "$DESKTOP_TMP"
mv -f -- "$DESKTOP_TMP" "$APPS_DIR/gaydar.desktop"
DESKTOP_TMP=

# 4. Icons. Both the scalable source and a crisp 256px raster are installed;
#    desktops that only understand PNG still get a good icon.
ICON_SVG_TMP=$ICON_SCALABLE_DIR/.gaydar.svg.tmp.$$
install -m 0644 "$ICON_SVG_SRC" "$ICON_SVG_TMP"
mv -f -- "$ICON_SVG_TMP" "$ICON_SCALABLE_DIR/gaydar.svg"
ICON_SVG_TMP=

ICON_PNG_TMP=$ICON_256_DIR/.gaydar.png.tmp.$$
install -m 0644 "$ICON_PNG_SRC" "$ICON_PNG_TMP"
mv -f -- "$ICON_PNG_TMP" "$ICON_256_DIR/gaydar.png"
ICON_PNG_TMP=

# 5. Assets. Copy into a sibling staging directory, normalize permissions,
#    then swap the tree in. This removes stale files from older installs and
#    keeps the installed tree readable regardless of the checkout's umask.
ASSETS_TMP=$DATA_DIR/.assets.tmp.$$
mkdir "$ASSETS_TMP"
cp -R "$ASSETS_SRC/." "$ASSETS_TMP/"
find "$ASSETS_TMP" -type d -exec chmod 0755 {} +
find "$ASSETS_TMP" -type f -exec chmod 0644 {} +
rm -rf -- "$ASSETS_DEST"
mv -- "$ASSETS_TMP" "$ASSETS_DEST"
ASSETS_TMP=

# 6. Refresh the desktop database when the tool exists. Missing MIME metadata
#    is fine; this is best-effort and never fails the install.
if command -v update-desktop-database >/dev/null 2>&1; then
  if update-desktop-database "$APPS_DIR" >/dev/null 2>&1; then
    say "Updated desktop database: $APPS_DIR"
  else
    warn "update-desktop-database failed (ignored); the desktop entry is installed"
  fi
fi

say ""
say "GAYDAR FM installed."
say "  launcher : $BIN_DIR/gaydar"
say "  binary   : $BIN_DIR/gaydar.bin"
say "  assets   : $ASSETS_DEST"
say "  desktop  : $APPS_DIR/gaydar.desktop"
say "  icons    : $ICON_SCALABLE_DIR/gaydar.svg"
say "             $ICON_256_DIR/gaydar.png"
if [ -n "$DESTDIR" ]; then
  say ""
  say "This is a staged install under DESTDIR=$DESTDIR."
  say "At runtime the files live under prefix $PREFIX; the launcher resolves"
  say "its prefix from its own path, so the staged launcher is runnable too."
fi
say ""
say "Run: $BIN_DIR/gaydar"
