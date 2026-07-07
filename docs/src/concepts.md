# Concepts

**In brief:** A **spec** declares what your tool should do. A **contract** says which tests verify one scenario in that spec. `ah check` is the gate — it exits non-zero when a contract is missing or a test fails. See the [worked example in Installation](installation.md#worked-example) to see these pieces together.

---

## The mental model

espectacular enforces a contract between *what you said your tool does* (specs) and *whether it actually does it* (tests). Each behavioral claim lives in a spec file; each claim has a sidecar TOML contract that says how to verify it. `ah check` validates that every claim has a contract and that every contract's tests pass.

---

## Specs

A **spec** is a Markdown file under `openspec/specs/<name>/spec.md`. It describes the intended behavior of one component using `### Requirement:` groupings and `#### Scenario:` headings nested under them.

```markdown
### Requirement: Input validation

#### Scenario: Empty input is rejected
- **GIVEN** an empty string is passed to the parser
- **WHEN** the parser runs
- **THEN** it exits non-zero with a descriptive error message
```

Specs are checked into version control. They are append-only: you never rewrite a deployed scenario's intent — instead you add new scenarios or supersede old ones. The git log becomes a traceable record of when each behavior was declared, extended, or superseded.

---

## Scenarios

A **scenario** is one `#### Scenario:` heading nested under a `### Requirement:` grouping in a spec file. It has:

- A **slug** — derived from the scenario heading (by convention, lowercased with hyphens, e.g. `empty-input-is-rejected`). This slug must match the contract filename and the `id` field inside it.
- A **body** in Given/When/Then form describing the behavior

`ah check` discovers scenarios by parsing `#### Scenario:` headings from spec files. Each scenario must have a corresponding contract file, or `ah check` emits a `no-toml` finding. Run `ah explain no-toml` for the fix.

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

The filename, the `id` field, and the scenario slug must all agree — `ah check` emits `id-mismatch` if they disagree.

### Test entry types

| Entry | How it runs |
|-------|-------------|
| `[[tests.shell]]` | Runs `command` directly via `/bin/sh -c` |
| `[[tests.pytest]]` | Prepends the configured pytest runner to `flags` |
| `[[tests.cargo]]` | Prepends the configured cargo runner to `flags` |
| `[[tests.vitest]]` | Prepends the configured vitest runner to `flags` |
| `[[tests.custom]]` | Invokes a custom runner and parses its JSON envelope |
| `[[tests.<type>]]` | Any other type, looked up in `[runners.<type>]` in config |

### Staged contracts

During a change in progress, contracts live under `.espectacular/changes/<change>/<spec>/`. After the change merges, `ah archive <change>` moves them into `.espectacular/<spec>/`. See [Installation — Step 5](installation.md#step-5--archive-when-merged).

---

## Archetypes

Every contract declares an **archetype**, a short code that classifies the kind of behavior being verified. Archetypes guide test design: a `PF` scenario should have deterministic unit tests; an `SA` scenario needs to cover state transitions.

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

**Local pre-commit** (`ah init` installs this): catches structural findings before `git push` instead of in CI.
**CI** (`ah check` in a workflow step): the enforcement gate, source of truth.

Quality findings (`quality-mutation`, `quality-property`, `quality-snapshot`) are informational — they appear in the output but never cause a non-zero exit. See [Command Reference — ah check](commands.md#ah-check) for the full finding kind table.

---

## Adapters

An **adapter** maps a test type name to the right runner invocation and normalizes failure output into a common shape. espectacular ships built-in adapters for pytest, cargo, and vitest.

**Detection signals** (highest-priority source wins):

| Adapter | Explicit config | Manifest detection | Binary detection |
|---------|----------------|-------------------|-----------------|
| `pytest` | `runners.pytest` in config | `pyproject.toml [tool.pytest]`, `pytest.ini` | `pytest` on PATH or `.py` with `import pytest` |
| `cargo` | `runners.cargo` in config | `Cargo.toml` present | — |
| `vitest` | `runners.vitest` in config | `package.json` devDependency | — |

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

| Signal | What it tracks |
|--------|---------------|
| `quality-mutation` | Mutation testing kill rate vs. threshold |
| `quality-property` | Property-based testing is active and passing |
| `quality-snapshot` | Snapshot testing is active and passing |

Enable them in `.espectacular/config.toml` or via `ah doctor --enable mutation` (etc.). Quality findings are informational and never cause `ah check` to exit non-zero.

---

## File layout

```text
openspec/
└── specs/<spec>/spec.md            # deployed spec source

.espectacular/
├── config.toml                     # runner and capability config
├── AGENTS.md                       # AI agent guidance
├── <spec>/
│   └── <scenario-id>.toml          # deployed contracts
└── changes/
    └── <change>/
        └── <spec>/
            └── <scenario-id>.toml  # staged change contracts

schemas/
├── check-output.schema.json        # ah check JSON envelope
├── config.schema.json              # .espectacular/config.toml
├── scenario-contract.schema.json   # contract TOML files
└── custom-runner.schema.json       # custom runner JSON envelope
```
