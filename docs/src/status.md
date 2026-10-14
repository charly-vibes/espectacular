# Implementation Status

This page maps the deployed behavioral specs to the commands and capabilities that implement them. Each row is a scenario in a spec; if `ah check` passes, that behavior is verified in CI.

Last refreshed from the deployed corpus at **v0.11.0** (spec scenario counts re-derive any time via `openspec/specs/*/spec.md`).

## Deployed specs

### `gate` — Core verification engine (91 scenarios)

Covers what `ah check` does: scenario discovery, contract correspondence, test execution (including batched structured-output runners and range-based selection), JSON output, agent-actionability, quality signals, and property-derived contracts with `ah sync`.

| Requirement | What it verifies |
|-------------|------------------|
| Scenario Discovery | `ah check` finds all scenarios from spec headings; mirrored headings deduplicate; same-id/different-body collisions fail |
| Sidecar Contract Correspondence | every scenario has a contract; every contract has a scenario |
| Contract Schema | TOML contracts validate against the schema; superseded status requires a replacement |
| Test Runner Execution | declared tests run per-binding with exit-code verdicts; eligible structured-output bindings batch into one invocation above the threshold (with per-binding attribution, matched-zero, fallback, timeout semantics) |
| JSON Findings | output is a stable JSON envelope with scope/summary/findings |
| Change Overlay Scope | `--changes` adds staged scenarios; illegal overlays are rejected |
| Non-Regression Archetype | `NR` contracts run in overlay scope |
| Deterministic Scope Boundary | the gate never inspects test internals or hashes prose |
| Agent-action fields | every finding carries `suggested_action`, `playbook_command`, and conditionally `apply_command` |
| Quality measurement capabilities | `quality-*` findings are informational; tool failures fail the gate |
| Quality contract schema | quality config is not a test entry; property/snapshot are runnable |
| Conformance coverage matrix | per-spec/per-archetype coverage, machine-readable |
| Property-Derived Contracts | lint-clean dual-format specs derive one contract per Properties row (`ah sync`, with stale-drift detection and human-owned field preservation) |
| Range-Based Test Selection | `ah check --run-tests --base/--head` selects by commit range with total precedence and conservative fallback |

(All gate scenarios are verified — run `ah check --run-tests` for the live count.)

### `cli` — Command surface (53 scenarios)

Covers the full `ah` command interface: init, check, doctor, report, explain, type, scenario, archive, upgrade, lint, signals, feedback, sync, completions — plus JSON-envelope exit semantics (OK agrees with the exit code) and recommendation findings.

| Requirement | What it verifies |
|-------------|------------------|
| CLI Command Name | binary is named `ah` |
| Project Initialization | `ah init` creates `.espectacular/` and hook integration |
| Correspondence Check Command | `ah check` validates specs and runs tests |
| Spec Lint Command | `ah lint` analyzes spec files for authoring-quality findings |
| Sync Command | `ah sync` derives contracts from Properties rows (`--check` mode for CI) |
| Health Check Command | `ah doctor` diagnoses config, paths, hooks, archetypes |
| Archetype Documentation Commands | `ah type` lists and explains archetypes |
| Scenario Lifecycle Commands | `ah scenario new` and `ah scenario supersede` |
| Archive Companion Command | `ah archive` promotes staged contracts |
| Upgrade Command | `ah upgrade` detects and reports tool-version drift |
| Doctor enable flag | `ah doctor --enable <capability>` writes config blocks |
| Explain subcommand | `ah explain` prints guidance for finding kinds and actions |
| Coverage report command | `ah report` displays a conformance coverage matrix |
| Recommendation findings | findings that need human review are flagged as such |

### `lint` — Spec-quality heuristics (32 scenarios)

Covers the `ah lint` checks: vague qualifiers, imperative steps, conjunctive bloat, missing negative scenarios, missing non-goals, unresolved ambiguity, entangled specs, finding schema, and the specodelic bridge (dual-format relay + relay completeness + property trace).

| Requirement | What it verifies |
|-------------|------------------|
| Vague Qualifier Detection | vague-qualifier precision (bounded qualifiers not flagged) |
| Imperative Step Detection | imperative-step detection |
| Conjunctive Step Bloat Detection | conjunctive-bloat with configurable limit |
| Missing Negative Scenario Detection | happy-path-only requirements flagged |
| Missing Non-Goals / Unresolved Ambiguity / Entangled Specification Detection | structural authoring checks |
| Lint Finding Schema | findings validate the lint schema |
| Dual-Format Lint Bridge / Relay Completeness / Scenario Property Trace | specodelic rule findings relay through `ah lint`; scenario/property trace advisories |

### `adapters` — Language adapter dispatch (27 scenarios)

Covers how `ah check` maps test types to runners and normalizes output.

| Requirement | What it verifies |
|-------------|------------------|
| Adapter detection precedence | config > manifest > binary on PATH |
| Python pytest adapter | pytest detection, invocation, failure normalization |
| Rust cargo test adapter | cargo detection, invocation, failure normalization |
| TypeScript vitest adapter | vitest detection, invocation, failure normalization |
| No-adapter-configured path | `missing-runner` finding when type has no runner |
| Custom runner plugin protocol | custom runners emit JSON envelopes parsed by `ah check` |

### `explain` — Playbook system (19 scenarios)

Covers the `ah explain` topic system and its compile-time completeness guarantee.

| Requirement | What it verifies |
|-------------|------------------|
| Playbook is compile-enforced | every finding kind has an `ah explain` topic at compile time |
| Topic coverage | all finding kinds and suggested actions are covered |
| Structured JSON output | `--json` emits a machine-readable topic list |
| Topic listing / Quality finding topics / Adapter topics | listing shape; quality and adapter topics ship with their features |
| Unknown topic handling | unknown topics exit 1 with "did you mean" suggestions |

### `config`, `cli-core`, `spec-authoring` — Infrastructure (16 scenarios)

Shared config management (4), CLI scaffold guide + `ah sync` command (7 across 2 requirements), dual-format spec authoring (5).

---

## How to read this page

- **Deployed** means the scenario has a passing contract in `ah check` on `main`.
- **Planned** means the scenario is staged in an OpenSpec change but not yet implemented (none currently — all active changes are fully implemented and archived).
- Run `ah check` locally to see current pass/fail state.
- Spec source lives in [`openspec/specs/`](https://github.com/charly-vibes/espectacular/tree/main/openspec/specs).
