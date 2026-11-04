[English](README.md) | [Русский](README_RU.md)

# House Builder test plugin

A plugin for exercising BIMCraft's plugin system. One button builds a small house with regular BIMCraft commands:
four walls, a floor, a door, a window, a room, a roof and a text note. All of it is **one undo step**;
if any command fails, the plugin returns an error and rolls back the whole call.

> **Important:** BIMCraft **does not execute `src/lib.rs`**. A plugin is `plugin.json` plus a compiled
> **WebAssembly module** (`house_builder.wasm`) named in the `module` field.
> Just dropping an `.rs` file into the plugins folder is not enough — it will not load.
>
> This folder **already contains** a ready `house_builder.wasm`, so to install and run it
> **you do not need to build anything**: copying the folder is enough. You only need to build
> if you change `src/lib.rs` — see the "Rebuilding the module" section.

## Do I need to build?

| What you are doing                                         | Do you need a build?                                          |
| ---------------------------------------------------------- | ------------------------------------------------------------- |
| Just installing this plugin and pressing Build House       | **No** — the folder already has `house_builder.wasm`          |
| Editing `src/lib.rs` or `Cargo.toml`                       | Yes — rebuild the `.wasm` and put it next to `plugin.json`    |
| Moving the folder to another machine                       | No — copy the whole folder, the `.wasm` is already inside     |
| Putting only `src/lib.rs` into the plugins folder          | It will not work: you need `plugin.json` and the `.wasm`      |

## What's inside

```text
test-plugin-house/
├── plugin.json          manifest: id house-builder, ABI 1, two functions, Test Plugins tab
├── house_builder.wasm   compiled module (wasm32-unknown-unknown, release)
├── Cargo.toml           plugin sources (separate cdylib, SDK from the BIMCraft checkout)
├── scripts/
│   ├── build-wasm.sh    runs the build (keeps the terminal window open)
│   ├── build-wasm.py    interactive .wasm build from src/lib.rs
│   └── test-plugin.sh   runs the built plugin through bimcraft-cli
└── src/lib.rs           the build and check functions
```

Functions:

| Command                      | What it does                                                                                                     |
| ---------------------------- | ---------------------------------------------------------------------------------------------------------------- |
| `plugin:house-builder:build` | builds a house; parameters `at`, `width`, `depth`, `height`, `room` (all optional)                               |
| `plugin:house-builder:check` | reads the model and returns a **text report** about its contents (`report` and `counts` fields); in the window the report opens in a dialog; it does not create an undo step |

### The Check House report

The `check` function returns JSON with a string field `report` — human-ready text:

```json
{"report": "MODEL REPORT - …", "counts": {"wall": 4, "floor": 1, "roof": 1, "door": 1, "window": 1, "room": 1}}
```

If a plugin command response has a string field `report`, the desktop opens it in a dialog
with the button label as the title (the plugin's text is not translated) and **Copy** / **Close** buttons;
in the CLI and MCP it is just JSON in the command response. The `check` report contains: the project name and its
standard, levels with elevations, elements by category, what stands on each level, rooms with areas and
the enclosed flag, walls (total length, height, types), doors and windows (marks, types, dimensions), floors
with area, roofs, text notes, library size (how many types, materials, families and
parameters), warnings and the number of history steps.

## Installation

### Option 1: point an environment variable at the folder (quick, no copying)

`BIMCRAFT_PLUGINS` can point directly at the plugin folder (the one containing `plugin.json`):

```sh
cd /media/kat/Seagate5-2/00_Opensource/27_BIMCraft-0100/BIMCraft-plugins-clean
BIMCRAFT_PLUGINS=/media/kat/Seagate5-2/00_Opensource/27_BIMCraft-0100/test-plugin-house \
  cargo run -q -p bimcraft-cli -- run --cmd 'plugin:house-builder:build {}'
```

For the desktop, the same thing before launching:

```sh
BIMCRAFT_PLUGINS=/media/kat/Seagate5-2/00_Opensource/27_BIMCraft-0100/test-plugin-house \
  cargo run -q -p bimcraft
```

An empty value `BIMCRAFT_PLUGINS=` disables plugins completely.

### Option 2: put it into the regular plugins folder (the way a user installs it)

Copy the whole `test-plugin-house` folder into the user's plugins folder:

```sh
# Linux
mkdir -p ~/.config/bimcraft/plugins
cp -r /media/kat/Seagate5-2/00_Opensource/27_BIMCraft-0100/test-plugin-house ~/.config/bimcraft/plugins/

# macOS
mkdir -p ~/Library/Application\ Support/BIMCraft/plugins
cp -r /media/kat/Seagate5-2/00_Opensource/27_BIMCraft-0100/test-plugin-house ~/Library/Application\ Support/BIMCraft/plugins/
```

```powershell
# Windows (PowerShell)
New-Item -ItemType Directory -Force "$env:APPDATA\BIMCraft\plugins"
Copy-Item -Recurse "C:\path\test-plugin-house" "$env:APPDATA\BIMCraft\plugins\"
```

If `XDG_CONFIG_HOME` is set, on Linux the plugins folder is `$XDG_CONFIG_HOME/bimcraft/plugins`.
After copying, restart the application or press **Refresh** in **Manage > Plugins**.

Checking the installation in the CLI:

```sh
bimcraft-cli commands | grep house-builder
```

The list must contain `plugin:house-builder:build` and `plugin:house-builder:check` with the field `"plugin": "house-builder"`.

## How to check it in the UI

1. Launch the desktop (with `BIMCRAFT_PLUGINS=...` or after copying into the plugins folder).
2. Open **Manage > Plugins**. The list will show `House Builder (test) 0.1.0`, status **Loaded**,
   author `BIMCraft test`, two functions; below that is the load log.
3. A **Test Plugins** tab appears in the ribbon with a **House** panel and the **Build House** and **Check House** buttons.
4. Press **Build House** — walls, a floor, a door, a window, a room and a roof appear on the plan;
   there must be no errors in the status bar. In the plugin log: `building a 8000 x 6000 mm house at 0, 0`
   and `the house is built: one undo step`.
5. Press **Ctrl+Z** (Undo) — the whole house disappears in one step labelled
   `House Builder (test): Build House`.
6. **Check House** opens the **Check House** dialog with a text report about the model: levels with elevations,
   elements by category, what stands on each level, rooms with areas, walls with total length,
   doors and windows with dimensions, floors, roofs, notes, the library, warnings and history steps
   (for a freshly built house — `Walls 4 · Floors 1 · Doors 1 · Windows 1 · Rooms 1`, etc.).
   There will be no undo step; in **Manage > Plugins** the log will show `the model report has N lines`.
7. In **Manage > Plugins** press **Disable** — the **Test Plugins** tab and the commands disappear immediately;
   **Enable** brings them back. **Refresh** re-reads the folder without a restart.

## The same checks in the CLI

```sh
export BIMCRAFT_PLUGINS=/media/kat/Seagate5-2/00_Opensource/27_BIMCraft-0100/test-plugin-house
cd /media/kat/Seagate5-2/00_Opensource/27_BIMCraft-0100/BIMCraft-plugins-clean

# 1. Empty project: check before building
cargo run -q -p bimcraft-cli -- run --cmd 'plugin:house-builder:check'

# 2. Build (defaults: 8000 x 6000 mm at [0, 0])
cargo run -q -p bimcraft-cli -- run --cmd 'plugin:house-builder:build {}'

# 3. Custom dimensions and room name
cargo run -q -p bimcraft-cli -- run \
  --cmd 'plugin:house-builder:build {"at":[1000,2000],"width":5000,"depth":4000,"room":"Studio"}'

# 4. One undo step: after undo the counters are zero again
cargo run -q -p bimcraft-cli -- run \
  --cmd 'plugin:house-builder:build {}' --cmd 'undo' --cmd 'plugin:house-builder:check'

# 5. Save the project and look at the plan
cargo run -q -p bimcraft-cli -- run \
  --cmd 'plugin:house-builder:build {}' --save /tmp/house.bimcraft
cargo run -q -p bimcraft-cli -- run /tmp/house.bimcraft \
  --cmd 'render {"path":"/tmp/house.png","width":900,"height":600}'
```

Expected responses for an 8000 × 6000 house:

- `build` returns `size`, the wall ids (`walls.ids` — 4), the floor, door, window, room, roof and note;
  the floor and room area is 48 m².
- `check` returns `{"report": "<report text>", "counts":{"wall":4,"floor":1,"roof":1,"door":1,"window":1,"room":1}}`
  (the report text is multiline; in the CLI it is visible in the response JSON, in the desktop it opens in a dialog).
- `undo` — `{"undone":"House Builder (test): Build House","count":1}`.

## Through MCP

The `execute` tool of the existing MCP server accepts a plugin command; there are no separate tools
for plugins:

```sh
BIMCRAFT_PLUGINS=/media/kat/Seagate5-2/00_Opensource/27_BIMCraft-0100/test-plugin-house \
  cargo run -q -p bimcraft-cli -- mcp
```

Then, for example, `tools/call` with `{"name":"execute","arguments":{"command":"plugin:house-builder:build","params":{}}}`.
`list_commands` will show the plugin commands with `"plugin":"house-builder"`.

## Testing the built plugin

`scripts/test-plugin.sh` tests the **already built** module: it takes `bimcraft-cli` from the BIMCraft
checkout, points `BIMCRAFT_PLUGINS` at the plugin folder and runs `check` in four scenarios —
an empty project, a freshly built house, `--sample` and `--highrise` (20 floors, about 10,000 elements) —
and then prints a table and the plugin log for the failed scenarios:

```sh
cd /media/kat/Seagate5-2/00_Opensource/27_BIMCraft-0100/test-plugin-house
./scripts/test-plugin.sh
```

The script is interactive: it asks for the plugin folder and the BIMCraft checkout (Enter accepts the default),
builds nothing and changes nothing. Double-clicking works too. If the `.wasm` is older than `src/lib.rs`,
it warns about that.

The expected picture is **4 out of 4**. If a scenario fails with `all fuel consumed by WebAssembly`, the
plugin has hit the sandbox fuel limit: parsing a response costs about 0.7 thousand fuel per byte, so
20 million is about 25 KB of responses per call. That is why the report calls `inspect {"brief": true}` (levels,
counters, warnings, history and the **size** of the library — a quarter of a kilobyte instead of nineteen)
and shortens element lists on a large model. Details are in
`BIMCraft-plugins-clean/docs/areas/plugins.md`, section "Where exactly the call failed": it also describes the
log line `the call ran 24 command(s) and used … M fuel; the last steps, and the fuel left when each ran: …`,
which shows what the fuel was spent on.

## Rebuilding the module

The ready `.wasm` is already in the folder. After editing `src/lib.rs` you can rebuild it with a script
(it asks for the plugin folder, target, release and where to put the built module):

```sh
cd /media/kat/Seagate5-2/00_Opensource/27_BIMCraft-0100/test-plugin-house
./scripts/build-wasm.sh
```

`build-wasm.sh` is a thin wrapper: it runs `scripts/build-wasm.py` and does not close the
terminal after success or failure. It can also be started with a double click. The Python script
is interactive: Enter accepts the default values. If the `wasm32-unknown-unknown` target is
not installed, it will offer to run `rustup target add`.

Manual equivalent:

```sh
cd /media/kat/Seagate5-2/00_Opensource/27_BIMCraft-0100/test-plugin-house
rustup target add wasm32-unknown-unknown
cargo build --target wasm32-unknown-unknown --release
cp target/wasm32-unknown-unknown/release/house_builder.wasm ./house_builder.wasm
```

`scripts/build-wasm.py` reads the package name from `Cargo.toml`, finds
`target/<target>/release/<name>.wasm`, checks the WebAssembly signature and copies the file
to the path from `"module"` in `plugin.json` (`house_builder.wasm`). The script requires Python 3.10+
and the standard library — there are no third-party packages.

`Cargo.toml` takes the SDK from the relative path `../BIMCraft-plugins-clean/crates/plugin-sdk`.
If the plugin folder is moved, fix `path` to point at your checkout or build the plugin next to it.

## Troubleshooting

- **The plugin is not in the list.** Check `plugin.json` in the folder and the `BIMCRAFT_PLUGINS` path; the reason
  for the failure is always written to **Manage > Plugins** and to the CLI stderr.
- **The `there's no type ...` error / the door or window is not placed.** The plugin uses the project's default
  types; in a project without the standard door/window types, create them or specify the type manually.
- **The roof is not built.** The roof is placed over the walls of the level below; if walls already exist on another level
  or the project is empty, build the house in a new project (`--cmd 'new {}'`).
- **The house is built on top of the old one.** The plugin does not clear the model: use a new project or a different `at`.
- The plugin has no `files` permission: it does not read or write files, it only calls model commands.

The plugin folder lives outside the git repository and is not part of PR #256; it is a separate test artifact.



Yes, that's right: `.rs` alone does not work. BIMCraft does not compile Rust on the fly — it only loads the ready WebAssembly module named in the manifest:
`"module": "house_builder.wasm"`
So a plugin is two required files:
`plugin.json` — manifest
`house_builder.wasm` — built module
`src/lib.rs` and `Cargo.toml` are the sources for building; the application does not read them.
At the same time, `test-plugin-house` already contains a built `house_builder.wasm` (107,510 bytes), so to install and run it you do not need to build anything — just copy the folder into the plugins folder.
If you leave only the `.rs` file in the folder, loading fails: the `house_builder.wasm` module is not found, the plugin appears in Manage > Plugins with an error status, and its commands are not registered.