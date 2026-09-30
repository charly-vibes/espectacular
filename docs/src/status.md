# Implementation Status

This page maps the deployed behavioral specs to the commands and capabilities that implement them. Each row is a scenario in a spec; if `ah check` passes, that behavior is verified in CI.

## Deployed specs

### `gate` — Core verification engine (72 scenarios)

Covers what `ah check` does: scenario discovery, contract correspondence, test execution, JSON output, falsifiability classification, and quality signals.

| Scenario | What it verifies |
|----------|-----------------|
| Scenario Discovery | `ah check` finds all scenarios from spec headings |
| Sidecar Contract Correspondence | every scenario has a contract; every contract has a scenario |
| Contract Schema | TOML contracts validate against the schema |
| Test Runner Execution | declared tests are executed and results captured |
| JSON Findings | output is a stable JSON envelope with scope/summary/findings |
| Change Overlay Scope | `--changes` adds staged scenarios to scope |
| Non-Regression Archetype | `NR` contracts are checked without special treatment |
| Deterministic Scope Boundary | scope is stable across repeated runs |
| Finding severity | findings carry `severity` (`"error"` gate-failing, `"warning"` non-gating); warning-only runs exit 0 |
| Falsifiability classification | `falsifiability_class = "safety"` / `"liveness"`; invalid values fail the gate; liveness without bounded timeout warns |
| Quality measurement capabilities | `quality-*` findings are emitted and informational |
| Conformance coverage matrix | all finding kinds are covered by at least one contract |

(All 72 gate scenarios have active contracts — run `ah report` for the full per-scenario matrix.)

### `cli` — Command surface (47 scenarios)

Covers the full `ah` command interface: init, check, doctor, report, explain, type, scenario, archive, upgrade, lint, signals, feedback, completions.

| Scenario | What it verifies |
|----------|-----------------|
| CLI Command Name | binary is named `ah` |
| Project Initialization | `ah init` creates `.espectacular/` and hook integration |
| Correspondence Check Command | `ah check` validates specs and runs tests |
| Spec Quality Lint Command | `ah lint` analyzes spec files for authoring-quality findings |
| Health Check Command | `ah doctor` diagnoses config, paths, hooks, archetypes |
| Archetype Documentation Commands | `ah type` lists and explains archetypes |
| Scenario Lifecycle Commands | `ah scenario new` and `ah scenario supersede` |
| Archive Companion Command | `ah archive` promotes staged contracts |
| Upgrade Command | `ah upgrade` detects and reports tool-version drift |
| Doctor enable flag | `ah doctor --enable <capability>` writes config blocks |
| Explain subcommand | `ah explain` prints guidance for finding kinds and actions |
| Coverage report command | `ah report` displays a conformance coverage matrix |

### `lint` — Spec-quality heuristics (26 scenarios)

Covers the `ah lint` checks: vague qualifiers, imperative steps, conjunctive bloat, missing negative scenarios, missing non-goals, unresolved ambiguity, entangled specs, and the specodelic bridge.

| Scenario | What it verifies |
|----------|-----------------|
| Flag unbound qualifier / Do not flag bounded qualifier | vague-qualifier precision |
| Flag explicit click / URL navigation steps | imperative-step detection |
| Flag overlong scenario / Default maximum is configurable | conjunctive-bloat with configurable limit |
| Flag requirement with only happy-path scenarios | missing-negative-scenario |
| Missing non-goals / unresolved ambiguity / entangled spec detection | structural authoring checks |
| Malformed spec file | emits an error-severity finding instead of aborting |

### `adapters` — Language adapter dispatch (25 scenarios)

Covers how `ah check` maps test types to runners and normalizes output.

| Scenario | What it verifies |
|----------|-----------------|
| Adapter detection precedence | config > manifest > binary on PATH |
| Python pytest adapter | pytest detection, invocation, failure normalization |
| Rust cargo test adapter | cargo detection, invocation, failure normalization |
| TypeScript vitest adapter | vitest detection, invocation, failure normalization |
| No-adapter-configured path | `missing-runner` finding when type has no runner |
| Custom runner plugin protocol | custom runners emit JSON envelopes parsed by `ah check` |

### `explain` — Playbook system (15 scenarios)

Covers the `ah explain` topic system and its compile-time completeness guarantee.

| Scenario | What it verifies |
|----------|-----------------|
| Playbook is compile-enforced | every finding kind has an `ah explain` topic at compile time |
| Topic coverage | all finding kinds and suggested actions are covered |
| Structured JSON output | `--json` emits a machine-readable topic list |
| Unknown topic handling | unknown topics exit 1 with "did you mean" suggestions |

### `config`, `cli-core`, `spec-authoring` — Infrastructure (8 scenarios)

Config store/registry wiring (3), genesis guide/ErrorSink integration (2), dual-format spec authoring and migration (3).

---

## How to read this page

- **Deployed** means the scenario has a passing contract in `ah check` on `main`.
- **Planned** means the scenario is staged in an OpenSpec change but not yet implemented (none currently — all active changes are fully implemented and archived).
- Run `ah check` locally to see current pass/fail state.
- Spec source lives in [`openspec/specs/`](https://github.com/charly-vibes/espectacular/tree/main/openspec/specs).
