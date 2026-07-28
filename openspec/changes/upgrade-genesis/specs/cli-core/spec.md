# cli-core spec delta: adopt genesis::guide

## MODIFIED Requirements

### Requirement: CLI scaffold uses Guide

The tool's `main.rs` CLI setup SHALL use `genesis::guide::Guide::builder()`
for a coherent CLI scaffold.

#### Scenario: Guide builder replaces ad-hoc setup

- **GIVEN** the tool has a `main.rs` with CLI setup
- **WHEN** it adopts `genesis::guide`
- **THEN** `main.rs` SHALL use `Guide::builder(...)` to set up the CLI
- **AND** command handlers SHALL return `Output<T>` or use `ErrorSink`
- **AND** `cargo test` SHALL pass

#### Scenario: ErrorSink for self-healing errors

- **GIVEN** the tool adopts `genesis::guide`
- **WHEN** a command returns an error
- **THEN** `ErrorSink` SHALL print the error with a suggestion footer
- **AND** it SHALL write to the error scratch (for `--from-last-error`)