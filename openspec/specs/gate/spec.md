# gate Specification

## Purpose
TBD - created by archiving change add-spec-assertions. Update Purpose after archive.
## Requirements
### Requirement: Scenario Discovery
The system SHALL discover OpenSpec scenarios from `#### Scenario:` headings in `spec.md` files.

#### Scenario: Discover deployed scenario
- **GIVEN** `openspec/specs/compiler/spec.md` contains `#### Scenario: Empty input rejected`
- **WHEN** `ah check` scans deployed specs
- **THEN** it discovers a scenario with id `empty-input-rejected`
- **AND** associates it with the `compiler` spec

#### Scenario: Reject duplicate scenario ids
- **GIVEN** two scenarios in the same spec slugify to the same id
- **WHEN** `ah check` validates the spec
- **THEN** it emits a structural finding for the slug collision
- **AND** exits non-zero

### Requirement: Sidecar Contract Correspondence
The system SHALL require exactly one TOML sidecar contract for each discovered scenario in scope.

#### Scenario: Missing contract fails
- **GIVEN** a scenario id `empty-input-rejected` exists under `openspec/specs/compiler/spec.md`
- **AND** `.espectacular/compiler/empty-input-rejected.toml` does not exist
- **WHEN** a user runs `ah check`
- **THEN** the command emits a `no-toml` structural finding
- **AND** exits non-zero

#### Scenario: Orphan contract fails
- **GIVEN** `.espectacular/compiler/empty-input-rejected.toml` exists
- **AND** no matching scenario exists under `openspec/specs/compiler/spec.md`
- **WHEN** a user runs `ah check`
- **THEN** the command emits an `orphan-toml` structural finding
- **AND** exits non-zero

#### Scenario: Contract id mismatch fails
- **GIVEN** `.espectacular/compiler/empty-input-rejected.toml` contains `id = "different-id"`
- **AND** the matching scenario slug is `empty-input-rejected`
- **WHEN** a user runs `ah check`
- **THEN** the command emits an `id-mismatch` structural finding
- **AND** exits non-zero

#### Scenario: Empty test set fails
- **GIVEN** a scenario contract declares no tests
- **WHEN** a user runs `ah check`
- **THEN** the command emits a `no-tests-declared` structural finding
- **AND** exits non-zero

### Requirement: Contract Schema
The system SHALL validate per-scenario TOML contracts before running tests.

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

### Requirement: Test Runner Execution
The system SHALL run each declared test command and use its exit code as the execution verdict.

#### Scenario: Run configured unit test
- **GIVEN** `.espectacular/config.toml` maps `unit` to `["uv", "run", "pytest"]`
- **AND** a scenario contract declares `[[tests.unit]]` with `flags = "tests/test_parser.py::test_empty_input"`
- **WHEN** a user runs `ah check`
- **THEN** the command executes argv `["uv", "run", "pytest", "tests/test_parser.py::test_empty_input"]` without a shell from the repository root
- **AND** records the command exit code in JSON output

#### Scenario: Run shell test
- **GIVEN** a scenario contract declares `[[tests.shell]]` with `command = "ah --version | grep -q 'ah '"`
- **WHEN** a user runs `ah check`
- **THEN** the command executes the shell command through `/bin/sh -c` from the repository root
- **AND** records the command exit code in JSON output

#### Scenario: Enforce test timeout
- **GIVEN** a declared test command runs longer than its configured timeout
- **WHEN** a user runs `ah check`
- **THEN** the command stops the test command
- **AND** emits a `test-failing` execution finding with `timed_out = true`
- **AND** exits non-zero

#### Scenario: Capture bounded output tails
- **GIVEN** a declared test command writes more than 8 KiB to stdout and stderr
- **WHEN** `ah check` emits JSON output
- **THEN** the execution finding includes only the final 8 KiB of stdout
- **AND** includes only the final 8 KiB of stderr

#### Scenario: Missing runner fails structurally
- **GIVEN** a scenario contract declares `[[tests.integration]]`
- **AND** `.espectacular/config.toml` does not define `runners.integration`
- **WHEN** a user runs `ah check`
- **THEN** the command emits a `missing-runner` structural finding
- **AND** exits non-zero

#### Scenario: Invalid TOML syntax fails structurally
- **GIVEN** a scenario contract file contains invalid TOML syntax
- **WHEN** a user runs `ah check`
- **THEN** the command emits a `malformed-contract` structural finding
- **AND** exits non-zero

#### Scenario: Malformed test entry fails structurally
- **GIVEN** a non-shell test entry omits `flags`
- **WHEN** a user runs `ah check`
- **THEN** the command emits a `malformed-contract` structural finding
- **AND** exits non-zero

#### Scenario: Non-zero declared test fails check
- **GIVEN** a declared test command exits non-zero
- **WHEN** a user runs `ah check`
- **THEN** the command emits a `test-failing` execution finding
- **AND** exits non-zero

### Requirement: JSON Findings
The system SHALL emit stable JSON output for `ah check` results.

#### Scenario: Report success with empty findings
- **GIVEN** every scenario in scope has a valid contract
- **AND** every declared test command exits zero
- **WHEN** a user runs `ah check`
- **THEN** the command exits zero
- **AND** emits JSON with `findings = []`

#### Scenario: Report all findings in stable order
- **GIVEN** multiple scenarios have findings
- **WHEN** a user runs `ah check`
- **THEN** the JSON output includes all findings
- **AND** orders them by spec path and scenario id

#### Scenario: Include actionable scenario context
- **GIVEN** a scenario has a finding
- **WHEN** `ah check` emits JSON output
- **THEN** the finding includes the scenario id, spec path, scenario title, and scenario body markdown

#### Scenario: Extract scenario body boundaries
- **GIVEN** a scenario heading is followed by markdown body lines and then another `####` heading
- **WHEN** `ah check` emits JSON output for that scenario
- **THEN** `body_markdown` contains only the lines after the scenario heading and before the next heading whose level is `####` or higher

#### Scenario: Include checked scope
- **WHEN** `ah check` emits JSON output
- **THEN** the top-level JSON includes whether deployed specs were checked
- **AND** includes any selected OpenSpec changes

#### Scenario: Include command details for execution findings
- **GIVEN** a declared test command exits non-zero
- **WHEN** `ah check` emits JSON output
- **THEN** the finding includes the test type, command, exit code, timeout flag, stdout tail, and stderr tail when available

### Requirement: Change Overlay Scope
The system SHALL support checking selected OpenSpec changes as overlays on deployed specs.

#### Scenario: Check selected change overlay
- **GIVEN** `openspec/changes/add-parser/specs/compiler/spec.md` adds a scenario
- **AND** `.espectacular/changes/add-parser/compiler/<scenario>.toml` exists
- **WHEN** a user runs `ah check --changes add-parser`
- **THEN** the command validates the deployed compiler spec plus the `add-parser` scenario overlay

#### Scenario: Apply staged metadata update for deployed scenario
- **GIVEN** `.espectacular/changes/add-parser/compiler/old-behavior.toml` has `status = "superseded"`
- **AND** deployed spec `compiler` contains scenario `old-behavior`
- **WHEN** a user runs `ah check --changes add-parser`
- **THEN** the command validates the staged contract as the active contract for `old-behavior` in the overlay

#### Scenario: Reject supersession with missing replacement
- **GIVEN** `.espectacular/changes/add-parser/compiler/old-behavior.toml` has `status = "superseded"`
- **AND** `superseded_by = "new-behavior"`
- **AND** no scenario `new-behavior` exists in deployed specs or the selected change overlay
- **WHEN** a user runs `ah check --changes add-parser`
- **THEN** the command emits a structural finding for the missing replacement scenario
- **AND** exits non-zero

#### Scenario: Reject conflicting overlays
- **GIVEN** two selected changes define the same new scenario id for the same spec
- **WHEN** a user runs `ah check --changes first --changes second`
- **THEN** the command emits a structural finding for the conflict
- **AND** exits non-zero

#### Scenario: Reject conflicting staged updates for one deployed scenario
- **GIVEN** two selected changes both stage metadata updates for the same deployed scenario id in the same spec
- **WHEN** a user runs `ah check --changes first --changes second`
- **THEN** the command emits an `overlay-conflict` structural finding
- **AND** exits non-zero

#### Scenario: Overlay resolution is deterministic
- **GIVEN** selected changes do not conflict
- **WHEN** a user runs `ah check --changes zeta --changes alpha`
- **THEN** the command resolves selected changes in sorted change-id order
- **AND** produces the same validation scope as `ah check --changes alpha --changes zeta`

### Requirement: Non-Regression Archetype
The system SHALL support an `NR` (Non-Regression) archetype for contracts that assert existing behavior is preserved during change proposals.

#### Scenario: NR contract is valid
- **GIVEN** a scenario contract has `archetype = "NR"`
- **WHEN** a user runs `ah check`
- **THEN** the gate accepts `NR` as a valid archetype value
- **AND** validates and runs the contract's declared tests identically to other archetypes

#### Scenario: NR contract runs in change overlay scope
- **GIVEN** a change proposal modifies a capability
- **AND** an existing scenario is covered by a contract with `archetype = "NR"`
- **WHEN** a user runs `ah check --changes <change-id>`
- **THEN** the NR contract is validated as part of the overlay scope
- **AND** a failing NR test exits non-zero

#### Scenario: ah upgrade reports NR as archetype addition
- **GIVEN** `.espectacular/config.toml` pins a tool version that predates `NR` support
- **WHEN** a user runs `ah upgrade`
- **THEN** the command reports `NR` as a newly available archetype before updating the configured tool version

### Requirement: Deterministic Scope Boundary
The system SHALL avoid semantic evaluation of test quality or scenario prose.

#### Scenario: Do not inspect test internals
- **GIVEN** a declared test command exists and exits zero
- **WHEN** a user runs `ah check`
- **THEN** the command treats the test as passing
- **AND** does not inspect assertions, fixtures, mocks, or setup code

#### Scenario: Do not hash scenario prose
- **GIVEN** the body text under an existing scenario heading changes
- **WHEN** a user runs `ah check`
- **THEN** the command does not fail solely because the prose changed

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

