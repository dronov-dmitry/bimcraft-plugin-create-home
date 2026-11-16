# bimcraft-plugin-sdk (vendored)

This directory is a verbatim copy of `crates/plugin-sdk` from
[BIMCraft](https://github.com/IPedrax/BIMCraft), placed here so that this
plugin repository builds without a BIMCraft checkout.

Only `Cargo.toml` differs from upstream: version, edition, license and
rust-version are written out instead of inherited from the workspace, and the
`serde_json` dependency is pinned by version rather than by workspace.

- Upstream version: 0.12.0
- Host ABI: 1
- License: MIT OR Apache-2.0 (see `LICENSE-MIT` and `LICENSE-APACHE`)

Refresh it from a checkout with `scripts/sync-sdk.sh`. Keep the copy in step
with the BIMCraft release the plugin is tested against: the module and the host
must speak the same ABI.
