# Installation & Quick Start

**In brief:** install `ah`, run `ah init` in your repo, then `ah check` to validate. The worked example below walks through writing a spec scenario, creating its contract, and seeing a passing check. Both `ah` and `espectacular` are installed — they are the same binary under two names.

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

Verify:

```bash
ah --version
# ah 0.1.0
espectacular --version
# espectacular 0.1.0
```

## Prerequisites

espectacular requires [OpenSpec](https://github.com/charly-vibes/openspec) to manage your spec files. OpenSpec is the directory structure and tooling that stores specs under `openspec/specs/` and staged changes under `openspec/changes/`. Run `openspec init` once in your repo to set it up before running `ah init`.

## Set up espectacular

Run `ah init` once in the root of your repo:

```bash
ah init
```

This creates (or refreshes):

- `.espectacular/config.toml` — runner and capability config
- `.espectacular/AGENTS.md` — guidance block for AI agents
- Hook integration for `lefthook` or `prek` if detected

If `ah init` reports concerns, use `ah doctor` to diagnose.

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

Empty `findings` and `structural: 0, execution: 0` means everything is green. If you have specs without contracts yet, you'll see `no-toml` findings instead — see [When check finds something](#when-check-finds-something) below.

## Worked example

This walks through the full loop: open a change → write a scenario → create a contract → run the check.

### Step 1 — Open a change

In OpenSpec, work-in-progress lives under a "change." Create one:

```bash
openspec new my-feature
```

This creates `openspec/changes/my-feature/specs/` where your staged spec lives.

### Step 2 — Write a scenario

Edit (or create) the spec for the component you're working on, e.g. `openspec/changes/my-feature/specs/parser/spec.md`:

```markdown
### Requirement: Empty input is rejected

Given an empty string is passed to the parser,
When the parser runs,
Then it exits non-zero with a descriptive error message.
```

### Step 3 — Create the contract

Generate the contract stub:

```bash
ah scenario new my-feature parser \
  --requirement "empty-input-is-rejected" \
  "Empty input is rejected"
```

This creates `.espectacular/changes/my-feature/parser/empty-input-is-rejected.toml`. Open it and add your test:

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

For archetype choices (`PF`, `SA`, `BP`, `CE`, `NR`) see [Concepts — Archetypes](concepts.md#archetypes) or run `ah type`.

### Step 4 — Check with the change in scope

```bash
ah check --changes my-feature
```

A passing run exits 0 with `passed: 1` in the summary.

### Step 5 — Archive when merged

Once the change merges, promote the contract to deployed:

```bash
ah archive my-feature
```

After this, `ah check` (without `--changes`) will include the scenario in its scope.

## When check finds something

A failing check exits 1 and includes findings in the JSON. For example, if a scenario has no contract yet:

```json
{
  "scope": { "deployed": true, "changes": [] },
  "summary": { "structural": 1, "execution": 0, "passed": 0, "counts_by_kind": { "no-toml": 1 } },
  "findings": [
    {
      "kind": "no-toml",
      "category": "structural",
      "spec": "parser",
      "spec_path": "openspec/specs/parser/spec.md",
      "scenario": { "id": "empty-input-is-rejected", "title": "Empty input is rejected" },
      "suggested_action": "run_ah_scenario_new",
      "playbook_command": "ah explain run_ah_scenario_new"
    }
  ]
}
```

Each finding includes a `suggested_action` and a `playbook_command` you can run directly:

```bash
ah explain run_ah_scenario_new
```

Use `ah explain --list` to see all explainable topics.

## What's next

- [Command Reference](commands.md) — all `ah` subcommands with flags and examples
- [Concepts](concepts.md) — understand specs, contracts, archetypes, and the gate model
