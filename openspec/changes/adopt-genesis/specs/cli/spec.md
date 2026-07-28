# cli spec delta: adopt genesis

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