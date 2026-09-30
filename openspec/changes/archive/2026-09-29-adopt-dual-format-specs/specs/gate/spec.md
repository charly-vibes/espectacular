---
id: spec
kind: intent
statement: "WHEN a spec file carries both the specodelic and openspec grammars THE gate SHALL discover each scenario exactly once, treating mirrored requirement sections and structured tables as never producing duplicate or extra scenarios."
---

# dual-format discovery

espectacular's gate already ignores prose it does not understand. This change
makes that ignorance precise for the specodelic four-layer grammar: files may
carry both grammars, and discovery must yield exactly one scenario per logical
scenario — not one per markdown occurrence.

A side benefit: because mirrored sections must stay textually identical
(specodelic section-sync), a same-id-different-body collision now doubles as
an in-gate section-sync drift detector for repos that opt into dual format —
no `spk lint` invocation required.

The specodelic half of this file is linted by `spk lint`; the openspec half by
`openspec validate --strict`. Neither tool reads the other's half.

## Constraints

| id | kind | expr | traces_to |
|----|------|------|-----------|
| C-mirror-dedupe | invariant | two `#### Scenario:` headings in the same file with the same slugified id and identical bodies yield exactly one discovered scenario | [[spec]] |
| C-distinct-bodies-collide | invariant | two `#### Scenario:` headings in the same file with the same slugified id but different bodies emit a slug-collision structural finding and fail the gate | [[spec]] |
| C-tables-inert | invariant | YAML frontmatter, Constraints rows, Model states and transitions, and Properties rows contribute no discovered scenarios | [[spec]] |
| C-opt-in | advisory | the gate never requires the specodelic half; a plain openspec spec file remains valid and discovers scenarios unchanged | [[spec]] |
| C-prose-untouched | advisory | prose outside frontmatter and the fixed-schema tables is never parsed and never alters the discovered scenario set | [[spec]] |

## Model

### States

- `scanning`
- `deduplicating`
- `reported`

### Transitions

| id | from | to | guard |
|----|------|----|-------|
| t-scan | scanning | deduplicating | [[spec.C-tables-inert]] |
| t-dedupe | deduplicating | reported | [[spec.C-mirror-dedupe]] |
| t-collide | deduplicating | reported | [[spec.C-distinct-bodies-collide]] |

## Properties

| id | kind | derives_from | generator | predicate |
|----|------|--------------|-----------|-----------|
| P-dedupe | unit | [[spec.C-mirror-dedupe]] | any markdown file whose mirrored requirement sections repeat a `#### Scenario:` heading verbatim | discovered scenario count for the mirrored id is exactly 1 |
| P-collision | unit | [[spec.C-distinct-bodies-collide]] | two headings with equal slugified id and differing bodies in one file | gate emits a slug-collision finding and exits non-zero |
| P-tables | unit | [[spec.C-tables-inert]] | a dual-format file with fully populated frontmatter, Constraints, Model, and Properties tables | no discovered scenario id originates from a table row or frontmatter field |
| P-optin | unit | [[spec.C-opt-in]] | a plain openspec spec file with no frontmatter and no specodelic tables | discovery output is byte-identical to pre-change behavior |
| P-prose | unit | [[spec.C-prose-untouched]] | arbitrary prose inserted between any sections of a dual-format file | the discovered scenario set is unchanged by the inserted prose |

## Purpose

Adopt the specodelic dual-format protocol as an opt-in authoring mode so a
single `spec.md` can be machine-linted by both tools, with scenario discovery
guaranteed to count each logical scenario exactly once. This modifies the
deployed Scenario Discovery requirement: slug collisions now mean genuinely
distinct scenarios (same id, different bodies), never section-sync mirrors.

## MODIFIED Requirements

### Requirement: Scenario Discovery
The system SHALL discover OpenSpec scenarios from `#### Scenario:` headings in `spec.md` files, including dual-format files that additionally carry the specodelic four-layer grammar, deduplicating section-sync mirrors and ignoring structured layers.

#### Scenario: Discover deployed scenario
- **GIVEN** `openspec/specs/compiler/spec.md` contains `#### Scenario: Empty input rejected`
- **WHEN** `ah check` scans deployed specs
- **THEN** it discovers a scenario with id `empty-input-rejected`
- **AND** associates it with the `compiler` spec

#### Scenario: Reject duplicate scenario ids
- **GIVEN** two scenarios in the same spec slugify to the same id **and have different bodies**
- **WHEN** `ah check` validates the spec
- **THEN** it emits a structural finding for the slug collision
- **AND** exits non-zero

#### Scenario: Deduplicate mirrored delta sections
- **GIVEN** a change delta carries identical `#### Scenario:` headings with identical bodies in both `## ADDED Requirements` and the mirrored `## Requirements` section
- **WHEN** a user runs `ah check --changes <change-id>`
- **THEN** each mirrored scenario is discovered exactly once
- **AND** the gate emits no slug-collision finding for the mirror

#### Scenario: Parse dual-format deployed spec
- **GIVEN** a deployed spec under `openspec/specs/` carries YAML frontmatter and specodelic tables in addition to its `## Requirements` section
- **WHEN** `ah check` scans deployed specs
- **THEN** scenarios are discovered from the `#### Scenario:` headings exactly as before
- **AND** frontmatter, constraint rows, model states and transitions, and property rows contribute no scenarios

#### Scenario: Plain openspec specs remain valid
- **GIVEN** a spec file contains no frontmatter and no specodelic tables
- **WHEN** `ah check` scans the spec
- **THEN** discovery behaves identically to before this change

## Requirements

### Requirement: Scenario Discovery
The system SHALL discover OpenSpec scenarios from `#### Scenario:` headings in `spec.md` files, including dual-format files that additionally carry the specodelic four-layer grammar, deduplicating section-sync mirrors and ignoring structured layers.

#### Scenario: Discover deployed scenario
- **GIVEN** `openspec/specs/compiler/spec.md` contains `#### Scenario: Empty input rejected`
- **WHEN** `ah check` scans deployed specs
- **THEN** it discovers a scenario with id `empty-input-rejected`
- **AND** associates it with the `compiler` spec

#### Scenario: Reject duplicate scenario ids
- **GIVEN** two scenarios in the same spec slugify to the same id **and have different bodies**
- **WHEN** `ah check` validates the spec
- **THEN** it emits a structural finding for the slug collision
- **AND** exits non-zero

#### Scenario: Deduplicate mirrored delta sections
- **GIVEN** a change delta carries identical `#### Scenario:` headings with identical bodies in both `## ADDED Requirements` and the mirrored `## Requirements` section
- **WHEN** a user runs `ah check --changes <change-id>`
- **THEN** each mirrored scenario is discovered exactly once
- **AND** the gate emits no slug-collision finding for the mirror

#### Scenario: Parse dual-format deployed spec
- **GIVEN** a deployed spec under `openspec/specs/` carries YAML frontmatter and specodelic tables in addition to its `## Requirements` section
- **WHEN** `ah check` scans deployed specs
- **THEN** scenarios are discovered from the `#### Scenario:` headings exactly as before
- **AND** frontmatter, constraint rows, model states and transitions, and property rows contribute no scenarios

#### Scenario: Plain openspec specs remain valid
- **GIVEN** a spec file contains no frontmatter and no specodelic tables
- **WHEN** `ah check` scans the spec
- **THEN** discovery behaves identically to before this change
