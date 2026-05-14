# Capability: adapters

Language adapters normalize test execution across Python, Rust, and TypeScript into the shared finding schema. Each adapter detects its framework, invokes it, and maps exit codes and output into findings.

## ADDED Requirements

### Requirement: Adapter detection precedence
The system SHALL detect framework availability through a defined precedence chain before invoking any adapter.

#### Scenario: Manifest declaration takes precedence
- **GIVEN** a project declares a test framework in its language manifest (e.g., `pyproject.toml`, `Cargo.toml`, `package.json`)
- **WHEN** adapter detection runs
- **THEN** the manifest declaration is treated as the strongest signal, overriding environment and source signals

#### Scenario: Environment detection is second
- **GIVEN** a framework is not declared in a manifest but is installed in the environment
- **WHEN** adapter detection runs
- **THEN** the environment presence is used to confirm availability

#### Scenario: Source import is weakest signal
- **GIVEN** a framework is not in the manifest or environment, but is imported in a test file
- **WHEN** adapter detection runs
- **THEN** the source import is recognized as the weakest positive signal

#### Scenario: Manifest signal wins over conflicting environment signal
- **GIVEN** a project manifest declares pytest as the test framework
- **AND** the environment also has vitest installed
- **WHEN** adapter detection runs
- **THEN** pytest is selected as the active adapter (manifest takes precedence over environment)
- **AND** the vitest environment presence is not treated as a detection conflict

### Requirement: Python pytest adapter
The system SHALL provide a bundled pytest adapter that detects pytest, runs the declared test command, and normalizes output into the finding schema.

#### Scenario: Pytest adapter detects via pyproject.toml
- **GIVEN** a project contains `pyproject.toml` with pytest in `[tool.pytest.ini_options]` or as a dependency
- **WHEN** the pytest adapter runs detection
- **THEN** it reports pytest as available with the detected version

#### Scenario: Pytest adapter normalizes zero exit to pass
- **GIVEN** a contract declares a pytest test command
- **WHEN** the adapter runs the command and pytest exits zero
- **THEN** the adapter emits no `test-failing` finding for that contract

#### Scenario: Pytest adapter normalizes non-zero exit to test-failing
- **GIVEN** a contract declares a pytest test command
- **WHEN** the adapter runs the command and pytest exits non-zero
- **THEN** the adapter emits a `test-failing` finding with bounded stdout/stderr tails

### Requirement: Rust cargo test adapter
The system SHALL provide a bundled cargo test adapter that detects cargo, runs the declared test command, and normalizes output into the finding schema.

#### Scenario: Cargo adapter detects via Cargo.toml
- **GIVEN** a project contains `Cargo.toml`
- **WHEN** the cargo adapter runs detection
- **THEN** it reports cargo test as available

#### Scenario: Cargo adapter normalizes zero exit to pass
- **GIVEN** a contract declares a cargo test command
- **WHEN** the adapter runs the command and cargo exits zero
- **THEN** the adapter emits no `test-failing` finding for that contract

#### Scenario: Cargo adapter normalizes non-zero exit to test-failing
- **GIVEN** a contract declares a cargo test command
- **WHEN** the adapter runs the command and cargo exits non-zero
- **THEN** the adapter emits a `test-failing` finding with bounded stdout/stderr tails

### Requirement: TypeScript vitest adapter
The system SHALL provide a bundled vitest adapter that detects vitest, runs the declared test command, and normalizes output into the finding schema.

#### Scenario: Vitest adapter detects via package.json
- **GIVEN** a project contains `package.json` with vitest in `dependencies` or `devDependencies`
- **WHEN** the vitest adapter runs detection
- **THEN** it reports vitest as available with the detected version

#### Scenario: Vitest adapter normalizes zero exit to pass
- **GIVEN** a contract declares a vitest test command
- **WHEN** the adapter runs the command and vitest exits zero
- **THEN** the adapter emits no `test-failing` finding for that contract

#### Scenario: Vitest adapter normalizes non-zero exit to test-failing
- **GIVEN** a contract declares a vitest test command
- **WHEN** the adapter runs the command and vitest exits non-zero
- **THEN** the adapter emits a `test-failing` finding with bounded stdout/stderr tails

### Requirement: No-adapter-configured path
The system SHALL emit a clear finding when a contract declares a test command but no adapter is configured or detected for the project's language.

#### Scenario: Missing adapter emits no-tests-declared finding
- **GIVEN** a contract declares a test command
- **AND** no adapter is configured in `.espectacular/config.toml` for the project's language
- **AND** adapter detection finds no matching framework
- **WHEN** `ah check` runs
- **THEN** a `no-tests-declared` finding is emitted with a message directing the user to run `ah doctor`

### Requirement: Custom runner plugin protocol
The system SHALL support `[runners.custom.<name>]` config blocks that wire arbitrary shell commands into the adapter layer via a documented JSON envelope defined in `schemas/custom-runner.schema.json`.

Note: The envelope schema (`schemas/custom-runner.schema.json`) specifies the top-level structure the shell command must emit. Individual findings within the `findings` array conform to the full finding schema (`schemas/check-output.schema.json`), not to the envelope schema.

#### Scenario: Custom runner emits required envelope fields
- **GIVEN** a custom runner is configured and invoked
- **WHEN** the runner's stdout is parsed
- **THEN** the envelope contains at minimum: `exit_code` (integer), `passed` (boolean), `findings` (array)
- **AND** each finding in the array conforms to the full finding schema

#### Scenario: Empty findings array with zero exit is a pass
- **GIVEN** a custom runner exits zero
- **AND** the envelope `findings` array is empty
- **WHEN** the adapter processes the result
- **THEN** no `test-failing` finding is emitted for that contract

#### Scenario: Custom runner non-zero exit without valid envelope is an error finding
- **GIVEN** a custom runner exits non-zero
- **AND** stdout is not a valid envelope
- **WHEN** the adapter processes the result
- **THEN** a `test-failing` finding is emitted with the raw stdout/stderr tails

#### Scenario: Custom runner is not invoked without explicit config
- **GIVEN** no `[runners.custom.<name>]` block exists in config
- **WHEN** `ah check` runs
- **THEN** no custom runner is invoked
