# espectacular

[![tracked with wai](https://img.shields.io/badge/tracked%20with-wai-blue)](https://github.com/charly-vibes/wai)

Behavioral verification layer for the charly AI development ecosystem.

## Tooling

This repo is configured to use:

- `wai` — project context, reasoning, handoffs
- `bd` (beads) — issue tracking and dependencies
- `openspec` — specs and change proposals
- `dont` — epistemic claim tracking and evidence grounding

## Quick start

```bash
just prime
just status
just validate
```

Or run the tools directly:

```bash
wai status
bd ready
openspec list
dont prime --plain
```

## `ah` workflow

`ah` is the CLI that enforces spec-test correspondence.

### Local pre-commit vs CI

- Local hooks are a convenience layer. `ah init` installs `ah check` into supported pre-commit frameworks when it finds `lefthook.yml` or `.prek`.
- CI is the enforcement gate. Run `ah check` in CI and fail the job on any non-zero exit.
- `ah doctor` helps explain setup drift; it is not the enforcement command.

### Commands

| Command | Purpose |
| --- | --- |
| `ah init` | Create or refresh `.espectacular/` files and hook integration |
| `ah doctor` | Diagnose config, path, hook, collision, orphan, and archetype issues |
| `ah check` | Validate deployed specs and run declared tests |
| `ah check --changes <id>` | Validate deployed specs plus one or more staged change overlays |
| `ah type` | List built-in archetypes |
| `ah type <code>` | Print full built-in documentation for one archetype |
| `ah scenario new <change> <spec> --requirement "<requirement>" "<heading>"` | Append a new scenario and stage its TOML contract |
| `ah scenario supersede <spec> <old-id> --with=<new-id> --in-change=<change>` | Stage a supersession update for an existing contract |
| `ah archive <change>` | Move staged change contracts into deployed `.espectacular/` locations |
| `ah upgrade` | Report tool-version drift and update `.espectacular/config.toml` only |

`ah upgrade` exits non-zero when it detects drift, even after rewriting `tool_version`, so automation can notice compatibility changes.

## `.espectacular/` layout

```text
.espectacular/
├── AGENTS.md
├── config.toml
├── <spec>/
│   └── <scenario-id>.toml
└── changes/
    └── <change>/
        └── <spec>/
            └── <scenario-id>.toml
```

- `.espectacular/<spec>/<scenario-id>.toml` stores deployed scenario contracts.
- `.espectacular/changes/<change>/<spec>/<scenario-id>.toml` stores staged change overlays.
- `openspec/specs/<spec>/spec.md` is the deployed spec source.
- `openspec/changes/<change>/specs/<spec>/spec.md` is the staged change spec source.

## Schemas and file formats

Normative schema files:

- `schemas/check-output.schema.json` — `ah check` JSON envelope
- `openspec/changes/add-spec-assertions/schemas/config.schema.json` — `.espectacular/config.toml`
- `openspec/changes/add-spec-assertions/schemas/scenario-contract.schema.json` — scenario TOML contracts

### `.espectacular/config.toml`

```toml
tool_version = "0.1.0"

[paths]
specs = "openspec/specs"
changes = "openspec/changes"

[runners]
pytest = ["pytest"]
cargo = ["cargo", "test"]
```

Required fields:

- `tool_version`: pinned `ah` version for compatibility mode
- `paths.specs`: deployed OpenSpec root
- `paths.changes`: staged OpenSpec change root
- `runners.<type>`: argv array used to execute non-shell test entries

### Scenario contract TOML

```toml
id = "empty-input-rejected"
description = "Empty input is rejected before parsing."
archetype = "PF"
status = "active"
superseded_by = ""
authored_with = "0.1.0"

[[tests.unit]]
flags = "tests/compiler/test_parser.py::test_empty_input_rejected"
timeout_seconds = 60
```

Required top-level fields:

- `id`
- `description`
- `archetype`
- `status` (`active` or `superseded`)
- `superseded_by` (non-empty when `status = "superseded"`)
- `authored_with`
- `tests`

Test entry rules:

- `[[tests.shell]]` entries use `command`
- non-shell `[[tests.<type>]]` entries use `flags`
- `timeout_seconds` is optional but must be positive when present

## Append-only authoring workflow

Scenarios are append-only.

- Do not rewrite or delete a deployed scenario to change intent.
- Add a new scenario under the targeted requirement with `ah scenario new ...`.
- If the old scenario is replaced, stage a supersession with `ah scenario supersede ...`.
- Once the change is accepted, run `ah archive <change>` to move staged TOML files into deployed `.espectacular/` paths.

Requirement targeting is explicit: `ah scenario new` appends under the named `### Requirement:` block and fails if that requirement is missing.

## `ah check` JSON output

`ah check` always prints a stable JSON envelope to stdout.

Success shape:

```json
{
  "scope": { "deployed": true, "changes": [] },
  "summary": { "structural": 0, "execution": 0, "passed": 1 },
  "findings": []
}
```

Top-level fields:

- `scope.deployed`: always `true` in v1
- `scope.changes`: selected change ids, sorted and deduplicated
- `summary.structural`: count of structural findings
- `summary.execution`: count of execution findings
- `summary.passed`: count of passing declared tests
- `findings`: sorted by `(spec_path, scenario.id, kind, test)`

### `ah check` finding kinds

| Kind | Category | Meaning |
| --- | --- | --- |
| `no-toml` | structural | scenario has no matching `.espectacular/.../*.toml` contract |
| `orphan-toml` | structural | contract exists without a matching OpenSpec scenario |
| `slug-collision` | structural | two scenarios in one spec slugify to the same id |
| `id-mismatch` | structural | scenario slug, TOML filename, and TOML `id` disagree |
| `no-tests-declared` | structural | contract has no runnable test entries |
| `missing-runner` | structural | a non-shell test type has no configured runner |
| `malformed-contract` | structural | TOML cannot be parsed or validated |
| `missing-replacement` | structural | a superseded contract points to a replacement scenario that is absent from scope |
| `overlay-conflict` | structural | selected changes define conflicting staged scenarios or staged contract updates |
| `test-failing` | execution | a declared test timed out or exited non-zero |

`test-failing` findings include test execution details: `type`, `command`, `exit_code`, `timed_out`, `stdout_tail`, and `stderr_tail`.

## `ah doctor` diagnostics

`ah doctor` exits zero with:

```text
healthy: all checks passed
```

Otherwise it exits non-zero and emits diagnostics such as:

- `bad-config`
- `version-drift`
- `missing-path`
- `collision`
- `orphan-contract`
- `unknown-archetype`
- `missing-managed-block`
- `hook-absent`

## Archetypes

Use `ah type` to list current archetypes and `ah type <code>` for full guidance.

Current catalog:

- `PF` — Pure Functional
- `SA` — Stateful API
- `BP` — Boundary Protocol
- `CE` — Contract/Event
- `NR` — Non-Regression

The catalog is embedded, append-only, and version-addressable so older pinned projects remain readable in compatibility mode.

## Non-goals

v1 does **not**:

- judge whether a test is meaningful
- inspect test internals for assertion quality
- detect prose drift inside a scenario body

Review and supersession discipline handle those concerns; `ah` only checks deterministic correspondence and execution.

## Notes

- `wai` state lives in `.wai/`
- beads state lives in `.beads/`
- OpenSpec files live in `openspec/`
- `dont` state lives in `.dont/`
