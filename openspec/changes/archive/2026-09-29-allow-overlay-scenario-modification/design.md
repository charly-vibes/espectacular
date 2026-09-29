# Design: allow-overlay-scenario-modification

## Decision: staged contract overrides signal text modification (b)

Three candidate designs were considered (recorded in beads
`espectacular-hct` before implementation):

1. **(a) supersession-based text modification** — overlay scenario supersedes
   the deployed one by explicit staged declaration. Rejected: the existing
   supersession machinery (`status = "superseded"` + `superseded_by`) models
   replacing a scenario with a *different* scenario id; same-id text
   modification would need a new status value or a self-referential
   `superseded_by`, both of which stretch the contract format and its
   validation (`reject-unknown-status`, `validate-superseded-status`).
2. **(b) staged contract override implies modification** — if the change
   defines a scenario id that exists in the deployed base scope AND stages a
   contract for that `(spec, id)`, the overlay prose replaces the base prose
   in scope. **Chosen.**
3. **(c) keep conflicts, document a workflow** — rejected outright: fails the
   acceptance criterion (a MODIFIED-requirement fixture must pass
   `ah check --changes` with zero overlay-conflict findings).

Rationale for (b):

- Minimal footprint: one condition in `resolve_scope` plus scenario
  replacement; no contract-format or IR change (respecting the "no contract
  format changes without a proposal" constraint — this change needs no such
  change at all).
- Staging a contract override is already a deliberate, meaningful act (the
  "Apply staged metadata update for deployed scenario" scenario gives it
  semantics). Reusing it as the modification signal requires no new
  declaration syntax.
- The modified scenario needs its own contract anyway — the modified text
  must be proven by tests, and under the overlay the staged contract is the
  active one. Signal and proof coincide.
- Safety preserved where it matters: an *unsignaled* redefinition (no staged
  contract) still fails; two changes touching the same `(spec, id)` still
  conflict; sorted-change determinism is unchanged.

Accepted limitation: modifying a scenario now requires staging a contract
even if the tests did not change. This is the explicitness tax that replaces
the blanket conflict; documented in the delta and docs.

## Test shape (defined before implementation)

- `run_check_overlay_modification_with_staged_contract_passes` — deployed
  scenario `green-path` redefined in a change delta with different body +
  staged contract → zero `overlay-conflict` findings, validation proceeds.
- `run_check_unsignaled_scenario_redefinition_still_conflicts` — same delta
  without a staged contract → `overlay-conflict` remains.
- `run_check_conflicting_scenario_modifications_across_changes` — two changes
  both redefine `green-path` with staged contracts → `overlay-conflict`.
- Existing tests (reject-conflicting-overlays, staged-metadata-update,
  deterministic resolution) must stay green unchanged.
