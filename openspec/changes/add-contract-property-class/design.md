# Design: add-contract-property-class

## Context

Scenario contracts are the machine-readable pairing between spec scenarios and verifying tests (`src/contracts.rs`). The contract format is governance-protected: even additive changes require an openspec proposal, hence this document. Source research: `.wai/projects/espectacular/research/2026-09-05-machine-verifiable-spec-engineering-framework-analysis.md` (improvement #4).

## Goals / Non-Goals

- Goals: give liveness claims a machine-checkable home; enforce timeout semantics on liveness-tagged tests.
- Non-Goals: no new archetype; no gate failure from warnings; no statistical-assertion envelope (deferred with #6).

## Decisions

- **Optional enum field, default empty.** Untagged contracts are unaffected; backward compatibility is structural, not behavioral.
- **Invalid value → error; missing timeout on liveness → warning.** Asymmetry is deliberate: a typo in the field is authoring error and must fail (`invalid-property-class`, mirroring `invalid-status`). An untagged-timeout liveness claim is a *quality* signal — failing the gate on it would break adopters' existing contracts the moment they opt in, contradicting the warning-first precedent set by the `add-spec-quality-checks` draft.
- **Severity field is owned here.** Check findings have no `severity` today (verified: no occurrence in `schemas/check-output.schema.json`, `src/report.rs`, `src/check.rs`). This change adds it (`"error"` default on all existing findings, `"warning"` for `missing-liveness-timeout`); the lint draft's finding-schema task inherits the shared field instead of adding it independently.
- **Alternatives considered:** (a) new archetype for liveness behavior — rejected: orthogonal taxonomy change requiring two+ adopters per research recommendation; (b) inferring liveness from scenario text — rejected: inference is the lint's job (`add-spec-quality-checks`), the contract records the author's judgment.

## Risks / Trade-offs

- Field may go unused (adoption risk) → mitigation: `ah doctor` suggests tagging contracts whose scenario text contains "eventually"; dogfooding task tags espectacular's own contracts.
- Enum future-proofing (`verification_kind` generalization) → the field name is scoped to `property_class` now; a later proposal can supersede it through the normal append-only cycle.

## Migration Plan

None required. Adoption is per-contract, opt-in. Rollback: remove the field from any tagged contract; the validator ignores unknown keys (serde `Deserialize` default tolerance, `src/contracts.rs:7`), so contracts carrying the field still load for the release after removal — verified against the actual deserializer, not assumed.

## Open Questions

- Should `ah signals` surface `missing-liveness-timeout` warnings as dont drift signals? Deferred until the warning exists and we see its noise rate.
