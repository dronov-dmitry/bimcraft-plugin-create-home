#!/usr/bin/env python3
"""Build and sign the plugin's update feed (schema 1) for a GitHub release.

The feed is what a BIMCraft host reads from the `update.feed` address in
`plugin.json`: the version, the archive to download, its size and digest, and
an Ed25519 signature over the canonical JSON of those fields.

Values come from `plugin.json` and from environment variables, so the release
workflow and a maintainer's terminal run the same code:

    FEED_ARCHIVE      the release archive (default: dist/house-builder-<version>.zip)
    FEED_ARCHIVE_URL  its public https address (default: this repository's release)
    FEED_RELEASED     the release date (default: today, UTC)
    FEED_NOTES        a short "what changed" line (default: "House Builder <version>")
    FEED_KEY          the Ed25519 private key in PEM (default: keys/house-builder-signing-key.pem)
    FEED_OUT          where to write the feed (default: dist/latest.json)

Minimum Python: 3.10.
"""

from __future__ import annotations

import base64
import datetime
import hashlib
import json
import logging
import os
import subprocess
import sys
import tempfile
from pathlib import Path

LOG = logging.getLogger("make-feed")
ROOT = Path(__file__).resolve().parent.parent
DEFAULT_REPO = "dronov-dmitry/bimcraft-plugin-create-home"
SCHEMA = 1
# The publisher's key: the maintainer's file, or the one the workflow writes from the secret.
KEY = Path(os.environ.get("FEED_KEY", "").strip() or ROOT / "keys" / "house-builder-signing-key.pem")


def manifest() -> dict:
    """The plugin's manifest, read from the repository root."""
    path = ROOT / "plugin.json"
    return json.loads(path.read_text(encoding="utf-8"))


def env(name: str, default: str) -> str:
    """An environment value, or the default when it is unset or empty."""
    return os.environ.get(name, "").strip() or default


def sha256_of(path: Path) -> str:
    """The digest of a file, as 64 lower-case hex digits."""
    digest = hashlib.sha256()
    with path.open("rb") as handle:
        for chunk in iter(lambda: handle.read(1 << 20), b""):
            digest.update(chunk)
    return digest.hexdigest()


def canonical(feed: dict) -> bytes:
    """The bytes a signature covers: every field but `signature`, in a fixed order.

    This mirrors the host's own `canonical`: same key order, no spaces, and the
    same string escaping as `serde_json`, so the two agree byte for byte.
    """
    text = lambda value: json.dumps(value, ensure_ascii=False)
    parts = [
        '{"schema":%d' % feed["schema"],
        ',"plugin":{"id":%s,"abi":%d}' % (text(feed["plugin"]["id"]), feed["plugin"]["abi"]),
        ',"version":%s' % text(feed["version"]),
        ',"released":%s' % text(feed["released"]),
        ',"notes":%s' % text(feed["notes"]),
        ',"module":{"url":%s,"sha256":%s,"size":%d}' % (
            text(feed["module"]["url"]),
            text(feed["module"]["sha256"]),
            feed["module"]["size"],
        ),
    ]
    if "public_key" in feed:
        parts.append(',"public_key":%s' % text(feed["public_key"]))
    parts.append("}")
    return "".join(parts).encode("utf-8")


def openssl(*args: str, data: bytes | None = None) -> bytes:
    """Run openssl and return its standard output."""
    completed = subprocess.run(
        ["openssl", *args],
        input=data,
        check=False,
        capture_output=True,
    )
    if completed.returncode != 0:
        raise RuntimeError(
            "openssl " + " ".join(args) + " failed: "
            + completed.stderr.decode("utf-8", "replace").strip()
        )
    return completed.stdout


def sign(message: bytes, key: Path) -> str:
    """An Ed25519 signature as `ed25519:BASE64`, the field's form."""
    if not key.is_file():
        raise FileNotFoundError(
            f"no signing key at {key}: run scripts/make-signing-key.sh, "
            "or set FEED_KEY (the release workflow writes the secret there)"
        )
    # `pkeyutl` with `-rawin` needs a seekable input: Ed25519 signs the whole message at once.
    with tempfile.TemporaryDirectory(prefix="feed-sign-") as folder:
        data = Path(folder) / "message.bin"
        data.write_bytes(message)
        signature = openssl(
            "pkeyutl", "-sign", "-rawin", "-inkey", str(key), "-in", str(data)
        )
    return "ed25519:" + base64.b64encode(signature).decode("ascii")


def public_key_pem(key: str) -> bytes:
    """`ed25519:BASE64` from the manifest as a PEM public key."""
    body = key.removeprefix("ed25519:")
    raw = base64.b64decode(body, validate=True)
    if len(raw) != 32:
        raise ValueError(f"the manifest's public key is {len(raw)} bytes; an Ed25519 key is 32")
    # The DER header of an Ed25519 SubjectPublicKeyInfo followed by the raw key.
    der = bytes.fromhex("302a300506032b6570032100") + raw
    encoded = base64.b64encode(der).decode("ascii")
    lines = [encoded[i:i + 64] for i in range(0, len(encoded), 64)]
    return ("-----BEGIN PUBLIC KEY-----\n" + "\n".join(lines) + "\n-----END PUBLIC KEY-----\n").encode("ascii")


def verify(message: bytes, signature: str, key: str) -> None:
    """Raise unless `signature` is `key`'s over `message`."""
    raw = base64.b64decode(signature.removeprefix("ed25519:"), validate=True)
    with tempfile.TemporaryDirectory(prefix="feed-verify-") as folder:
        pem = Path(folder) / "public.pem"
        sig = Path(folder) / "signature.bin"
        data = Path(folder) / "message.bin"
        pem.write_bytes(public_key_pem(key))
        sig.write_bytes(raw)
        data.write_bytes(message)
        openssl(
            "pkeyutl", "-verify", "-rawin", "-pubin", "-inkey", str(pem),
            "-in", str(data), "-sigfile", str(sig),
        )


def build() -> dict:
    """The feed, its signature checked against the manifest's public key."""
    data = manifest()
    version = data["version"]
    repo = env("FEED_REPO", DEFAULT_REPO)
    archive = Path(env("FEED_ARCHIVE", str(ROOT / "dist" / f"house-builder-{version}.zip")))
    if not archive.is_file():
        raise FileNotFoundError(
            f"no archive at {archive}: build and pack the module first "
            "(the release workflow stages it in dist/)"
        )
    url = env(
        "FEED_ARCHIVE_URL",
        f"https://github.com/{repo}/releases/download/v{version}/house-builder-{version}.zip",
    )
    feed = {
        "schema": SCHEMA,
        "plugin": {"id": data["id"], "abi": data["abi"]},
        "version": version,
        "released": env("FEED_RELEASED", datetime.datetime.now(datetime.UTC).date().isoformat()),
        "notes": env("FEED_NOTES", f"House Builder {version}"),
        "module": {"url": url, "sha256": sha256_of(archive), "size": archive.stat().st_size},
    }
    declared = (data.get("update") or {}).get("public_key")
    if not declared:
        raise ValueError("plugin.json declares no update.public_key; a signed feed needs one")
    feed["signature"] = sign(canonical(feed), KEY)
    verify(canonical(feed), feed["signature"], declared)
    return feed


def main() -> int:
    """Write the signed feed, print a summary, exit non-zero on any failure."""
    logging.basicConfig(level=logging.INFO, format="%(message)s")
    try:
        feed = build()
        out = Path(env("FEED_OUT", str(ROOT / "dist" / "latest.json")))
        out.parent.mkdir(parents=True, exist_ok=True)
        out.write_text(json.dumps(feed, indent=2, ensure_ascii=False) + "\n", encoding="utf-8")
        LOG.info("Plugin: %s %s (ABI %d)", feed["plugin"]["id"], feed["version"], feed["plugin"]["abi"])
        LOG.info("Archive: %s (%d bytes)", feed["module"]["url"], feed["module"]["size"])
        LOG.info("SHA-256: %s", feed["module"]["sha256"])
        LOG.info("Signature verified against the manifest's public key.")
        LOG.info("Feed: %s", out)
        return 0
    except (OSError, ValueError, RuntimeError) as error:
        LOG.error("Feed failed: %s", error)
        return 1


if __name__ == "__main__":
    sys.exit(main())
