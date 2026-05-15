# Design: Quality measurement and language adapters

## Architecture overview

This change extends `ah` with three interconnected layers:

1. **Adapter layer** (`src/adapters/`) — normalizes test execution across languages into the finding schema
2. **Explain layer** (`src/explain.rs`) — compile-enforced playbook, one topic per enum variant
3. **Finding schema extension** — adds agent-action fields and quality-measurement fields to the existing schema

All three layers are stateless: `ah check` remains a pure function of the checked-in state.

### New source modules

```text
src/adapters/
  mod.rs           # Adapter trait and dispatch
  python.rs        # pytest detection and normalization
  rust.rs          # cargo test detection and normalization
  typescript.rs    # vitest detection and normalization
  custom.rs        # [runners.custom.<name>] plugin protocol
src/explain.rs     # ah explain <topic> subcommand
src/quality.rs     # Stateless quality capability dispatch
schemas/
  check-output.schema.json   # Extended with agent-action fields
  custom-runner.schema.json  # [runners.custom.<name>] envelope spec
```

## D1. Quality measurement is in v1 — measurement, not enforcement

The scenario contract schema gains `tests.property` and `tests.snapshot` as first-class optional test-entry arrays from day one. Mutation uses a separate `[quality.mutation]` table because it is a measurement over declared tests, not a runnable test selector itself. Mutation runs behind a flag, off in pre-commit, on in nightly CI. `ah doctor` warns below per-archetype thresholds; `ah check` does not fail on them.

**Rationale**: deciding the schema shape now is cheap; retrofitting it later forces a migration across every contract in every adopting project.

## D2. Hybrid adapter model

v1 ships first-class adapters for curated stacks. Each adapter is a small Rust module with two responsibilities: detect the framework's presence and version, and invoke it then normalize output into the finding schema.

Detection follows a precedence chain per adapter and per contract test type:
1. Declared in the language manifest (strongest) — e.g., `pyproject.toml`, `Cargo.toml`, `package.json`
2. Installed in environment — e.g., `pytest` on `$PATH`
3. Imported in source — e.g., `import pytest` in a test file

A multi-language repository may have multiple available adapters at once. There is no single global active adapter; dispatch selects the configured or detected adapter for each declared test type.

A `[runners.custom.<name>]` config block lets users wire arbitrary shell commands that emit a documented JSON envelope. The envelope schema lives in `schemas/custom-runner.schema.json` and is part of this change.

**Rationale**: a purely declarative model gives `ah doctor` nothing concrete to detect; a fully bundled model makes every framework version bump an `ah` release.

## D3. Three languages in v1

Python (pytest), Rust (cargo test), TypeScript (vitest). Scope is deliberately constrained to keep the maintenance budget honest. jest is a v0.2 fast-follow.

Each v1 baseline adapter does exactly one thing: run the test command in the contract, capture exit code and bounded stdout/stderr tails, normalize into the finding schema. PBT shrinking, mutation parsing, and snapshot review arrive as separate adapter modules in later minor versions, gated by `ah doctor` detection.

## D4. Progressive enablement UX

On detecting a newly-available framework, `ah doctor` prints a structured recommendation block. Capability-specific flags — `ah doctor --enable property`, `--enable mutation`, `--enable snapshot` — write a single config block. Nothing else turns features on.

Recommendations appear as a dedicated finding kind so an agent can read them, gate on human approval, and enable selectively.

**Rationale**: `ah`'s behavior stays a pure function of the checked-in config, not of the transitive dependency tree.

## D5. Rich JSON finding schema

The finding schema is the most permanent external surface. This change preserves baseline finding-kind names from `add-spec-assertions` and only adds new names for new concepts. Per-finding fields:

**Required on every finding:**
- `kind` — enum extending the baseline names: `no-toml | orphan-toml | slug-collision | id-mismatch | invalid-status | no-tests-declared | missing-runner | missing-adapter | malformed-contract | missing-replacement | overlay-conflict | test-failing | recommendation | unknown-action | quality-mutation | quality-property | quality-snapshot`
- `severity` — `error | warning | info`
- `scope` — `deployed | change:<id>`
- `message` — human-readable one-liner

**Present when applicable:**
- `scenario_id`, `scenario_path`, `scenario_prose` (full markdown body, verbatim, no truncation)
- `contract_path`, `command`, `command_exit`, `stdout_tail` (final 8 KiB), `stderr_tail` (final 8 KiB)

**Agent-action fields:**
- `suggested_action` — enum: `run_ah_init | run_ah_scenario_new | run_ah_scenario_supersede | edit_code_not_scenario | enable_capability | review_and_apply | human_review_required` — **always present**
- `apply_command` — shell-safe argv — **conditionally present**: set when a concrete shell command can resolve the finding (e.g., `enable_capability`, `run_ah_init`); null when the action is non-mechanical (e.g., `human_review_required`, `edit_code_not_scenario`)
- `playbook_command` — e.g. `ah explain run_ah_scenario_supersede` — **always present**

Envelope: `{ scope, summary: { counts_by_kind, exit_status }, findings: [...] }`. Findings sorted by `(spec_path, scenario_id, kind)` for diff-stable output.

**Rationale**: `scenario_prose` verbatim closes the most common agent failure mode (editing code without reading what the scenario demands); `suggested_action` as an enum lets the playbook be a deterministic switch rather than NLP.

## D6. Playbook as `ah explain <topic>`

The playbook ships in the binary. Each Rust enum variant carries its playbook body inline. The build fails if a variant has no body.

- `ah explain <topic>` prints markdown guidance
- `--json` emits structured output: `topic`, `summary`, `when`, `do`, `human_approval`, `related_topics`, `hints`
- Topics: every `SuggestedAction` value, every `FindingKind` value, plus general topics
- Every finding sets `playbook_command`
- `ah explain --list` enumerates all topics

**Rationale**: collapses the enum, playbook, and AGENTS.md managed block into one compile-enforced source of truth.

## D7. v1 is a pure function — no persistent state

Every `ah check` reads specs, contracts, and source, runs the declared tests, and emits findings with no on-disk history. Features deferred to v0.2+: flake detection, test-impact caching, mutation trend reporting.

## Dependency

This change depends on `add-spec-assertions` being deployed. The `[runners.custom.<name>]` JSON envelope schema (`schemas/custom-runner.schema.json`) is defined as the first deliverable of this change (Section 0 of tasks.md) — it gates all adapter implementation and must be merged before sections 3–6 begin.
