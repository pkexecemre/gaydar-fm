#!/usr/bin/env bash
# Build an RPM with nfpm (no rpmbuild required).
# Put nfpm on PATH, then run packaging/build-rpm.sh.
# Output: dist/gaydar-fm-<version>.<arch>.rpm
set -euo pipefail
ROOT=$(cd "$(dirname "$0")/.." && pwd -P)
cd "$ROOT"
BIN="$ROOT/target/release/gaydar"
[[ -x "$BIN" ]] || { echo "error: build target/release/gaydar first" >&2; exit 1; }
command -v nfpm >/dev/null 2>&1 || { echo "error: nfpm not on PATH" >&2; exit 1; }
VERSION=$(sed -n 's/^version = "\([^"]*\)"/\1/p' Cargo.toml | head -1)
sed "s/^version: .*/version: $VERSION/" packaging/nfpm.yaml > /tmp/gaydar-nfpm.yaml
mkdir -p dist
nfpm pkg --packager rpm --config /tmp/gaydar-nfpm.yaml --target dist/
ls -lh dist/*.rpm
sha256sum dist/*.rpm | tee dist/gaydar-fm.rpm.sha256
