# Change: Add property_class field to scenario contracts

Change-id: `add-contract-property-class`
Status: draft

## Why

Contracts verify behavior without distinguishing *safety* claims (bad states never occur — a single deterministic test can falsify them) from *liveness* claims (good states eventually occur — only falsifiable with explicit timeout/termination semantics). A scenario asserting "every request eventually settles" reads as verified even when its tests have no timeout enforcement, because the contract format has no place to record the distinction. Follow-up research (2026-09-05, framework source: TLA+ safety/liveness classification) identified this as a metadata gap with concrete enforcement value.

## What Changes

- Add optional `property_class = "safety" | "liveness"` field to scenario contracts (`src/contracts.rs`, `schemas/scenario-contract.schema.json`). Default: empty (field may be omitted — no deprecation, no churn on existing contracts). In v1, `safety` is annotation-only: it records the author's judgment and gives downstream tooling a hook; only `liveness` has gate-enforced semantics.
- Introduce a `severity` field on `ah check` findings (`src/report.rs`, `schemas/check-output.schema.json`): `"error"` for structural/execution findings, `"warning"` for the new liveness check. This is the first warning in the gate; the `add-spec-quality-checks` draft's finding schema inherits the shared field rather than adding it twice.
- Invalid `property_class` values emit an `invalid-property-class` structural finding and exit non-zero, mirroring `invalid-status` handling.
- A `liveness`-tagged contract whose test entries all lack `timeout_seconds` emits a `missing-liveness-timeout` **warning** finding; the gate still passes (exit zero on warnings only). Warnings never fire when the field is absent, so existing behavior is byte-identical for untagged contracts. `timeout_seconds` is already supported on every test entry type (shared `TestEntry` struct, `src/contracts.rs:20`), so no entry-type work is needed.

## Impact

- Affected specs: `gate` (Contract Schema requirement — MODIFIED)
- Affected code: `src/contracts.rs` (parsing + validation), `src/check.rs` (warning finding), `src/report.rs` (finding severity field), `schemas/scenario-contract.schema.json`, `schemas/check-output.schema.json`, `src/explain.rs` (two new finding-kind topics), `src/doctor.rs` (tagging suggestion)
- Docs: `docs/src/concepts.md` contract section
- **BREAKING**: none. The field is optional and absent-by-default; untagged contracts validate exactly as before.

## Relationship to other work

- Companion to the `add-spec-quality-checks` draft: that change lints *spec text*; this change extends *contract metadata*. No overlap.
- Research opportunity #6 (archetype for non-deterministic scenarios) intentionally deferred: a statistical `verification_kind` would generalize this field, but is blocked until two+ adopters need it — adding it now would expand taxonomy without proven demand.
