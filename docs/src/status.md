# Implementation Status

This page maps the deployed behavioral specs to the commands and capabilities that implement them. Each row is a scenario in a spec; if `ah check` passes, that behavior is verified in CI.

## Deployed specs

### `gate` — Core verification engine (13 scenarios)

Covers what `ah check` does: scenario discovery, contract correspondence, test execution, and JSON output.

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
| JSON finding schema includes agent-action fields | findings carry `suggested_action` and `playbook_command` |
| Quality measurement capabilities | `quality-*` findings are emitted and informational |
| Quality contract schema | quality fields validate in the contract schema |
| Conformance coverage matrix | all finding kinds are covered by at least one contract |
| apply_command is conditionally present | `apply_command` appears only when applicable |

### `cli` — Command surface (12 scenarios)

Covers the full `ah` command interface: init, check, doctor, explain, type, scenario, archive, upgrade.

| Scenario | What it verifies |
|----------|-----------------|
| CLI Command Name | binary is named `ah` |
| Project Initialization | `ah init` creates `.espectacular/` and hook integration |
| Correspondence Check Command | `ah check` validates specs and runs tests |
| Health Check Command | `ah doctor` diagnoses config, paths, hooks, archetypes |
| Archetype Documentation Commands | `ah type` lists and explains archetypes |
| Scenario Lifecycle Commands | `ah scenario new` and `ah scenario supersede` |
| Archive Companion Command | `ah archive` promotes staged contracts |
| Upgrade Command | `ah upgrade` detects and reports tool-version drift |
| Doctor enable flag | `ah doctor --enable <capability>` writes config blocks |
| Explain subcommand | `ah explain` prints guidance for finding kinds and actions |
| Coverage report command | quality findings surface in check output |
| Recommendation findings | `ah doctor` emits recommendations for detected-but-unconfigured adapters |

### `adapters` — Language adapter dispatch (6 scenarios)

Covers how `ah check` maps test types to runners and normalizes output.

| Scenario | What it verifies |
|----------|-----------------|
| Adapter detection precedence | config > manifest > binary on PATH |
| Python pytest adapter | pytest detection, invocation, failure normalization |
| Rust cargo test adapter | cargo detection, invocation, failure normalization |
| TypeScript vitest adapter | vitest detection, invocation, failure normalization |
| No-adapter-configured path | `missing-runner` finding when type has no runner |
| Custom runner plugin protocol | custom runners emit JSON envelopes parsed by `ah check` |

### `explain` — Playbook system (7 scenarios)

Covers the `ah explain` topic system and its compile-time completeness guarantee.

| Scenario | What it verifies |
|----------|-----------------|
| Playbook is compile-enforced | every finding kind has an `ah explain` topic at compile time |
| Topic coverage | all finding kinds and suggested actions are covered |
| Structured JSON output | `--json` emits a machine-readable topic list |
| Topic listing | `--list` enumerates all topics |
| Unknown topic handling | unknown topics exit 1 with "did you mean" suggestions |
| Quality finding kind topics | `quality-*` finding kinds have topics |
| Adapter topics ship with adapters | adapter-specific topics exist for each adapter |

---

## In progress

### `add-spec-quality-checks` — `ah lint` command (0/21 tasks)

Adds spec quality linting: `ah lint` checks spec files for vague qualifiers, imperative steps, conjunctive bloat, missing negative scenarios, missing non-goals, and unresolved ambiguities.

| Scenario | Status |
|----------|--------|
| Spec Lint Command | planned |
| Vague Qualifier Detection | planned |
| Imperative Step Detection | planned |
| Conjunctive Step Bloat Detection | planned |
| Missing Negative Scenario Detection | planned |
| Missing Non-Goals Detection | planned |
| Unresolved Ambiguity Detection | planned |
| Lint Finding Schema | planned |

---

## How to read this page

- **Deployed** means the scenario has a passing contract in `ah check` on `main`.
- **Planned** means the scenario is staged in an OpenSpec change but not yet implemented.
- Run `ah check` locally to see current pass/fail state.
- Spec source lives in [`openspec/specs/`](https://github.com/charly-vibes/espectacular/tree/main/openspec/specs).
