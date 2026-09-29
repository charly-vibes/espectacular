# Proposal: allow-overlay-scenario-modification

## Why

`resolve_scope` flags any change-overlay scenario whose `(spec_path, id)`
already exists in the deployed base scope as an `overlay-conflict` structural
finding. A change proposal that MODIFIES a deployed requirement's scenario
text can therefore never pass `ah check --changes` — even though modifying
contract-backed scenario behavior is exactly when drift risk is highest.
Discovered while authoring `adopt-dual-format-specs`, whose MODIFIED Scenario
Discovery requirement produces two expected-but-unfixable overlay conflicts
(beads `espectacular-hct`); the change currently cannot be archived cleanly.

## What Changes

- A change delta that redefines a deployed scenario id replaces the deployed
  scenario text in scope **when the change also stages a contract for that
  `(spec, id)`** (`.espectacular/changes/<change>/<spec>/<id>.toml`). The
  staged contract is the explicit signal that the redefinition is deliberate.
- Without a staged contract, redefining a deployed scenario id remains an
  `overlay-conflict` structural finding (unchanged).
- Two selected changes touching the same `(spec, id)` — whether both stage
  contracts or one adds/redefines — still conflict (unchanged).
- No changes to the scenario contract TOML format or JSON Schema IR.

## Impact

- Affected specs: `gate` (MODIFIED requirement: Change Overlay Scope)
- Affected code: `resolve_scope` in `src/check.rs` (one condition + scenario
  replacement), tests, docs.
- Unblocks archiving `adopt-dual-format-specs` (its two residual overlay
  conflicts resolve once it stages contracts for its modified scenarios).
