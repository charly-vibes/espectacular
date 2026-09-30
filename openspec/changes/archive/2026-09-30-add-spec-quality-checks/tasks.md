> Tracker of record: beads (espectacular-9xq → rty/aar → 6fp → eia/zlw; task 4.1
> routed via 9xq, §5–6 via eia). Tick checkboxes here at change-archive time,
> not per-ticket — tickets carry the live status.

## 1. Lint engine foundation

- [x] 1.1 Add `lint` feature flag and `src/lint.rs` module entry point
- [x] 1.2 Define `LintFinding` type reusing shared finding schema (kind, severity, spec_path, scenario_id, message, suggestion, suggested_action, playbook_command)
- [x] 1.3 Implement spec file walker that visits each `#### Scenario:` block and its parent requirement

## 2. Test fixtures

- [x] 2.1 Write fixture spec files covering each check: one "defective" fixture triggering the finding and one "clean" fixture that should pass silently
- [x] 2.2 Write failing unit tests for each check (seven tests, one per kind) against the fixtures from 2.1
- [x] 2.3 Write failing integration test: `ah lint` on defective fixture produces expected finding kinds
- [x] 2.4 Write failing integration test: `ah lint` on clean fixture produces zero findings
- [x] 2.5 Write failing integration test: `ah lint --json` emits valid JSON matching the finding schema

## 3. Check implementations

Implement each check to make the corresponding failing tests pass.

- [x] 3.1 `vague-qualifier`: flag scenarios/requirements containing unbound qualifiers (fast, scalable, user-friendly, easy, simple, intuitive) without an adjacent numeric bound
- [x] 3.2 `imperative-step`: flag WHEN/THEN lines referencing UI mechanics (clicks, CSS selectors, navigates to URL, fills field)
- [x] 3.3 `conjunctive-bloat`: flag scenarios with more than the configured maximum AND-chained steps (default: 5)
- [x] 3.4 `missing-negative-scenario`: flag requirements with no scenario testing an error, rejection, or boundary violation
- [x] 3.5 `missing-non-goals`: flag spec files that lack a Non-Goals or Out-of-Scope section at the capability level
- [x] 3.6 `unresolved-ambiguity`: flag requirements or scenarios containing `[NEEDS CLARIFICATION` markers
- [x] 3.7 `entangled-spec`: flag scenarios with contract archetype `PF` or `SA` whose text references presentation primitives; matcher list excludes domain-legitimate terms (e.g., "event routing")

## 4. CLI integration

- [x] 4.1 Add `ah lint` subcommand routing to lint engine
- [x] 4.2 Support `--changes <id>` overlay (lint staged change specs in addition to deployed)
- [x] 4.3 Support `--json` flag for machine-readable output
- [x] 4.4 Support `--check <kind>` flag to run a single check category
- [x] 4.5 Exit zero when only warning-severity findings; exit non-zero on error-severity findings

## 5. Doctor integration

- [x] 5.1 Have `ah doctor` suggest running `ah lint` when no lint run has completed in the current session

## 6. Explain topics

- [x] 6.1 Add `ah explain` topics for each lint finding kind (7 topics)

## 7. Dual-format lint bridge (resolves beads espectacular-9ov)

- [x] 7.1 RED: dual-format fixture file (with `id: spec` frontmatter) → `ah lint` emits both a prose-check finding and a relayed `spk.<rule_id>` finding
- [x] 7.2 RED: plain openspec fixture → `ah lint` output byte-identical to pre-bridge behavior (bridge inert)
- [x] 7.3 RED: `spk` absent from PATH → single advisory `spk-unavailable` finding, exit code unchanged
- [x] 7.4 RED: `spk` reports lint errors on the file → relayed findings, all `severity = "warning"`, exit still warning-only
- [x] 7.5 GREEN: implement bridge — detect dual-format frontmatter, invoke `spk lint --json` (or best available flag), parse envelope, map to `kind = "spk.<rule_id>"` with `severity = "warning"`, merge into findings; advisory finding on invocation/parse failure
