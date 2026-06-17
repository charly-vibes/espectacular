# Capability: gate

The correspondence gate validates the alignment between OpenSpec scenarios and the test contracts that cover them, emitting stable JSON findings.

## ADDED Requirements

### Requirement: JSON finding schema includes agent-action fields
The system SHALL include agent-action fields on every finding in the JSON output.

#### Scenario: Every finding carries suggested_action
- **GIVEN** `ah check` produces any finding
- **WHEN** the JSON output is inspected
- **THEN** every finding object contains a `suggested_action` field with a value from the documented enum

#### Scenario: Every finding carries playbook_command
- **GIVEN** `ah check` produces any finding
- **WHEN** the JSON output is inspected
- **THEN** every finding object contains a `playbook_command` field with a valid `ah explain <topic>` invocation

#### Scenario: scenario_prose is verbatim and untruncated
- **GIVEN** a finding references a scenario
- **WHEN** the JSON output is inspected
- **THEN** the `scenario_prose` field contains the full markdown body of the scenario heading, verbatim, without truncation

#### Scenario: Findings are sorted deterministically
- **GIVEN** `ah check` produces multiple findings
- **WHEN** the JSON output is inspected
- **THEN** the `findings` array is sorted by `(spec_path, scenario_id, kind)` in ascending lexicographic order

#### Scenario: Summary counts by kind
- **GIVEN** `ah check` produces findings of multiple kinds
- **WHEN** the JSON output is inspected
- **THEN** the envelope `summary.counts_by_kind` object contains the count of each finding kind present

### Requirement: Quality measurement capabilities
The system SHALL support opt-in quality measurement capabilities that run during `ah check` and emit measurement findings without failing the gate.

#### Scenario: Mutation testing runs when enabled
- **GIVEN** a contract declares `[quality.mutation] enabled = true`
- **AND** a mutation tool is configured in `.espectacular/config.toml`
- **WHEN** a user runs `ah check --mutation`
- **THEN** the gate runs the mutation tool against the contract's declared tests
- **AND** emits a `quality-mutation` info finding with the measured score
- **AND** exits zero when the score is below any configured threshold

#### Scenario: Property-based testing runs when declared
- **GIVEN** a contract declares a `tests.property` entry
- **WHEN** a user runs `ah check`
- **THEN** the gate runs the property test command
- **AND** emits a `quality-property` finding with the run result

#### Scenario: Snapshot testing runs when declared
- **GIVEN** a contract declares a `tests.snapshot` entry
- **WHEN** a user runs `ah check`
- **THEN** the gate runs the snapshot test command
- **AND** emits a `quality-snapshot` finding with the run result

#### Scenario: Quality scores below threshold do not fail the gate in v1
- **GIVEN** a quality measurement capability completes successfully and produces a score below threshold
- **WHEN** a user runs `ah check`
- **THEN** the finding severity is `warning` or `info`
- **AND** the overall exit status is zero

#### Scenario: Property or snapshot command failure fails the gate
- **GIVEN** a contract declares `[[tests.property]]` or `[[tests.snapshot]]`
- **AND** the declared command exits non-zero or times out
- **WHEN** a user runs `ah check`
- **THEN** the command emits a `test-failing` execution finding
- **AND** the overall exit status is non-zero

#### Scenario: Mutation tool execution failure fails the gate
- **GIVEN** mutation measurement is enabled and the mutation tool command exits non-zero before producing a measurement
- **WHEN** a user runs `ah check --mutation`
- **THEN** the command emits a `test-failing` execution finding
- **AND** the overall exit status is non-zero

#### Scenario: Mutation is off in pre-commit scope by default
- **GIVEN** mutation testing is configured
- **AND** `ah check` is invoked without an explicit `--mutation` flag
- **WHEN** the command runs in pre-commit mode
- **THEN** mutation testing is skipped

### Requirement: Quality contract schema
The system SHALL represent quality measurements without changing the baseline rule that `tests.<type>` entries are arrays of runnable test declarations.

#### Scenario: Mutation configuration is not a test entry
- **GIVEN** mutation measurement is enabled for a scenario contract
- **WHEN** the contract is validated
- **THEN** mutation settings are read from a `[quality.mutation]` table
- **AND** `tests.mutation` as a boolean is rejected as a malformed contract

#### Scenario: Property and snapshot are runnable test entries
- **GIVEN** a scenario contract declares `[[tests.property]]` or `[[tests.snapshot]]`
- **WHEN** the contract is validated
- **THEN** each entry follows the same runnable test-entry shape as other `tests.<type>` arrays

### Requirement: Conformance coverage matrix
The system SHALL compute a per-spec, per-archetype coverage matrix aggregating scenario contract status across all specs in scope.

#### Scenario: Matrix counts covered scenarios
- **GIVEN** `openspec/specs/` contains multiple specs, each with scenarios that have contracts
- **WHEN** a user runs `ah report`
- **THEN** the command emits a matrix row for each spec with columns for each archetype
- **AND** each cell contains `covered`, `missing`, and `failing` counts

#### Scenario: Matrix includes archetype totals
- **GIVEN** `ah report` runs against deployed specs
- **WHEN** the output is inspected
- **THEN** the matrix includes a totals row summing counts across all specs

#### Scenario: Missing contracts appear as uncovered
- **GIVEN** a deployed scenario has no sidecar contract
- **WHEN** `ah report` runs
- **THEN** the scenario is counted as `missing` for its spec row
- **AND** the `archetype` column is `unassigned`

#### Scenario: Machine-readable matrix output
- **WHEN** a user runs `ah report --json`
- **THEN** the command emits a JSON object with a `matrix` array
- **AND** each row contains `spec`, `archetype`, `covered`, `missing`, and `failing` integer fields

### Requirement: apply_command is conditionally present
The system SHALL set `apply_command` only when the finding's `suggested_action` maps to a concrete, mechanical shell command; it SHALL be null for findings that require non-mechanical human action.

#### Scenario: apply_command is set for enable_capability findings
- **GIVEN** `ah check` or `ah doctor` produces a finding with `suggested_action = enable_capability`
- **WHEN** the JSON output is inspected
- **THEN** `apply_command` contains the `ah doctor --enable <capability>` invocation

#### Scenario: apply_command is null for human_review_required findings
- **GIVEN** `ah check` produces a finding with `suggested_action = human_review_required`
- **WHEN** the JSON output is inspected
- **THEN** `apply_command` is null or absent

#### Scenario: apply_command is null for edit_code_not_scenario findings
- **GIVEN** `ah check` produces a finding with `suggested_action = edit_code_not_scenario`
- **WHEN** the JSON output is inspected
- **THEN** `apply_command` is null or absent
