#!/usr/bin/env bash
# Refresh the vendored BIMCraft plugin SDK from a BIMCraft checkout.
#
# The plugin builds against vendor/plugin-sdk, a copy of BIMCraft's
# crates/plugin-sdk, so this repository needs no checkout to build. The module
# and the host must speak the same ABI, so refresh the copy when BIMCraft's SDK
# changes, then rebuild and release the plugin.
#
# Run it from anywhere: ./scripts/sync-sdk.sh (double-click works too).
#
# Requirements: bash 4+.

set -euo pipefail

ROOT="$(cd -P -- "$(dirname -- "${BASH_SOURCE[0]}")/.." && pwd)"
VENDOR="$ROOT/vendor/plugin-sdk"

finish() {
    if [ -t 0 ] && [ -t 1 ]; then
        read -r -n 1 -s -p "Press any key to close this window..." || true
        echo
    fi
}
trap finish EXIT

fail() {
    echo "error: $*" >&2
    exit 1
}

ask() {
    local prompt="$1" default="$2" answer
    read -r -p "$prompt [$default]: " answer
    printf '%s\n' "${answer:-$default}"
}

default_checkout="$ROOT/../BIMCraft-plugin-update"
if [ ! -d "$default_checkout/crates/plugin-sdk" ]; then
    default_checkout="$ROOT/.."
fi

checkout="$(ask "BIMCraft checkout" "$default_checkout")"
checkout="${checkout/#\~/$HOME}"
checkout="$(cd -P -- "$checkout" 2>/dev/null && pwd)" || fail "no such folder: $checkout"

source_dir="$checkout/crates/plugin-sdk"
[ -f "$source_dir/src/lib.rs" ] || fail "no crates/plugin-sdk in $checkout"

version="$(sed -n 's/^version = "\(.*\)"/\1/p' "$checkout/Cargo.toml" | head -n 1)"
cp "$source_dir/src/lib.rs" "$VENDOR/src/lib.rs"
[ -f "$checkout/LICENSE-MIT" ] && cp "$checkout/LICENSE-MIT" "$VENDOR/LICENSE-MIT"
[ -f "$checkout/LICENSE-APACHE" ] && cp "$checkout/LICENSE-APACHE" "$VENDOR/LICENSE-APACHE"

echo "Copied crates/plugin-sdk from $checkout"
echo "Upstream workspace version: ${version:-unknown}"
echo "vendored Cargo.toml says:   $(sed -n 's/^version = "\(.*\)"/\1/p' "$VENDOR/Cargo.toml" | head -n 1)"
echo
echo "If the two differ, update version and rust-version in $VENDOR/Cargo.toml,"
echo "then rebuild: cargo build --release --target wasm32-unknown-unknown"
