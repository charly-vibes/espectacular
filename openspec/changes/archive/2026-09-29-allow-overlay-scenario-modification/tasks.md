# Tasks: allow-overlay-scenario-modification

## 1. TDD: overlay modification semantics in resolve_scope

- [x] 1.1 RED: `run_check_overlay_modification_with_staged_contract_passes` — deployed scenario redefined in delta + staged contract → zero overlay-conflicts
- [x] 1.2 RED: `run_check_unsignaled_scenario_redefinition_still_conflicts` — no staged contract → overlay-conflict remains
- [x] 1.3 RED: `run_check_conflicting_scenario_modifications_across_changes` — two changes redefining same deployed id → overlay-conflict
- [x] 1.4 GREEN: implement staged-contract-signaled modification in `resolve_scope` (src/check.rs)

## 2. Gate validation

- [x] 2.1 `openspec validate allow-overlay-scenario-modification --strict` exits zero
- [x] 2.2 Existing overlay tests stay green unchanged (reject-conflicting-overlays, staged metadata update, deterministic resolution)
- [x] 2.3 `ah check --changes adopt-dual-format-specs` reports zero overlay-conflicts after its modified scenarios get staged contracts
- [x] 2.4 Contract for the new scenario under `.espectacular/changes/allow-overlay-scenario-modification/gate/`

## 3. Docs

- [x] 3.1 Document the modification workflow (stage the contract to signal a deliberate redefinition) in docs/
