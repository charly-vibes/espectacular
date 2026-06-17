# Concepts

## The mental model

espectacular enforces a contract between *what you said your tool does* (specs) and *whether it actually does it* (tests). Each behavioral claim lives in a spec file; each claim has a sidecar TOML contract that says how to verify it. `ah check` validates that every claim has a contract and that every contract's tests pass.

---

## Specs

A **spec** is a Markdown file under `openspec/specs/<name>/spec.md`. It describes the intended behavior of one component using scenario headings.

```markdown
### Requirement: Empty input is rejected

Given an empty string is passed to the parser,
When the parser runs,
Then it exits non-zero with a descriptive error message.
```

Specs are authored by humans (or AI agents) and checked into version control. They are append-only: you never rewrite a deployed scenario's intent — instead you add new scenarios or supersede old ones.

---

## Scenarios

A **scenario** is one `### Requirement:` heading in a spec file. It has:

- A **slug** derived from the heading (lowercased, non-alphanumeric → hyphens, e.g. `empty-input-is-rejected`)
- A **body** in Given/When/Then form describing the behavior

`ah check` discovers scenarios by parsing headings from spec files. Each scenario must have a corresponding contract file, or `ah check` emits a `no-toml` finding.

---

## Contracts

A **contract** is a TOML file at `.espectacular/<spec>/<scenario-id>.toml`. It is the machine-readable pairing between a scenario and the tests that verify it.

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

Required fields: `id`, `description`, `archetype`, `status`, `superseded_by`, `authored_with`, `tests`.

The filename must match the `id` field and the scenario slug — `ah check` emits `id-mismatch` if they disagree.

### Test entry types

- `[[tests.shell]]` — runs `command` directly via `/bin/sh -c`
- `[[tests.pytest]]` — prepends the configured pytest runner to `flags`
- `[[tests.cargo]]` — prepends the configured cargo runner to `flags`
- `[[tests.vitest]]` — prepends the configured vitest runner to `flags`
- `[[tests.custom]]` — invokes a custom runner and parses its JSON envelope
- `[[tests.<type>]]` — any other type, looked up in `[runners.<type>]` in config

### Staged contracts

During a change in progress, contracts live under `.espectacular/changes/<change>/<spec>/`. After the change merges, `ah archive <change>` moves them into `.espectacular/<spec>/`.

---

## Archetypes

Every contract declares an **archetype** — a short code that classifies the kind of behavior being verified. Archetypes guide test design: a `PF` scenario should have deterministic unit tests; an `SA` scenario needs to cover state transitions.

| Code | Name | Description |
|------|------|-------------|
| `PF` | Pure Functional | Deterministic behavior; outputs are a function of explicit inputs |
| `SA` | Stateful API | State transitions, persisted data, or ordered operations |
| `BP` | Boundary Protocol | Behavior at an external boundary or protocol seam |
| `CE` | Contract/Event | Emitted events, messages, claims, or cross-tool signals |
| `NR` | Non-Regression | Existing guarantees remain true while nearby changes land |

Run `ah type <code>` for full documentation on any archetype, or `ah type` to list all.

---

## The gate

`ah check` is the enforcement gate. It:

1. Discovers all scenarios from deployed specs (and staged change overlays if `--changes` is passed)
2. Checks structural correspondence: every scenario has a contract, every contract has a scenario, no collisions, no orphans
3. Runs every declared test
4. Emits a JSON envelope and exits 0 if clean, 1 if any structural or execution finding exists

**Local pre-commit** (`ah init` installs this): convenience layer, catches issues early.
**CI** (`ah check` in a workflow step): enforcement gate, the source of truth.

Quality findings (`quality-mutation`, `quality-property`, `quality-snapshot`) are informational — they appear in the output but never cause a non-zero exit.

---

## Adapters

An **adapter** maps a test type name to the right runner invocation and normalizes failure output. espectacular ships adapters for:

- **pytest** — detected via `runners.pytest` config, `pyproject.toml`, `pytest` on PATH, or `.py` files with `import pytest`
- **cargo** — detected via `runners.cargo` config or `Cargo.toml`
- **vitest** — detected via `runners.vitest` config or `package.json` devDependency

Detection precedence (highest to lowest): explicit config → project manifest → binary on PATH.

`ah doctor` shows which frameworks are detected and their source. Use `ah doctor --enable <framework>` to write the config entry for a detected-but-unconfigured adapter.

### Custom runners

A `[[tests.custom]]` entry invokes a configured runner and expects a JSON envelope on stdout:

```json
{ "exit_code": 0, "passed": true, "findings": [] }
```

Non-zero `exit_code` or `passed: false` maps to a `test-failing` finding. The `findings` array lets custom runners emit structured findings directly into `ah check` output.

---

## Quality signals

Quality signals are optional capabilities that surface test health metadata as findings:

- **`quality-mutation`** — runs a mutation testing tool; emits a finding with kill rate and threshold
- **`quality-property`** — marks that property-based testing is active
- **`quality-snapshot`** — marks that snapshot testing is active

Enable them in `.espectacular/config.toml` or via `ah doctor --enable mutation` (etc.).

---

## File layout

```text
openspec/
└── specs/<spec>/spec.md          # deployed spec source

.espectacular/
├── config.toml                   # runner and capability config
├── AGENTS.md                     # AI agent guidance
├── <spec>/
│   └── <scenario-id>.toml        # deployed contracts
└── changes/
    └── <change>/
        └── <spec>/
            └── <scenario-id>.toml  # staged change contracts
```

Schemas live in `schemas/`:
- `check-output.schema.json` — `ah check` JSON envelope
- `config.schema.json` — `.espectacular/config.toml`
- `scenario-contract.schema.json` — contract TOML files
- `custom-runner.schema.json` — custom runner JSON envelope
