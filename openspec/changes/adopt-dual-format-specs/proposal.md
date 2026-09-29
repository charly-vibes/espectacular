# Proposal: adopt-dual-format-specs

## Why

espectacular consumes openspec `spec.md` files but treats them as unstructured
prose plus `#### Scenario:` headings. The specodelic format adds a machine-linted
four-layer grammar (intent, constraints, model, properties) that can live in the
same markdown file via its dual-format protocol. Adopting it tightens the *spec*
side of spec-code drift detection — espectacular's core mission — without
abandoning openspec.

An investigation (Rule-of-5 reviewed) found:

- Dual-format **deployed specs** parse correctly with today's discovery logic.
- Dual-format **change deltas** break `ah check --changes`: mirrored
  `## ADDED Requirements` / `## Requirements` sections duplicate every
  `#### Scenario:` heading, tripping the slug-collision check.
- Enforcement must stay **opt-in**; mandating dual-format would break
  backward compatibility for plain openspec repos.

## What Changes

- **gate capability (MODIFIED)**: the deployed **Scenario Discovery** requirement
  is modified — mirrored identical scenarios deduplicate to one; structured
  layers (frontmatter, Constraints, Model, Properties tables) contribute no
  scenarios; collisions now mean same slug with **different bodies**. The
  deployed `reject-duplicate-scenario-ids` contract is updated accordingly
  (its collision fixture must use differing bodies).
- The spec delta under `specs/gate/spec.md` is itself authored in dual format,
  serving as the adoption pilot and a living example.
- No changes to the scenario contract TOML format, JSON Schema IR, archetypes,
  or runner adapters.

## Capabilities

### Modified
- `gate` — Scenario Discovery handles dual-format spec files.

## Impact

- `src/openspec.rs` (scenario parsing/dedup) and its tests.
- Docs: authoring guidance for dual-format specs.
- Adoption is opt-in; plain openspec specs remain fully valid and CI never
  requires the specodelic half.
