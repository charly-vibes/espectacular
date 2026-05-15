# Capability: cli

The standalone command-line interface for espectacular is exposed as `ah` and provides deterministic spec-test correspondence workflows for AI coding harnesses.

## MODIFIED Requirements

### Requirement: Doctor capability detection
When `ah doctor` detects an available framework or quality tool that is not configured, the system SHALL emit a `recommendation` finding with `kind`, `suggested_action`, `apply_command`, `playbook_command`, and `detection_source`.

#### Scenario: Detect installed pytest
- **GIVEN** a project contains a Python test file or has `pytest` installed
- **WHEN** a user runs `ah doctor`
- **THEN** the command reports that pytest is available
- **AND** recommends enabling the pytest adapter if not already configured

#### Scenario: Detect installed cargo test
- **GIVEN** a project contains a `Cargo.toml`
- **WHEN** a user runs `ah doctor`
- **THEN** the command reports that cargo test is available
- **AND** recommends enabling the cargo adapter if not already configured

#### Scenario: Detect installed vitest
- **GIVEN** a project contains a `package.json` with vitest in dependencies or devDependencies
- **WHEN** a user runs `ah doctor`
- **THEN** the command reports that vitest is available
- **AND** recommends enabling the vitest adapter if not already configured

#### Scenario: Detect property-based testing capability
- **GIVEN** a project has a supported PBT framework available (e.g., hypothesis for Python, proptest for Rust)
- **WHEN** a user runs `ah doctor`
- **THEN** the command reports the PBT framework and recommends enabling the property capability

#### Scenario: No silent capability changes
- **GIVEN** a new framework becomes available in the project environment
- **WHEN** a user runs `ah doctor`
- **THEN** the command always reports the detection result explicitly — never silently enables or silently ignores

#### Scenario: Multi-language project reports all detections
- **GIVEN** a project contains both a `Cargo.toml` and a `package.json` with vitest
- **WHEN** a user runs `ah doctor`
- **THEN** the command reports both cargo test and vitest as available
- **AND** recommends enabling each adapter that is not yet configured

## ADDED Requirements

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
