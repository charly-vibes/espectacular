# Design: adopt-dual-format-specs

## Context

`src/openspec.rs::parse_scenarios_from_spec` scans every line of a `spec.md`
file for the `#### Scenario: ` prefix and extracts bodies up to the next `#`
heading. `detect_slug_collisions` keys on `(spec_path, id)`. This is correct
for deployed dual-format specs (which keep only `## Requirements`) but breaks
on change deltas, where the specodelic dual-format protocol mirrors identical
requirement text in both `## ADDED Requirements` and `## Requirements`.

## Decision: dedupe identical mirrors, not section-aware parsing

Two candidate fixes for the delta duplication:

1. **Section-aware parsing** — only discover scenarios inside
   `## ADDED Requirements` / `## Requirements` sections.
2. **Mirror deduplication** — within one file, treat two `#### Scenario:`
   headings with the *same slugified id and identical body* as one scenario;
   same id with a *different body* remains a collision.

**Chosen: (2) mirror deduplication.**

Rationale:

- Minimal footprint: one guard in `parse_scenarios_from_spec` plus a set of
  seen `(id, body)` pairs — no new section-state machine, no change to body
  extraction boundaries (`Extract scenario body boundaries` scenario keeps
  holding verbatim).
- Robust to openspec grammar drift: option 1 hard-codes openspec section
  spellings; option 2 only relies on the specodelic section-sync invariant
  ("the two requirement sections carry identical text"), which the specodelic
  linter already enforces upstream.
- The existing collision finding keeps its meaning: it now fires only on
  *genuinely distinct* scenarios that share a slug — arguably what it should
  have meant all along.

Consequence: the deployed **Scenario Discovery** requirement changes behavior
(its `Reject duplicate scenario ids` scenario must require differing bodies),
so the delta is authored as `## MODIFIED Requirements` carrying the complete
requirement — not as an ADDED sibling requirement. The contract
`.espectacular/gate/reject-duplicate-scenario-ids.toml` must be updated in the
same change so the deployed contract tracks the new behavior.

Bonus: since specodelic section-sync requires mirrored sections to be
textually identical, a same-id-different-body collision doubles as an in-gate
section-sync drift detector — `ah check` catches mirrored-section drift even
where `spk lint` is not run.

Edge: two headings with identical id and identical body that are *not* section
mirrors (plain copy-paste duplication) also dedupe. This loses a pathological
error signal, but the scenario prose is verbatim-identical, so the gate's
findings are unaffected and contracts still map 1:1. Accepted.

## Enforcement scope: opt-in only

specodelic's own CI rule ("capability spec under `openspec/specs/` must be
dual-format or CI fails") is **not** adopted. Requiring the four-layer grammar
would break plain openspec repos and violate espectacular's backward-compat
constraint. A repo opts in by authoring files dual-format; `ah check` treats
both grammars as inert decoration it happens to understand.

## Open question: MODIFIED requirements in overlay scope (escalated)

The delta MODIFIES the deployed Scenario Discovery requirement, but
`resolve_scope` conflicts any overlay scenario whose id exists in base scope —
so the two modified scenarios conflict until archive. This is a pre-existing
gate-model gap (openspec MODIFIED requirements are inexpressible), filed as
beads `espectacular-hct`; this change works around it by documenting the
expected residue in task 2.4 rather than expanding scope.

## Open question: archive round-trip

`openspec archive` merges deltas into deployed specs. Whether it preserves
frontmatter and the four-layer tables verbatim is unverified; task 4.1
covers verifying (or documenting) the round-trip before archiving this
change. If archiving strips the specodelic half, C-tables-inert makes the
deployed result harmless.

## Non-goals

- No scenario-contract TOML, JSON Schema IR, archetype, or adapter changes.
- No integration of `spk lint` / `spk model-check` into the gate — shell test
  entries already support that today; a separate proposal can formalize it
  later if wanted.
