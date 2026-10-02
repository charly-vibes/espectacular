---
id: spec
kind: intent
statement: "WHEN the tool adopts genesis::guide or a user runs ah sync THE CLI SHALL build the scaffold via Guide::builder, return Output or use ErrorSink with errors surfaced via suggestion footers, create or refresh property-derived contracts for the current scope, exit non-zero when spk lint fails or spk is unavailable, and support a check mode that writes nothing and fails on drift."

---

# cli-core spec delta: adopt genesis::guide

## Constraints

| id | kind | expr | traces_to |
|----|------|----|-----------|
| C-guide-builder | invariant | the tool's `main.rs` CLI setup uses `genesis::guide::Guide::builder()` when the tool adopts `genesis::guide` (adoption is the maintainer's decision — this spec fixes the contract, not the timing), with `cargo test` passing | [[spec]] |
| C-output-handlers | advisory | command handlers SHOULD return `Output<T>` or use `ErrorSink` | [[spec]] |
| C-errorsink-selfheal | advisory | on a command error, `ErrorSink` SHOULD print the error with a suggestion footer and write to the error scratch (for `--from-last-error`) | [[spec]] |
| C-sync-refusal | invariant | ah sync exits non-zero without writing when spk lint fails on any target file or when the spk binary is missing, reporting spk-unavailable for the latter | [[spec]] |
| C-sync-check | invariant | ah sync --check writes nothing, exits zero when all derived contracts exist and are fresh, and exits non-zero on drift or missing contracts | [[spec]] |
| C-sync-registration | invariant | ah sync is registered in the Command enum with help text and completions like every other subcommand | [[spec]] |
| C-sync-scope | advisory | without an explicit scope, ah sync operates on the same default scope as ah check | [[spec]] |

## Model

### States

- `ad-hoc-cli`
- `guide-built`
- `linting`
- `parsing`
- `written`
- `checked`

### Transitions

| id | from | to | guard |
|----|------|----|-------|
| t-build | ad-hoc-cli | guide-built | [[spec.C-guide-builder]] |
| t-refuse | linting | linting | [[spec.C-sync-refusal]] |
| t-parse | linting | parsing | [[spec.C-sync-refusal]] |
| t-write | parsing | written | [[spec.C-sync-registration]] |
| t-check | parsing | checked | [[spec.C-sync-check]] |

## Properties

| id | kind | derives_from | generator | predicate |
|----|------|--------------|-----------|-----------|
| P-guide-builder | unit | [[spec.C-guide-builder]] | the adopted `main.rs` | CLI setup is constructed via `Guide::builder()` and tests pass |
| P-handlers | unit | [[spec.C-output-handlers]] | each command handler | handlers return `Output<T>` or use `ErrorSink` where the advisory is honored |
| P-selfheal | unit | [[spec.C-errorsink-selfheal]] | a failing command run | the error prints with a suggestion footer and lands in the error scratch readable by `--from-last-error` |
| P-refuse | unit | [[spec.C-sync-refusal]] | a scope containing a lint-dirty dual-format file, and a scope with no spk binary on PATH | sync exits non-zero with a reason in both cases and writes nothing |
| P-check | unit | [[spec.C-sync-check]] | a scope with fresh contracts, drifted contracts, and missing contracts | check mode exits zero, non-zero, and non-zero respectively; no file is modified in any case |
| P-register | unit | [[spec.C-sync-registration]] | ah --help and shell completion generation | sync appears with help text and completes like other subcommands |
| P-scope | unit | [[spec.C-sync-scope]] | sync invoked with and without an explicit scope | the default scope matches ah check's default scope |

## Purpose
Adopt `genesis::guide` for the CLI scaffold: `Guide::builder()` replaces
ad-hoc setup, handlers standardize on `Output<T>`/`ErrorSink`, and errors
become self-healing via suggestion footers and the error scratch.

Give adopters a single command that turns lint-clean Properties rows into
gate-readable contracts, safely refreshable in CI via `--check`.
## Requirements
### Requirement: CLI scaffold uses Guide

The tool's `main.rs` CLI setup SHALL use `genesis::guide::Guide::builder()`
when the tool adopts `genesis::guide`. Adoption is determined by the tool's
maintainer — this spec defines the contract for how to do it.

#### Scenario: Guide builder replaces ad-hoc setup

- **GIVEN** the tool adopts `genesis::guide`
- **WHEN** `main.rs` is updated to use `Guide::builder(...)`
- **THEN** command handlers SHOULD return `Output<T>` or use `ErrorSink`
- **AND** `cargo test` SHALL pass
- **VERIFIES** [[spec.P-guide-builder]]

#### Scenario: ErrorSink for self-healing errors

- **GIVEN** the tool adopts `genesis::guide`
- **WHEN** a command returns an error
- **THEN** `ErrorSink` SHOULD print the error with a suggestion footer
- **AND** it SHOULD write to the error scratch (for `--from-last-error`)
- **VERIFIES** [[spec.P-handlers]]
- **VERIFIES** [[spec.P-selfheal]]

### Requirement: Sync Command
`ah sync` SHALL create or refresh property-derived contracts for the current scope, SHALL exit non-zero without writing when `spk lint` fails on any target file or when `spk` is unavailable, and SHALL support `--check`, which writes nothing and exits non-zero on drift or missing contracts.

#### Scenario: Sync creates derived contracts
- **GIVEN** the default scope contains a lint-clean dual-format spec with Properties rows
- **WHEN** `ah sync` runs
- **THEN** a derived contract exists per property row with `derived_from` recorded
- **VERIFIES** [[spec.P-scope]]

#### Scenario: Sync is registered like every subcommand
- **GIVEN** the tool's CLI is built
- **WHEN** a user runs `ah --help` or generates shell completions
- **THEN** `sync` appears with help text and completes like the other subcommands
- **VERIFIES** [[spec.P-register]]

#### Scenario: Sync refuses on lint failure
- **GIVEN** the scope contains a dual-format spec that fails `spk lint`
- **WHEN** `ah sync` runs
- **THEN** it exits non-zero, names the offending file, and writes nothing
- **VERIFIES** [[spec.P-refuse]]
#### Scenario: Missing spk reports spk-unavailable
- **GIVEN** `spk` is not on PATH
- **WHEN** `ah sync` runs
- **THEN** it exits non-zero with an `spk-unavailable` reason
- **VERIFIES** [[spec.P-refuse]]
#### Scenario: Check mode is CI-safe
- **GIVEN** a scope with one drifted and one missing derived contract
- **WHEN** `ah sync --check` runs
- **THEN** it exits non-zero, reports both, and modifies no files
- **VERIFIES** [[spec.P-check]]
