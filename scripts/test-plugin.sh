#!/usr/bin/env bash
# Tests a built BIMCraft plugin end to end: it loads the plugin into bimcraft-cli with BIMCRAFT_PLUGINS and
# runs its functions against an empty project, the house the plugin builds, the sample house and the
# twenty-storey sample, then prints a table of what each did and the plugin log of the ones that failed.
#
# Usage: ./scripts/test-plugin.sh
#   It asks for the plugin folder and the BIMCraft checkout; Enter keeps each default.
#   Needs bash 4+, python3, and a bimcraft-cli built in the checkout (target/release or target/debug).
set -euo pipefail

pause() {
    read -n 1 -s -r -p "Press any key to close this window..." || true
    echo
}

die() {
    echo "error: $*" >&2
    pause
    exit 1
}

ask() { # prompt, default
    local answer
    read -r -p "$1 [$2]: " answer || true
    printf '%s\n' "${answer:-$2}"
}

# The manifest's id, module file name and function ids, one `key=value` a line.
manifest_info() { # path
    python3 - "$1" <<'PY'
import json
import sys

data = json.load(open(sys.argv[1], encoding="utf-8"))
print("id=" + str(data.get("id", "")))
print("module=" + str(data.get("module", "plugin.wasm")))
print("functions=" + ",".join(str(f.get("id", "")) for f in data.get("functions", [])))
PY
}

# How many lines the last JSON line of `file` has in its `report`, and the report's first line, tab-separated.
report_of() { # path
    python3 - "$1" <<'PY'
import json
import sys

report = None
with open(sys.argv[1], encoding="utf-8", errors="replace") as handle:
    for line in reversed(handle.read().splitlines()):
        line = line.strip()
        if not line:
            continue
        try:
            value = json.loads(line)
        except ValueError:
            continue
        if isinstance(value, dict) and isinstance(value.get("report"), str):
            report = value["report"]
            break
if report is None:
    print("0\t")
else:
    lines = report.splitlines()
    print(f"{len(lines)}\t{lines[0] if lines else ''}")
PY
}

here="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
default_plugin="$(dirname "$here")"

command -v python3 >/dev/null 2>&1 || die "python3 is required but was not found in PATH"

echo "BIMCraft plugin test"
plugin_dir="$(ask "Plugin folder" "$default_plugin")"
checkout="$(ask "BIMCraft checkout" "$plugin_dir/../BIMCraft-plugins-clean")"
[ -d "$plugin_dir" ] || die "no folder at $plugin_dir"
[ -d "$checkout" ] || die "no folder at $checkout"
plugin_dir="$(cd "$plugin_dir" && pwd)"
checkout="$(cd "$checkout" && pwd)"

cli=""
for candidate in "$checkout/target/debug/bimcraft-cli" "$checkout/target/release/bimcraft-cli"; do
    if [ -x "$candidate" ]; then
        cli="$candidate"
        break
    fi
done
if [ -z "$cli" ]; then
    cli="$(command -v bimcraft-cli || true)"
fi
[ -n "$cli" ] || die "no bimcraft-cli: build it with 'cargo build -p bimcraft-cli' in $checkout"

manifest="$plugin_dir/plugin.json"
[ -f "$manifest" ] || die "no plugin.json in $plugin_dir"

plugin_id=""
module="plugin.wasm"
functions=""
while IFS='=' read -r key value; do
    case "$key" in
        id) plugin_id="$value" ;;
        module) module="$value" ;;
        functions) functions="$value" ;;
    esac
done < <(manifest_info "$manifest")

[ -n "$plugin_id" ] || die "the manifest has no id"
module_path="$plugin_dir/$module"
[ -f "$module_path" ] || die "no module at $module_path: build the plugin first"

case ",$functions," in
    *",check,"*) ;;
    *) die "the manifest has no check function: its functions are $functions" ;;
esac
has_build="no"
case ",$functions," in
    *",build,"*) has_build="yes" ;;
esac

newer="$(find "$plugin_dir/src" -name '*.rs' -newer "$module_path" -print -quit 2>/dev/null || true)"
if [ -n "$newer" ]; then
    echo "warning: $module_path is older than $newer: rebuild the plugin first"
fi

echo
printf 'plugin   %s\n' "$plugin_id"
printf 'folder   %s\n' "$plugin_dir"
printf 'module   %s (%s bytes)\n' "$module" "$(wc -c < "$module_path" | tr -d ' ')"
printf 'cli      %s\n' "$cli"
echo

work="$(mktemp -d)"
trap 'rm -rf "$work"' EXIT

passed=0
failed=0
failed_names=()
failed_logs=()

# Runs one scenario: its label, then the arguments for `bimcraft-cli run`. Sets `result`, `detail` and `log`.
scenario() { # label, args...
    local label="$1"
    shift
    local slug
    slug="$(printf '%s' "$label" | tr -cs 'a-zA-Z0-9' '-')"
    local out="$work/$slug.out"
    local err="$work/$slug.err"
    local start end status

    start=$(date +%s%N)
    status=0
    ( cd "$checkout" && BIMCRAFT_PLUGINS="$plugin_dir" "$cli" run "$@" ) >"$out" 2>"$err" || status=$?
    end=$(date +%s%N)

    took=$(( (end - start) / 1000000 ))
    log="$err"
    if [ "$status" -eq 0 ]; then
        result="PASS"
        local lines first
        IFS=$'\t' read -r lines first < <(report_of "$out")
        if [ "$lines" -gt 0 ]; then
            detail="$lines lines: $first"
        else
            detail="no report in the reply"
        fi
    else
        result="FAIL"
        detail="$(grep -E 'failed|fuel|error' "$err" | tail -1 | cut -c1-120 || true)"
        [ -n "$detail" ] || detail="exit code $status"
    fi
}

check() { # label, args...
    local label="$1"
    shift
    scenario "$label" "$@"
    printf '%-32s %-6s %7s ms  %s\n' "$label" "$result" "$took" "$detail"
    if [ "$result" = "PASS" ]; then
        passed=$((passed + 1))
    else
        failed=$((failed + 1))
        failed_names+=("$label")
        failed_logs+=("$log")
    fi
}

check "empty project / check" \
    --cmd "plugin:$plugin_id:check"
if [ "$has_build" = "yes" ]; then
    check "house built / build + check" \
        --cmd "plugin:$plugin_id:build" --cmd "plugin:$plugin_id:check"
else
    check "house built / no build function" \
        --cmd "plugin:$plugin_id:check"
fi
check "sample house / check" \
    --sample --cmd "plugin:$plugin_id:check"
check "twenty-storey sample / check" \
    --highrise --cmd "plugin:$plugin_id:check"

echo
printf '%d of %d scenarios passed\n' "$passed" "$((passed + failed))"

if [ "$failed" -gt 0 ]; then
    echo
    for i in "${!failed_names[@]}"; do
        echo "--- ${failed_names[$i]}: plugin log ---"
        grep '^bimcraft: plugin' "${failed_logs[$i]}" | tail -12 || true
        echo "--- ${failed_names[$i]}: error ---"
        grep -v '^bimcraft: plugin' "${failed_logs[$i]}" | tail -3 || true
        echo
    done
    echo "A call that fails with 'all fuel consumed by WebAssembly' used its whole fuel budget. The log line"
    echo "'the call ran N command(s) ...' says what it went on: each command and the fuel it left. The reply"
    echo "of a command is what costs: about 0.7 thousand fuel a byte, so a command that answers with a lot"
    echo "of JSON can use the whole 20 million by itself."
fi

if [ "$failed" -eq 0 ]; then
    echo "All good."
fi
pause
[ "$failed" -eq 0 ]