# Installation & Quick Start

## Install

**From source** (requires Rust toolchain):

```bash
cargo install --path .
```

Or clone and build:

```bash
git clone https://github.com/charly-vibes/espectacular
cd espectacular
cargo build --release
# Binary is at target/release/ah
```

Verify the install:

```bash
ah --version
```

## Prerequisites

Your project must use [OpenSpec](https://github.com/charly-vibes/openspec) for its spec directory (`openspec/`). Run `openspec init` in your repo before `ah init`.

## Set up a project

Run `ah init` once in the root of your repo:

```bash
ah init
```

This creates (or refreshes):

- `.espectacular/config.toml` — runner and capability config
- `.espectacular/AGENTS.md` — guidance block for AI agents
- Hook integration for `lefthook` or `prek` if detected

If `ah init` reports concerns (missing hook framework, no spec directory), use `ah doctor` to diagnose.

## Run your first check

```bash
ah check
```

Output is always a JSON envelope:

```json
{
  "scope": { "deployed": true, "changes": [] },
  "summary": { "structural": 0, "execution": 0, "passed": 2, "counts_by_kind": {} },
  "findings": []
}
```

An empty `findings` array and `structural: 0, execution: 0` means everything is green.

## Worked example

Suppose you have a spec at `openspec/specs/parser/spec.md` with this scenario:

```markdown
### Requirement: Empty input is rejected

Given an empty string is passed to the parser...
```

After `ah init`, create a contract for it:

```bash
ah scenario new my-feature parser \
  --requirement "empty-input-is-rejected" \
  "Empty input is rejected"
```

This stubs `.espectacular/changes/my-feature/parser/empty-input-is-rejected.toml`. Open it and add a test:

```toml
id = "empty-input-is-rejected"
description = "Empty input is rejected before parsing."
archetype = "PF"
status = "active"
superseded_by = ""
authored_with = "0.1.0"

[[tests.pytest]]
flags = "tests/test_parser.py::test_empty_input_rejected"
timeout_seconds = 60
```

Then check with the staged change included:

```bash
ah check --changes my-feature
```

A passing run exits 0. A `test-failing` finding means the test ran but failed. Use `ah explain test-failing` for guidance.

## What's next

- [Command Reference](commands.md) — all `ah` subcommands with flags and examples
- [Concepts](concepts.md) — understand specs, contracts, archetypes, and the gate model
