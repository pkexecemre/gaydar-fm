#!/bin/sh
# GAYDAR FM uninstaller.
#
# Removes exactly the files packaging/install.sh writes under DESTDIR+PREFIX:
# the launcher, the ELF, the assets tree, the desktop entry, and both icons.
# It never touches the user's GAYDAR data/cache and never calls sudo.
# Running it twice is harmless: missing files are reported and ignored.
set -eu
umask 022

PREFIX=${HOME:-/nonexistent}/.local
DESTDIR=${DESTDIR:-}
DRY_RUN=0
PROG=gaydar-uninstall

usage() {
  cat <<'EOF'
Usage: packaging/uninstall.sh [options]

Remove GAYDAR FM from a prefix. Defaults to $HOME/.local.

Options:
  --prefix PATH     install prefix (absolute; default: $HOME/.local)
  --destdir PATH    staging root prepended to the prefix (or set DESTDIR)
  --dry-run         print what would be removed, change nothing
  -h, --help        show this help

Environment:
  DESTDIR           staging root, same as --destdir

The user's library, settings, and artwork cache live outside the install prefix
(under XDG_DATA_HOME/gaydar and XDG_CACHE_HOME/gaydar) and are not removed.
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
    --dry-run)
      DRY_RUN=1
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
  fail "refusing to uninstall directly from /; pass the prefix you installed into"
fi

if [ "$PREFIX" = / ]; then
  INSTALL_ROOT=$DESTDIR
else
  INSTALL_ROOT=$DESTDIR$PREFIX
fi
BIN_DIR=$INSTALL_ROOT/bin
SHARE_DIR=$INSTALL_ROOT/share
DATA_DIR=$SHARE_DIR/gaydar
ASSETS_DIR=$DATA_DIR/assets
APPS_DIR=$SHARE_DIR/applications
ICON_SCALABLE=$SHARE_DIR/icons/hicolor/scalable/apps/gaydar.svg
ICON_256=$SHARE_DIR/icons/hicolor/256x256/apps/gaydar.png

remove_file() {
  path=$1
  if [ ! -e "$path" ] && [ ! -L "$path" ]; then
    say "not installed: $path"
    return 0
  fi
  if [ "$DRY_RUN" -eq 1 ]; then
    say "would remove: $path"
  else
    rm -f -- "$path"
    say "removed: $path"
  fi
}

remove_tree() {
  path=$1
  if [ ! -e "$path" ] && [ ! -L "$path" ]; then
    say "not installed: $path"
    return 0
  fi
  if [ "$DRY_RUN" -eq 1 ]; then
    say "would remove tree: $path"
  else
    rm -rf -- "$path"
    say "removed tree: $path"
  fi
}

say "Uninstalling GAYDAR FM from $INSTALL_ROOT (prefix: $PREFIX)"
remove_file "$BIN_DIR/gaydar"
remove_file "$BIN_DIR/gaydar.bin"
remove_file "$APPS_DIR/gaydar.desktop"
remove_file "$ICON_SCALABLE"
remove_file "$ICON_256"
remove_tree "$ASSETS_DIR"

if [ -d "$DATA_DIR" ] && [ ! -L "$DATA_DIR" ]; then
  if [ "$DRY_RUN" -eq 1 ]; then
    say "would remove empty directory if empty: $DATA_DIR"
  elif rmdir "$DATA_DIR" 2>/dev/null; then
    say "removed empty directory: $DATA_DIR"
  else
    say "kept non-empty directory: $DATA_DIR"
  fi
fi

if [ "$DRY_RUN" -eq 0 ] && command -v update-desktop-database >/dev/null 2>&1; then
  if update-desktop-database "$APPS_DIR" >/dev/null 2>&1; then
    say "Updated desktop database: $APPS_DIR"
  else
    warn "update-desktop-database failed (ignored)"
  fi
fi

if [ "$DRY_RUN" -eq 1 ]; then
  say "Dry run only; nothing was changed."
else
  say "GAYDAR FM uninstalled from $PREFIX."
  say "User data and caches under XDG_DATA_HOME/gaydar and XDG_CACHE_HOME/gaydar were not touched."
fi
