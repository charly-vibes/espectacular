# espectacular

**In brief:** `ah` is a CLI that enforces a contract between your specs and your tests. You write behavioral specs in Markdown, attach a TOML contract to each scenario declaring which tests cover it, then run `ah check` in CI to catch drift.

---

**espectacular** is a behavioral verification tool for Rust CLI projects. It lets you write machine-readable specs that describe what your tool does, then continuously check that behavior with `ah check`. Each spec scenario is paired with a sidecar TOML contract that lists the tests verifying it — when a test is missing, unconfigured, or failing, `ah check` exits non-zero and blocks the merge.

- [Installation & Quick Start](installation.md) — get `ah` on your PATH and run your first check in minutes
- [Command Reference](commands.md) — all `ah` subcommands with flags, exit codes, and example output
- [Concepts](concepts.md) — understand specs, scenarios, contracts, archetypes, and the gate model
