---
id: spec
kind: intent
statement: "WHEN ah lint inspects specification files THE linter SHALL flag vague qualifiers, imperative UI steps, conjunctive step bloat, missing negative scenarios, missing non-goals, unresolved ambiguities, and entangled presentation concerns, emit findings in the shared check finding schema with warning severity by default, and bridge specodelic findings for dual-format files without reimplementing specodelic rules."

---

# lint Specification

## Constraints

| id | kind | expr | traces_to |
|----|------|------|-----------|
| C-vague-qualifier | invariant | requirement bodies and scenario steps containing unbound qualitative terms without an adjacent numeric measurement emit a `vague-qualifier` finding suggesting a measurable condition; a qualifier with an adjacent numeric bound emits none | [[spec]] |
| C-imperative-step | invariant | WHEN/THEN steps describing UI mechanics (explicit click, URL navigation) emit an `imperative-step` finding suggesting business-action phrasing; declarative business steps emit none | [[spec]] |
| C-conjunctive-bloat | invariant | scenarios chaining more than the configured maximum of AND-linked steps emit a `conjunctive-bloat` finding suggesting a split; a scenario at exactly the limit is accepted; the maximum is configurable via `[lint] max_and_steps` in `.espectacular/config.toml` | [[spec]] |
| C-missing-negative | invariant | a requirement with no scenario exercising an error, rejection, or boundary condition emits a `missing-negative-scenario` finding suggesting a failure-mode scenario; one negative scenario suppresses it | [[spec]] |
| C-missing-non-goals | invariant | a spec capability file lacking a `Non-Goals`/`Non Goals`/`non-goals`/`Out of Scope` heading emits a `missing-non-goals` finding for that file | [[spec]] |
| C-unresolved-ambiguity | invariant | requirement or scenario text containing `[NEEDS CLARIFICATION` emits an `unresolved-ambiguity` finding including the marker as context; text without the marker emits none | [[spec]] |
| C-entangled-spec | invariant | a scenario with contract archetype `PF` or `SA` whose text references presentation-layer primitives (buttons, modals, colors, CSS selectors, component names, client routes) emits an `entangled-spec` finding suggesting the domain event or state; the same text in a `BP` scenario and domain-legitimate lookalike terms are not flagged | [[spec]] |
| C-finding-schema | invariant | lint findings use the same stable JSON envelope as `ah check`: each carries `kind`, `severity`, `spec_path`, `message`, `suggested_action`, `playbook_command`, plus `scenario_id`/`scenario_title` when scenario-specific; findings default to `severity = "warning"`; a malformed spec file emits a `severity = "error"` finding and exits non-zero | [[spec]] |
| C-dual-bridge | invariant | for dual-format files (`id: spec` frontmatter) `ah lint` invokes `spk lint` and relays each specodelic finding as `kind = "spk.<rule_id>"` with `severity = "warning"`; plain openspec files cause no specodelic invocation and identical pre-bridge output; a missing `spk` binary emits one advisory `spk-unavailable` finding with unchanged exit code; a non-envelope `spk` failure emits one advisory finding and linting continues | [[spec]] |

## Model

### States

- `scanning`
- `bridging`
- `reported`

### Transitions

| id | from | to | guard |
|----|------|----|-------|
| t-textual-rules | scanning | scanning | [[spec.C-vague-qualifier]], [[spec.C-imperative-step]], [[spec.C-conjunctive-bloat]], [[spec.C-missing-negative]], [[spec.C-missing-non-goals]], [[spec.C-unresolved-ambiguity]], or [[spec.C-entangled-spec]] |
| t-bridge | scanning | bridging | [[spec.C-dual-bridge]] (dual-format file present) |
| t-inert | scanning | scanning | [[spec.C-dual-bridge]] (plain file — no invocation) |
| t-schema | bridging | reported | [[spec.C-finding-schema]] |
| t-degrade | bridging | reported | [[spec.C-dual-bridge]] (missing binary or failed invocation — advisory only) |

## Properties

| id | kind | derives_from | generator | predicate |
|----|------|--------------|-----------|-----------|
| P-vague | unit | [[spec.C-vague-qualifier]] | bodies with unbound qualifiers vs numerically bounded qualifiers, in requirements and THEN steps | findings exactly in the unbound cases, with measurable-condition suggestions |
| P-imperative | unit | [[spec.C-imperative-step]] | WHEN steps with click/URL mechanics vs declarative business steps | findings exactly in the mechanical cases, with business-action suggestions |
| P-bloat | unit | [[spec.C-conjunctive-bloat]] | scenarios at limit-1, limit, and limit+1 AND-steps under default and custom maximums | finding only above the configured limit; exact limit accepted |
| P-negative | unit | [[spec.C-missing-negative]] | requirements with only happy-path scenarios vs one negative scenario | finding exactly in the happy-path-only case |
| P-non-goals | unit | [[spec.C-missing-non-goals]] | spec files with and without any non-goals heading variant | finding exactly when no variant is present |
| P-ambiguity | unit | [[spec.C-unresolved-ambiguity]] | bodies with and without `[NEEDS CLARIFICATION` markers | finding with marker context exactly when present |
| P-entangled | unit | [[spec.C-entangled-spec]] | PF/SA scenarios with presentation primitives vs BP scenarios and domain lookalikes | findings exactly in the PF/SA primitive cases |
| P-schema | unit | [[spec.C-finding-schema]] | lint runs over well-formed and malformed specs | shared finding fields throughout, warning default, malformed → error + non-zero exit |
| P-bridge | unit | [[spec.C-dual-bridge]] | dual files with spk findings, plain files, missing binary, and failing invocation | relaying with `spk.` kind prefix; inert for plain files; advisory-only degradation in both failure modes |

## Purpose
TBD - created by archiving change add-spec-quality-checks. Update Purpose after archive.

## Requirements

### Requirement: Vague Qualifier Detection
The system SHALL flag requirements and scenario steps that contain unbound qualitative terms without an adjacent numeric measurement.

#### Scenario: Flag unbound qualifier in requirement
- **GIVEN** a requirement body contains the word "fast" without a numeric time bound
- **WHEN** `ah lint` runs
- **THEN** the command emits a `vague-qualifier` finding for that requirement
- **AND** the finding suggests adding a measurable response condition (e.g., "within 200 ms")
- **VERIFIES** [[spec.P-vague]]

#### Scenario: Do not flag bounded qualifier
- **GIVEN** a requirement body contains "fast (p95 < 200 ms)"
- **WHEN** `ah lint` runs
- **THEN** no `vague-qualifier` finding is emitted for that requirement
- **VERIFIES** [[spec.P-vague]]

#### Scenario: Flag qualifier in scenario step
- **GIVEN** a THEN step contains "the response is user-friendly"
- **WHEN** `ah lint` runs
- **THEN** the command emits a `vague-qualifier` finding for that scenario step
- **VERIFIES** [[spec.P-vague]]

### Requirement: Imperative Step Detection
The system SHALL flag WHEN and THEN steps that describe UI mechanics rather than business intent, coupling the spec to implementation details.

#### Scenario: Flag explicit click step
- **GIVEN** a WHEN step contains "clicks the Submit button"
- **WHEN** `ah lint` runs
- **THEN** the command emits an `imperative-step` finding
- **AND** the finding suggests rephrasing to describe the business action (e.g., "submits the form")
- **VERIFIES** [[spec.P-imperative]]

#### Scenario: Flag URL navigation step
- **GIVEN** a WHEN step contains "navigates to /dashboard/settings"
- **WHEN** `ah lint` runs
- **THEN** the command emits an `imperative-step` finding
- **VERIFIES** [[spec.P-imperative]]

#### Scenario: Do not flag declarative business steps
- **GIVEN** a WHEN step contains "the user submits a payment"
- **WHEN** `ah lint` runs
- **THEN** no `imperative-step` finding is emitted
- **VERIFIES** [[spec.P-imperative]]

### Requirement: Conjunctive Step Bloat Detection
The system SHALL flag scenarios that chain more than the configured maximum number of AND-linked steps, indicating a scenario that tests multiple behaviors at once.

#### Scenario: Flag overlong scenario
- **GIVEN** a scenario has eight AND-linked steps
- **AND** the configured maximum is five
- **WHEN** `ah lint` runs
- **THEN** the command emits a `conjunctive-bloat` finding
- **AND** the finding suggests splitting the scenario into focused single-behavior scenarios
- **VERIFIES** [[spec.P-bloat]]

#### Scenario: Accept scenario within limit
- **GIVEN** a scenario has four AND-linked steps
- **AND** the configured maximum is five
- **WHEN** `ah lint` runs
- **THEN** no `conjunctive-bloat` finding is emitted for that scenario
- **VERIFIES** [[spec.P-bloat]]

#### Scenario: Scenario exactly at limit is accepted
- **GIVEN** a scenario has five AND-linked steps
- **AND** the configured maximum is five
- **WHEN** `ah lint` runs
- **THEN** no `conjunctive-bloat` finding is emitted for that scenario
- **VERIFIES** [[spec.P-bloat]]

#### Scenario: Default maximum is configurable
- **GIVEN** `.espectacular/config.toml` sets `[lint] max_and_steps = 3`
- **WHEN** `ah lint` runs
- **THEN** the command uses 3 as the maximum AND-step count
- **VERIFIES** [[spec.P-bloat]]

### Requirement: Missing Negative Scenario Detection
The system SHALL flag requirements that have no scenario exercising an error condition, rejection, or boundary violation, indicating incomplete behavioral specification.

#### Scenario: Flag requirement with only happy-path scenarios
- **GIVEN** a requirement has two scenarios, both describing successful outcomes
- **AND** no scenario contains rejection, error, or boundary language
- **WHEN** `ah lint` runs
- **THEN** the command emits a `missing-negative-scenario` finding
- **AND** the finding suggests adding a scenario for the corresponding failure mode
- **VERIFIES** [[spec.P-negative]]

#### Scenario: Accept requirement with at least one negative scenario
- **GIVEN** a requirement has a scenario whose THEN step contains "the command exits non-zero"
- **WHEN** `ah lint` runs
- **THEN** no `missing-negative-scenario` finding is emitted for that requirement
- **VERIFIES** [[spec.P-negative]]

### Requirement: Missing Non-Goals Detection
The system SHALL flag spec capability files that lack an explicit Non-Goals or Out-of-Scope section, since their absence enables silent scope creep during AI-assisted implementation.

#### Scenario: Flag spec without non-goals section
- **GIVEN** a `spec.md` file contains no heading matching `Non-Goals`, `Non Goals`, `non-goals`, or `Out of Scope`
- **WHEN** `ah lint` runs
- **THEN** the command emits a `missing-non-goals` finding for that spec file
- **VERIFIES** [[spec.P-non-goals]]

#### Scenario: Accept spec with non-goals section
- **GIVEN** a `spec.md` file contains a `## Non-Goals` heading
- **WHEN** `ah lint` runs
- **THEN** no `missing-non-goals` finding is emitted for that file
- **VERIFIES** [[spec.P-non-goals]]

### Requirement: Unresolved Ambiguity Detection
The system SHALL flag requirements and scenarios that contain `[NEEDS CLARIFICATION` markers, indicating authoring-time decisions deferred but not yet resolved.

#### Scenario: Flag needs-clarification marker
- **GIVEN** a requirement body contains `[NEEDS CLARIFICATION: which auth provider?]`
- **WHEN** `ah lint` runs
- **THEN** the command emits an `unresolved-ambiguity` finding
- **AND** the finding includes the marker text as context
- **VERIFIES** [[spec.P-ambiguity]]

#### Scenario: Accept requirement with no ambiguity markers
- **GIVEN** a requirement body contains no `[NEEDS CLARIFICATION` substring
- **WHEN** `ah lint` runs
- **THEN** no `unresolved-ambiguity` finding is emitted for that requirement
- **VERIFIES** [[spec.P-ambiguity]]

### Requirement: Entangled Specification Detection
The system SHALL flag scenarios whose contract archetype is `PF` or `SA` but whose text references presentation-layer primitives (buttons, modals, colors, CSS selectors, component names, client routes), since the archetype declares the scenario belongs to a layer where UI mechanics must not appear. The same text in a `BP` scenario is not flagged, as transport mechanics are legitimate at a boundary seam.

#### Scenario: Flag UI primitive in PF scenario
- **GIVEN** a scenario with contract archetype `PF` whose THEN step contains "a green notification banner is displayed"
- **WHEN** `ah lint` runs
- **THEN** the command emits an `entangled-spec` finding
- **AND** the finding suggests specifying the domain event or state instead of the visual primitive
- **VERIFIES** [[spec.P-entangled]]

#### Scenario: Do not flag transport mechanics in BP scenario
- **GIVEN** a scenario with contract archetype `BP` whose WHEN step contains "navigates to /dashboard/settings"
- **WHEN** `ah lint` runs
- **THEN** no `entangled-spec` finding is emitted for that scenario
- **VERIFIES** [[spec.P-entangled]]

#### Scenario: Do not flag domain language resembling UI terms
- **GIVEN** a scenario with contract archetype `SA` whose text contains "the event routing layer delivers the message"
- **WHEN** `ah lint` runs
- **THEN** no `entangled-spec` finding is emitted, as the matcher list excludes domain-legitimate uses of superficially similar terms
- **VERIFIES** [[spec.P-entangled]]

### Requirement: Lint Finding Schema
The system SHALL emit lint findings using the same stable JSON envelope as `ah check`, enabling agent harnesses to consume both without separate parsing logic.

#### Scenario: Lint findings share check finding fields
- **GIVEN** `ah lint --json` produces findings
- **WHEN** the JSON output is inspected
- **THEN** each finding contains `kind`, `severity`, `spec_path`, `message`, `suggested_action`, and `playbook_command`
- **AND** findings that reference a specific scenario also contain `scenario_id` and `scenario_title`
- **VERIFIES** [[spec.P-schema]]

#### Scenario: Lint findings are warning severity by default
- **GIVEN** `ah lint` produces warning-severity findings for any lint check category
- **WHEN** the JSON output is inspected
- **THEN** every finding has `severity = "warning"`
- **VERIFIES** [[spec.P-schema]]

#### Scenario: Malformed spec file is error severity
- **GIVEN** a spec file contains invalid Markdown that prevents scenario parsing
- **WHEN** `ah lint` runs
- **THEN** the command emits a finding with `severity = "error"`
- **AND** exits non-zero
- **VERIFIES** [[spec.P-schema]]

### Requirement: Dual-Format Lint Bridge
The system SHALL relay specodelic lint findings for dual-format spec files into the shared lint finding schema by invoking `spk lint`, without reimplementing specodelic rules.

#### Scenario: Relay specodelic findings for dual-format files
- **GIVEN** a spec file carries `id: spec` frontmatter (dual-format)
- **AND** `spk lint` reports findings for that file
- **WHEN** `ah lint` runs
- **THEN** each specodelic finding appears in the output with `kind = "spk.<rule_id>"`
- **AND** every relayed finding has `severity = "warning"`
- **VERIFIES** [[spec.P-bridge]]

#### Scenario: Bridge is inert for plain openspec files
- **GIVEN** a spec file contains no dual-format frontmatter
- **WHEN** `ah lint` runs
- **THEN** no specodelic invocation occurs and the output is identical to pre-bridge behavior
- **VERIFIES** [[spec.P-bridge]]

#### Scenario: Missing spk binary is advisory only
- **GIVEN** a dual-format spec file
- **AND** the `spk` binary is not available on PATH
- **WHEN** `ah lint` runs
- **THEN** the command emits a single advisory `spk-unavailable` finding
- **AND** the exit code is unchanged relative to a successful warning-only lint run
- **VERIFIES** [[spec.P-bridge]]

#### Scenario: Specodelic invocation failure does not hard-fail
- **GIVEN** a dual-format spec file
- **AND** `spk lint` exits with a non-envelope error or unparseable output
- **WHEN** `ah lint` runs
- **THEN** the command emits a single advisory finding describing the failure
- **AND** continues linting the remaining spec files
- **VERIFIES** [[spec.P-bridge]]
