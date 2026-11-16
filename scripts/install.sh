#!/usr/bin/env bash
# Install or update the House Builder BIMCraft plugin from the newest GitHub
# release of dronov-dmitry/bimcraft-plugin-create-home.
#
# One command (Linux, macOS):
#   curl -fsSL https://raw.githubusercontent.com/dronov-dmitry/bimcraft-plugin-create-home/main/scripts/install.sh | bash
#
# Environment, all optional:
#   BIMCRAFT_PLUGINS  the plugins folder to install into
#   PLUGIN_REPO       "owner/name" on GitHub
#   PLUGIN_VERSION    a release tag without "v" (default: the newest one)
#   PLUGIN_ZIP_URL    a direct archive URL, overriding the repository and version
#
# Requirements: bash 4+, curl, and either unzip or python3.
# Exit codes: 0 on success, 1 on any failure.

set -euo pipefail

PLUGIN_ID="house-builder"
PLUGIN_MODULE="house_builder.wasm"
REPO="${PLUGIN_REPO:-dronov-dmitry/bimcraft-plugin-create-home}"
VERSION="${PLUGIN_VERSION:-}"
KEEP_BACKUPS=3

# Paths of the two temporaries the run may leave behind; the EXIT trap removes them.
STAGE=""
ARCHIVE=""

pause_on_tty() {
    # A double-clicked window stays open; `curl | bash` and CI are not interrupted.
    if [ -t 0 ] && [ -t 1 ]; then
        read -r -n 1 -s -p "Press any key to close this window..." || true
        echo
    fi
}

finish() {
    if [ -n "$STAGE" ]; then
        rm -rf "$STAGE"
    fi
    if [ -n "$ARCHIVE" ]; then
        rm -f "$ARCHIVE"
    fi
    pause_on_tty
}

fail() {
    echo "error: $*" >&2
    exit 1
}

trap finish EXIT

plugins_dir() {
    if [ -n "${BIMCRAFT_PLUGINS:-}" ]; then
        printf '%s\n' "$BIMCRAFT_PLUGINS"
        return
    fi
    case "$(uname -s)" in
        Darwin)
            printf '%s\n' "$HOME/Library/Application Support/BIMCraft/plugins"
            ;;
        *)
            printf '%s\n' "${XDG_CONFIG_HOME:-$HOME/.config}/bimcraft/plugins"
            ;;
    esac
}

archive_url() {
    if [ -n "${PLUGIN_ZIP_URL:-}" ]; then
        printf '%s\n' "$PLUGIN_ZIP_URL"
    elif [ -n "$VERSION" ]; then
        printf 'https://github.com/%s/releases/download/v%s/%s-%s.zip\n' "$REPO" "$VERSION" "$PLUGIN_ID" "$VERSION"
    else
        printf 'https://github.com/%s/releases/latest/download/%s.zip\n' "$REPO" "$PLUGIN_ID"
    fi
}

extract() {
    local archive="$1" target="$2"
    mkdir -p "$target"
    if command -v unzip >/dev/null 2>&1; then
        unzip -q -o "$archive" -d "$target"
    else
        python3 -m zipfile -e "$archive" "$target"
    fi
}

check_stage() {
    local stage="$1" magic
    [ -f "$stage/plugin.json" ] || fail "the archive has no plugin.json"
    [ -f "$stage/$PLUGIN_MODULE" ] || fail "the archive has no $PLUGIN_MODULE"
    grep -Eq "\"id\"[[:space:]]*:[[:space:]]*\"$PLUGIN_ID\"" "$stage/plugin.json" \
        || fail "the archive's plugin.json is not $PLUGIN_ID"
    magic="$(head -c 4 "$stage/$PLUGIN_MODULE" | od -An -tx1 | tr -d ' \n')"
    [ "$magic" = "0061736d" ] || fail "$PLUGIN_MODULE is not a WebAssembly module"
}

prune_backups() {
    local folder="$1" index=0 backup
    local -a backups=()
    while IFS= read -r backup; do
        backups+=("$backup")
    done < <(ls -1dt "$folder"/."$PLUGIN_ID".backup-* 2>/dev/null || true)
    for backup in ${backups[@]+"${backups[@]}"}; do
        index=$((index + 1))
        if [ "$index" -gt "$KEEP_BACKUPS" ]; then
            rm -rf "$backup"
        fi
    done
}

main() {
    command -v curl >/dev/null 2>&1 || fail "curl is required"
    command -v unzip >/dev/null 2>&1 || command -v python3 >/dev/null 2>&1 \
        || fail "either unzip or python3 is required"

    local plugins destination backup url
    plugins="$(plugins_dir)"
    destination="$plugins/$PLUGIN_ID"
    url="$(archive_url)"
    mkdir -p "$plugins"

    # Both temporaries are inside the plugins folder: the final move is a rename
    # on one filesystem, so a half-installed version never exists.
    STAGE="$plugins/.$PLUGIN_ID.new.$$"
    ARCHIVE="$plugins/.$PLUGIN_ID.zip.$$"
    rm -rf "$STAGE"

    echo "Downloading $url"
    curl -fsSL --proto '=https' --retry 2 -o "$ARCHIVE" "$url" \
        || fail "could not download the release archive"
    extract "$ARCHIVE" "$STAGE"
    rm -f "$ARCHIVE"
    ARCHIVE=""
    check_stage "$STAGE"

    if [ -e "$destination" ]; then
        backup="$plugins/.$PLUGIN_ID.backup-$(date -u +%Y%m%d-%H%M%S)-$$"
        while [ -e "$backup" ]; do
            backup="$backup-1"
        done
        mv "$destination" "$backup"
        echo "Previous version kept at $backup"
    fi
    mv "$STAGE" "$destination"
    STAGE=""
    prune_backups "$plugins"

    echo "Installed $PLUGIN_ID into $destination"
    echo "Restart BIMCraft, or press Refresh in Manage > Plugins."
}

main "$@"
