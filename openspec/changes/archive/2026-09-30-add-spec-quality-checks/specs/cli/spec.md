# Capability: cli

## ADDED Requirements

### Requirement: Spec Lint Command
The system SHALL provide `ah lint` to statically analyze OpenSpec scenario files for quality findings without modifying files or running tests.

#### Scenario: Lint deployed specs
- **WHEN** a user runs `ah lint`
- **THEN** the command analyzes all spec files under `openspec/specs/`
- **AND** emits lint findings to stdout
- **AND** exits zero when only warning-severity findings are present

#### Scenario: Lint a change overlay
- **WHEN** a user runs `ah lint --changes add-parser`
- **THEN** the command analyzes deployed specs plus the `add-parser` change spec overlay

#### Scenario: Lint a single check category
- **WHEN** a user runs `ah lint --check vague-qualifier`
- **THEN** the command runs only the `vague-qualifier` check and skips all others

#### Scenario: Lint JSON output
- **WHEN** a user runs `ah lint --json`
- **THEN** the command emits a JSON object in the same schema as `ah check --json`
- **AND** each finding includes `kind`, `severity`, `spec_path`, `scenario_id`, `message`, and `suggestion`

#### Scenario: Clean spec exits zero with empty findings
- **GIVEN** all analyzed spec files pass all lint checks
- **WHEN** a user runs `ah lint`
- **THEN** the command exits zero
- **AND** emits no findings
