# Gate spec delta: property_class on scenario contracts

## MODIFIED Requirements

### Requirement: Contract Schema
The system SHALL validate per-scenario TOML contracts before running tests. Contracts MAY declare an optional `property_class` field with value `safety` or `liveness`; an invalid value is a structural finding, and a `liveness`-tagged contract whose test entries all lack `timeout_seconds` produces a warning that does not fail the gate.

#### Scenario: Validate scenario metadata
- **GIVEN** a scenario contract contains `id`, `description`, `archetype`, `status`, and `authored_with`
- **WHEN** a user runs `ah check`
- **THEN** the command validates the metadata fields before executing tests

#### Scenario: Reject unknown status
- **GIVEN** a scenario contract has `status = "paused"`
- **WHEN** a user runs `ah check`
- **THEN** the command emits an `invalid-status` structural finding
- **AND** exits non-zero

#### Scenario: Validate superseded status
- **GIVEN** a scenario contract has `status = "superseded"`
- **WHEN** a user runs `ah check`
- **THEN** the command requires a non-empty `superseded_by` value
- **AND** still runs the scenario's declared tests

#### Scenario: Accept contract without property_class
- **GIVEN** a scenario contract declares no `property_class` field
- **WHEN** a user runs `ah check`
- **THEN** the contract validates and behaves exactly as it did before the field existed
- **AND** no finding related to `property_class` is emitted

#### Scenario: Accept valid property_class values
- **GIVEN** a scenario contract has `property_class = "safety"` (or `"liveness"`)
- **WHEN** a user runs `ah check`
- **THEN** the contract validates and runs its declared tests normally

#### Scenario: Reject invalid property_class value
- **GIVEN** a scenario contract has `property_class = "eventual"`
- **WHEN** a user runs `ah check`
- **THEN** the command emits an `invalid-property-class` structural finding
- **AND** exits non-zero without running tests

#### Scenario: Warn on liveness contract without any test timeout
- **GIVEN** a scenario contract has `property_class = "liveness"`
- **AND** every declared test entry omits `timeout_seconds`
- **WHEN** a user runs `ah check`
- **THEN** the command emits a `missing-liveness-timeout` finding with `severity = "warning"`
- **AND** the finding suggests adding explicit timeout semantics, since liveness claims are only falsifiable under bounded execution

#### Scenario: No warning when liveness contract declares a timeout
- **GIVEN** a scenario contract has `property_class = "liveness"`
- **AND** at least one declared test entry specifies `timeout_seconds`
- **WHEN** a user runs `ah check`
- **THEN** no `missing-liveness-timeout` finding is emitted

#### Scenario: Warnings do not fail the gate
- **GIVEN** `ah check` produces only `missing-liveness-timeout` warnings and no error-severity findings
- **WHEN** the run completes
- **THEN** the command exits zero
