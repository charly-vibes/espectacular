---
id: gate
kind: intent
statement: "WHEN a runner type's contracts declare more test bindings than the batching threshold and the runner provides structured per-test output THE gate SHALL execute those bindings as one batched invocation and SHALL derive each binding's execution verdict from that invocation's structured per-test output."
---

# batched runner spawns

`ah check --run-tests` spawns one runner process per contract test binding.
On measured corpora (tambor: 658 bindings, ~93% of the push gate) this is
dominated by runner startup per spawn, and the bindings themselves carry no
file notion — only scenario-scoped test-name patterns. This change lets the
gate execute the eligible bindings of one runner type as a single batched
invocation with an OR-joined pattern and a structured reporter, then
attribute each binding's verdict from the per-test structured output.

Batching never weakens falsification: a contract whose pattern matches no
test still fails, a matched test failing fails its contract, skipped tests
never cover a contract, and every degenerate batched outcome (unparseable
output, invocation error, timeout) degrades to today's per-binding exit-code
path behind a named signal. Runner types without structured per-test output
keep per-binding spawns byte-identically.

## Constraints

| id | kind | expr | traces_to |
|----|------|------|-----------|
| C-batch-threshold | invariant | bindings of one runner type with structured-reporter support and pattern-only flags batch into a single OR-joined invocation when their count exceeds the fixed threshold (8), and never batch at or below it | [[gate]] |
| C-batch-attribution | invariant | a batched contract passes only when every structured-output test matched by its pattern reports passed; any matched test failed or skipped/todo emits a test-failing or no-tests-ran verdict for that contract | [[gate]] |
| C-batch-matched-zero | invariant | a batched contract whose pattern matches no test in the structured output emits no-tests-ran regardless of the batched invocation exit code | [[gate]] |
| C-batch-fallback | invariant | an unparseable, errored, or timed-out batched invocation emits a named fallback signal and re-runs its bindings per-contract with exit-code verdicts | [[gate]] |
| C-batch-timeout | invariant | the batched invocation runs under the maximum timeout_seconds across its batched entries and reports timed_out on expiry before falling back | [[gate]] |
| C-unbatched-unchanged | invariant | runner types without structured-reporter support, bindings with JS-only regex constructs, and sets at or below the threshold keep per-binding exit-code execution byte-identical | [[gate]] |

## Model

### States
- validating
- executing
- attributed
- reported

### Transitions

| id | from | to | event | guard |
|----|------|----|-------|-------|
| t-load | validating | executing | scope resolved and contracts loaded | [[gate.C-unbatched-unchanged]] |
| t-attribute | executing | attributed | batched or per-binding invocations produce per-binding results | [[gate.C-batch-attribution]] |
| t-report | attributed | reported | verdicts emit findings and matched-zero emits no-tests-ran | [[gate.C-batch-matched-zero]] |

## Properties

| id | kind | derives_from | generator | predicate |
|----|------|--------------|-----------|-----------|
| P-batch-count | unit | [[gate.C-batch-threshold]] | a corpus with more than 8 vitest bindings sharing one runner | exactly one runner invocation executes for that runner type; a corpus with 8 or fewer keeps per-binding spawns |
| P-attribution | unit | [[gate.C-batch-attribution]] | a batched contract whose pattern matches two tests where one fails | that contract emits test-failing while contracts whose matched tests all pass succeed |
| P-zero | unit | [[gate.C-batch-matched-zero]] | a batched invocation whose JSON output contains no test matched by one contract's pattern | that contract emits no-tests-ran while the invocation's exit code does not cover it |
| P-fallback | unit | [[gate.C-batch-fallback]] | a batched invocation whose structured output is truncated | a named fallback signal is emitted and every affected binding re-runs per-contract |
| P-timeout | unit | [[gate.C-batch-timeout]] | batched entries declaring timeouts of 10 and 30 seconds | the invocation runs under a 30-second bound and a hang reports timed_out before fallback |
| P-unchanged | unit | [[gate.C-unbatched-unchanged]] | a corpus of cargo and shell bindings | check output is byte-identical to pre-change behavior |

## Purpose

Cut runner startup cost from one process per contract to one process per
eligible runner type per run, measured at order-of-magnitude push-gate
savings on pattern-scoped corpora, while keeping every per-contract
falsification property (matched-zero, timeout, per-binding attribution)
exactly as strong as before.

## MODIFIED Requirements

### Requirement: Test Runner Execution
The system SHALL run every declared test binding and derive each binding's execution verdict from its command's exit code, or, when eligible bindings of one structured-output runner are batched into a single invocation, from that invocation's structured per-test output.

#### Scenario: Run configured unit test
- **GIVEN** `.espectacular/config.toml` maps `unit` to `["uv", "run", "pytest"]`
- **AND** a scenario contract declares `[[tests.unit]]` with `flags = "tests/test_parser.py::test_empty_input"`
- **WHEN** a user runs `ah check`
- **THEN** the command executes argv `["uv", "run", "pytest", "tests/test_parser.py::test_empty_input"]` without a shell from the repository root
- **AND** records the command exit code in JSON output
- **VERIFIES** [[gate.P-execution]]

#### Scenario: Run shell test
- **GIVEN** a scenario contract declares `[[tests.shell]]` with `command = "ah --version | grep -q 'ah '"`
- **WHEN** a user runs `ah check`
- **THEN** the command executes the shell command through `/bin/sh -c` from the repository root
- **AND** records the command exit code in JSON output
- **VERIFIES** [[gate.P-execution]]

#### Scenario: Enforce test timeout
- **GIVEN** a declared test command runs longer than its configured timeout
- **WHEN** a user runs `ah check`
- **THEN** the command stops the test command
- **AND** emits a `test-failing` execution finding with `timed_out = true`
- **AND** exits non-zero
- **VERIFIES** [[gate.P-execution]]

#### Scenario: Capture bounded output tails
- **GIVEN** a declared test command writes more than 8 KiB to stdout and stderr
- **WHEN** `ah check` emits JSON output
- **THEN** the execution finding includes only the final 8 KiB of stdout
- **AND** includes only the final 8 KiB of stderr
- **VERIFIES** [[gate.P-execution]]

#### Scenario: Missing runner fails structurally
- **GIVEN** a scenario contract declares `[[tests.integration]]`
- **AND** `.espectacular/config.toml` does not define `runners.integration`
- **WHEN** a user runs `ah check`
- **THEN** the command emits a `missing-runner` structural finding
- **AND** exits non-zero
- **VERIFIES** [[gate.P-execution]]

#### Scenario: Invalid TOML syntax fails structurally
- **GIVEN** a scenario contract file contains invalid TOML syntax
- **WHEN** a user runs `ah check`
- **THEN** the command emits a `malformed-contract` structural finding
- **AND** exits non-zero
- **VERIFIES** [[gate.P-execution]]

#### Scenario: Malformed test entry fails structurally
- **GIVEN** a non-shell test entry omits `flags`
- **WHEN** a user runs `ah check`
- **THEN** the command emits a `malformed-contract` structural finding
- **AND** exits non-zero
- **VERIFIES** [[gate.P-execution]]

#### Scenario: Non-zero declared test fails check
- **GIVEN** a declared test command exits non-zero
- **WHEN** a user runs `ah check`
- **THEN** the command emits a `test-failing` execution finding
- **AND** exits non-zero
- **VERIFIES** [[gate.P-execution]]

#### Scenario: Batch eligible bindings into one invocation
- **GIVEN** more than 8 contracts declare bindings for one runner type with structured-reporter support and pattern-only flags
- **WHEN** a user runs `ah check --run-tests`
- **THEN** exactly one runner invocation executes for that runner type with an OR-joined pattern and the structured reporter
- **VERIFIES** [[gate.P-batch-count]]

#### Scenario: Attribute batched contract from structured output
- **GIVEN** a batched contract whose pattern matches two tests where one fails
- **WHEN** a user runs `ah check --run-tests`
- **THEN** that contract emits a `test-failing` execution finding
- **AND** contracts whose matched tests all passed succeed
- **VERIFIES** [[gate.P-attribution]]

#### Scenario: Matched-zero batched contract still fails
- **GIVEN** a batched invocation whose structured output contains no test matched by one contract's pattern
- **WHEN** a user runs `ah check --run-tests`
- **THEN** that contract emits a `no-tests-ran` execution finding
- **AND** the batched invocation's exit code does not cover it
- **VERIFIES** [[gate.P-zero]]

#### Scenario: Batch fallback re-runs per binding
- **GIVEN** a batched invocation whose structured output is truncated and unparseable
- **WHEN** a user runs `ah check --run-tests`
- **THEN** a named fallback signal is emitted in JSON output
- **AND** every affected binding re-runs per-contract with exit-code verdicts
- **VERIFIES** [[gate.P-fallback]]

#### Scenario: Batched timeout bound
- **GIVEN** batched entries declare timeouts of 10 and 30 seconds
- **WHEN** the batched invocation hangs beyond 30 seconds
- **THEN** the invocation stops with `timed_out = true`
- **AND** every affected binding re-runs per-contract with exit-code verdicts
- **VERIFIES** [[gate.P-timeout]]

#### Scenario: Unbatched runners unchanged
- **GIVEN** a corpus of cargo and shell bindings
- **WHEN** a user runs `ah check --run-tests`
- **THEN** each binding spawns per-contract with exit-code verdicts
- **AND** the check output is byte-identical to pre-change behavior
- **VERIFIES** [[gate.P-unchanged]]

## Requirements

### Requirement: Test Runner Execution
The system SHALL run every declared test binding and derive each binding's execution verdict from its command's exit code, or, when eligible bindings of one structured-output runner are batched into a single invocation, from that invocation's structured per-test output.

#### Scenario: Run configured unit test
- **GIVEN** `.espectacular/config.toml` maps `unit` to `["uv", "run", "pytest"]`
- **AND** a scenario contract declares `[[tests.unit]]` with `flags = "tests/test_parser.py::test_empty_input"`
- **WHEN** a user runs `ah check`
- **THEN** the command executes argv `["uv", "run", "pytest", "tests/test_parser.py::test_empty_input"]` without a shell from the repository root
- **AND** records the command exit code in JSON output
- **VERIFIES** [[gate.P-execution]]

#### Scenario: Run shell test
- **GIVEN** a scenario contract declares `[[tests.shell]]` with `command = "ah --version | grep -q 'ah '"`
- **WHEN** a user runs `ah check`
- **THEN** the command executes the shell command through `/bin/sh -c` from the repository root
- **AND** records the command exit code in JSON output
- **VERIFIES** [[gate.P-execution]]

#### Scenario: Enforce test timeout
- **GIVEN** a declared test command runs longer than its configured timeout
- **WHEN** a user runs `ah check`
- **THEN** the command stops the test command
- **AND** emits a `test-failing` execution finding with `timed_out = true`
- **AND** exits non-zero
- **VERIFIES** [[gate.P-execution]]

#### Scenario: Capture bounded output tails
- **GIVEN** a declared test command writes more than 8 KiB to stdout and stderr
- **WHEN** `ah check` emits JSON output
- **THEN** the execution finding includes only the final 8 KiB of stdout
- **AND** includes only the final 8 KiB of stderr
- **VERIFIES** [[gate.P-execution]]

#### Scenario: Missing runner fails structurally
- **GIVEN** a scenario contract declares `[[tests.integration]]`
- **AND** `.espectacular/config.toml` does not define `runners.integration`
- **WHEN** a user runs `ah check`
- **THEN** the command emits a `missing-runner` structural finding
- **AND** exits non-zero
- **VERIFIES** [[gate.P-execution]]

#### Scenario: Invalid TOML syntax fails structurally
- **GIVEN** a scenario contract file contains invalid TOML syntax
- **WHEN** a user runs `ah check`
- **THEN** the command emits a `malformed-contract` structural finding
- **AND** exits non-zero
- **VERIFIES** [[gate.P-execution]]

#### Scenario: Malformed test entry fails structurally
- **GIVEN** a non-shell test entry omits `flags`
- **WHEN** a user runs `ah check`
- **THEN** the command emits a `malformed-contract` structural finding
- **AND** exits non-zero
- **VERIFIES** [[gate.P-execution]]

#### Scenario: Non-zero declared test fails check
- **GIVEN** a declared test command exits non-zero
- **WHEN** a user runs `ah check`
- **THEN** the command emits a `test-failing` execution finding
- **AND** exits non-zero
- **VERIFIES** [[gate.P-execution]]

#### Scenario: Batch eligible bindings into one invocation
- **GIVEN** more than 8 contracts declare bindings for one runner type with structured-reporter support and pattern-only flags
- **WHEN** a user runs `ah check --run-tests`
- **THEN** exactly one runner invocation executes for that runner type with an OR-joined pattern and the structured reporter
- **VERIFIES** [[gate.P-batch-count]]

#### Scenario: Attribute batched contract from structured output
- **GIVEN** a batched contract whose pattern matches two tests where one fails
- **WHEN** a user runs `ah check --run-tests`
- **THEN** that contract emits a `test-failing` execution finding
- **AND** contracts whose matched tests all passed succeed
- **VERIFIES** [[gate.P-attribution]]

#### Scenario: Matched-zero batched contract still fails
- **GIVEN** a batched invocation whose structured output contains no test matched by one contract's pattern
- **WHEN** a user runs `ah check --run-tests`
- **THEN** that contract emits a `no-tests-ran` execution finding
- **AND** the batched invocation's exit code does not cover it
- **VERIFIES** [[gate.P-zero]]

#### Scenario: Batch fallback re-runs per binding
- **GIVEN** a batched invocation whose structured output is truncated and unparseable
- **WHEN** a user runs `ah check --run-tests`
- **THEN** a named fallback signal is emitted in JSON output
- **AND** every affected binding re-runs per-contract with exit-code verdicts
- **VERIFIES** [[gate.P-fallback]]

#### Scenario: Batched timeout bound
- **GIVEN** batched entries declare timeouts of 10 and 30 seconds
- **WHEN** the batched invocation hangs beyond 30 seconds
- **THEN** the invocation stops with `timed_out = true`
- **AND** every affected binding re-runs per-contract with exit-code verdicts
- **VERIFIES** [[gate.P-timeout]]

#### Scenario: Unbatched runners unchanged
- **GIVEN** a corpus of cargo and shell bindings
- **WHEN** a user runs `ah check --run-tests`
- **THEN** each binding spawns per-contract with exit-code verdicts
- **AND** the check output is byte-identical to pre-change behavior
- **VERIFIES** [[gate.P-unchanged]]