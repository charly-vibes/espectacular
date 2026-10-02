---
id: spec
kind: intent
statement: "WHEN a user runs ah sync THE cli SHALL create or refresh property-derived contracts for the current scope, SHALL exit non-zero when spk lint fails or spk is unavailable, and SHALL support a check mode that writes nothing and fails on drift."
---

# sync command

Property-derived contracts need a staging command: `ah sync` bridges the
specodelic half (Properties rows, read via `spk parse --json`) and the gate
(contract TOMLs). The command refuses to derive from a file whose `spk lint`
is not clean or whose `spk` binary is missing, because derived contracts are
only as trustworthy as the lint-clean spec they came from.

This registers the subcommand in the `Command` enum per the cli-core
conventions (there is no `src/cli/` module — the CLI lives in `src/main.rs`).

## Constraints

| id | kind | expr | traces_to |
|----|------|------|-----------|
| C-sync-refusal | invariant | ah sync exits non-zero without writing when spk lint fails on any target file or when the spk binary is missing, reporting spk-unavailable for the latter | [[spec]] |
| C-sync-check | invariant | ah sync --check writes nothing, exits zero when all derived contracts exist and are fresh, and exits non-zero on drift or missing contracts | [[spec]] |
| C-sync-registration | invariant | ah sync is registered in the Command enum with help text and completions like every other subcommand | [[spec]] |
| C-sync-scope | advisory | without an explicit scope, ah sync operates on the same default scope as ah check | [[spec]] |

## Model

### States

- `linting`
- `parsing`
- `written`
- `checked`

### Transitions

| id | from | to | guard |
|----|------|----|-------|
| t-refuse | linting | linting | [[spec.C-sync-refusal]] |
| t-parse | linting | parsing | [[spec.C-sync-refusal]] |
| t-write | parsing | written | [[spec.C-sync-registration]] |
| t-check | parsing | checked | [[spec.C-sync-check]] |

## Properties

| id | kind | derives_from | generator | predicate |
|----|------|--------------|-----------|-----------|
| P-refuse | unit | [[spec.C-sync-refusal]] | a scope containing a lint-dirty dual-format file, and a scope with no spk binary on PATH | sync exits non-zero with a reason in both cases and writes nothing |
| P-check | unit | [[spec.C-sync-check]] | a scope with fresh contracts, drifted contracts, and missing contracts | check mode exits zero, non-zero, and non-zero respectively; no file is modified in any case |
| P-register | unit | [[spec.C-sync-registration]] | ah --help and shell completion generation | sync appears with help text and completes like other subcommands |
| P-scope | unit | [[spec.C-sync-scope]] | sync invoked with and without an explicit scope | the default scope matches ah check's default scope |

## Purpose

Give adopters a single command that turns lint-clean Properties rows into
gate-readable contracts, safely refreshable in CI via `--check`.

## ADDED Requirements

### Requirement: Sync Command
`ah sync` SHALL create or refresh property-derived contracts for the current scope, SHALL exit non-zero without writing when `spk lint` fails on any target file or when `spk` is unavailable, and SHALL support `--check`, which writes nothing and exits non-zero on drift or missing contracts.

#### Scenario: Sync creates derived contracts
- **GIVEN** the default scope contains a lint-clean dual-format spec with Properties rows
- **WHEN** `ah sync` runs
- **THEN** a derived contract exists per property row with `derived_from` recorded

#### Scenario: Sync refuses on lint failure
- **GIVEN** the scope contains a dual-format spec that fails `spk lint`
- **WHEN** `ah sync` runs
- **THEN** it exits non-zero, names the offending file, and writes nothing

#### Scenario: Missing spk reports spk-unavailable
- **GIVEN** `spk` is not on PATH
- **WHEN** `ah sync` runs
- **THEN** it exits non-zero with an `spk-unavailable` reason

#### Scenario: Check mode is CI-safe
- **GIVEN** a scope with one drifted and one missing derived contract
- **WHEN** `ah sync --check` runs
- **THEN** it exits non-zero, reports both, and modifies no files

## Requirements

### Requirement: Sync Command
`ah sync` SHALL create or refresh property-derived contracts for the current scope, SHALL exit non-zero without writing when `spk lint` fails on any target file or when `spk` is unavailable, and SHALL support `--check`, which writes nothing and exits non-zero on drift or missing contracts.

#### Scenario: Sync creates derived contracts
- **GIVEN** the default scope contains a lint-clean dual-format spec with Properties rows
- **WHEN** `ah sync` runs
- **THEN** a derived contract exists per property row with `derived_from` recorded

#### Scenario: Sync refuses on lint failure
- **GIVEN** the scope contains a dual-format spec that fails `spk lint`
- **WHEN** `ah sync` runs
- **THEN** it exits non-zero, names the offending file, and writes nothing

#### Scenario: Missing spk reports spk-unavailable
- **GIVEN** `spk` is not on PATH
- **WHEN** `ah sync` runs
- **THEN** it exits non-zero with an `spk-unavailable` reason

#### Scenario: Check mode is CI-safe
- **GIVEN** a scope with one drifted and one missing derived contract
- **WHEN** `ah sync --check` runs
- **THEN** it exits non-zero, reports both, and modifies no files