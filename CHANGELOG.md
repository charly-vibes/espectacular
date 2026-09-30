# Changelog

All notable changes to `ah` are documented here.

Format: [Keep a Changelog](https://keepachangelog.com/en/1.1.0/).
Versioning: [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

---

## [Unreleased]

## [0.8.0] — 2026-10-01

### Added

- **`falsifiability_class` contract field** — scenario contracts may declare
  `falsifiability_class = "safety"` or `"liveness"` (optional, absent by
  default; contracts without it validate exactly as before). Liveness claims
  are only falsifiable under bounded execution: a liveness-tagged contract
  whose test entries all omit `timeout_seconds` emits a
  `missing-liveness-timeout` **warning** suggesting an explicit timeout —
  warnings do not fail the gate.
- **`invalid-falsifiability-class` structural finding** — an invalid value
  fails the gate and blocks the scenario's declared tests from running.
- **`severity` field on findings** — gate-level severity (`"error"` for
  structural/execution findings, `"warning"` for non-gating findings);
  external custom-runner findings that omit it default to `"error"`.
- **`ah doctor` falsifiability nudge** — session suggestion (advisory, never
  fails the doctor run) to tag contracts whose scenario text contains
  "eventually" — the classic liveness marker.
- **`ah lint` malformed-spec finding** — a spec file that cannot be parsed
  emits an error-severity lint finding instead of aborting the walk.
- **Mutation ownership doctor check** — raw mutator commands
  (cargo-mutants, mutmut, stryker, pit) in `quality.mutation.command` are
  flagged as `mutation-ownership`: pretender owns mutation execution and
  scoring; the supported wiring is
  `command = ["pretender", "mutation", "--format", "json"]`.
- **Contract schema** — `falsifiability_class` added to
  `schemas/scenario-contract.schema.json`; 7 new gate spec scenarios carry
  repo-local proof contracts wired to their unit/CLI tests.

### Changed

- **Specodelic dual-format spec corpus** — all 9 deployed specs migrated to
  specodelic dual format (YAML frontmatter intent + fixed-schema tables),
  constraint `traces_to` wiring wired corpus-wide, and specodelic gates
  (`spk lint`) added to repo tooling and CI.

### Fixed

- **CI** — tap/scoop update steps now skip cleanly when `TAP_GITHUB_TOKEN`
  is absent (secrets context is unavailable in step-level `if`); specodelic
  is installed from git in both CI jobs.
- **Contract** — `cli/enable-already-active-capability-is-a-no-op` shell
  entry repointed at the surviving test; the stale `_is_noop` filter matched
  0 tests and emitted a `no-tests-ran` finding on every `--run-tests` run
  (espectacular-65d).

## [0.7.0] — 2026-09-30

### Added

- **`ah lint` — spec-quality linter** — finding registry and walker over
  `.espectacular/` contracts with mirror dedupe, routed as a CLI subcommand:
  - **Scenario-flow checks** — vague-qualifier, imperative-step,
    conjunctive-bloat (`[lint] max_and_steps`, default 5),
    missing-negative-scenario
  - **Spec-shape checks** — missing-non-goals, unresolved-ambiguity,
    entangled-spec
  - **CLI flags** — `--changes`, `--check` scoping and exit-code helper
  - **Dual-format bridge** — relays `spk.<rule_id>` findings (advisory-only)
  - **Doctor/explain integration** — lint session suggestion plus lint-kind
    explain topics
  - Deployed `lint` spec (9 requirements) via the archived
    `add-spec-quality-checks` change

### Fixed

- **Mutation gate runnable by default** — `doctor --enable mutation` writes
  command `["{}"]` (was the unspawnable default `[""]`) and the mutation
  engine substitutes the `{}` placeholder in the program position too, so the
  gate runs end-to-end without a custom runner (espectacular-mu5).
- **Fixture version literals** — test fixture `tool_version` literals derive
  from `CARGO_PKG_VERSION` instead of hardcoded strings that only break when
  the crate version changes (espectacular-n8o).

## [0.6.0] — 2026-09-29

### Added

- **Pre-push gating** — `ah init` now wires `ah check` into the lefthook
  `pre-push` stage in addition to `pre-commit`, so spec-test correspondence
  gates pushes too.
- **`hook-wired` doctor diagnostic** — stage-scoped check (via
  `genesis::git_hooks::lefthook::is_wired`): emits an Error when the
  lefthook `pre-commit` stage does not run `ah check`, and when the
  `pre-push` stage does not run it. A detected-but-unwired hook is
  decorative.

### Changed

- **genesis v0.8.1** — dependency bump (0.6 → 0.8.1, suite-wide round).
- **Hook detection delegated to `genesis::git_hooks`** — the local
  `HookFramework`/`detect_hook_framework` parallel types are deleted;
  `git_hooks::framework()` is the canonical detector. Behavior changes:
  - prek is now detected via `prek.toml` (was `.prek`/`prek.yml`)
  - Husky repos are detected via hook-file sigils; `ah init` reports a
    manual-wiring concern instead of erroring, and `ah doctor` no longer
    flags them as an unsupported framework
  - `resolve_hooks_dir` honors `core.hooksPath` (previously ignored)

---

## [0.5.0] — 2026-08-05

### Added

- **Suite-trio quality signals** — `quality-composability` (vampiro),
  `quality-cost` (crua), `quality-boundary-coverage` (livin) finding kinds
  with `[quality.*]` config blocks. Each tool emits a custom-runner JSON
  envelope; findings are informational only.
- **Config path traversal validation** — `paths.specs` and `paths.changes`
  now reject absolute (`/`) or `..`-containing paths.

### Changed

- **genesis v0.6.0** — adopt breaking envelope API: `cli_version` is now
  caller-supplied (`env!("CARGO_PKG_VERSION")`) in all `Envelope::success`
  and `Envelope::error` calls.
- **Remove local doctor types** — `DoctorReport`, `DoctorDiagnostic`,
  `DoctorRecommendation` replaced with `genesis::doctor` equivalents.
  JSON output now routes through `genesis::envelope::Envelope`. Domain-
  specific `CapabilitySuggestion` retained for framework recommendations.

### Fixed

- `ah --help` now shows descriptions for all subcommands (was blank for
  check, doctor, init, report, archive, type, explain, upgrade, scenario).
- `ah init` stubs use `CARGO_PKG_VERSION` instead of hardcoded `"0.1.0"`.
- README no longer falsely claims an `espectacular` binary.
- README commands table now includes `ah check --run-tests`, `ah report`,
  `ah feedback`, `ah completions`.
- AGENTS.md version string updated from 0.3.0 to 0.4.0.
- 3 pre-existing clippy warnings removed (unused imports).
- 9 bugs from previous session: zero_tests_ran false-positive, report
  archetype attribution + failing counts, configured specs root,
  quality_findings schema, process group timeout, README/docs JSON
  envelope, doctor --enable mutation writes, `{}` placeholder substitution
  in mutation runner.

---

## [0.4.0] — 2026-07-31

### Added

- `ah doctor --json` — emit structured recommendation findings
- `ah report` — display a conformance coverage matrix
- genesis v0.4.0 modules: doctor, feedback, cli, status, scaffold
- Behavioral guardrails in AGENTS.md

### Changed

- `ah doctor` output now routes through `genesis::envelope`

---

## [0.3.0] — 2026-07-21

### Added

- `ah check --run-tests` — explicit flag to execute contract tests. Without this
  flag, `ah check` runs fast static analysis only (spec/contract correspondence).
- `just validate` recipe across all 13 charly OpenSpec projects — one-command
  spec-test correspondence gate via `ah check`.
- `docs/src/agent-workflow.md` — replicable agent workflow pattern for `ah check`
  as a specification gate.
- `docs/src/audit-spec-validation-patterns.md` — audit of agent spec-validation
  patterns from April–July 2026.

### Changed

- `ah check` now defaults to fast static analysis. Use `--run-tests` to execute
  contract tests. Previous behavior ran all contract tests unconditionally.

---

## [0.2.2] — 2026-07-16

### Fixed

- `ah check` without `--json` now outputs human-readable diagnostics instead of
  raw JSON, matching `ah doctor` behavior. The `--json` flag continues to emit
  machine-readable JSON for CI/scripting use.

---

## [0.2.0] — 2026-07-08

### Added

- `ah doctor --json` — emit structured recommendation findings (`kind=recommendation`, `playbook_command`, `apply_command`) instead of text lines.
- `ah report` — display a conformance coverage matrix across all deployed specs and archetypes.
- `ah report --json` — emit the coverage matrix as machine-readable JSON.

### Changed

- `ah check` now reports 0 structural findings on a clean project (all contracts wired).

### Fixed

- `diagnose-correspondence-wiring` contract test filter changed from `ah_doctor_` (matched 0 tests) to `doctor_` (matches all 11 doctor integration tests).
- All 61 remaining stub contracts wired with test entries across adapters, gate, and explain specs.

---

## [0.1.0] — 2026-06-17

Initial stable release. Covers two deployed change proposals:
`add-spec-assertions` and `add-quality-measurement-and-adapters`.

### Added

#### Core gate (`add-spec-assertions`)

- `ah check` — validate deployed specs and run all declared tests; emits stable JSON to stdout.
- `ah check --changes <id>` — validate with one or more staged change overlays.
- `ah init` — create or refresh `.espectacular/` directory, config, AGENTS.md managed block, and pre-commit hook integration (lefthook, prek).
- `ah doctor` — diagnose config, path, hook, collision, orphan, archetype, and managed-block issues.
- `ah scenario new` — append a new scenario to a staged change spec and create its TOML contract stub.
- `ah scenario supersede` — stage a supersession update for an existing deployed contract.
- `ah archive <change>` — promote staged change contracts into deployed `.espectacular/` paths.
- `ah upgrade` — detect and repair `tool_version` drift in `.espectacular/config.toml`.
- `ah type` / `ah type <code>` — list or describe built-in archetypes (PF, SA, BP, CE, NR).
- Versioned, append-only archetype catalog embedded in the binary.
- Stable JSON output schema at `schemas/check-output.schema.json`.
- Scenario contract TOML schema at `schemas/scenario-contract.schema.json`.
- Config TOML schema at `schemas/config.schema.json`.

#### Language adapters and quality signals (`add-quality-measurement-and-adapters`)

- **Language adapter dispatch** — `ah check` auto-detects and routes test execution through the correct adapter based on config and project manifest:
  - Python: pytest (via `pyproject.toml`, `pytest.ini`, `setup.cfg`, environment import, or explicit config)
  - Rust: cargo test (via `Cargo.toml` or explicit config)
  - TypeScript: vitest (via `package.json` dependency or explicit config)
  - Custom: arbitrary runner via JSON envelope protocol (see below)
- **Custom runner protocol** — configure any test runner under `[runners.custom.<name>]`; the runner must emit a JSON envelope (`exit_code`, `passed`, `findings[]`) on stdout. Schema at `schemas/custom-runner.schema.json`.
- **`ah explain <topic>`** — print playbook guidance for any finding kind or suggested action; supports `--json` and `--list`.
- **`ah doctor --enable <adapter>`** — detect adapter readiness and write the corresponding runner or quality config block into `.espectacular/config.toml`.
- **Quality signals** (opt-in, emitted as `quality-*` findings when checks pass):
  - `quality-mutation` — mutation kill rate measured against a configured threshold.
  - `quality-property` — property-based test suite present and passing.
  - `quality-snapshot` — snapshot test suite present and passing.
- **`ah signals`** — read `.dont/events/*.json` and emit drift signal JSON for integration with the `dont` evidence layer.
- All findings now carry `suggested_action` and `playbook_command` fields for agent-consumable remediation.
- `summary.counts_by_kind` added to `ah check` JSON output.

### Finding kinds

| Kind | Category |
| --- | --- |
| `no-toml` | structural |
| `orphan-toml` | structural |
| `slug-collision` | structural |
| `id-mismatch` | structural |
| `no-tests-declared` | structural |
| `missing-runner` | structural |
| `malformed-contract` | structural |
| `missing-replacement` | structural |
| `overlay-conflict` | structural |
| `test-failing` | execution |
| `quality-mutation` | quality |
| `quality-property` | quality |
| `quality-snapshot` | quality |

---

[Unreleased]: https://github.com/charly-vibes/espectacular/compare/v0.8.0...HEAD
[0.8.0]: https://github.com/charly-vibes/espectacular/compare/v0.7.0...v0.8.0
[0.7.0]: https://github.com/charly-vibes/espectacular/compare/v0.6.0...v0.7.0
[0.6.0]: https://github.com/charly-vibes/espectacular/compare/v0.5.0...v0.6.0
[0.5.0]: https://github.com/charly-vibes/espectacular/releases/tag/v0.5.0
[0.4.0]: https://github.com/charly-vibes/espectacular/releases/tag/v0.4.0
[0.3.0]: https://github.com/charly-vibes/espectacular/releases/tag/v0.3.0
[0.2.2]: https://github.com/charly-vibes/espectacular/releases/tag/v0.2.2
[0.2.0]: https://github.com/charly-vibes/espectacular/releases/tag/v0.2.0
[0.1.0]: https://github.com/charly-vibes/espectacular/releases/tag/v0.1.0
