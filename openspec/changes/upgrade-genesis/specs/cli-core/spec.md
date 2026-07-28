# cli-core spec delta: adopt genesis::guide

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