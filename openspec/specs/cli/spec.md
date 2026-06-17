# cli Specification

## Purpose
TBD - created by archiving change add-spec-assertions. Update Purpose after archive.
## Requirements
### Requirement: CLI Command Name
The system SHALL expose the standalone command-line interface as `ah`.

#### Scenario: Invoke help
- **WHEN** a user runs `ah --help`
- **THEN** the CLI displays available `ah` commands

### Requirement: Project Initialization
The system SHALL provide an idempotent `ah init` command that prepares a repository for spec-test correspondence checks.

#### Scenario: Initialize project files
- **GIVEN** a repository contains an `openspec/` directory
- **WHEN** a user runs `ah init`
- **THEN** the command creates `.espectacular/config.toml` when it is missing
- **AND** writes `.espectacular/AGENTS.md`
- **AND** creates top-level `AGENTS.md` and `CLAUDE.md` when they are absent
- **AND** refreshes managed `ah` blocks in top-level instruction files

#### Scenario: Refuse initialization without OpenSpec
- **GIVEN** a repository does not contain an `openspec/` directory
- **WHEN** a user runs `ah init`
- **THEN** the command fails without creating `.espectacular/`

#### Scenario: Stub existing deployed scenarios
- **GIVEN** deployed OpenSpec scenarios exist under `openspec/specs/`
- **WHEN** a user runs `ah init`
- **THEN** the command creates matching `.espectacular/<spec>/<scenario>.toml` stubs for scenarios without contracts
- **AND** the stubs declare no tests until the user or AI fills them in

#### Scenario: Install supported pre-commit integration
- **GIVEN** the repository uses `lefthook`
- **WHEN** a user runs `ah init`
- **THEN** the command installs or refreshes a managed pre-commit integration that runs `ah check`

#### Scenario: Prefer lefthook before prek
- **GIVEN** the repository has both `lefthook` and `prek` configured
- **WHEN** a user runs `ah init`
- **THEN** the command installs the managed pre-commit integration through `lefthook`

#### Scenario: Fall back to prek
- **GIVEN** the repository uses `prek`
- **AND** does not use `lefthook`
- **WHEN** a user runs `ah init`
- **THEN** the command installs or refreshes a managed pre-commit integration through `prek`

#### Scenario: Report missing hook framework
- **GIVEN** the repository does not use `lefthook` or `prek`
- **WHEN** a user runs `ah init`
- **THEN** the command reports a concern that the user or AI must set up pre-commit integration
- **AND** does not write a raw `.git/hooks/pre-commit` fallback

### Requirement: Correspondence Check Command
The system SHALL provide `ah check` as the deterministic gate command.

#### Scenario: Check deployed specs
- **WHEN** a user runs `ah check`
- **THEN** the command validates deployed specs under `openspec/specs/`
- **AND** validates matching contracts under `.espectacular/<spec>/`
- **AND** emits JSON output

#### Scenario: Check an OpenSpec change overlay
- **WHEN** a user runs `ah check --changes add-parser`
- **THEN** the command validates deployed specs plus the `add-parser` change overlay
- **AND** validates staged contracts under `.espectacular/changes/add-parser/`
- **AND** includes the selected change in the JSON scope

### Requirement: Health Check Command
The system SHALL provide `ah doctor` for installation health checks.

#### Scenario: Diagnose project setup
- **WHEN** a user runs `ah doctor`
- **THEN** the command validates `.espectacular/config.toml`
- **AND** checks managed instruction blocks
- **AND** checks supported hook integration
- **AND** reports tool-version compatibility concerns

#### Scenario: Diagnose correspondence wiring
- **WHEN** a user runs `ah doctor`
- **THEN** the command reports slug collisions as errors
- **AND** reports orphan contracts as errors
- **AND** reports unknown archetype names as warnings

### Requirement: Archetype Documentation Commands
The system SHALL expose built-in archetype guidance through `ah type` commands.

#### Scenario: List archetypes
- **WHEN** a user runs `ah type`
- **THEN** the command lists all known archetypes with one-line descriptions
- **AND** includes `PF`, `SA`, `BP`, `CE`, and `NR`

#### Scenario: Show archetype details
- **WHEN** a user runs `ah type PF`
- **THEN** the command prints the full built-in documentation for the `PF` archetype

### Requirement: Scenario Lifecycle Commands
The system SHALL provide commands for append-only scenario authoring.

#### Scenario: Create scenario in a change
- **GIVEN** `openspec/changes/add-parser/specs/compiler/spec.md` contains `### Requirement: Parser Input Validation`
- **WHEN** a user runs `ah scenario new add-parser compiler --requirement "Parser Input Validation" "Empty input rejected"`
- **THEN** the command appends a scenario heading under that requirement
- **AND** writes placeholder `WHEN` and `THEN` lines under the scenario heading
- **AND** creates `.espectacular/changes/add-parser/compiler/empty-input-rejected.toml` with `id`, empty `description`, empty `archetype`, `status = "active"`, empty `superseded_by`, and `authored_with`

#### Scenario: Reject scenario creation without target requirement
- **GIVEN** `openspec/changes/add-parser/specs/compiler/spec.md` does not contain `### Requirement: Parser Input Validation`
- **WHEN** a user runs `ah scenario new add-parser compiler --requirement "Parser Input Validation" "Empty input rejected"`
- **THEN** the command fails without creating or modifying files

#### Scenario: Supersede a scenario
- **GIVEN** scenario `new-behavior` exists in the deployed-plus-`add-parser` overlay for spec `compiler`
- **WHEN** a user runs `ah scenario supersede compiler old-behavior --with=new-behavior --in-change=add-parser`
- **THEN** the command stages `.espectacular/changes/add-parser/compiler/old-behavior.toml`
- **AND** marks the staged contract as superseded
- **AND** records `new-behavior` as the replacement scenario id

#### Scenario: Reject supersession with missing replacement
- **GIVEN** scenario `new-behavior` does not exist in the deployed-plus-`add-parser` overlay for spec `compiler`
- **WHEN** a user runs `ah scenario supersede compiler old-behavior --with=new-behavior --in-change=add-parser`
- **THEN** the command fails without creating or modifying files

### Requirement: Archive Companion Command
The system SHALL provide `ah archive <change>` to move staged scenario contracts after OpenSpec archive.

#### Scenario: Archive staged contracts
- **GIVEN** `openspec archive add-parser` has applied the OpenSpec change
- **AND** every staged contract id exists in deployed `openspec/specs/`
- **WHEN** a user runs `ah archive add-parser`
- **THEN** the command moves new contracts from `.espectacular/changes/add-parser/<spec>/` to `.espectacular/<spec>/`

#### Scenario: Refuse archive before OpenSpec archive
- **GIVEN** `.espectacular/changes/add-parser/compiler/new-behavior.toml` exists
- **AND** deployed `openspec/specs/compiler/spec.md` does not contain scenario `new-behavior`
- **WHEN** a user runs `ah archive add-parser`
- **THEN** the command fails without moving staged contracts

#### Scenario: Refuse archive collision
- **GIVEN** `.espectacular/compiler/foo.toml` already exists
- **AND** `.espectacular/changes/add-parser/compiler/foo.toml` is not a superseded metadata update for `foo`
- **WHEN** a user runs `ah archive add-parser`
- **THEN** the command fails without overwriting the deployed contract

### Requirement: Upgrade Command
The system SHALL provide `ah upgrade` to make tool-version drift explicit.

#### Scenario: Report compatibility changes
- **GIVEN** `.espectacular/config.toml` pins an older tool version than the installed `ah`
- **WHEN** a user runs `ah upgrade`
- **THEN** the command reports config schema version changes, execution default changes, archetype additions, and archetype deprecations before updating the configured tool version
- **AND** does not rewrite existing scenario contract `authored_with` values

### Requirement: Doctor enable flag
When a user runs `ah doctor --enable <capability>` for a detected inactive capability, the system SHALL write exactly one config table for that capability and SHALL print the path and table name written.

#### Scenario: Enable pytest adapter
- **GIVEN** pytest is detected by `ah doctor`
- **WHEN** a user runs `ah doctor --enable pytest`
- **THEN** the command writes `[runners.pytest] command = ["pytest"]` to `.espectacular/config.toml`
- **AND** prints `.espectacular/config.toml` and `[runners.pytest]`

#### Scenario: Enable cargo adapter
- **GIVEN** cargo is detected by `ah doctor`
- **WHEN** a user runs `ah doctor --enable cargo`
- **THEN** the command writes `[runners.cargo] command = ["cargo", "test"]` to `.espectacular/config.toml`
- **AND** prints `.espectacular/config.toml` and `[runners.cargo]`

#### Scenario: Enable vitest adapter
- **GIVEN** vitest is detected by `ah doctor`
- **WHEN** a user runs `ah doctor --enable vitest`
- **THEN** the command writes `[runners.vitest] command = ["vitest", "run"]` to `.espectacular/config.toml`
- **AND** prints `.espectacular/config.toml` and `[runners.vitest]`

#### Scenario: Enable mutation capability
- **GIVEN** a mutation testing tool is detected
- **WHEN** a user runs `ah doctor --enable mutation`
- **THEN** the command writes `[capabilities.mutation] enabled = true` to `.espectacular/config.toml`
- **AND** prints `.espectacular/config.toml` and `[capabilities.mutation]`

#### Scenario: Enable property capability
- **GIVEN** a property-based testing framework is detected
- **WHEN** a user runs `ah doctor --enable property`
- **THEN** the command writes `[capabilities.property] enabled = true` to `.espectacular/config.toml`
- **AND** prints `.espectacular/config.toml` and `[capabilities.property]`

#### Scenario: Enable snapshot capability
- **GIVEN** a snapshot testing framework is detected
- **WHEN** a user runs `ah doctor --enable snapshot`
- **THEN** the command writes `[capabilities.snapshot] enabled = true` to `.espectacular/config.toml`
- **AND** prints `.espectacular/config.toml` and `[capabilities.snapshot]`

#### Scenario: Enable unknown capability is an error
- **WHEN** a user runs `ah doctor --enable nonexistent`
- **THEN** the command exits non-zero
- **AND** prints `unrecognized capability: nonexistent`

#### Scenario: Enable already-active capability is a no-op
- **GIVEN** a capability is already present in `.espectacular/config.toml`
- **WHEN** a user runs `ah doctor --enable <capability>`
- **THEN** the command reports it is already enabled and makes no changes

### Requirement: Explain subcommand
The system SHALL provide an `ah explain <topic>` subcommand that prints playbook guidance for a finding kind or suggested action.

#### Scenario: Explain a finding kind
- **WHEN** a user runs `ah explain no-toml`
- **THEN** the command prints markdown guidance for the `no-toml` finding kind

#### Scenario: Explain a suggested action
- **WHEN** a user runs `ah explain run_ah_scenario_new`
- **THEN** the command prints markdown guidance for the `run_ah_scenario_new` suggested action

#### Scenario: Explain a general topic
- **WHEN** a user runs `ah explain workflow`
- **THEN** the command prints markdown guidance for the general `workflow` topic

#### Scenario: Explain with JSON output
- **WHEN** a user runs `ah explain no-toml --json`
- **THEN** the command emits a JSON object with fields: `topic`, `summary`, `when`, `do`, `human_approval`, `related_topics`, `hints`
- **AND** each `hints` item contains `kind` and `message` string fields

#### Scenario: List all topics
- **WHEN** a user runs `ah explain --list`
- **THEN** the command prints all available topic identifiers, one per line

#### Scenario: Unknown topic is an error
- **WHEN** a user runs `ah explain no-such-topic`
- **THEN** the command exits non-zero
- **AND** prints either `Run ah explain --list` or the sorted list of available topic identifiers

### Requirement: Coverage report command
The system SHALL provide `ah report` to display a conformance coverage matrix across all deployed specs and archetype tiers, modeled on the OpenTelemetry per-language compliance matrix pattern.

#### Scenario: Report coverage by spec and archetype
- **WHEN** a user runs `ah report`
- **THEN** the command prints a table showing each spec as a row and each archetype as a column
- **AND** each cell shows covered/missing/failing counts

#### Scenario: Report exits zero when coverage is complete
- **GIVEN** every deployed scenario has a valid, passing contract
- **WHEN** a user runs `ah report`
- **THEN** the command exits zero

#### Scenario: Report exits non-zero when scenarios are missing contracts
- **GIVEN** at least one deployed scenario has no sidecar contract
- **WHEN** a user runs `ah report`
- **THEN** the command exits non-zero

#### Scenario: Report JSON output
- **WHEN** a user runs `ah report --json`
- **THEN** the command emits a JSON conformance matrix consumable by CI dashboards and agent harnesses

### Requirement: Recommendation findings
The system SHALL emit `recommendation` findings when `ah doctor` detects capabilities that are available but not yet configured.

#### Scenario: Recommendation finding carries enable command
- **GIVEN** `ah doctor` detects an available framework not yet configured
- **WHEN** the output is inspected (JSON or text)
- **THEN** a `recommendation` finding is present with `suggested_action = enable_capability`
- **AND** `apply_command` contains the `ah doctor --enable <capability>` invocation

#### Scenario: Recommendation finding is a finding kind, not a log line
- **GIVEN** `ah doctor` detects an available but unconfigured framework
- **WHEN** the output is requested as JSON (`ah doctor --json`)
- **THEN** the finding appears in the `findings` array with `kind = recommendation`
- **AND** it carries a `playbook_command` field

