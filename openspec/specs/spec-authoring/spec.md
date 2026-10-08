---
id: spec.authoring
kind: intent
statement: "WHEN any spec file in this repository is added, edited, or deployed by archive THE corpus SHALL stay lint-clean under specodelic dual format, with the mandate enforced in the repo hook chain and CI and the specodelic half re-derived from the archived delta whenever archive strips it."

---

# spec-authoring corpus mandate

Dogfooding change: every spec file in this repository — deployed capability
specs and change deltas alike — carries the specodelic four-layer grammar in
addition to its openspec grammar. This is repo practice, not product behavior:
espectacular remains opt-in for adopters (`C-opt-in`, deployed gate spec), and
this mandate never reaches `ah check`'s product logic.

This delta is itself authored in dual format, extending the pilot from
`adopt-dual-format-specs` to the whole corpus.

## Constraints

| id | kind | expr | traces_to |
|----|------|------|-----------|
| C-corpus-dual | invariant | every `spec.md` under `openspec/specs/` and under active `openspec/changes/*/specs/` carries YAML frontmatter (`id: spec`, `kind: intent`, EARS statement) and the Constraints, Model, and Properties tables | [[spec.authoring]] |
| C-lint-clean | invariant | `spk lint` exits zero over the corpus, and the repo hook chain plus CI run it on every change to a spec file | [[spec.authoring]] |
| C-open-half-stable | invariant | migrating a file leaves its openspec requirement and scenario text textually identical (section-sync mirror and `ah check` stay green) | [[spec.authoring]] |
| C-adopters-unaffected | advisory | no espectacular product behavior requires dual format; plain openspec repos remain fully valid | [[spec.authoring]] |
| C-archive-rederive | invariant | when `openspec archive` merges new requirements into a deployed spec and strips its specodelic half, the half is re-derived — frontmatter copied from the archived delta, surviving constraint/property rows carried over from the pre-archive version, rows added for newly merged requirements — and the gates re-run | [[spec.authoring]] |

## Model

### States

- `plain`
- `dual`
- `linted`

### Transitions

| id | from | to | guard |
|----|------|----|-------|
| t-migrate | plain | dual | [[spec.authoring.C-corpus-dual]] |
| t-lint | dual | linted | [[spec.authoring.C-lint-clean]] |
| t-stable | linted | linted | [[spec.authoring.C-open-half-stable]] |
| t-strip | linted | dual | [[spec.authoring.C-archive-rederive]] |

## Properties

| id | kind | derives_from | generator | predicate |
|----|------|--------------|-----------|-----------|
| P-corpus | unit | [[spec.authoring.C-corpus-dual]] | any `spec.md` under `openspec/` | the file parses as dual format: frontmatter plus all three tables present |
| P-lint | unit | [[spec.authoring.C-lint-clean]] | the full corpus after any edit to a spec file | `spk lint` exits zero |
| P-open-half | unit | [[spec.authoring.C-open-half-stable]] | the openspec half of a migrated file diffed against its pre-migration git blob | the diff is empty outside frontmatter and the three tables |
| P-adopter | unit | [[spec.authoring.C-adopters-unaffected]] | a plain openspec spec file with no frontmatter | `ah check` discovers scenarios and imposes no dual-format finding |
| P-rederive | unit | [[spec.authoring.C-archive-rederive]] | a deployed spec whose specodelic half was stripped by an archive that also merged new requirements | after re-derivation, constraint and property rows cover both pre-existing and newly merged requirements, and `spk lint` exits zero |

## Purpose

Make the entire spec corpus of this repository dual-format and machine-linted,
flipping this repo's own enforcement from opt-in to mandated while preserving
espectacular's opt-in promise to adopters.

## Non-Goals

- Making any espectacular product behavior depend on dual format — adopters' plain openspec repos remain fully valid (`C-adopters-unaffected`)
- Automating archive-time re-derivation inside the tool: re-deriving the stripped specodelic half is a contributor step after `openspec archive`, not tool behavior
- Retroactively migrating archived change deltas — the mandate covers deployed specs and active changes only
- Extending the specodelic grammar itself — upstream concern (specodelic)

## Requirements

### Requirement: Dual-Format Spec Corpus
The repository SHALL keep every spec file under `openspec/` in specodelic dual format, lint-clean under `spk lint`, with the mandate enforced by the repo hook chain and CI, and SHALL re-derive the specodelic half after `openspec archive` merges new requirements into a deployed spec and strips it.

#### Scenario: Plain spec file rejected
- **GIVEN** a `spec.md` under `openspec/` carries no YAML frontmatter and no specodelic tables
- **WHEN** the repo hook chain or CI lints the corpus
- **THEN** `spk lint` emits a finding for the file
- **AND** the change cannot be committed
- **VERIFIES** [[spec.authoring.P-adopter]]

#### Scenario: Migration keeps the openspec half identical
- **GIVEN** a spec file is migrated to dual format
- **WHEN** its openspec requirement and scenario text is diffed against the pre-migration version
- **THEN** the diff shows no change outside the frontmatter and the Constraints, Model, and Properties tables
- **AND** `ah check` output is unchanged
- **VERIFIES** [[spec.authoring.P-open-half]]

#### Scenario: Archive strip is re-derived
- **GIVEN** `openspec archive` merged a delta's requirements into a deployed spec and stripped its specodelic half
- **WHEN** the contributor re-derives the half — frontmatter from the archived delta, surviving constraint and property rows from the pre-archive version, rows added for the newly merged requirements — and re-runs the gates
- **THEN** `spk lint` exits zero on the deployed spec again
- **AND** the constraints and properties cover both pre-existing and newly merged requirements
- **AND** the corpus-wide lint stays green
- **VERIFIES** [[spec.authoring.P-rederive]]

#### Scenario: Corpus parses as dual format
- **GIVEN** any non-archived `spec.md` under `openspec/`
- **WHEN** the corpus gate (`just spec-lint`) runs
- **THEN** every file parses as dual format: frontmatter plus Constraints, Model, and Properties tables present
- **VERIFIES** [[spec.authoring.P-corpus]]

#### Scenario: Corpus is lint clean
- **GIVEN** the full spec corpus after any edit to a spec file
- **WHEN** `spk lint` runs over the non-archived corpus
- **THEN** it exits zero with no findings
- **VERIFIES** [[spec.authoring.P-lint]]
