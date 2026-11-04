#!/usr/bin/env bash
# Run build-wasm.py, which builds a BIMCraft plugin's WebAssembly module and
# copies it next to plugin.json. Keeps this terminal open after success or
# failure, so it can be started by double-clicking it in a file manager.
#
# Usage: ./scripts/build-wasm.sh (no command-line flags; the Python script asks
# the questions and Enter accepts each default).
set -euo pipefail

here="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
script="$here/build-wasm.py"

pause() {
    read -n 1 -s -r -p "Press any key to close this window..." || true
    echo
}

if ! command -v python3 >/dev/null 2>&1; then
    echo "error: python3 is required but was not found in PATH" >&2
    pause
    exit 1
fi
if [ ! -f "$script" ]; then
    echo "error: $script was not found" >&2
    pause
    exit 1
fi

status=0
set +e
python3 "$script"
status=$?
set -e

echo
if [ "$status" -eq 0 ]; then
    echo "The plugin module was built."
else
    echo "The plugin module was not built (exit code $status)." >&2
fi
pause
exit "$status"