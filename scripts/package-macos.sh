#!/bin/sh
set -eu
cd "$(dirname "$0")/.."
sh scripts/bundle-macos.sh release
if [ -n "${SIGNING_IDENTITY:-}" ] || [ -n "${NOTARY_PROFILE:-}" ]; then
  sh scripts/sign-macos.sh
  label="Signed and notarized"
else
  label="Unsigned internal"
fi
version=$(sed -n 's/^version = "\([^"]*\)"/\1/p' Cargo.toml | head -n 1)
arch=$(uname -m)
mkdir -p target/dist
archive="target/dist/informe-$version-macos-$arch.tar.gz"
staging=$(mktemp -d)
trap 'rm -rf "$staging"' EXIT
cp -R 'target/release/Informe.app' "$staging/"
cp -R skills "$staging/skills"
cp README.md LICENSE CONTRIBUTING.md rust-toolchain.toml "$staging/"
cp -R docs examples schema artifacts packaging assets "$staging/"
mkdir -p "$staging/benchmarks/results"
cp benchmarks/README.md "$staging/benchmarks/README.md"
# Keep result documentation/raw data, excluding local benchmark screenshots.
mkdir -p "$staging/benchmarks/results/pilot"
cp benchmarks/results/pilot/*.md benchmarks/results/pilot/*.json benchmarks/results/pilot/*.jsonl "$staging/benchmarks/results/pilot/"
mkdir -p "$staging/benchmarks/results/viewer-2026-10-03"
cp benchmarks/results/viewer-2026-10-03/*.md benchmarks/results/viewer-2026-10-03/*.json benchmarks/results/viewer-2026-10-03/*.jsonl "$staging/benchmarks/results/viewer-2026-10-03/"
# Review sidecars may contain private comments and full report snapshots.
# Git ignores them, but copied example directories can still contain local data.
tar --exclude='*.review.json' --exclude='*.review.lock' --exclude='*.review.pending-*' -czf "$archive" -C "$staging" .
shasum -a 256 "$archive" > "$archive.sha256"
printf '%s archive: %s\n' "$label" "$archive"
