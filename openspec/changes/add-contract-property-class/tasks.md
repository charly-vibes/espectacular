# Tasks: add-contract-property-class

Each numbered group is one TDD cycle: red (failing test) → green (implementation) → refactor.

## 1. Contract field parsing

- [x] 1.1 Failing unit test: contract with `falsifiability_class = "safety"` parses; contract without the field parses with empty default (red)
- [x] 1.2 Add optional `falsifiability_class` to contract struct in `src/contracts.rs`, defaulting to empty (green)
- [x] 1.3 Refactor: shared enum parsing with `status` if duplication emerges (condition evaluated 2026-10-02: no duplication emerged — status validates in `contracts.rs` bail-path, falsifiability_class validates in `check.rs` via `FALSIFIABILITY_CLASSES` by design, per gate delta C-falsifiability-class-values)

## 2. Structural validation

- [x] 2.1 Failing unit test: `falsifiability_class = "eventual"` produces `invalid-falsifiability-class` finding, non-zero exit (red)
- [x] 2.2 Implement validation mirroring `invalid-status` handling in `src/contracts.rs`/`src/check.rs`; register `invalid-falsifiability-class` in the finding `kind` enum (`schemas/check-output.schema.json`) and in `FindingKind` (`src/explain.rs`, mirroring the `invalid-status` registration) (green)

## 3. Liveness timeout warning + severity field

- [x] 3.1 Failing unit test: `falsifiability_class = "liveness"` with all test entries lacking `timeout_seconds` produces `missing-liveness-timeout` warning; exit stays zero (red)
- [x] 3.2 Failing unit test: same contract with one `timeout_seconds` entry produces no warning (red)
- [x] 3.3 Failing unit test: check finding JSON gains `severity` field — `"error"` on existing structural findings, `"warning"` on the new kind (red)
- [x] 3.4 Implement warning emission + severity field in `src/check.rs`/`src/report.rs`; update `schemas/check-output.schema.json` (severity property AND `missing-liveness-timeout` in the `kind` enum + `FindingKind` registry) (green)
- [x] 3.5 Verify all existing finding kinds serialize with `severity = "error"` and remain byte-compatible apart from the added field (every_preexisting_finding_kind_serializes_error_severity + external custom-runner findings default to error via serde default)
- [x] 3.6 Precedence test: `falsifiability_class = "liveness"` with zero test entries emits `no-tests-declared` (error) and no liveness warning — the warning fires only when ≥1 test entry exists (Rule of 5 review 2026-10-01, EDGE-001) (red)

## 4. Schema, docs, playbooks

- [x] 4.1 Update `schemas/scenario-contract.schema.json` with the optional enum field
- [x] 4.2 Update `docs/src/concepts.md` contract section (field, semantics, warning behavior; `safety` is annotation-only in v1)
- [x] 4.3 Add `ah explain` topics for `invalid-falsifiability-class` and `missing-liveness-timeout` (compile-enforced per explain spec) — both topics landed with cycles 2 and 3 respectively (registration is compile-forced); remaining 4.3 verification is covered by explain CLI tests
- [x] 4.4 `ah doctor` suggests tagging contracts whose scenario text contains "eventually" (advisory nudge, per design risk mitigation) — MUST use the suggestion path, NOT a finding: genesis maps any LintResult (incl. Advisory) to CheckStatus::Warn → `ah doctor` exits 1; a failing nudge would recreate the mu5/djw foot-gun (Rule of 5 review 2026-10-01, EXCL-003) (session_suggestion with anti-goal guard test: never flips health verdict)
- [x] 4.5 Dogfood: audit espectacular's own `.espectacular/` contracts and tag any obvious liveness/safety claims (e.g., timeout-related scenarios) — no repo scenario prose contains "eventually" (doctor nudge stays silent, verified); gate/enforce-test-timeout tagged "safety" (bounded-termination claim, falsified by a single deterministic run, tests already carry timeout_seconds); no liveness candidates

## 5. Validation

- [ ] 5.1 Full `just validate` + `ah check` pass on own repo
- [ ] 5.2 Close bd cycle tickets as each cycle lands — one tracer-bullet ticket per cycle group, filed 2026-10-02 (issue-review remediation): espectacular-yq3 (§1) → espectacular-7js (§2) → espectacular-vej (§3) → espectacular-jul (§4) → espectacular-n8b (§5, this validation pass). Tick this file's checkboxes in the same commits as the code.
