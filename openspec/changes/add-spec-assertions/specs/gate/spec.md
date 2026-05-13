# Capability: gate

The correspondence gate validates that OpenSpec scenarios have explicit sidecar contracts and runnable tests.

## ADDED Requirements

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
