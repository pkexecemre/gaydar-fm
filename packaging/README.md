# GAYDAR FM — Linux packaging

This directory is the installable Linux desktop layer for GAYDAR FM. It adds a
freedesktop entry, app icons, a prefix-aware launcher, and an idempotent
installer/uninstaller. It does not change the Rust sources, Cargo.toml, or the
asset library; installation copies the checkout's existing assets.

## Requirements

At build time:

- Linux with a POSIX shell, coreutils (`install`, `find`, `cp`), and Rust/cargo.
  The crate uses edition 2024; the test host ran rustc 1.98.1.
- The native dependencies already required by BUI 0.5 and ALSA (the same
  dependencies needed for `cargo build --release`).

At runtime:

- `ffmpeg` on `PATH` (the player pipes MP3 transcodings to ffmpeg → ALSA).
- An ALSA device. Audio is direct ALSA; PipeWire's ALSA compatibility works.
- X11 or Wayland for the BUI window. Screenshot mode is headless.

`rsvg-convert` and ImageMagick are only needed to regenerate the icons, not to
install them.

## Install

From the repository root:

~~~sh
./packaging/install.sh
~~~

That installs into `$HOME/.local` by default and runs
`cargo build --release` first. Repeated runs are safe and leave the same
installed tree.

Common forms:

~~~sh
# explicit prefix
./packaging/install.sh --prefix=/opt/gaydar

# use an already-built binary
./packaging/install.sh --no-build

# staged/package install; files are written under /tmp/stage/usr
DESTDIR=/tmp/stage ./packaging/install.sh --prefix=/usr

# same, with the flag form
./packaging/install.sh --prefix=/usr --destdir=/tmp/stage
~~~

The installer never invokes `sudo`. For a system prefix, run it with the
permissions you already have or use `DESTDIR` and let your package manager
own the final copy. Installation writes only under `DESTDIR+PREFIX`; the
cargo build writes to its target directory (`target/` by default).

`--prefix=/` is refused unless `DESTDIR` is set, so a typo cannot write into the
live root; with `DESTDIR` it stages a root-filesystem layout.

Environment:

- `DESTDIR` — staging root, same as `--destdir`.
- `CARGO_TARGET_DIR` — custom cargo target directory.
- `CARGO_BUILD_TARGET` — cargo target triple, if you use one.

The release binary is currently not stripped beyond cargo's debuginfo setting;
distro packagers may strip it further.

## Installed layout

For `PREFIX=$HOME/.local`:

| Path | Contents |
|---|---|
| `$PREFIX/bin/gaydar` | POSIX sh launcher; sets `GAYDAR_ASSETS_DIR`, then execs the ELF |
| `$PREFIX/bin/gaydar.bin` | compiled release ELF |
| `$PREFIX/share/gaydar/assets/` | bundled fonts, covers, icons, patterns, stickers, seeds, licenses |
| `$PREFIX/share/applications/gaydar.desktop` | freedesktop entry with an absolute `Exec` |
| `$PREFIX/share/icons/hicolor/scalable/apps/gaydar.svg` | scalable icon |
| `$PREFIX/share/icons/hicolor/256x256/apps/gaydar.png` | 256×256 RGBA icon |

`update-desktop-database` is run on the applications directory when present;
it may create `share/applications/mimeinfo.cache`. That is a shared generated
file, not an app file.

The test host's staged tree contained 76 asset files (3,684,437 bytes) and a
~22 MB total install with the unstripped 18.6 MB ELF.

## Launcher and asset resolution

The app resolves assets in this order (`src/theme.rs`):

1. `GAYDAR_ASSETS_DIR`, if it names an existing directory
2. `./assets` in the current working directory
3. `<exe dir>/assets`
4. `<exe dir>/../share/gaydar/assets`
5. the compile-time source tree

`bin/gaydar` is therefore a launcher, not the ELF. It resolves its own real path,
walks up one directory to the prefix, sets
`GAYDAR_ASSETS_DIR=$prefix/share/gaydar/assets` unless the caller already set it,
and execs `bin/gaydar.bin`. This has two purposes:

- It prevents an unrelated `./assets` directory in the desktop session's cwd from
  shadowing the installed assets.
- It keeps a staged `DESTDIR` tree runnable: the prefix is computed from the
  launcher's location, not hard-coded to the final root.

The real ELF also works without the launcher because of rule 4. Both paths were
tested, as were invocation through a symlink and through `PATH`; see Evidence
below. The desktop entry's `Exec` points at the launcher.

## Desktop entry

`packaging/gaydar.desktop` is the source template. It is valid on its own with
`Exec=gaydar` when the prefix's `bin` is on `PATH`. The installer rewrites
`Exec` to the absolute installed launcher and quotes it per the Desktop Entry
spec, so prefixes containing spaces work.

- Name: GAYDAR FM
- GenericName: Music Player
- Comment: A queer SoundCloud radio player - no login, no closet
- Categories: `AudioVideo;Audio;Player;`
- Terminal: false
- Icon: `gaydar`
- StartupNotify: false

There is deliberately no `StartupWMClass`. BUI 0.5's X11 backend creates the
window without `XSetClassHint`; under Xvfb, `XGetClassHint` reports no
`WM_CLASS` at all (the window title is
`GAYDAR FM — no login, no closet`). Adding a guessed class would be false and
would not improve startup matching. Fixing that needs a window-backend change, not
a packaging change.

This is a GUI application, so there is no systemd service in this directory.

## Run

~~~sh
$HOME/.local/bin/gaydar
# or launch "GAYDAR FM" from the desktop menu
~~~

For a custom prefix, add `$PREFIX/bin` to `PATH` or use the absolute path.
The desktop entry does not depend on `PATH`.

The installer does not create or touch the library. GAYDAR stores it under the
XDG directories, normally:

- data/settings: `$HOME/.local/share/gaydar`
- artwork cache: `$HOME/.cache/gaydar/art`

## Uninstall

~~~sh
./packaging/uninstall.sh                  # same default prefix
./packaging/uninstall.sh --prefix=/opt/gaydar
DESTDIR=/tmp/stage ./packaging/uninstall.sh --prefix=/usr
./packaging/uninstall.sh --prefix=/opt/gaydar --dry-run
~~~

It removes only the six installed app paths listed above (the assets directory
recursively) and refuses to run directly against `/` without `DESTDIR`. User data
and caches are left alone. A second uninstall prints "not installed" and exits 0.
The shared icon directories and the generated `mimeinfo.cache` are not removed.

## Icons

`gaydar.svg` is an exact copy of
`assets/brand/app-mark.svg` (SHA-256
`340b1ef7dc76e4f52119c7abd8ad1fb1957f5a3d760750da87cf10387dd78df3`).
`gaydar-256.png` was rendered from that SVG with rsvg-convert at 1024×1024,
then downsampled to 256×256 with Lanczos filtering and stripped:

~~~sh
rsvg-convert --width=1024 --height=1024 --keep-aspect-ratio \
  packaging/gaydar.svg -o /tmp/gaydar-icon-1024.png
magick /tmp/gaydar-icon-1024.png -filter Lanczos -resize 256x256 \
  -strip -define png:color-type=6 packaging/gaydar-256.png
~~~

The result is a 256×256 8-bit sRGBA PNG (69,361 bytes, SHA-256
`7f2a873a4f997b65b471b4d3d7d1de0e36625fbaea839b61bca380cfb2f09479`).
The installer verifies nothing about the icon contents; regenerate both files
together if the brand mark changes.

## Screenshots

`scripts/gaydar-screenshots.sh` renders the production screens headlessly
(no X/Wayland, GPU, network, or ALSA):

~~~sh
./scripts/gaydar-screenshots.sh /tmp/gaydar-shots
~~~

It writes `home, gaydar, search, liked, history, settings, now` at 1440×900,
plus `SHA256SUMS`. It builds the release binary only if it is missing. The
default output is `docs/screenshots`; pass a directory to override it.
`GAYDAR_BIN`, `GAYDAR_SCREENS`, `GAYDAR_SCREENSHOT_WIDTH`, and
`GAYDAR_SCREENSHOT_HEIGHT` are supported.

## Evidence and tested paths

Tested 2026-11-08 on openSUSE Tumbleweed 20261103 (x86_64), cargo/rustc 1.98.1,
ffmpeg 9.0.2, and Xvfb at 1440×900×24. No system files were touched.

### Staged install, full release build

~~~sh
cd /home/emre/Projeler/gaydar
DESTDIR=/tmp/gaydar-pkg-test ./packaging/install.sh --prefix=/usr/local
~~~

Result: exit 0; cargo finished the release build in 11.91 s; the tree hash was
`8d8fe74cd0642e2c0afa57d9c852ca23493982f5d693c9e1e1e670d6b89458a0` (the hash
changes whenever the app binary changes). Running the same install again with
`--no-build` produced the identical tree hash. `diff -r assets .../share/gaydar/assets` was empty. The installed
binary was an ELF at `/tmp/gaydar-pkg-test/usr/local/bin/gaydar.bin`, and the
launcher at `.../bin/gaydar` was a POSIX shell script.

The installed desktop entry had:

~~~ini
Exec="/usr/local/bin/gaydar"
Icon=gaydar
Terminal=false
Categories=AudioVideo;Audio;Player;
~~~

`desktop-file-validate` on the source and installed entries exited 0. The
absolute `Exec` is the final runtime path, not the DESTDIR staging path, as
required for a package build.

### Launcher launch under Xvfb

~~~sh
cd /tmp/gaydar-final-launch/wrapper-cwd   # contains a decoy ./assets/fonts
DISPLAY=:102 GAYDAR_ALSA_DEVICE=null \
GAYDAR_DATA_DIR=/tmp/gaydar-final-launch/data \
GAYDAR_CACHE_DIR=/tmp/gaydar-final-launch/cache \
timeout 12 /tmp/gaydar-pkg-test/usr/local/bin/gaydar
~~~

Result: still running after the check window; the binary's environment contained
`GAYDAR_ASSETS_DIR=/tmp/gaydar-pkg-test/usr/local/share/gaydar/assets`; the X11
window title was `GAYDAR FM — no login, no closet`; `WM_CLASS` was absent.
The decoy `./assets` was not used.

A separate strace run of the real ELF with `GAYDAR_ASSETS_DIR` unset, from a
neutral cwd, exited 124 (still alive at timeout) and opened:

~~~text
/tmp/gaydar-pkg-test/usr/local/share/gaydar/assets/fonts/unbounded/Unbounded-Black.ttf
/tmp/gaydar-pkg-test/usr/local/share/gaydar/assets/fonts/dm-sans/DMSans-Regular.ttf
/tmp/gaydar-pkg-test/usr/local/share/gaydar/assets/brand/app-mark.svg
~~~

That confirms both the launcher override and the executable-relative fallback.

### Prefix containing spaces

~~~sh
./packaging/install.sh --prefix="/tmp/gaydar space prefix" --no-build
~~~

The entry contained `Exec="/tmp/gaydar space prefix/bin/gaydar"`, validated, and
the extracted `Exec` path launched under Xvfb (status 124 at timeout) with
`GAYDAR_ASSETS_DIR=/tmp/gaydar space prefix/share/gaydar/assets`. It was then
uninstalled cleanly.

### Uninstall

- Default prefix with a fake HOME: install into `/tmp/gaydar-home-test/.local`,
  put a marker at `.../share/gaydar/settings.json`, run `uninstall.sh`; the
  launcher and app files were removed and the marker survived.
- `DESTDIR=/tmp/gaydar-uninstall-stage ... --prefix=/usr`: install, dry-run,
  uninstall, and a second uninstall all exited 0. The only remaining file was the
  regenerated shared `mimeinfo.cache`; the app binary, assets, desktop entry,
  and both icons were gone.

### Screenshot script

Two consecutive runs of seven 1440×900 PNGs (`/tmp/gaydar-shots-c` and
`/tmp/gaydar-shots-d`, same binary) matched every checksum
(`sha256sum -c SHA256SUMS`) and had a pixel-difference AE of 0. The script does
not require a display.

## Shell syntax check

The scripts are POSIX sh and pass `sh -n`. They use GNU/Linux utilities
(`install -d`, `readlink`, `find -exec ... +`); this packaging layer targets Linux.
`shellcheck` is not installed on the test host, so shellcheck was not run.
