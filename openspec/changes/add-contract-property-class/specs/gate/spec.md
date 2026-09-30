---
id: spec
kind: intent
statement: "WHEN a scenario contract declares an optional property_class field THE gate SHALL validate the contract schema before running tests, accepting safety and liveness, rejecting invalid values, and warning without failing when a liveness contract's tests all lack timeout_seconds."
---

# Gate spec delta: property_class on scenario contracts

## Constraints

| id | kind | expr | traces_to |
|----|------|------|-----------|
| C-schema-first | invariant | per-scenario TOML contracts are validated (`id`, `description`, `archetype`, `status`, `authored_with`) before any declared test executes | |
| C-status-values | invariant | an unknown `status` value emits an `invalid-status` structural finding and exits non-zero; `status = "superseded"` requires a non-empty `superseded_by` and still runs the declared tests | |
| C-property-class-values | invariant | `property_class` is absent (behaving exactly as before the field existed), `safety`, or `liveness`; any other value emits an `invalid-property-class` structural finding and exits non-zero without running tests | |
| C-liveness-timeout-warning | invariant | a `liveness` contract whose test entries all omit `timeout_seconds` emits a `missing-liveness-timeout` finding with `severity = "warning"` suggesting bounded execution semantics; at least one declared timeout suppresses it; warnings alone exit zero | |

## Model

### States

- `parsing`
- `executing`
- `reported`

### Transitions

| id | from | to | guard |
|----|------|----|-------|
| t-schema | parsing | parsing | [[spec.C-schema-first]] |
| t-reject | parsing | reported | [[spec.C-property-class-values]] OR [[spec.C-status-values]] |
| t-execute | parsing | executing | contract schema valid, including superseded-with-non-empty-superseded_by |
| t-warn | executing | reported | [[spec.C-liveness-timeout-warning]] |
| t-clean | reported | reported | warnings alone never fail the gate |

## Properties

| id | kind | derives_from | generator | predicate |
|----|------|--------------|-----------|-----------|
| P-schema-first | unit | [[spec.C-schema-first]] | a contract with malformed metadata | no declared test runs before schema validation emits its findings |
| P-status | unit | [[spec.C-status-values]] | contracts with `status = "paused"` and `status = "superseded"` | the first exits non-zero with `invalid-status`; the second runs its tests only with a non-empty `superseded_by` |
| P-property-class | unit | [[spec.C-property-class-values]] | contracts with `property_class` absent, `"safety"`, `"liveness"`, and `"eventual"` | the first three validate and behave per their class; the last exits non-zero with `invalid-property-class` and runs no tests |
| P-liveness-timeout | unit | [[spec.C-liveness-timeout-warning]] | liveness contracts with all tests lacking `timeout_seconds` vs at least one timeout | warning emitted exactly in the first case with `severity = "warning"`; suppressed in the second; warnings alone exit zero |

## Purpose

Extend the per-scenario TOML contract schema with an optional `property_class`
(`safety` / `liveness`) so contracts can declare their falsifiability class.
Invalid values are structural; liveness contracts without bounded execution
warn without failing the gate, keeping backward compatibility for contracts
that omit the field.

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

## Requirements

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
