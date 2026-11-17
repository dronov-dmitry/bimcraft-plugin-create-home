[English](README.md) | [Русский](README_RU.md)

# House Builder test plugin

A plugin for exercising BIMCraft's plugin system. One button builds a small house with regular BIMCraft commands:
four walls, a floor, a door, a window, a room, a roof and a text note. All of it is **one undo step**;
if any command fails, the plugin returns an error and rolls back the whole call.

Video: https://youtu.be/OuOVO3mU8HE

The plugin is published on GitHub as a release, so it goes in with **one terminal command**.
By default it is built for every ABI 1 host: it imports only `execute`, `log` and `info`, so the same
module runs in BIMCraft builds with and without plugin self-update. Self-update is an optional Cargo
feature (`--features self-update`) for hosts that link `update_check` and `update_apply`: see
[Installation](#installation) and [Releases and self-update](#releases-and-self-update).

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
├── plugin.json          manifest: id house-builder, ABI 1, three functions, the `files` permission
├── house_builder.wasm   compiled module (wasm32-unknown-unknown, release; no self-update imports)
├── Cargo.toml           plugin sources (separate cdylib, SDK taken from vendor/)
├── vendor/plugin-sdk/   a copy of BIMCraft's crates/plugin-sdk, so the build needs no checkout
├── .github/workflows/
│   ├── build.yml        builds the module on every push and pull request
│   └── release.yml      on a v* tag: build, pack, sign the feed, publish the release
├── scripts/
│   ├── install.sh       installs or updates from the newest release (Linux, macOS)
│   ├── install.ps1      the same for Windows PowerShell
│   ├── make-feed.py     writes and signs latest.json for a release
│   ├── make-signing-key.sh  creates the Ed25519 key pair that signs the feed
│   ├── sync-sdk.sh      refreshes vendor/plugin-sdk from a BIMCraft checkout
│   ├── build-wasm.sh    runs the build (keeps the terminal window open)
│   ├── build-wasm.py    interactive .wasm build from src/lib.rs
│   └── test-plugin.sh   runs the built plugin through bimcraft-cli
├── src/lib.rs           the build, check and open_file functions; `update` behind `self-update`
└── keys/                the feed signing key (gitignored, never committed)
```

Functions:

| Command                      | What it does                                                                                                     |
| ---------------------------- | ---------------------------------------------------------------------------------------------------------------- |
| `plugin:house-builder:build` | builds a house; parameters `at`, `width`, `depth`, `height`, `room` (all optional)                               |
| `plugin:house-builder:check` | reads the model and returns a **report** about its contents (`report` and `counts` fields) — Markdown with tables on a host that reports `markdown_reports`, plain text otherwise; in the window the report opens in a dialog; it does not create an undo step |
| `plugin:house-builder:open_file` | **Open File** — opens a project file (`.bimcraft` or `.ifc`) by absolute path, replacing the project; needs the `files` permission |
| `plugin:house-builder:update` | **Update Plugin** — optional: built only with `--features self-update`, for a BIMCraft that links `update_check`/`update_apply`. Asks the release channel what the newest version is and answers with `current`, `available`, `released`, `notes`, `host` and `key`; with `install: true` it installs that version; needs the `update` permission |

### The Check House report

On a host that reports `markdown_reports` in `bimcraft.info` — the desktop with Markdown reports — `check`
returns a Markdown report, and the window draws its headings, lists and tables:

```json
{"report": {"text": "# Model report — Project\n\n## Levels (2)\n\n| Name | Elevation |\n|:---|---:|\n| Level 1 | 0 mm |\n| Level 2 | 3000 mm |\n", "format": "markdown"}, "counts": {"wall": 4, "floor": 1, "roof": 1, "door": 1, "window": 1, "room": 1}}
```

An older host knows only a string `report`, so there the same content comes as a plain monospace string.

The desktop opens the report in a dialog with the button label as the title (the plugin's text is not
translated) and **Copy** / **Close** buttons; in the CLI and MCP it is just JSON in the command response.
The `check` report contains: the project name and its standard, levels with elevations, elements by category,
what stands on each level, rooms with areas and the enclosed flag, walls (total length, height, types),
doors and windows (marks, types, dimensions), floors with area, roofs, text notes, library size (how many
types, materials, families and parameters), warnings and the number of history steps. The levels,
elements, rooms and openings are Markdown tables.

## Installation

### One command, from the newest release

The plugin is published as a GitHub release, so installing or updating it is one line in a terminal.
The script resolves the newest release, checks the archive, installs it, and keeps the previous three
versions as backups next to it:

```sh
# Linux and macOS
curl -fsSL https://raw.githubusercontent.com/dronov-dmitry/bimcraft-plugin-create-home/main/scripts/install.sh | bash
```

```powershell
# Windows (PowerShell)
irm https://raw.githubusercontent.com/dronov-dmitry/bimcraft-plugin-create-home/main/scripts/install.ps1 | iex
```

It uses the standard plugins folder: `~/.config/bimcraft/plugins` on Linux
(`$XDG_CONFIG_HOME/bimcraft/plugins` when that is set), `~/Library/Application Support/BIMCraft/plugins`
on macOS, `%APPDATA%\BIMCraft\plugins` on Windows. Four environment variables override the defaults,
all optional:

| Variable           | Meaning                                                                  |
| ------------------ | ------------------------------------------------------------------------ |
| `BIMCRAFT_PLUGINS` | the plugins folder to install into                                       |
| `PLUGIN_VERSION`   | a release tag without `v`, e.g. `0.2.0`, instead of the newest release   |
| `PLUGIN_ZIP_URL`   | a direct archive URL, for a local build you packed yourself              |
| `PLUGIN_REPO`      | `owner/name` on GitHub, when you forked the plugin                       |

A different folder, for example:

```sh
curl -fsSL https://raw.githubusercontent.com/dronov-dmitry/bimcraft-plugin-create-home/main/scripts/install.sh \
  | BIMCRAFT_PLUGINS=/opt/bimcraft/plugins bash
```

Then restart BIMCraft or press **Refresh** in **Manage > Plugins**. The installed folder holds exactly
two files, `plugin.json` and `house_builder.wasm`: that is the same archive the update channel uses.

### Option 2: point an environment variable at the folder (quick, no copying)

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

### Option 3: put it into the regular plugins folder (the way a user installs it)

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
2. Open **Manage > Plugins**. The list will show `House Builder (test) 0.3.0`, status **Loaded**,
   author `BIMCraft test`, three functions; the description mentions the `files` request;
   below that is the load log.
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

`Cargo.toml` takes the SDK from `vendor/plugin-sdk`, a copy of BIMCraft's `crates/plugin-sdk`, so the
repository builds on its own and CI needs no checkout. When BIMCraft's SDK changes, refresh the copy
with `./scripts/sync-sdk.sh` (it asks for a BIMCraft checkout and copies `src/lib.rs` and the licenses),
keep `version` in `vendor/plugin-sdk/Cargo.toml` in step, and rebuild.

## Releases and self-update

A release is a tag. The `release` workflow builds the module, packs the archive and publishes:

| Asset                        | What it is                                                                 |
| ---------------------------- | -------------------------------------------------------------------------- |
| `house-builder-<version>.zip`| the release archive: exactly `plugin.json` and the module, nothing else    |
| `house-builder.zip`          | the same archive under a stable name, so `releases/latest/download/house-builder.zip` always means the newest one |
| `latest.json`                | the signed feed behind `releases/latest/download/latest.json`, only when `plugin.json` declares `update.public_key` |

The default build has no self-update and declares no `update` block, because older ABI 1 hosts refuse
it. Such a release has no feed: `scripts/install.sh` installs the stable archive, and updating means
running the installer again.

To publish a version:

```sh
# 1. Set "version" in plugin.json and Cargo.toml to the same number.
# 2. Commit, then tag and push the tag:
git tag -a v0.2.0 -m "House Builder 0.2.0: install and update from releases"
git push origin v0.2.0
```

The workflow refuses a tag that disagrees with `plugin.json`, so the manifest and the feed never drift.

### The update channel

`plugin.json` declares where updates come from and the key that signs them:

```json
"permissions": ["files", "update"],
"update": {
  "feed": "https://github.com/dronov-dmitry/bimcraft-plugin-create-home/releases/latest/download/latest.json",
  "public_key": "ed25519:wZadJhhSSUG+bQwWLo2caMWPxSU5OCeYajloeoqTQUU="
}
```

The feed names the version, the archive URL, its size and SHA-256, and carries an Ed25519 signature over
those fields. The host fetches the feed, checks the signature against the key in the manifest, downloads
the archive and replaces the plugin folder atomically. The plugin itself has no network and no disk:
the host does both.

**The permission is the user's.** BIMCraft asks nobody automatically: in **Manage > Plugins** the
**Allow updating** switch (one switch for the permission and for automatic installs) is off by default.
The first install after turning it on shows the channel host, the key fingerprint, the version and the
size; later versions install without further questions. A new permission in a newer version, or a new
publisher key, stops the automation and asks again.

The plugin also exposes the channel as a command, which is handy for a scripted check:

```sh
# Ask only:
cargo run -q -p bimcraft-cli -- run --cmd 'plugin:house-builder:update {}'
# {"current":"0.2.0","available":"0.3.0","upToDate":false,"channel":{...}}

# Install what the channel announces (the host still asks the user):
cargo run -q -p bimcraft-cli -- run --cmd 'plugin:house-builder:update {"install":true}'
```

A new module runs after BIMCraft restarts; until then the old version keeps working. Backup folders sit
next to the plugin (`.house-builder.backup-…`, the newest three are kept), and **Manage > Plugins** has
**Check for update**, **Update now** and **Roll back**.

### The signing key

The repository secret `PLUGIN_SIGNING_KEY` holds the Ed25519 **private** key in PEM; the workflow signs
`latest.json` with it. `scripts/make-signing-key.sh` creates the pair, prints the public key for
`plugin.json` and explains where to paste the private one. The key lives in `keys/`, which is
gitignored: a key in the repository would let anyone publish an update the plugin would trust.

Rotating the key means a new `public_key` in the manifest; every user is asked to accept the change once.

> **Self-update is optional, and the module in this folder is built without it.** It needs a BIMCraft
> whose ABI 1 host links `bimcraft.update_check` and `bimcraft.update_apply`; a host with only `execute`,
> `log` and `info` refuses any module that imports them, even when the function is never called. That is
> why the default build imports only the three original host functions, and `plugin.json` declares only
> the `files` permission: the same module and manifest load in both kinds of build. To get the **Update
> Plugin** button back on a host with self-update, restore the `update` block, the `update` permission and
> the fourth function in the manifest, then rebuild with `cargo build --target wasm32-unknown-unknown
> --release --features self-update`.

## Troubleshooting

- **The plugin is not in the list.** Check `plugin.json` in the folder and the `BIMCRAFT_PLUGINS` path; the reason
  for the failure is always written to **Manage > Plugins** and to the CLI stderr.
- **The `there's no type ...` error / the door or window is not placed.** The plugin uses the project's default
  types; in a project without the standard door/window types, create them or specify the type manually.
- **The roof is not built.** The roof is placed over the walls of the level below; if walls already exist on another level
  or the project is empty, build the house in a new project (`--cmd 'new {}'`).
- **The house is built on top of the old one.** The plugin does not clear the model: use a new project or a different `at`.
- **`build` and `check` need no permission.** Only `open_file` touches the disk (`files`), and only
  `update` reaches the network channel (`update`); refuse those two and the rest keeps working.
- **The plugin is missing from Manage > Plugins entirely, with an unknown-permission error.** The
  manifest asks for `update`, and this BIMCraft build predates the plugin update channel. Update
  BIMCraft, or remove the `update` permission, the `update` block and the update function from
  `plugin.json` and rebuild.
- **`plugin:house-builder:update` answers `the plugin is not allowed to update itself`.** The `update`
  permission is not granted: turn on **Allow updating** for the plugin in **Manage > Plugins**.
- **The update check fails on the signature.** The feed was signed with a different key than
  `update.public_key`, or `latest.json` was edited by hand: a feed is signed over its exact bytes, so
  regenerate it with `scripts/make-feed.py` instead of editing it.

This repository is the plugin: its root holds `plugin.json` and the module, so a release archive is
exactly those two files. The old note that the folder lived outside the git repository is no longer
true — releases are built from it by GitHub Actions.



Yes, that's right: `.rs` alone does not work. BIMCraft does not compile Rust on the fly — it only loads the ready WebAssembly module named in the manifest:
`"module": "house_builder.wasm"`
So a plugin is two required files:
`plugin.json` — manifest
`house_builder.wasm` — built module
`src/lib.rs` and `Cargo.toml` are the sources for building; the application does not read them.
At the same time, `test-plugin-house` already contains a built `house_builder.wasm` (150,072 bytes), so to install and run it you do not need to build anything — just copy the folder into the plugins folder, or use the one-command installer above.
If you leave only the `.rs` file in the folder, loading fails: the `house_builder.wasm` module is not found, the plugin appears in Manage > Plugins with an error status, and its commands are not registered.
