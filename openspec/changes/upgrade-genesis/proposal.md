# Change: Upgrade genesis to v0.2.0 (config + guide)

## Why

genesis v0.2.0 adds `genesis::config` (shared config management) and
`genesis::guide` (CLI scaffold). This change upgrades the dependency
and adopts the new modules.

## What Changes

- Bump genesis dependency from `v0.1.0` to `v0.2.0`.
- Adopt `genesis::config`:
  - Thin `src/config.rs` to just the struct + `ConfigFile` impl.
  - Register with `ConfigRegistry` at startup.
  - Remove dead config parsing code.
- Adopt `genesis::guide` (optional):
  - Replace `main.rs` CLI setup with `Guide::builder(...)`.
  - Convert command handlers to return `Output<T>`.
  - Remove dead error-handling code.

## Impact

- Affected code: `Cargo.toml`, `src/config.rs`, `src/main.rs`.
- Blocked by: genesis tagging v0.2.0.
