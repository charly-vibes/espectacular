# Tasks: add-contract-property-class

Each numbered group is one TDD cycle: red (failing test) → green (implementation) → refactor.

## 1. Contract field parsing

- [ ] 1.1 Failing unit test: contract with `property_class = "safety"` parses; contract without the field parses with empty default (red)
- [ ] 1.2 Add optional `property_class` to contract struct in `src/contracts.rs`, defaulting to empty (green)
- [ ] 1.3 Refactor: shared enum parsing with `status` if duplication emerges

## 2. Structural validation

- [ ] 2.1 Failing unit test: `property_class = "eventual"` produces `invalid-property-class` finding, non-zero exit (red)
- [ ] 2.2 Implement validation mirroring `invalid-status` handling in `src/contracts.rs`/`src/check.rs` (green)

## 3. Liveness timeout warning + severity field

- [ ] 3.1 Failing unit test: `property_class = "liveness"` with all test entries lacking `timeout_seconds` produces `missing-liveness-timeout` warning; exit stays zero (red)
- [ ] 3.2 Failing unit test: same contract with one `timeout_seconds` entry produces no warning (red)
- [ ] 3.3 Failing unit test: check finding JSON gains `severity` field — `"error"` on existing structural findings, `"warning"` on the new kind (red)
- [ ] 3.4 Implement warning emission + severity field in `src/check.rs`/`src/report.rs`; update `schemas/check-output.schema.json` (green)
- [ ] 3.5 Verify all existing finding kinds serialize with `severity = "error"` and remain byte-compatible apart from the added field

## 4. Schema, docs, playbooks

- [ ] 4.1 Update `schemas/scenario-contract.schema.json` with the optional enum field
- [ ] 4.2 Update `docs/src/concepts.md` contract section (field, semantics, warning behavior; `safety` is annotation-only in v1)
- [ ] 4.3 Add `ah explain` topics for `invalid-property-class` and `missing-liveness-timeout` (compile-enforced per explain spec)
- [ ] 4.4 `ah doctor` suggests tagging contracts whose scenario text contains "eventually" (advisory nudge, per design risk mitigation)
- [ ] 4.4 Dogfood: audit espectacular's own `.espectacular/` contracts and tag any obvious liveness/safety claims (e.g., timeout-related scenarios)

## 5. Validation

- [ ] 5.1 Full `just validate` + `ah check` pass on own repo
- [ ] 5.2 Update tasks in `bd` per project workflow; close tickets as each cycle lands
