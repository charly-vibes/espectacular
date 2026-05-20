# espectacular

This directory contains espectacular configuration and scenario contracts.

## Layout

- `config.toml` — project configuration (tool version, paths, runners)
- `<spec>/<scenario>.toml` — per-scenario test contracts
- `changes/<change>/` — staged contracts for in-flight OpenSpec changes

## Workflow

Run `ah check` from the repo root to validate all spec-test correspondence.
Run `ah check --changes <name>` to validate a change overlay before merging.
