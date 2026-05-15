# Capability: explain

The `ah explain` subcommand provides compile-enforced playbook guidance, with one topic per finding-kind or suggested-action enum variant. Topics ship in the binary; the build fails if any variant lacks a body.

## ADDED Requirements

### Requirement: Playbook is compile-enforced
The system SHALL fail to build if any `FindingKind` or `SuggestedAction` enum variant lacks a corresponding `ah explain` topic body.

#### Scenario: Missing topic body is a compile error
- **GIVEN** a `FindingKind` or `SuggestedAction` variant has no associated playbook body
- **WHEN** the project is built with `cargo build`
- **THEN** the build fails with an error identifying the missing topic

#### Scenario: All variants have topics at build time
- **GIVEN** all enum variants have associated playbook bodies
- **WHEN** the project is built
- **THEN** the build succeeds and `ah explain --list` enumerates them all

### Requirement: Topic coverage
The system SHALL provide `ah explain` topics for every `FindingKind` value, every `SuggestedAction` value, and a set of general topics.

#### Scenario: Finding kind topic exists
- **WHEN** a user runs `ah explain no-toml`
- **THEN** the command prints guidance for the `no-toml` finding kind and exits zero

#### Scenario: Suggested action topic exists
- **WHEN** a user runs `ah explain run_ah_scenario_new`
- **THEN** the command prints guidance for the `run_ah_scenario_new` action and exits zero

#### Scenario: General topic exists
- **WHEN** a user runs `ah explain workflow`
- **THEN** the command prints general workflow guidance and exits zero

### Requirement: Structured JSON output
The system SHALL support `--json` output for `ah explain` that emits a machine-readable object.

#### Scenario: JSON output has required fields
- **WHEN** a user runs `ah explain no-toml --json`
- **THEN** the output is a valid JSON object containing: `topic` (string), `summary` (string), `when` (string), `do` (array of strings), `human_approval` (boolean), `related_topics` (array of strings), `hints` (array — shape provisional until v0.2)

#### Scenario: JSON output is valid for every topic
- **GIVEN** any valid topic identifier
- **WHEN** `ah explain <topic> --json` is run
- **THEN** the output passes JSON schema validation

### Requirement: Topic listing
The system SHALL enumerate all available topics on demand.

#### Scenario: List enumerates all topics
- **WHEN** a user runs `ah explain --list`
- **THEN** the command prints all topic identifiers, one per line, and exits zero

#### Scenario: List is stable across runs
- **WHEN** `ah explain --list` is run twice in succession
- **THEN** the output is identical (topics are sorted alphabetically)

### Requirement: Unknown topic handling
The system SHALL reject unknown topics with a clear error and exit non-zero.

#### Scenario: Unknown topic exits non-zero
- **WHEN** a user runs `ah explain no-such-topic`
- **THEN** the command exits non-zero
- **AND** the error message lists available topics or directs the user to `ah explain --list`

### Requirement: Quality finding kind topics
The system SHALL provide `ah explain` topics for every quality finding kind introduced by this change: `quality-mutation`, `quality-property`, `quality-snapshot`. These are `FindingKind` values and are therefore subject to the compile-enforcement requirement.

#### Scenario: quality-mutation topic exists
- **WHEN** a user runs `ah explain quality-mutation`
- **THEN** the command prints guidance explaining what the mutation score means, how to enable mutation testing, and when the finding appears

#### Scenario: quality-property topic exists
- **WHEN** a user runs `ah explain quality-property`
- **THEN** the command prints guidance for the `quality-property` finding kind and exits zero

#### Scenario: quality-snapshot topic exists
- **WHEN** a user runs `ah explain quality-snapshot`
- **THEN** the command prints guidance for the `quality-snapshot` finding kind and exits zero

### Requirement: Adapter topics ship with adapters
The system SHALL include `ah explain` topics for progressive-enablement capabilities when their adapter modules are compiled in.

#### Scenario: Pytest adapter contributes topic
- **GIVEN** the pytest adapter is compiled into the binary
- **WHEN** a user runs `ah explain pytest`
- **THEN** the command prints guidance for enabling and using the pytest adapter

#### Scenario: Duplicate topic registration is a compile error
- **GIVEN** two adapter modules attempt to register the same topic identifier
- **WHEN** the project is built
- **THEN** the build fails identifying the conflicting topic name
