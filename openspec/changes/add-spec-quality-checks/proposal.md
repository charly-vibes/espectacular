# Change: Add spec quality linting to `ah`

Change-id: `add-spec-quality-checks`
Status: draft
Depends on: `add-spec-assertions` and `add-quality-measurement-and-adapters` being archived/deployed first.

## Why

`ah check` verifies that scenarios have sidecar contracts and passing tests, but does not evaluate the intrinsic quality of the scenarios themselves. Research into SDD best practices (EARS, NASA atomicity checks, BDD declarative style, Spectral-analogous linting) identifies a consistent set of spec defects — vague qualifiers, imperative steps, conjunctive bloat, missing negative scenarios — that cause AI-generated code to be correct-by-gate but wrong-by-intent. A lightweight spec linter gives authors and AI agents feedback at authoring time, before test wiring begins.

## What Changes

- Add `ah lint` command that statically analyzes OpenSpec scenario files for quality findings.
- Emit findings in the same stable JSON schema as `ah check`, making them agent-consumable without a new parsing surface.
- Cover six check categories derived from research: `vague-qualifier`, `imperative-step`, `conjunctive-bloat`, `missing-negative-scenario`, `missing-non-goals`, and `unresolved-ambiguity`.
- All lint findings are `warning` severity in v1; `ah lint` exits non-zero only when findings with `severity = error` exist (structural issues such as malformed spec files).
- Integrate `ah lint` into `ah doctor` as a suggested step, not as a gate blocker.

## Non-Goals

- `ah lint` does not modify spec files or auto-fix issues.
- `ah lint` does not evaluate test quality or test coverage — that is `ah check`'s domain.
- `ah lint` does not enforce EARS syntax — it detects quality signals, not grammar conformance.
- No threshold-based gate failures in v1; findings are advisory.
- No ML-based semantic analysis; all checks are heuristic and pattern-based.

## Impact

- Affected specs: `cli` (new `ah lint` requirement)
- New specs: `lint` (quality check capability and finding catalog)
- No gate spec changes: `ah lint` is a static authoring tool and does not affect gate evaluation behavior.
- Affected code: `src/lint.rs`, `src/lint/checks/` module tree, finding schema (new `lint-*` kinds), `schemas/check-output.schema.json`
- **BREAKING**: none. `ah lint` is additive; existing `ah check` behavior is unchanged.
