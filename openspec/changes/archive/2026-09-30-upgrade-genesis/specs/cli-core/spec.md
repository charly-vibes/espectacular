---
id: spec
kind: intent
statement: "WHEN the tool adopts genesis::guide THE CLI scaffold SHALL be built via Guide::builder, with command handlers returning Output or using ErrorSink and errors surfaced with suggestion footers."
---

# cli-core spec delta: adopt genesis::guide

## Constraints

| id | kind | expr | traces_to |
|----|------|----|-----------|
| C-guide-builder | invariant | the tool's `main.rs` CLI setup uses `genesis::guide::Guide::builder()` when the tool adopts `genesis::guide` (adoption is the maintainer's decision — this spec fixes the contract, not the timing), with `cargo test` passing | [[spec]] |
| C-output-handlers | advisory | command handlers SHOULD return `Output<T>` or use `ErrorSink` | [[spec]] |
| C-errorsink-selfheal | advisory | on a command error, `ErrorSink` SHOULD print the error with a suggestion footer and write to the error scratch (for `--from-last-error`) | [[spec]] |

## Model

### States

- `ad-hoc-cli`
- `guide-built`

### Transitions

| id | from | to | guard |
|----|------|----|-------|
| t-build | ad-hoc-cli | guide-built | [[spec.C-guide-builder]] |

## Properties

| id | kind | derives_from | generator | predicate |
|----|------|--------------|-----------|-----------|
| P-guide-builder | unit | [[spec.C-guide-builder]] | the adopted `main.rs` | CLI setup is constructed via `Guide::builder()` and tests pass |
| P-handlers | unit | [[spec.C-output-handlers]] | each command handler | handlers return `Output<T>` or use `ErrorSink` where the advisory is honored |
| P-selfheal | unit | [[spec.C-errorsink-selfheal]] | a failing command run | the error prints with a suggestion footer and lands in the error scratch readable by `--from-last-error` |

## Purpose

Adopt `genesis::guide` for the CLI scaffold: `Guide::builder()` replaces
ad-hoc setup, handlers standardize on `Output<T>`/`ErrorSink`, and errors
become self-healing via suggestion footers and the error scratch.

## ADDED Requirements

### Requirement: CLI scaffold uses Guide

The tool's `main.rs` CLI setup SHALL use `genesis::guide::Guide::builder()`
when the tool adopts `genesis::guide`. Adoption is determined by the tool's
maintainer — this spec defines the contract for how to do it.

#### Scenario: Guide builder replaces ad-hoc setup

- **GIVEN** the tool adopts `genesis::guide`
- **WHEN** `main.rs` is updated to use `Guide::builder(...)`
- **THEN** command handlers SHOULD return `Output<T>` or use `ErrorSink`
- **AND** `cargo test` SHALL pass

#### Scenario: ErrorSink for self-healing errors

- **GIVEN** the tool adopts `genesis::guide`
- **WHEN** a command returns an error
- **THEN** `ErrorSink` SHOULD print the error with a suggestion footer
- **AND** it SHOULD write to the error scratch (for `--from-last-error`)
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

#### Scenario: ErrorSink for self-healing errors

- **GIVEN** the tool adopts `genesis::guide`
- **WHEN** a command returns an error
- **THEN** `ErrorSink` SHOULD print the error with a suggestion footer
- **AND** it SHOULD write to the error scratch (for `--from-last-error`)