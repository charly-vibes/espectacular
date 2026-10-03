---
id: spec
kind: intent
statement: "WHEN ah check validates a spec corpus THE gate SHALL discover scenarios, require one schema-valid sidecar contract each, execute declared tests with exit-code verdicts, emit stable JSON findings with agent-action fields, support deterministic change overlays and opt-in quality measurement plus an NR archetype and a conformance coverage matrix, never semantically evaluate test quality or scenario prose, derive one scenario contract per Properties row in a lint-clean dual-format spec, cover scenarios carrying a VERIFIES link via that property's contract, and detect derived-contract drift against its source row hash."

---

# gate Specification

## Constraints

| id | kind | expr | traces_to |
|----|------|------|-----------|
| C-scenario-discovery | invariant | scenarios are discovered from `#### Scenario:` headings in `spec.md` files including dual-format files; identical mirrored headings deduplicate to one; same slugified id with different bodies emits a slug-collision finding and exits non-zero; frontmatter and structured table layers contribute no scenarios | [[spec]] |
| C-sidecar-correspondence | invariant | each discovered scenario in scope requires exactly one TOML sidecar contract; missing, orphaned, mismatched-id, or empty-test-set contracts emit the corresponding structural finding and fail the gate | [[spec]] |
| C-contract-schema | invariant | contract metadata (`id`, `description`, `archetype`, `status`, `authored_with`) is validated before tests run; unknown `status` emits `invalid-status` and exits non-zero; `superseded` requires a non-empty `superseded_by` and still runs declared tests | [[spec]] |
| C-runner-execution | invariant | each declared test command runs with its exit code as the verdict: unit tests map through config runners, shell tests run inline, timeouts are enforced, output tails are bounded, missing runners / invalid TOML / malformed test entries are structural findings, and a non-zero declared test fails the check | [[spec]] |
| C-json-findings | invariant | `ah check` emits stable JSON: success with empty findings, findings in stable order, actionable scenario context with verbatim body boundaries, checked scope, and command details for execution findings | [[spec]] |
| C-overlay-scope | invariant | `ah check --changes <id>` overlays selected changes on deployed specs: added scenarios, staged metadata updates, and signaled modified scenario text apply; unsignaled redefinition, supersession with missing replacement, conflicting overlays, and conflicting staged updates for one scenario are rejected; resolution is deterministic | [[spec]] |
| C-nr-archetype | invariant | the `NR` (Non-Regression) archetype is a valid contract archetype, runs in change overlay scope, and `ah upgrade` reports it as an archetype addition | [[spec]] |
| C-scope-boundary | invariant | the gate never semantically evaluates test quality or scenario prose — it does not inspect test internals and does not hash scenario prose | [[spec]] |
| C-agent-actions | invariant | every finding carries `suggested_action` and `playbook_command`, `scenario_prose` is verbatim and untruncated, findings sort deterministically, and the summary counts by kind | [[spec]] |
| C-quality-measurement | invariant | opt-in quality measurement capabilities run during `ah check` emitting measurement findings without failing the gate on below-threshold scores in v1; property/snapshot command failure and mutation tool execution failure fail the gate; mutation is off in pre-commit scope by default | [[spec]] |
| C-quality-schema | invariant | quality measurement config (`[quality.mutation]`, property, snapshot) is not a test entry, while property and snapshot declarations are runnable test entries — baseline `tests.<type>` arrays stay runnable declarations | [[spec]] |
| C-coverage-matrix | invariant | a per-spec, per-archetype coverage matrix aggregates scenario contract status across all specs in scope, counts covered scenarios and archetype totals, reports missing contracts as uncovered, and is available machine-readable | [[spec]] |
| C-apply-command | invariant | `apply_command` is set only when the finding's `suggested_action` maps to a concrete mechanical shell command (e.g., `enable_capability`), and is null for findings requiring human review or code edits | [[spec]] |
| C-property-contract | invariant | in a dual-format spec whose specodelic half is spk lint-clean, each Properties row derives exactly one scenario contract whose id is the slugified property id | [[spec]] |
| C-verifies-covers | invariant | a scenario carrying a VERIFIES bullet linking to an existing property id is covered by that property's derived contract and emits no no-tests-declared finding | [[spec]] |
| C-unlinked-unchanged | invariant | a scenario without a VERIFIES link keeps the per-scenario contract requirement and may still emit no-tests-declared | [[spec]] |
| C-derived-stale | invariant | a derived contract whose derived_from hash does not match the canonical serialization of its source property row emits a contract-stale structural finding | [[spec]] |
| C-sync-ownership | invariant | ah sync refreshes only derived fields (id, description, archetype, falsifiability_class, derived_from) and never overwrites human-owned fields (tests, status, superseded_by) | [[spec]] |
| C-plain-unaffected | advisory | plain openspec specs and dual-format specs without Properties rows discover, gate, and check byte-identically to before this change | [[spec]] |

## Model

### States

- `discovering`
- `validating`
- `executing`
- `reporting`
- `deriving`

### Transitions

| id | from | to | guard |
|----|------|----|-------|
| t-discover | discovering | discovering | [[spec.C-scenario-discovery]] ([[spec.C-overlay-scope]] widens scope) |
| t-correspond | discovering | validating | [[spec.C-sidecar-correspondence]] |
| t-schema | validating | validating | [[spec.C-contract-schema]] |
| t-execute | validating | executing | [[spec.C-runner-execution]] ([[spec.C-nr-archetype]] and [[spec.C-quality-schema]] shape what runs) |
| t-quality | executing | executing | [[spec.C-quality-measurement]] |
| t-report | executing | reporting | [[spec.C-json-findings]] AND [[spec.C-agent-actions]] AND [[spec.C-coverage-matrix]] AND [[spec.C-apply-command]] |
| t-bound | reporting | reporting | [[spec.C-scope-boundary]] |
| t-derive | discovering | deriving | [[spec.C-property-contract]] |
| t-match | deriving | validating | [[spec.C-verifies-covers]] |
| t-stale | deriving | reporting | [[spec.C-derived-stale]] |
| t-own | deriving | validating | [[spec.C-sync-ownership]] |
| t-unlinked | discovering | validating | [[spec.C-unlinked-unchanged]] |

## Properties

| id | kind | derives_from | generator | predicate |
|----|------|--------------|-----------|-----------|
| P-discovery | unit | [[spec.C-scenario-discovery]] | deployed and dual-format specs, mirrored deltas, and same-id/different-body collisions | one scenario per logical scenario; collisions fail with findings; structured layers inert |
| P-correspondence | unit | [[spec.C-sidecar-correspondence]] | scenarios with missing, orphaned, mismatched, and empty contracts | each emits its structural finding and fails the gate |
| P-schema | unit | [[spec.C-contract-schema]] | contracts with valid, unknown, and superseded status | validation precedes execution; invalid status fails; superseded runs only with a replacement |
| P-execution | unit | [[spec.C-runner-execution]] | unit, shell, timeout, bounded-output, structural-breakage, and failing-test contracts | exit-code verdicts with bounded tails; structural breakage fails before running; non-zero fails the check |
| P-json | unit | [[spec.C-json-findings]] | check runs from clean to finding-rich scopes | stable ordering, verbatim bodies, scope and command details present |
| P-overlay | unit | [[spec.C-overlay-scope]] | overlays applying additions, staged updates, signaled modifications, and each rejection case | legal overlays apply deterministically; every illegal case is rejected |
| P-nr | unit | [[spec.C-nr-archetype]] | an NR contract in overlay scope and an `ah upgrade` run | NR validates, runs in scope, and upgrades report the archetype addition |
| P-boundary | unit | [[spec.C-scope-boundary]] | a passing test with poor quality and prose mutations | verdicts and outputs are unchanged by test internals or prose content |
| P-agent-actions | unit | [[spec.C-agent-actions]] | any finding-producing check run | every finding has both agent fields; prose verbatim; deterministic sort; kind counts in summary |
| P-quality | unit | [[spec.C-quality-measurement]] | enabled mutation, declared property/snapshot, threshold crossings, tool failures, pre-commit scope | measurement findings never fail on scores in v1; tool/command failures do; mutation off pre-commit |
| P-quality-schema | unit | [[spec.C-quality-schema]] | contracts with quality blocks and quality-declared runnable entries | quality config never counted as a test entry; property/snapshot run as declared tests |
| P-matrix | unit | [[spec.C-coverage-matrix]] | multi-spec scopes with missing contracts | matrix counts coverage per spec and archetype, marks missing as uncovered, and is machine-readable |
| P-apply-command | unit | [[spec.C-apply-command]] | findings across suggested_action classes | `apply_command` non-null only for mechanical actions |
| P-derive | unit | [[spec.C-property-contract]] | a lint-clean dual-format file with N property rows | the contract set contains exactly N derived contracts with slugified ids |
| P-covers | unit | [[spec.C-verifies-covers]] | a scenario carrying a VERIFIES link to an existing property id | no-tests-declared is suppressed for that scenario |
| P-unlinked | unit | [[spec.C-unlinked-unchanged]] | a scenario without a VERIFIES link | the gate requires its own contract exactly as before this change |
| P-stale | unit | [[spec.C-derived-stale]] | a derived contract whose source row predicate is edited after sync | contract-stale is emitted and ah sync --check exits non-zero without writing |
| P-ownership | unit | [[spec.C-sync-ownership]] | a derived contract with hand-filled tests and status, re-synced after a predicate edit | derived fields change; tests, status, and superseded_by are byte-identical |
| P-unaffected | unit | [[spec.C-plain-unaffected]] | a corpus of plain openspec specs and Properties-less dual-format specs | discovery and check output is byte-identical to pre-change behavior |

## Non-Goals

- Evaluating test quality, assertion strength, or scenario prose semantics — the gate only checks correspondence and runs declared commands (see C-scope-boundary)
- Authoring or repairing the tests inside scenario contracts — test content is human/agent-owned and never generated by the gate or `ah sync`
- Reimplementing specodelic lint, graph, or derivation rules — the gate consumes `spk` output verbatim and degrades silently when `spk` is unavailable
- CI dashboards, trend storage, or historical drift tracking — `ah report` and `ah check` emit point-in-time snapshots only

## Purpose
The behavioral verification gate: scenario discovery, sidecar correspondence, contract schema validation, test execution with exit-code verdicts, stable JSON findings, deterministic change overlays, opt-in quality measurement, the NR archetype, and the conformance coverage matrix. Since derive-contracts-from-specodelic, the gate also derives one scenario contract per Properties row of a lint-clean dual-format spec, covers VERIFIES-linked scenarios via that property's contract, and detects derived-contract drift against the source row hash.
## Requirements
### Requirement: Scenario Discovery
The system SHALL discover OpenSpec scenarios from `#### Scenario:` headings in `spec.md` files, including dual-format files that additionally carry the specodelic four-layer grammar, deduplicating section-sync mirrors and ignoring structured layers.

#### Scenario: Discover deployed scenario
- **GIVEN** `openspec/specs/compiler/spec.md` contains `#### Scenario: Empty input rejected`
- **WHEN** `ah check` scans deployed specs
- **THEN** it discovers a scenario with id `empty-input-rejected`
- **AND** associates it with the `compiler` spec
- **VERIFIES** [[spec.P-discovery]]

#### Scenario: Reject duplicate scenario ids
- **GIVEN** two scenarios in the same spec slugify to the same id **and have different bodies**
- **WHEN** `ah check` validates the spec
- **THEN** it emits a structural finding for the slug collision
- **AND** exits non-zero
- **VERIFIES** [[spec.P-discovery]]

#### Scenario: Deduplicate mirrored delta sections
- **GIVEN** a change delta carries identical `#### Scenario:` headings with identical bodies in both `## ADDED Requirements` and the mirrored `## Requirements` section
- **WHEN** a user runs `ah check --changes <change-id>`
- **THEN** each mirrored scenario is discovered exactly once
- **AND** the gate emits no slug-collision finding for the mirror
- **VERIFIES** [[spec.P-discovery]]

#### Scenario: Parse dual-format deployed spec
- **GIVEN** a deployed spec under `openspec/specs/` carries YAML frontmatter and specodelic tables in addition to its `## Requirements` section
- **WHEN** `ah check` scans deployed specs
- **THEN** scenarios are discovered from the `#### Scenario:` headings exactly as before
- **AND** frontmatter, constraint rows, model states and transitions, and property rows contribute no scenarios
- **VERIFIES** [[spec.P-discovery]]

#### Scenario: Plain openspec specs remain valid
- **GIVEN** a spec file contains no frontmatter and no specodelic tables
- **WHEN** `ah check` scans the spec
- **THEN** discovery behaves identically to before this change
- **VERIFIES** [[spec.P-discovery]]

### Requirement: Sidecar Contract Correspondence
The system SHALL require exactly one TOML sidecar contract for each discovered scenario in scope.

#### Scenario: Missing contract fails
- **GIVEN** a scenario id `empty-input-rejected` exists under `openspec/specs/compiler/spec.md`
- **AND** `.espectacular/compiler/empty-input-rejected.toml` does not exist
- **WHEN** a user runs `ah check`
- **THEN** the command emits a `no-toml` structural finding
- **AND** exits non-zero
- **VERIFIES** [[spec.P-correspondence]]

#### Scenario: Orphan contract fails
- **GIVEN** `.espectacular/compiler/empty-input-rejected.toml` exists
- **AND** no matching scenario exists under `openspec/specs/compiler/spec.md`
- **WHEN** a user runs `ah check`
- **THEN** the command emits an `orphan-toml` structural finding
- **AND** exits non-zero
- **VERIFIES** [[spec.P-correspondence]]

#### Scenario: Contract id mismatch fails
- **GIVEN** `.espectacular/compiler/empty-input-rejected.toml` contains `id = "different-id"`
- **AND** the matching scenario slug is `empty-input-rejected`
- **WHEN** a user runs `ah check`
- **THEN** the command emits an `id-mismatch` structural finding
- **AND** exits non-zero
- **VERIFIES** [[spec.P-correspondence]]

#### Scenario: Empty test set fails
- **GIVEN** a scenario contract declares no tests
- **WHEN** a user runs `ah check`
- **THEN** the command emits a `no-tests-declared` structural finding
- **AND** exits non-zero
- **VERIFIES** [[spec.P-correspondence]]

### Requirement: Contract Schema
The system SHALL validate per-scenario TOML contracts before running tests. Contracts MAY declare an optional `falsifiability_class` field with value `safety` or `liveness`; an invalid value is a structural finding, and a `liveness`-tagged contract whose test entries all lack `timeout_seconds` produces a warning that does not fail the gate.

#### Scenario: Validate scenario metadata
- **GIVEN** a scenario contract contains `id`, `description`, `archetype`, `status`, and `authored_with`
- **WHEN** a user runs `ah check`
- **THEN** the command validates the metadata fields before executing tests
- **VERIFIES** [[spec.P-correspondence]]

#### Scenario: Reject unknown status
- **GIVEN** a scenario contract has `status = "paused"`
- **WHEN** a user runs `ah check`
- **THEN** the command emits an `invalid-status` structural finding
- **AND** exits non-zero
- **VERIFIES** [[spec.P-schema]]

#### Scenario: Validate superseded status
- **GIVEN** a scenario contract has `status = "superseded"`
- **WHEN** a user runs `ah check`
- **THEN** the command requires a non-empty `superseded_by` value
- **AND** still runs the scenario's declared tests
- **VERIFIES** [[spec.P-schema]]

#### Scenario: Accept contract without falsifiability_class
- **GIVEN** a scenario contract declares no `falsifiability_class` field
- **WHEN** a user runs `ah check`
- **THEN** the contract validates and behaves exactly as it did before the field existed
- **AND** no finding related to `falsifiability_class` is emitted
- **VERIFIES** [[spec.P-schema]]

#### Scenario: Accept valid falsifiability_class values
- **GIVEN** a scenario contract has `falsifiability_class = "safety"` (or `"liveness"`)
- **WHEN** a user runs `ah check`
- **THEN** the contract validates and runs its declared tests normally
- **VERIFIES** [[spec.P-schema]]

#### Scenario: Reject invalid falsifiability_class value
- **GIVEN** a scenario contract has `falsifiability_class = "eventual"`
- **WHEN** a user runs `ah check`
- **THEN** the command emits an `invalid-falsifiability-class` structural finding
- **AND** exits non-zero without running tests
- **VERIFIES** [[spec.P-schema]]

#### Scenario: Warn on liveness contract without any test timeout
- **GIVEN** a scenario contract has `falsifiability_class = "liveness"`
- **AND** every declared test entry omits `timeout_seconds`
- **WHEN** a user runs `ah check`
- **THEN** the command emits a `missing-liveness-timeout` finding with `severity = "warning"`
- **AND** the finding suggests adding explicit timeout semantics, since liveness claims are only falsifiable under bounded execution
- **VERIFIES** [[spec.P-schema]]

#### Scenario: No warning when liveness contract declares a timeout
- **GIVEN** a scenario contract has `falsifiability_class = "liveness"`
- **AND** at least one declared test entry specifies `timeout_seconds`
- **WHEN** a user runs `ah check`
- **THEN** no `missing-liveness-timeout` finding is emitted
- **VERIFIES** [[spec.P-schema]]

#### Scenario: No liveness warning when the contract declares no tests
- **GIVEN** a scenario contract has `falsifiability_class = "liveness"`
- **AND** the contract declares no test entries at all
- **WHEN** a user runs `ah check`
- **THEN** the command emits a `no-tests-declared` structural finding
- **AND** no `missing-liveness-timeout` warning is emitted
- **VERIFIES** [[spec.P-schema]]

#### Scenario: Warnings do not fail the gate
- **GIVEN** `ah check` produces only `missing-liveness-timeout` warnings and no error-severity findings
- **WHEN** the run completes
- **THEN** the command exits zero
- **VERIFIES** [[spec.P-schema]]

### Requirement: Test Runner Execution
The system SHALL run each declared test command and use its exit code as the execution verdict.

#### Scenario: Run configured unit test
- **GIVEN** `.espectacular/config.toml` maps `unit` to `["uv", "run", "pytest"]`
- **AND** a scenario contract declares `[[tests.unit]]` with `flags = "tests/test_parser.py::test_empty_input"`
- **WHEN** a user runs `ah check`
- **THEN** the command executes argv `["uv", "run", "pytest", "tests/test_parser.py::test_empty_input"]` without a shell from the repository root
- **AND** records the command exit code in JSON output
- **VERIFIES** [[spec.P-execution]]

#### Scenario: Run shell test
- **GIVEN** a scenario contract declares `[[tests.shell]]` with `command = "ah --version | grep -q 'ah '"`
- **WHEN** a user runs `ah check`
- **THEN** the command executes the shell command through `/bin/sh -c` from the repository root
- **AND** records the command exit code in JSON output
- **VERIFIES** [[spec.P-execution]]

#### Scenario: Enforce test timeout
- **GIVEN** a declared test command runs longer than its configured timeout
- **WHEN** a user runs `ah check`
- **THEN** the command stops the test command
- **AND** emits a `test-failing` execution finding with `timed_out = true`
- **AND** exits non-zero
- **VERIFIES** [[spec.P-execution]]

#### Scenario: Capture bounded output tails
- **GIVEN** a declared test command writes more than 8 KiB to stdout and stderr
- **WHEN** `ah check` emits JSON output
- **THEN** the execution finding includes only the final 8 KiB of stdout
- **AND** includes only the final 8 KiB of stderr
- **VERIFIES** [[spec.P-execution]]

#### Scenario: Missing runner fails structurally
- **GIVEN** a scenario contract declares `[[tests.integration]]`
- **AND** `.espectacular/config.toml` does not define `runners.integration`
- **WHEN** a user runs `ah check`
- **THEN** the command emits a `missing-runner` structural finding
- **AND** exits non-zero
- **VERIFIES** [[spec.P-execution]]

#### Scenario: Invalid TOML syntax fails structurally
- **GIVEN** a scenario contract file contains invalid TOML syntax
- **WHEN** a user runs `ah check`
- **THEN** the command emits a `malformed-contract` structural finding
- **AND** exits non-zero
- **VERIFIES** [[spec.P-execution]]

#### Scenario: Malformed test entry fails structurally
- **GIVEN** a non-shell test entry omits `flags`
- **WHEN** a user runs `ah check`
- **THEN** the command emits a `malformed-contract` structural finding
- **AND** exits non-zero
- **VERIFIES** [[spec.P-execution]]

#### Scenario: Non-zero declared test fails check
- **GIVEN** a declared test command exits non-zero
- **WHEN** a user runs `ah check`
- **THEN** the command emits a `test-failing` execution finding
- **AND** exits non-zero
- **VERIFIES** [[spec.P-execution]]

### Requirement: JSON Findings
The system SHALL emit stable JSON output for `ah check` results.

#### Scenario: Report success with empty findings
- **GIVEN** every scenario in scope has a valid contract
- **AND** every declared test command exits zero
- **WHEN** a user runs `ah check`
- **THEN** the command exits zero
- **AND** emits JSON with `findings = []`
- **VERIFIES** [[spec.P-json]]

#### Scenario: Report all findings in stable order
- **GIVEN** multiple scenarios have findings
- **WHEN** a user runs `ah check`
- **THEN** the JSON output includes all findings
- **AND** orders them by spec path and scenario id
- **VERIFIES** [[spec.P-json]]

#### Scenario: Include actionable scenario context
- **GIVEN** a scenario has a finding
- **WHEN** `ah check` emits JSON output
- **THEN** the finding includes the scenario id, spec path, scenario title, and scenario body markdown
- **VERIFIES** [[spec.P-json]]

#### Scenario: Extract scenario body boundaries
- **GIVEN** a scenario heading is followed by markdown body lines and then another `####` heading
- **WHEN** `ah check` emits JSON output for that scenario
- **THEN** `body_markdown` contains only the lines after the scenario heading and before the next heading whose level is `####` or higher
- **VERIFIES** [[spec.P-json]]

#### Scenario: Include checked scope
- **WHEN** `ah check` emits JSON output
- **THEN** the top-level JSON includes whether deployed specs were checked
- **AND** includes any selected OpenSpec changes
- **VERIFIES** [[spec.P-json]]

#### Scenario: Include command details for execution findings
- **GIVEN** a declared test command exits non-zero
- **WHEN** `ah check` emits JSON output
- **THEN** the finding includes the test type, command, exit code, timeout flag, stdout tail, and stderr tail when available
- **VERIFIES** [[spec.P-json]]

### Requirement: Change Overlay Scope
The system SHALL support checking selected OpenSpec changes as overlays on deployed specs.

#### Scenario: Check selected change overlay
- **GIVEN** `openspec/changes/add-parser/specs/compiler/spec.md` adds a scenario
- **AND** `.espectacular/changes/add-parser/compiler/<scenario>.toml` exists
- **WHEN** a user runs `ah check --changes add-parser`
- **THEN** the command validates the deployed compiler spec plus the `add-parser` scenario overlay
- **VERIFIES** [[spec.P-overlay]]

#### Scenario: Apply staged metadata update for deployed scenario
- **GIVEN** `.espectacular/changes/add-parser/compiler/old-behavior.toml` has `status = "superseded"`
- **AND** deployed spec `compiler` contains scenario `old-behavior`
- **WHEN** a user runs `ah check --changes add-parser`
- **THEN** the command validates the staged contract as the active contract for `old-behavior` in the overlay
- **VERIFIES** [[spec.P-overlay]]

#### Scenario: Apply modified scenario text from change overlay
- **GIVEN** a change delta defines scenario `old-behavior` for spec `compiler`, which exists in the deployed specs, with different body text
- **AND** the change stages a contract at `.espectacular/changes/<change>/compiler/old-behavior.toml`
- **WHEN** a user runs `ah check --changes <change>`
- **THEN** the overlay scenario text replaces the deployed scenario text in scope
- **AND** no `overlay-conflict` finding is emitted
- **VERIFIES** [[spec.P-overlay]]

#### Scenario: Reject unsignaled scenario redefinition
- **GIVEN** a change delta defines scenario `old-behavior` for spec `compiler`, which exists in the deployed specs, with different body text
- **AND** the change stages no contract for `old-behavior`
- **WHEN** a user runs `ah check --changes <change>`
- **THEN** the command emits an `overlay-conflict` structural finding
- **AND** exits non-zero
- **VERIFIES** [[spec.P-overlay]]

#### Scenario: Reject supersession with missing replacement
- **GIVEN** `.espectacular/changes/add-parser/compiler/old-behavior.toml` has `status = "superseded"`
- **AND** `superseded_by = "new-behavior"`
- **AND** no scenario `new-behavior` exists in deployed specs or the selected change overlay
- **WHEN** a user runs `ah check --changes add-parser`
- **THEN** the command emits a structural finding for the missing replacement scenario
- **AND** exits non-zero
- **VERIFIES** [[spec.P-overlay]]

#### Scenario: Reject conflicting overlays
- **GIVEN** two selected changes define the same new scenario id for the same spec
- **WHEN** a user runs `ah check --changes first --changes second`
- **THEN** the command emits a structural finding for the conflict
- **AND** exits non-zero
- **VERIFIES** [[spec.P-overlay]]

#### Scenario: Reject conflicting staged updates for one deployed scenario
- **GIVEN** two selected changes both stage metadata updates for the same deployed scenario id in the same spec
- **WHEN** a user runs `ah check --changes first --changes second`
- **THEN** the command emits an `overlay-conflict` structural finding
- **AND** exits non-zero
- **VERIFIES** [[spec.P-overlay]]

#### Scenario: Overlay resolution is deterministic
- **GIVEN** selected changes do not conflict
- **WHEN** a user runs `ah check --changes zeta --changes alpha`
- **THEN** the command resolves selected changes in sorted change-id order
- **AND** produces the same validation scope as `ah check --changes alpha --changes zeta`
- **VERIFIES** [[spec.P-overlay]]

### Requirement: Non-Regression Archetype
The system SHALL support an `NR` (Non-Regression) archetype for contracts that assert existing behavior is preserved during change proposals.

#### Scenario: NR contract is valid
- **GIVEN** a scenario contract has `archetype = "NR"`
- **WHEN** a user runs `ah check`
- **THEN** the gate accepts `NR` as a valid archetype value
- **AND** validates and runs the contract's declared tests identically to other archetypes
- **VERIFIES** [[spec.P-nr]]

#### Scenario: NR contract runs in change overlay scope
- **GIVEN** a change proposal modifies a capability
- **AND** an existing scenario is covered by a contract with `archetype = "NR"`
- **WHEN** a user runs `ah check --changes <change-id>`
- **THEN** the NR contract is validated as part of the overlay scope
- **AND** a failing NR test exits non-zero
- **VERIFIES** [[spec.P-nr]]

#### Scenario: ah upgrade reports NR as archetype addition
- **GIVEN** `.espectacular/config.toml` pins a tool version that predates `NR` support
- **WHEN** a user runs `ah upgrade`
- **THEN** the command reports `NR` as a newly available archetype before updating the configured tool version
- **VERIFIES** [[spec.P-nr]]

### Requirement: Deterministic Scope Boundary
The system SHALL avoid semantic evaluation of test quality or scenario prose.

#### Scenario: Do not inspect test internals
- **GIVEN** a declared test command exists and exits zero
- **WHEN** a user runs `ah check`
- **THEN** the command treats the test as passing
- **AND** does not inspect assertions, fixtures, mocks, or setup code
- **VERIFIES** [[spec.P-boundary]]

#### Scenario: Do not hash scenario prose
- **GIVEN** the body text under an existing scenario heading changes
- **WHEN** a user runs `ah check`
- **THEN** the command does not fail solely because the prose changed
- **VERIFIES** [[spec.P-boundary]]

### Requirement: JSON finding schema includes agent-action fields
The system SHALL include agent-action fields on every finding in the JSON output.

#### Scenario: Every finding carries suggested_action
- **GIVEN** `ah check` produces any finding
- **WHEN** the JSON output is inspected
- **THEN** every finding object contains a `suggested_action` field with a value from the documented enum
- **VERIFIES** [[spec.P-agent-actions]]

#### Scenario: Every finding carries playbook_command
- **GIVEN** `ah check` produces any finding
- **WHEN** the JSON output is inspected
- **THEN** every finding object contains a `playbook_command` field with a valid `ah explain <topic>` invocation
- **VERIFIES** [[spec.P-agent-actions]]

#### Scenario: scenario_prose is verbatim and untruncated
- **GIVEN** a finding references a scenario
- **WHEN** the JSON output is inspected
- **THEN** the `scenario_prose` field contains the full markdown body of the scenario heading, verbatim, without truncation
- **VERIFIES** [[spec.P-agent-actions]]

#### Scenario: Findings are sorted deterministically
- **GIVEN** `ah check` produces multiple findings
- **WHEN** the JSON output is inspected
- **THEN** the `findings` array is sorted by `(spec_path, scenario_id, kind)` in ascending lexicographic order
- **VERIFIES** [[spec.P-agent-actions]]

#### Scenario: Summary counts by kind
- **GIVEN** `ah check` produces findings of multiple kinds
- **WHEN** the JSON output is inspected
- **THEN** the envelope `summary.counts_by_kind` object contains the count of each finding kind present
- **VERIFIES** [[spec.P-agent-actions]]

#### Scenario: Quality findings carry no scenario prose and never fail the gate
- **GIVEN** `ah check` produces only a quality finding whose mutation tool has no owning scenario
- **WHEN** the JSON output is inspected
- **THEN** the quality finding carries its message and action fields with `scenario_prose = null`
- **AND** no scenario body is fabricated for it
- **AND** the run exits zero — a scenario-less quality finding fails nothing
- **VERIFIES** [[spec.P-agent-actions]]

### Requirement: Quality measurement capabilities
The system SHALL support opt-in quality measurement capabilities that run during `ah check` and emit measurement findings without failing the gate.

#### Scenario: Mutation testing runs when enabled
- **GIVEN** a contract declares `[quality.mutation] enabled = true`
- **AND** a mutation tool is configured in `.espectacular/config.toml`
- **WHEN** a user runs `ah check --mutation`
- **THEN** the gate runs the mutation tool against the contract's declared tests
- **AND** emits a `quality-mutation` info finding with the measured score
- **AND** exits zero when the score is below any configured threshold
- **VERIFIES** [[spec.P-quality]]

#### Scenario: Property-based testing runs when declared
- **GIVEN** a contract declares a `tests.property` entry
- **WHEN** a user runs `ah check`
- **THEN** the gate runs the property test command
- **AND** emits a `quality-property` finding with the run result
- **VERIFIES** [[spec.P-quality]]

#### Scenario: Snapshot testing runs when declared
- **GIVEN** a contract declares a `tests.snapshot` entry
- **WHEN** a user runs `ah check`
- **THEN** the gate runs the snapshot test command
- **AND** emits a `quality-snapshot` finding with the run result
- **VERIFIES** [[spec.P-quality]]

#### Scenario: Quality scores below threshold do not fail the gate in v1
- **GIVEN** a quality measurement capability completes successfully and produces a score below threshold
- **WHEN** a user runs `ah check`
- **THEN** the finding severity is `warning` or `info`
- **AND** the overall exit status is zero
- **VERIFIES** [[spec.P-quality]]

#### Scenario: Property or snapshot command failure fails the gate
- **GIVEN** a contract declares `[[tests.property]]` or `[[tests.snapshot]]`
- **AND** the declared command exits non-zero or times out
- **WHEN** a user runs `ah check`
- **THEN** the command emits a `test-failing` execution finding
- **AND** the overall exit status is non-zero
- **VERIFIES** [[spec.P-quality]]

#### Scenario: Mutation tool execution failure fails the gate
- **GIVEN** mutation measurement is enabled and the mutation tool command exits non-zero before producing a measurement
- **WHEN** a user runs `ah check --mutation`
- **THEN** the command emits a `test-failing` execution finding
- **AND** the overall exit status is non-zero
- **VERIFIES** [[spec.P-quality]]

#### Scenario: Mutation is off in pre-commit scope by default
- **GIVEN** mutation testing is configured
- **AND** `ah check` is invoked without an explicit `--mutation` flag
- **WHEN** the command runs in pre-commit mode
- **THEN** mutation testing is skipped
- **VERIFIES** [[spec.P-quality]]

### Requirement: Quality contract schema
The system SHALL represent quality measurements without changing the baseline rule that `tests.<type>` entries are arrays of runnable test declarations.

#### Scenario: Mutation configuration is not a test entry
- **GIVEN** mutation measurement is enabled for a scenario contract
- **WHEN** the contract is validated
- **THEN** mutation settings are read from a `[quality.mutation]` table
- **AND** `tests.mutation` as a boolean is rejected as a malformed contract
- **VERIFIES** [[spec.P-quality-schema]]

#### Scenario: Property and snapshot are runnable test entries
- **GIVEN** a scenario contract declares `[[tests.property]]` or `[[tests.snapshot]]`
- **WHEN** the contract is validated
- **THEN** each entry follows the same runnable test-entry shape as other `tests.<type>` arrays
- **VERIFIES** [[spec.P-quality-schema]]

### Requirement: Conformance coverage matrix
The system SHALL compute a per-spec, per-archetype coverage matrix aggregating scenario contract status across all specs in scope.

#### Scenario: Matrix counts covered scenarios
- **GIVEN** `openspec/specs/` contains multiple specs, each with scenarios that have contracts
- **WHEN** a user runs `ah report`
- **THEN** the command emits a matrix row for each spec with columns for each archetype
- **AND** each cell contains `covered`, `missing`, and `failing` counts
- **VERIFIES** [[spec.P-matrix]]

#### Scenario: Matrix includes archetype totals
- **GIVEN** `ah report` runs against deployed specs
- **WHEN** the output is inspected
- **THEN** the matrix includes a totals row summing counts across all specs
- **VERIFIES** [[spec.P-matrix]]

#### Scenario: Missing contracts appear as uncovered
- **GIVEN** a deployed scenario has no sidecar contract
- **WHEN** `ah report` runs
- **THEN** the scenario is counted as `missing` for its spec row
- **AND** the `archetype` column is `unassigned`
- **VERIFIES** [[spec.P-matrix]]

#### Scenario: Machine-readable matrix output
- **WHEN** a user runs `ah report --json`
- **THEN** the command emits a JSON object with a `matrix` array
- **AND** each row contains `spec`, `archetype`, `covered`, `missing`, and `failing` integer fields
- **VERIFIES** [[spec.P-matrix]]

### Requirement: apply_command is conditionally present
The system SHALL set `apply_command` only when the finding's `suggested_action` maps to a concrete, mechanical shell command; it SHALL be null for findings that require non-mechanical human action.

#### Scenario: apply_command is set for enable_capability findings
- **GIVEN** `ah check` or `ah doctor` produces a finding with `suggested_action = enable_capability`
- **WHEN** the JSON output is inspected
- **THEN** `apply_command` contains the `ah doctor --enable <capability>` invocation
- **VERIFIES** [[spec.P-apply-command]]

#### Scenario: apply_command is null for human_review_required findings
- **GIVEN** `ah check` produces a finding with `suggested_action = human_review_required`
- **WHEN** the JSON output is inspected
- **THEN** `apply_command` is null or absent
- **VERIFIES** [[spec.P-apply-command]]

#### Scenario: apply_command is null for edit_code_not_scenario findings
- **GIVEN** `ah check` produces a finding with `suggested_action = edit_code_not_scenario`
- **WHEN** the JSON output is inspected
- **THEN** `apply_command` is null or absent
- **VERIFIES** [[spec.P-apply-command]]

#### Scenario: apply_command is null for run_ah_sync findings
- **GIVEN** `ah check` produces a `contract-stale` finding with `suggested_action = run_ah_sync`
- **WHEN** the JSON output is inspected
- **THEN** `apply_command` is null or absent — refreshing derived fields writes TOML, and a command that mutates contract files is refused the mechanical fast-path
- **VERIFIES** [[spec.P-apply-command]]

#### Scenario: review_and_apply findings carry no apply_command
- **GIVEN** `ah check` produces a structural finding with `suggested_action = review_and_apply` (e.g. a missing sidecar contract)
- **WHEN** the JSON output is inspected
- **THEN** `apply_command` is null or absent — correspondence repair requires judgment, and a fabricated one-liner would fail the mechanical-only contract
- **AND** no shell invocation is fabricated for it
- **VERIFIES** [[spec.P-apply-command]]

### Requirement: Property-Derived Contracts
The system SHALL derive one scenario contract per Properties row in a dual-format spec whose specodelic half is lint-clean, using the slugified property id as the contract id, and SHALL treat a scenario carrying a `**VERIFIES** [[spec.P-...]]` link as covered by that property's contract.

#### Scenario: Derive contracts from properties
- **GIVEN** a dual-format spec is `spk lint`-clean and carries three Properties rows
- **WHEN** `ah sync` runs
- **THEN** exactly three derived contracts exist with slugified property ids
- **AND** each records `derived_from = "<property-id>@<hash>"`
- **VERIFIES** [[spec.P-derive]]
#### Scenario: Cover linked scenario
- **GIVEN** a scenario carries `- **VERIFIES** [[spec.P-not-shipped]]` and the property's derived contract exists
- **WHEN** `ah check` validates the spec
- **THEN** the scenario emits no `no-tests-declared` finding
- **VERIFIES** [[spec.P-covers]]
#### Scenario: Unlinked scenario unchanged
- **GIVEN** a scenario carries no `VERIFIES` link
- **WHEN** `ah check` validates the spec
- **THEN** the scenario requires its own contract exactly as before this change
- **VERIFIES** [[spec.P-unlinked]]
#### Scenario: Plain specs unaffected
- **GIVEN** a corpus of plain openspec specs and dual-format specs without Properties rows
- **WHEN** discovery and check run over the corpus
- **THEN** their output is byte-identical to pre-change behavior
- **VERIFIES** [[spec.P-unaffected]]

#### Scenario: Refuse derivation from lint-dirty spec
- **GIVEN** a dual-format spec whose specodelic half has a `spk lint` violation (e.g. a duplicate row id)
- **WHEN** `ah sync` runs
- **THEN** the command refuses the file with a named refusal citing the specodelic rule
- **AND** no contracts are derived or written for that spec
- **VERIFIES** [[spec.P-derive]]
### Requirement: Derived Contract Drift
The system SHALL record `derived_from = "<property-id>@<hash>"` in each derived contract, emit a `contract-stale` structural finding when the hash no longer matches the source row's canonical serialization, and refresh only derived fields during `ah sync` — never `tests`, `status`, or `superseded_by`.

#### Scenario: Detect stale contract
- **GIVEN** a property row's predicate is edited after its contract was derived
- **WHEN** `ah check` validates the spec
- **THEN** a `contract-stale` structural finding is emitted
- **AND** `ah sync --check` exits non-zero without writing
- **VERIFIES** [[spec.P-stale]]
#### Scenario: Refresh preserves owned fields
- **GIVEN** a derived contract has hand-filled `tests` and `status`
- **WHEN** the source predicate is edited and `ah sync` re-runs
- **THEN** derived fields are updated to match the row
- **AND** `tests`, `status`, and `superseded_by` are byte-identical to before
- **VERIFIES** [[spec.P-ownership]]
