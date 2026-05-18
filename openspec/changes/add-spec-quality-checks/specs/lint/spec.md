# Capability: lint

The spec quality linter statically analyzes OpenSpec scenario files for authoring defects that cause AI agents to implement correct-by-gate but wrong-by-intent behavior.

## ADDED Requirements

### Requirement: Vague Qualifier Detection
The system SHALL flag requirements and scenario steps that contain unbound qualitative terms without an adjacent numeric measurement.

#### Scenario: Flag unbound qualifier in requirement
- **GIVEN** a requirement body contains the word "fast" without a numeric time bound
- **WHEN** `ah lint` runs
- **THEN** the command emits a `vague-qualifier` finding for that requirement
- **AND** the finding suggests adding a measurable response condition (e.g., "within 200 ms")

#### Scenario: Do not flag bounded qualifier
- **GIVEN** a requirement body contains "fast (p95 < 200 ms)"
- **WHEN** `ah lint` runs
- **THEN** no `vague-qualifier` finding is emitted for that requirement

#### Scenario: Flag qualifier in scenario step
- **GIVEN** a THEN step contains "the response is user-friendly"
- **WHEN** `ah lint` runs
- **THEN** the command emits a `vague-qualifier` finding for that scenario step

### Requirement: Imperative Step Detection
The system SHALL flag WHEN and THEN steps that describe UI mechanics rather than business intent, coupling the spec to implementation details.

#### Scenario: Flag explicit click step
- **GIVEN** a WHEN step contains "clicks the Submit button"
- **WHEN** `ah lint` runs
- **THEN** the command emits an `imperative-step` finding
- **AND** the finding suggests rephrasing to describe the business action (e.g., "submits the form")

#### Scenario: Flag URL navigation step
- **GIVEN** a WHEN step contains "navigates to /dashboard/settings"
- **WHEN** `ah lint` runs
- **THEN** the command emits an `imperative-step` finding

#### Scenario: Do not flag declarative business steps
- **GIVEN** a WHEN step contains "the user submits a payment"
- **WHEN** `ah lint` runs
- **THEN** no `imperative-step` finding is emitted

### Requirement: Conjunctive Step Bloat Detection
The system SHALL flag scenarios that chain more than the configured maximum number of AND-linked steps, indicating a scenario that tests multiple behaviors at once.

#### Scenario: Flag overlong scenario
- **GIVEN** a scenario has eight AND-linked steps
- **AND** the configured maximum is five
- **WHEN** `ah lint` runs
- **THEN** the command emits a `conjunctive-bloat` finding
- **AND** the finding suggests splitting the scenario into focused single-behavior scenarios

#### Scenario: Accept scenario within limit
- **GIVEN** a scenario has four AND-linked steps
- **AND** the configured maximum is five
- **WHEN** `ah lint` runs
- **THEN** no `conjunctive-bloat` finding is emitted for that scenario

#### Scenario: Scenario exactly at limit is accepted
- **GIVEN** a scenario has five AND-linked steps
- **AND** the configured maximum is five
- **WHEN** `ah lint` runs
- **THEN** no `conjunctive-bloat` finding is emitted for that scenario

#### Scenario: Default maximum is configurable
- **GIVEN** `.espectacular/config.toml` sets `[lint] max_and_steps = 3`
- **WHEN** `ah lint` runs
- **THEN** the command uses 3 as the maximum AND-step count

### Requirement: Missing Negative Scenario Detection
The system SHALL flag requirements that have no scenario exercising an error condition, rejection, or boundary violation, indicating incomplete behavioral specification.

#### Scenario: Flag requirement with only happy-path scenarios
- **GIVEN** a requirement has two scenarios, both describing successful outcomes
- **AND** no scenario contains rejection, error, or boundary language
- **WHEN** `ah lint` runs
- **THEN** the command emits a `missing-negative-scenario` finding
- **AND** the finding suggests adding a scenario for the corresponding failure mode

#### Scenario: Accept requirement with at least one negative scenario
- **GIVEN** a requirement has a scenario whose THEN step contains "the command exits non-zero"
- **WHEN** `ah lint` runs
- **THEN** no `missing-negative-scenario` finding is emitted for that requirement

### Requirement: Missing Non-Goals Detection
The system SHALL flag spec capability files that lack an explicit Non-Goals or Out-of-Scope section, since their absence enables silent scope creep during AI-assisted implementation.

#### Scenario: Flag spec without non-goals section
- **GIVEN** a `spec.md` file contains no heading matching `Non-Goals`, `Non Goals`, `non-goals`, or `Out of Scope`
- **WHEN** `ah lint` runs
- **THEN** the command emits a `missing-non-goals` finding for that spec file

#### Scenario: Accept spec with non-goals section
- **GIVEN** a `spec.md` file contains a `## Non-Goals` heading
- **WHEN** `ah lint` runs
- **THEN** no `missing-non-goals` finding is emitted for that file

### Requirement: Unresolved Ambiguity Detection
The system SHALL flag requirements and scenarios that contain `[NEEDS CLARIFICATION` markers, indicating authoring-time decisions deferred but not yet resolved.

#### Scenario: Flag needs-clarification marker
- **GIVEN** a requirement body contains `[NEEDS CLARIFICATION: which auth provider?]`
- **WHEN** `ah lint` runs
- **THEN** the command emits an `unresolved-ambiguity` finding
- **AND** the finding includes the marker text as context

#### Scenario: Accept requirement with no ambiguity markers
- **GIVEN** a requirement body contains no `[NEEDS CLARIFICATION` substring
- **WHEN** `ah lint` runs
- **THEN** no `unresolved-ambiguity` finding is emitted for that requirement

### Requirement: Lint Finding Schema
The system SHALL emit lint findings using the same stable JSON envelope as `ah check`, enabling agent harnesses to consume both without separate parsing logic.

#### Scenario: Lint findings share check finding fields
- **GIVEN** `ah lint --json` produces findings
- **WHEN** the JSON output is inspected
- **THEN** each finding contains `kind`, `severity`, `spec_path`, `message`, `suggested_action`, and `playbook_command`
- **AND** findings that reference a specific scenario also contain `scenario_id` and `scenario_title`

#### Scenario: Lint findings are warning severity by default
- **GIVEN** `ah lint` produces findings for any of the six check categories
- **WHEN** the JSON output is inspected
- **THEN** every finding has `severity = "warning"`

#### Scenario: Malformed spec file is error severity
- **GIVEN** a spec file contains invalid Markdown that prevents scenario parsing
- **WHEN** `ah lint` runs
- **THEN** the command emits a finding with `severity = "error"`
- **AND** exits non-zero
