---
id: spec
kind: intent
statement: "WHEN espectacular adopts genesis THE cli surface SHALL wrap check JSON in the shared genesis envelope, source init managed-block injection from genesis, and provide a feedback subcommand that files issues via gh, with the report verb unchanged."
---

# cli spec delta: adopt genesis

## Constraints

| id | kind | expr | traces_to |
|----|------|------|-----------|
| C-envelope-shape | invariant | `ah check --json` emits top-level keys `ok`, `envelope_version`, `cli_version`, `envelope_kind`, `data`, `warnings`, `hints`, `meta`, with `findings` and `summary` nested under `data` | |
| C-init-genesis-injector | invariant | `ah init` injects managed blocks via `genesis::managed_block` and no local injector code remains | |
| C-feedback-subcommand | invariant | `ah feedback bug --from-last-error --yes` reads its own error scratch, assembles and redacts the body via `genesis::feedback`, and invokes `gh issue create` against the `Cargo.toml` `repository` with labels `agent-reported`, `bug`, `has-repro` | |
| C-report-verb-stable | advisory | the `report` verb is not repurposed: it renders the coverage matrix as before | |

## Model

### States

- `pre-genesis`
- `genesis-wired`
- `reported`

### Transitions

| id | from | to | guard |
|----|------|----|-------|
| t-envelope | pre-genesis | genesis-wired | [[spec.C-envelope-shape]] |
| t-init | genesis-wired | genesis-wired | [[spec.C-init-genesis-injector]] |
| t-feedback | genesis-wired | reported | [[spec.C-feedback-subcommand]] |
| t-report-stable | reported | reported | [[spec.C-report-verb-stable]] |

## Properties

| id | kind | derives_from | generator | predicate |
|----|------|--------------|-----------|-----------|
| P-envelope | unit | [[spec.C-envelope-shape]] | run `ah check --json` on any spec corpus | top-level keys are exactly the eight envelope keys and `findings`/`summary` appear only under `data` |
| P-init | unit | [[spec.C-init-genesis-injector]] | run `ah init` in a fixture project | managed blocks injected via `genesis::managed_block`; no local injector code remains in the repo |
| P-feedback | unit | [[spec.C-feedback-subcommand]] | run `ah feedback bug --from-last-error --yes` after a non-zero exit | `gh issue create` invoked against the `Cargo.toml` repository with labels `agent-reported`, `bug`, `has-repro` and a redacted body |
| P-report | unit | [[spec.C-report-verb-stable]] | run `ah report` | the coverage matrix renders exactly as before the change |

## Purpose

Adopt genesis as the shared implementation substrate for the cli surface:
JSON envelope shape, managed-block injection, and issue filing come from
genesis, so espectacular's CLI contract matches the rest of the charly
toolchain while its `report` verb keeps its coverage-matrix meaning.

## MODIFIED Requirements

### Requirement: Correspondence Check Command

`ah check` JSON output SHALL wrap its payload in `genesis::envelope::Envelope`, mapping `findings` and `summary` under `data`, so espectacular's JSON shape matches wai/dont/pretender/testaruda across the suite.

#### Scenario: check emits shared envelope

- **WHEN** `ah check --json` is run after adopting genesis
- **THEN** the emitted JSON SHALL have top-level keys `ok`, `envelope_version`, `cli_version`, `envelope_kind`, `data`, `warnings`, `hints`, `meta`
- **AND** the existing `findings`/`summary` fields SHALL be nested under `data`.

### Requirement: Project Initialization

`ah init` SHALL source its managed-block injector mechanics from `genesis::managed_block`, while retaining espectacular's block content.

#### Scenario: init injects managed blocks via genesis

- **WHEN** `ah init` is run after adopting genesis
- **THEN** the `<!-- …:START -->`/`# ah:managed:start` blocks SHALL be injected via `genesis::managed_block`
- **AND** no local injector code SHALL remain.

## ADDED Requirements

### Requirement: feedback subcommand

espectacular SHALL provide a `feedback` subcommand that files a structured issue against espectacular's upstream repo via `gh`, wrapping `genesis::feedback`. The `report` verb is unchanged and keeps its "coverage matrix" meaning.

#### Scenario: agent files a bug with last error

- **WHEN** `ah feedback bug --from-last-error --yes` is run after a non-zero exit
- **THEN** espectacular SHALL read its own error scratch
- **AND** SHALL assemble and redact the body via `genesis::feedback`
- **AND** SHALL invoke `gh issue create` against espectacular's `Cargo.toml` `repository` with labels `agent-reported`, `bug`, `has-repro`.

#### Scenario: report verb is unchanged

- **WHEN** `ah report` is run
- **THEN** it SHALL render the coverage matrix as before (the `report` verb is NOT repurposed for issue filing).
## Requirements

### Requirement: feedback subcommand

espectacular SHALL provide a `feedback` subcommand that files a structured issue against espectacular's upstream repo via `gh`, wrapping `genesis::feedback`. The `report` verb is unchanged and keeps its "coverage matrix" meaning.

#### Scenario: agent files a bug with last error

- **WHEN** `ah feedback bug --from-last-error --yes` is run after a non-zero exit
- **THEN** espectacular SHALL read its own error scratch
- **AND** SHALL assemble and redact the body via `genesis::feedback`
- **AND** SHALL invoke `gh issue create` against espectacular's `Cargo.toml` `repository` with labels `agent-reported`, `bug`, `has-repro`.

#### Scenario: report verb is unchanged

- **WHEN** `ah report` is run
- **THEN** it SHALL render the coverage matrix as before (the `report` verb is NOT repurposed for issue filing).
