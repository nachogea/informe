#!/bin/sh
set -eu
cd "$(dirname "$0")/.."
"${CARGO:-$HOME/.cargo/bin/cargo}" run --locked --example render-icon
iconutil -c icns target/brand/AppIcon.iconset -o assets/brand/AppIcon.icns
printf 'Generated assets/brand/AppIcon.icns\n'
