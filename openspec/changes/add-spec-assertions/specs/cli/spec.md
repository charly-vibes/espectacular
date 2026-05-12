# Capability: cli

The standalone command-line interface for espectacular is exposed as `ah`.

## ADDED Requirements
### Requirement: CLI Command Name
The system SHALL expose the standalone command-line interface as `ah`.

#### Scenario: Invoke the compiler
- **WHEN** a user runs `ah compile`
- **THEN** the CLI starts the compilation workflow

#### Scenario: Invoke drift detection
- **WHEN** a user runs `ah drift`
- **THEN** the CLI starts the drift detection workflow

#### Scenario: Invoke reporting
- **WHEN** a user runs `ah report`
- **THEN** the CLI starts the reporting workflow
