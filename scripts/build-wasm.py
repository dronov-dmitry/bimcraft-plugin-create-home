#!/usr/bin/env python3
"""Build a BIMCraft plugin's WebAssembly module from its Rust source.

The script asks for the plugin folder, builds the crate for the wasm32 target,
checks the result, and copies the .wasm next to plugin.json, where BIMCraft
loads it. It is interactive on purpose: press Enter to accept each default.

Usage: run it directly or through build-wasm.sh in the same folder.
Minimum Python: 3.10.
"""

from __future__ import annotations

import json
import logging
import re
import shutil
import subprocess
import sys
from pathlib import Path

LOG = logging.getLogger("build-wasm")
DEFAULT_TARGET = "wasm32-unknown-unknown"


def ask(prompt: str, default: str) -> str:
    """Ask for one value; an empty answer keeps the default."""
    answer = input(f"{prompt} [{default}]: ").strip()
    return answer or default


def ask_yes(prompt: str, default: bool = True) -> bool:
    """Ask a yes/no question; Enter keeps the default."""
    hint = "[Y/n]" if default else "[y/N]"
    answer = input(f"{prompt} {hint} ").strip().lower()
    if not answer:
        return default
    return answer in {"y", "yes"}


def package_name(cargo_toml: Path) -> str:
    """Return [package] name from a Cargo.toml."""
    section = ""
    for line in cargo_toml.read_text(encoding="utf-8").splitlines():
        stripped = line.strip()
        if stripped.startswith("["):
            section = stripped
            continue
        if section != "[package]":
            continue
        match = re.match(r'name\s*=\s*"([^"]+)"', stripped)
        if match:
            return match.group(1)
    raise ValueError(f"no [package] name in {cargo_toml}")


def installed_targets() -> set[str]:
    """The targets rustup has installed."""
    try:
        result = subprocess.run(
            ["rustup", "target", "list", "--installed"],
            check=True,
            capture_output=True,
            text=True,
        )
    except FileNotFoundError as exc:
        raise RuntimeError(
            "rustup was not found in PATH; install Rust from https://rustup.rs"
        ) from exc
    except subprocess.CalledProcessError as exc:
        raise RuntimeError("rustup could not list its targets") from exc
    return {line.strip() for line in result.stdout.splitlines() if line.strip()}


def ensure_target(target: str) -> None:
    """Offer to install the WebAssembly target with rustup when it is missing."""
    if target in installed_targets():
        LOG.info("Rust target %s is installed.", target)
        return
    LOG.warning("Rust target %s is not installed.", target)
    if not ask_yes(f"Install it now with rustup target add {target}?", True):
        raise RuntimeError(
            f"the target {target} is missing: run rustup target add {target}"
        )
    subprocess.run(["rustup", "target", "add", target], check=True)


def build(plugin_dir: Path, target: str, release: bool) -> Path:
    """Build the crate and return the .wasm artifact path."""
    crate = package_name(plugin_dir / "Cargo.toml")
    command = ["cargo", "build", "--target", target]
    profile = "debug"
    if release:
        command.append("--release")
        profile = "release"
    LOG.info("Running: %s", " ".join(command))
    subprocess.run(command, cwd=plugin_dir, check=True)
    artifact = plugin_dir / "target" / target / profile / f"{crate.replace('-', '_')}.wasm"
    if not artifact.is_file():
        candidates = sorted((plugin_dir / "target" / target / profile).glob("*.wasm"))
        if len(candidates) == 1:
            artifact = candidates[0]
        else:
            raise FileNotFoundError(
                f"no single .wasm built in {artifact.parent}: {len(candidates)} found"
            )
    return artifact


def module_destination(plugin_dir: Path, artifact: Path) -> Path:
    """Where plugin.json says the module is; the artifact's own name otherwise."""
    manifest = plugin_dir / "plugin.json"
    if not manifest.is_file():
        return artifact
    data = json.loads(manifest.read_text(encoding="utf-8"))
    module = data.get("module")
    if not isinstance(module, str) or not module:
        return artifact
    return plugin_dir / module


def main() -> int:
    """Ask the questions, build, verify and copy the module."""
    logging.basicConfig(level=logging.INFO, format="%(message)s")
    default_plugin = Path(__file__).resolve().parent.parent
    try:
        raw = ask("Plugin folder", str(default_plugin))
        plugin_dir = Path(raw).expanduser().resolve()
        cargo_toml = plugin_dir / "Cargo.toml"
        if not cargo_toml.is_file():
            raise FileNotFoundError(f"no Cargo.toml at {plugin_dir}")

        target = ask("Rust target", DEFAULT_TARGET)
        release = ask_yes("Build the release profile?", True)
        LOG.info("Plugin: %s", plugin_dir)

        ensure_target(target)
        artifact = build(plugin_dir, target, release)
        if artifact.read_bytes()[:4] != b"\x00asm":
            raise ValueError(f"{artifact} is not a WebAssembly module")

        destination = module_destination(plugin_dir, artifact)
        if destination != artifact:
            if not ask_yes(f"Copy it to {destination.name} next to plugin.json?", True):
                LOG.info("Left the built module at %s", artifact)
                return 0
            destination.parent.mkdir(parents=True, exist_ok=True)
            shutil.copy2(artifact, destination)
            LOG.info("Copied %s -> %s", artifact.name, destination)
        LOG.info("Module: %s (%d bytes)", destination, destination.stat().st_size)
        LOG.info("Install the plugin folder in BIMCraft's plugins folder or set BIMCRAFT_PLUGINS.")
        return 0
    except (OSError, ValueError, RuntimeError, json.JSONDecodeError) as exc:
        LOG.error("Build failed: %s", exc)
        return 1
    except subprocess.CalledProcessError as exc:
        LOG.error("Command failed with exit code %s: %s", exc.returncode, exc.cmd)
        return 1


if __name__ == "__main__":
    sys.exit(main())