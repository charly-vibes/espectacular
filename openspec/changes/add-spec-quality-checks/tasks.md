## 1. Lint engine foundation

- [ ] 1.1 Add `lint` feature flag and `src/lint.rs` module entry point
- [ ] 1.2 Define `LintFinding` type reusing shared finding schema (kind, severity, spec_path, scenario_id, message, suggestion, suggested_action, playbook_command)
- [ ] 1.3 Implement spec file walker that visits each `#### Scenario:` block and its parent requirement

## 2. Test fixtures

- [ ] 2.1 Write fixture spec files covering each check: one "defective" fixture triggering the finding and one "clean" fixture that should pass silently
- [ ] 2.2 Write failing unit tests for each check (six tests, one per kind) against the fixtures from 2.1
- [ ] 2.3 Write failing integration test: `ah lint` on defective fixture produces expected finding kinds
- [ ] 2.4 Write failing integration test: `ah lint` on clean fixture produces zero findings
- [ ] 2.5 Write failing integration test: `ah lint --json` emits valid JSON matching the finding schema

## 3. Check implementations

Implement each check to make the corresponding failing tests pass.

- [ ] 3.1 `vague-qualifier`: flag scenarios/requirements containing unbound qualifiers (fast, scalable, user-friendly, easy, simple, intuitive) without an adjacent numeric bound
- [ ] 3.2 `imperative-step`: flag WHEN/THEN lines referencing UI mechanics (clicks, CSS selectors, navigates to URL, fills field)
- [ ] 3.3 `conjunctive-bloat`: flag scenarios with more than the configured maximum AND-chained steps (default: 5)
- [ ] 3.4 `missing-negative-scenario`: flag requirements with no scenario testing an error, rejection, or boundary violation
- [ ] 3.5 `missing-non-goals`: flag spec files that lack a Non-Goals or Out-of-Scope section at the capability level
- [ ] 3.6 `unresolved-ambiguity`: flag requirements or scenarios containing `[NEEDS CLARIFICATION` markers

## 4. CLI integration

- [ ] 4.1 Add `ah lint` subcommand routing to lint engine
- [ ] 4.2 Support `--changes <id>` overlay (lint staged change specs in addition to deployed)
- [ ] 4.3 Support `--json` flag for machine-readable output
- [ ] 4.4 Support `--check <kind>` flag to run a single check category
- [ ] 4.5 Exit zero when only warning-severity findings; exit non-zero on error-severity findings

## 5. Doctor integration

- [ ] 5.1 Have `ah doctor` suggest running `ah lint` when no lint run has completed in the current session

## 6. Explain topics

- [ ] 6.1 Add `ah explain` topics for each lint finding kind (6 topics)
