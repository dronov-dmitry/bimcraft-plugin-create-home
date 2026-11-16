#!/usr/bin/env bash
# Create the Ed25519 key that signs this plugin's update feed.
#
# The public half goes into plugin.json as `"update": {"public_key": "ed25519:…"}`.
# The private half is the GitHub secret PLUGIN_SIGNING_KEY, which the release
# workflow uses to sign latest.json; it must never be committed.
#
# Run it from anywhere: ./scripts/make-signing-key.sh (double-click works too).
#
# Requirements: bash 4+, openssl 3.

set -euo pipefail

ROOT="$(cd -P -- "$(dirname -- "${BASH_SOURCE[0]}")/.." && pwd)"
DEFAULT_KEY="$ROOT/keys/house-builder-signing-key.pem"

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

command -v openssl >/dev/null 2>&1 || fail "openssl is required"
openssl genpkey -algorithm ed25519 -out /dev/null 2>/dev/null \
    || fail "this openssl cannot make an Ed25519 key; openssl 3 is required"

key_path="$(ask "Private key file" "$DEFAULT_KEY")"
key_path="${key_path/#\~/$HOME}"

if [ -e "$key_path" ]; then
    echo "A key already exists at $key_path."
    read -r -p "Replace it? The installed plugin would stop updating until the new key is published. [y/N] " answer
    case "${answer:-n}" in
        y|Y|yes|YES) ;;
        *) echo "Kept the existing key."; exit 0 ;;
    esac
fi

mkdir -p "$(dirname -- "$key_path")"
umask 077
openssl genpkey -algorithm ed25519 -out "$key_path"
chmod 600 "$key_path"

public_key="$(openssl pkey -in "$key_path" -pubout -outform DER | tail -c 32 | base64 | tr -d '\n')"

echo
echo "Private key: $key_path (keep it off the repository; keys/ is gitignored)"
echo
echo "Public key, to paste into plugin.json as update.public_key:"
echo "  ed25519:$public_key"
echo
echo "GitHub secret, to add at Settings > Secrets and variables > Actions > New repository secret:"
echo "  name:  PLUGIN_SIGNING_KEY"
echo "  value: the contents of $key_path, BEGIN/END lines included"
echo
echo "Then tag a release; scripts/make-feed.py signs latest.json with this key."
