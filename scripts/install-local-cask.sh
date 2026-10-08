#!/bin/sh
set -eu
cd "$(dirname "$0")/.."
archive="${1:?Usage: install-local-cask.sh archive.tar.gz}"
version=$(sed -n 's/^version = "\([^"]*\)"/\1/p' Cargo.toml | head -n 1)
tap="nachogea/informe-local"
if ! brew --repository "$tap" >/dev/null 2>&1; then
  HOMEBREW_DEVELOPER=1 brew tap-new "$tap"
fi
tap_dir=$(brew --repository "$tap")
python3 scripts/local-cask.py "$archive" --version "$version" \
  --output "$tap_dir/Casks/nachogea-informe.rb"
HOMEBREW_NO_AUTO_UPDATE=1 brew install --cask "$tap/nachogea-informe"
