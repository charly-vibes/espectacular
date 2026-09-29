# Authoring Dual-Format Specs

espectacular supports the **specodelic dual-format protocol** as an *opt-in*
authoring mode: a single `spec.md` can carry both the openspec grammar
(`## ADDED|MODIFIED|REMOVED Requirements` + `#### Scenario:` headings) and
the specodelic four-layer grammar (frontmatter intent, Constraints, Model,
Properties), each linted by its own tool.

- openspec half → validated by `openspec validate <change-id> --strict`
- specodelic half → linted by `spk lint <file>`
- espectacular's gate (`ah check`) reads only the openspec half; the
  specodelic layers are inert decoration it happens to understand.

A worked example lives in
`openspec/changes/adopt-dual-format-specs/specs/gate/spec.md` — this page
explains its anatomy.

## Anatomy of a dual-format file

```markdown
---
id: spec                      ← 1. YAML frontmatter (intent layer)
kind: intent
statement: "WHEN … THE … SHALL …"
---

# <capability heading>

Prose (optional). Never parsed by the gate.

## Constraints                ← 2. Constraints table (invariant layer)

| id | kind | expr | traces_to |
|----|------|------|-----------|
| C-mirror-dedupe | invariant | two `#### Scenario:` headings … | |

## Model                      ← 3. Model (states + transitions)

### States
- `scanning`

### Transitions

| id | from | to | guard |
|----|------|----|-------|
| t-scan | scanning | deduplicating | [[spec.C-tables-inert]] |

## Properties                 ← 4. Properties table

| id | kind | derives_from | generator | predicate |
|----|------|--------------|-----------|-----------|
| P-dedupe | unit | [[spec.C-mirror-dedupe]] | … | … |

## MODIFIED Requirements      ← openspec half (unchanged grammar)

### Requirement: Scenario Discovery
…

#### Scenario: Reject duplicate scenario ids
- **GIVEN** …
- **WHEN** …
- **THEN** …
```

### Section-sync mirror

In change deltas, the specodelic protocol mirrors requirement text into a
trailing `## Requirements` section so deployed-spec tooling sees a stable
section spelling. The mirror **must be textually identical** to the delta
section (specodelic section-sync).

The gate understands this: two `#### Scenario:` headings in the same file
with the **same slugified id and identical bodies** are treated as one
logical scenario and discovered exactly once. It emits no slug-collision
finding for the mirror.

### Table derivation

Scenario prose may be derived into structured tables (e.g. a Properties
`generator`/`predicate` column quoting a scenario). This is safe by
construction: the gate only discovers scenarios from lines that **start**
with `#### Scenario: `. YAML frontmatter, Constraints rows, Model states
and transitions, and Properties rows never yield scenarios — no matter
what scenario-like text they quote.

## Enforcement is opt-in

espectacular deliberately does **not** adopt specodelic's CI rule
("capability specs must be dual-format or CI fails"). Requiring the
four-layer grammar would break plain openspec repos. A repo opts in by
authoring files dual-format; plain openspec files remain valid and discover
scenarios unchanged. `ah check` treats both grammars as valid.

## In-gate section-sync drift signal

Because mirrored sections must be textually identical, a **same-id,
different-body** collision in a dual-format repo is not just a naming
accident — it is section-sync drift: one of the two mirrored copies was
edited without the other. `ah check` catches this as a slug-collision
structural finding **without** running `spk lint`, which makes the gate a
cheap first-line drift detector for dual-format repos. (`spk lint` remains
the authoritative section-sync linter; the gate's signal is a byproduct of
collision detection, not a replacement.)

## Linting both halves

```bash
# specodelic half
spk lint openspec/changes/<change-id>/specs/<capability>/spec.md

# openspec half
openspec validate <change-id> --strict

# gate (both grammars in scope)
ah check --changes <change-id>
```

All three must exit zero before the change is deployed; the gate additionally
proves every discovered scenario has a contract.

## Archive round-trip

`openspec archive` merges the openspec half of a delta into the deployed
spec but **strips the specodelic half** (frontmatter and four-layer tables
do not survive). This is harmless by design: the gate only discovers
scenarios from `#### Scenario:` headings, so a deployed spec — now plain
openspec — behaves identically. Treat dual-format as a change-delta and
source-repo authoring practice, not a deployed-spec artifact.
