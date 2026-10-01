---
id: spec
kind: intent
statement: "WHEN a user drives the ah CLI THE system SHALL provide the command surface — idempotent init with hook integration, the deterministic check gate, doctor with capability enablement, archetype docs, append-only scenario lifecycle, archive companion, upgrade drift reporting, explain playbooks, conformance reporting, recommendation findings, and static spec lint — each exiting deterministically."

---

# cli Specification

## Constraints

| id | kind | expr | traces_to |
|----|------|------|-----------|
| C-cli-name | invariant | the standalone command-line interface is exposed as `ah` | [[spec]] |
| C-init-command | invariant | `ah init` is idempotent and prepares a repository for spec-test correspondence checks: it refuses without OpenSpec, stubs existing deployed scenarios, installs a supported pre-commit integration preferring lefthook before prek and falling back to prek, and reports when no hook framework is present | [[spec]] |
| C-check-command | invariant | `ah check` is the deterministic gate command, operating on deployed specs and accepting an OpenSpec change overlay | [[spec]] |
| C-doctor-command | invariant | `ah doctor` diagnoses project setup and correspondence wiring | [[spec]] |
| C-type-commands | invariant | `ah type` exposes built-in archetype guidance: listing archetypes and showing per-archetype details | [[spec]] |
| C-scenario-lifecycle | invariant | scenario lifecycle commands author append-only: creating a scenario in a change rejects without a target requirement; superseding rejects when the replacement is missing | [[spec]] |
| C-archive-companion | invariant | `ah archive <change>` moves staged scenario contracts after OpenSpec archive; it refuses to run before OpenSpec archive and refuses on collision | [[spec]] |
| C-upgrade-command | invariant | `ah upgrade` makes tool-version drift explicit by reporting compatibility changes | [[spec]] |
| C-doctor-enable | invariant | `ah doctor --enable <capability>` for a detected inactive capability writes exactly one config table for it and prints the path and table name written; enabling an unknown capability is an error; enabling an already-active capability is a no-op | [[spec]] |
| C-explain-subcommand | invariant | `ah explain <topic>` prints playbook guidance for finding kinds, suggested actions, and general topics, supports JSON output and listing, and errors on unknown topics | [[spec]] |
| C-report-command | invariant | `ah report` displays a conformance coverage matrix across deployed specs and archetype tiers (modeled on the OpenTelemetry per-language compliance matrix); it exits zero when coverage is complete, non-zero when scenarios lack contracts, and supports JSON output | [[spec]] |
| C-recommendation-findings | invariant | `ah doctor` emits `recommendation` findings for capabilities that are available but not yet configured; each carries its enable command and is a finding kind, not a log line | [[spec]] |
| C-lint-command | invariant | `ah lint` statically analyzes OpenSpec scenario files for quality findings without modifying files or running tests, operating on deployed specs and change overlays, scoping to a single check category, supporting JSON output, and exiting zero with empty findings on a clean spec | [[spec]] |

## Model

### States

- `bare`
- `prepared`
- `gated`
- `advised`

### Transitions

| id | from | to | guard |
|----|------|----|-------|
| t-init | bare | prepared | [[spec.C-init-command]] |
| t-check | prepared | gated | [[spec.C-check-command]] ([[spec.C-lint-command]] widens quality scope) |
| t-doctor | prepared | advised | [[spec.C-doctor-command]] ([[spec.C-doctor-enable]] and [[spec.C-recommendation-findings]] refine it) |
| t-lifecycle | prepared | prepared | [[spec.C-scenario-lifecycle]] AND [[spec.C-archive-companion]] AND [[spec.C-upgrade-command]] |
| t-guidance | advised | advised | [[spec.C-type-commands]] AND [[spec.C-explain-subcommand]] AND [[spec.C-report-command]] |

## Properties

| id | kind | derives_from | generator | predicate |
|----|------|--------------|-----------|-----------|
| P-name | unit | [[spec.C-cli-name]] | invoking the binary | it answers to `ah` with help |
| P-init | unit | [[spec.C-init-command]] | repositories with and without OpenSpec, deployed scenarios, lefthook, prek, and neither hook framework | idempotent preparation; refusal without OpenSpec; scenario stubs; hook preference order; honest missing-framework report |
| P-check | unit | [[spec.C-check-command]] | clean scopes and change overlays | deterministic gate behavior on both scopes |
| P-doctor | unit | [[spec.C-doctor-command]] | healthy and broken installations | diagnoses setup and correspondence wiring |
| P-type | unit | [[spec.C-type-commands]] | `ah type` list and detail invocations | all archetypes listed; details render per archetype |
| P-lifecycle | unit | [[spec.C-scenario-lifecycle]] | create-with and create-without target requirement, supersede with and without replacement | legal authoring succeeds append-only; both illegal cases rejected |
| P-archive | unit | [[spec.C-archive-companion]] | archive runs before OpenSpec archive, after it, and on collision | staged contracts move only post-archive and refuse collisions |
| P-upgrade | unit | [[spec.C-upgrade-command]] | version-drifted tool references | compatibility changes reported explicitly |
| P-enable | unit | [[spec.C-doctor-enable]] | enabled capabilities: pytest, cargo, vitest, mutation, property, snapshot, unknown, already-active | one config table per enable with reported path/table; unknown errors; active is a no-op |
| P-explain | unit | [[spec.C-explain-subcommand]] | finding-kind, suggested-action, and general topics with JSON, list, and unknown-topic variants | guidance printed per topic class; JSON valid; list complete; unknown errors |
| P-report | unit | [[spec.C-report-command]] | scopes with complete coverage vs missing contracts | zero exit on completeness, non-zero on gaps, JSON output available |
| P-recommendation | unit | [[spec.C-recommendation-findings]] | doctor runs over projects with unconfigured available capabilities | recommendation findings carry enable commands and are finding kinds |
| P-lint | unit | [[spec.C-lint-command]] | clean specs, dirty specs, overlays, single categories, JSON output | quality findings without file modification or test execution; clean exits zero with empty findings |

## Purpose
TBD - created by archiving change add-spec-assertions. Update Purpose after archive.

## Requirements

### Requirement: CLI Command Name
The system SHALL expose the standalone command-line interface as `ah`.

#### Scenario: Invoke help
- **WHEN** a user runs `ah --help`
- **THEN** the CLI displays available `ah` commands
- **VERIFIES** [[spec.P-name]]

### Requirement: Project Initialization
The system SHALL provide an idempotent `ah init` command that prepares a repository for spec-test correspondence checks.

#### Scenario: Initialize project files
- **GIVEN** a repository contains an `openspec/` directory
- **WHEN** a user runs `ah init`
- **THEN** the command creates `.espectacular/config.toml` when it is missing
- **AND** writes `.espectacular/AGENTS.md`
- **AND** creates top-level `AGENTS.md` and `CLAUDE.md` when they are absent
- **AND** refreshes managed `ah` blocks in top-level instruction files
- **VERIFIES** [[spec.P-init]]

#### Scenario: Refuse initialization without OpenSpec
- **GIVEN** a repository does not contain an `openspec/` directory
- **WHEN** a user runs `ah init`
- **THEN** the command fails without creating `.espectacular/`
- **VERIFIES** [[spec.P-init]]

#### Scenario: Stub existing deployed scenarios
- **GIVEN** deployed OpenSpec scenarios exist under `openspec/specs/`
- **WHEN** a user runs `ah init`
- **THEN** the command creates matching `.espectacular/<spec>/<scenario>.toml` stubs for scenarios without contracts
- **AND** the stubs declare no tests until the user or AI fills them in
- **VERIFIES** [[spec.P-init]]

#### Scenario: Install supported pre-commit integration
- **GIVEN** the repository uses `lefthook`
- **WHEN** a user runs `ah init`
- **THEN** the command installs or refreshes a managed pre-commit integration that runs `ah check`
- **VERIFIES** [[spec.P-init]]

#### Scenario: Prefer lefthook before prek
- **GIVEN** the repository has both `lefthook` and `prek` configured
- **WHEN** a user runs `ah init`
- **THEN** the command installs the managed pre-commit integration through `lefthook`
- **VERIFIES** [[spec.P-init]]

#### Scenario: Fall back to prek
- **GIVEN** the repository uses `prek`
- **AND** does not use `lefthook`
- **WHEN** a user runs `ah init`
- **THEN** the command installs or refreshes a managed pre-commit integration through `prek`
- **VERIFIES** [[spec.P-init]]

#### Scenario: Report missing hook framework
- **GIVEN** the repository does not use `lefthook` or `prek`
- **WHEN** a user runs `ah init`
- **THEN** the command reports a concern that the user or AI must set up pre-commit integration
- **AND** does not write a raw `.git/hooks/pre-commit` fallback
- **VERIFIES** [[spec.P-init]]

### Requirement: Correspondence Check Command
The system SHALL provide `ah check` as the deterministic gate command.

#### Scenario: Check deployed specs
- **WHEN** a user runs `ah check`
- **THEN** the command validates deployed specs under `openspec/specs/`
- **AND** validates matching contracts under `.espectacular/<spec>/`
- **AND** emits JSON output
- **VERIFIES** [[spec.P-check]]

#### Scenario: Check an OpenSpec change overlay
- **WHEN** a user runs `ah check --changes add-parser`
- **THEN** the command validates deployed specs plus the `add-parser` change overlay
- **AND** validates staged contracts under `.espectacular/changes/add-parser/`
- **AND** includes the selected change in the JSON scope
- **VERIFIES** [[spec.P-check]]

### Requirement: Health Check Command
The system SHALL provide `ah doctor` for installation health checks.

#### Scenario: Diagnose project setup
- **WHEN** a user runs `ah doctor`
- **THEN** the command validates `.espectacular/config.toml`
- **AND** checks managed instruction blocks
- **AND** checks supported hook integration
- **AND** reports tool-version compatibility concerns
- **VERIFIES** [[spec.P-doctor]]

#### Scenario: Diagnose correspondence wiring
- **WHEN** a user runs `ah doctor`
- **THEN** the command reports slug collisions as errors
- **AND** reports orphan contracts as errors
- **AND** reports unknown archetype names as warnings
- **VERIFIES** [[spec.P-doctor]]

### Requirement: Archetype Documentation Commands
The system SHALL expose built-in archetype guidance through `ah type` commands.

#### Scenario: List archetypes
- **WHEN** a user runs `ah type`
- **THEN** the command lists all known archetypes with one-line descriptions
- **AND** includes `PF`, `SA`, `BP`, `CE`, and `NR`
- **VERIFIES** [[spec.P-type]]

#### Scenario: Show archetype details
- **WHEN** a user runs `ah type PF`
- **THEN** the command prints the full built-in documentation for the `PF` archetype
- **VERIFIES** [[spec.P-type]]

### Requirement: Scenario Lifecycle Commands
The system SHALL provide commands for append-only scenario authoring.

#### Scenario: Create scenario in a change
- **GIVEN** `openspec/changes/add-parser/specs/compiler/spec.md` contains `### Requirement: Parser Input Validation`
- **WHEN** a user runs `ah scenario new add-parser compiler --requirement "Parser Input Validation" "Empty input rejected"`
- **THEN** the command appends a scenario heading under that requirement
- **AND** writes placeholder `WHEN` and `THEN` lines under the scenario heading
- **AND** creates `.espectacular/changes/add-parser/compiler/empty-input-rejected.toml` with `id`, empty `description`, empty `archetype`, `status = "active"`, empty `superseded_by`, and `authored_with`
- **VERIFIES** [[spec.P-lifecycle]]

#### Scenario: Reject scenario creation without target requirement
- **GIVEN** `openspec/changes/add-parser/specs/compiler/spec.md` does not contain `### Requirement: Parser Input Validation`
- **WHEN** a user runs `ah scenario new add-parser compiler --requirement "Parser Input Validation" "Empty input rejected"`
- **THEN** the command fails without creating or modifying files
- **VERIFIES** [[spec.P-lifecycle]]

#### Scenario: Supersede a scenario
- **GIVEN** scenario `new-behavior` exists in the deployed-plus-`add-parser` overlay for spec `compiler`
- **WHEN** a user runs `ah scenario supersede compiler old-behavior --with=new-behavior --in-change=add-parser`
- **THEN** the command stages `.espectacular/changes/add-parser/compiler/old-behavior.toml`
- **AND** marks the staged contract as superseded
- **AND** records `new-behavior` as the replacement scenario id
- **VERIFIES** [[spec.P-lifecycle]]

#### Scenario: Reject supersession with missing replacement
- **GIVEN** scenario `new-behavior` does not exist in the deployed-plus-`add-parser` overlay for spec `compiler`
- **WHEN** a user runs `ah scenario supersede compiler old-behavior --with=new-behavior --in-change=add-parser`
- **THEN** the command fails without creating or modifying files
- **VERIFIES** [[spec.P-lifecycle]]

### Requirement: Archive Companion Command
The system SHALL provide `ah archive <change>` to move staged scenario contracts after OpenSpec archive.

#### Scenario: Archive staged contracts
- **GIVEN** `openspec archive add-parser` has applied the OpenSpec change
- **AND** every staged contract id exists in deployed `openspec/specs/`
- **WHEN** a user runs `ah archive add-parser`
- **THEN** the command moves new contracts from `.espectacular/changes/add-parser/<spec>/` to `.espectacular/<spec>/`
- **VERIFIES** [[spec.P-archive]]

#### Scenario: Refuse archive before OpenSpec archive
- **GIVEN** `.espectacular/changes/add-parser/compiler/new-behavior.toml` exists
- **AND** deployed `openspec/specs/compiler/spec.md` does not contain scenario `new-behavior`
- **WHEN** a user runs `ah archive add-parser`
- **THEN** the command fails without moving staged contracts
- **VERIFIES** [[spec.P-archive]]

#### Scenario: Refuse archive collision
- **GIVEN** `.espectacular/compiler/foo.toml` already exists
- **AND** `.espectacular/changes/add-parser/compiler/foo.toml` is not a superseded metadata update for `foo`
- **WHEN** a user runs `ah archive add-parser`
- **THEN** the command fails without overwriting the deployed contract
- **VERIFIES** [[spec.P-archive]]

### Requirement: Upgrade Command
The system SHALL provide `ah upgrade` to make tool-version drift explicit.

#### Scenario: Report compatibility changes
- **GIVEN** `.espectacular/config.toml` pins an older tool version than the installed `ah`
- **WHEN** a user runs `ah upgrade`
- **THEN** the command reports config schema version changes, execution default changes, archetype additions, and archetype deprecations before updating the configured tool version
- **AND** does not rewrite existing scenario contract `authored_with` values
- **VERIFIES** [[spec.P-upgrade]]

### Requirement: Doctor enable flag
When a user runs `ah doctor --enable <capability>` for a detected inactive capability, the system SHALL write exactly one config table for that capability and SHALL print the path and table name written.

#### Scenario: Enable pytest adapter
- **GIVEN** pytest is detected by `ah doctor`
- **WHEN** a user runs `ah doctor --enable pytest`
- **THEN** the command writes `[runners.pytest] command = ["pytest"]` to `.espectacular/config.toml`
- **AND** prints `.espectacular/config.toml` and `[runners.pytest]`
- **VERIFIES** [[spec.P-enable]]

#### Scenario: Enable cargo adapter
- **GIVEN** cargo is detected by `ah doctor`
- **WHEN** a user runs `ah doctor --enable cargo`
- **THEN** the command writes `[runners.cargo] command = ["cargo", "test"]` to `.espectacular/config.toml`
- **AND** prints `.espectacular/config.toml` and `[runners.cargo]`
- **VERIFIES** [[spec.P-enable]]

#### Scenario: Enable vitest adapter
- **GIVEN** vitest is detected by `ah doctor`
- **WHEN** a user runs `ah doctor --enable vitest`
- **THEN** the command writes `[runners.vitest] command = ["vitest", "run"]` to `.espectacular/config.toml`
- **AND** prints `.espectacular/config.toml` and `[runners.vitest]`
- **VERIFIES** [[spec.P-enable]]

#### Scenario: Enable mutation capability
- **GIVEN** a mutation testing tool is detected
- **WHEN** a user runs `ah doctor --enable mutation`
- **THEN** the command writes `[capabilities.mutation] enabled = true` to `.espectacular/config.toml`
- **AND** prints `.espectacular/config.toml` and `[capabilities.mutation]`
- **VERIFIES** [[spec.P-enable]]

#### Scenario: Enable property capability
- **GIVEN** a property-based testing framework is detected
- **WHEN** a user runs `ah doctor --enable property`
- **THEN** the command writes `[capabilities.property] enabled = true` to `.espectacular/config.toml`
- **AND** prints `.espectacular/config.toml` and `[capabilities.property]`
- **VERIFIES** [[spec.P-enable]]

#### Scenario: Enable snapshot capability
- **GIVEN** a snapshot testing framework is detected
- **WHEN** a user runs `ah doctor --enable snapshot`
- **THEN** the command writes `[capabilities.snapshot] enabled = true` to `.espectacular/config.toml`
- **AND** prints `.espectacular/config.toml` and `[capabilities.snapshot]`
- **VERIFIES** [[spec.P-enable]]

#### Scenario: Enable unknown capability is an error
- **WHEN** a user runs `ah doctor --enable nonexistent`
- **THEN** the command exits non-zero
- **AND** prints `unrecognized capability: nonexistent`
- **VERIFIES** [[spec.P-enable]]

#### Scenario: Enable already-active capability is a no-op
- **GIVEN** a capability is already present in `.espectacular/config.toml`
- **WHEN** a user runs `ah doctor --enable <capability>`
- **THEN** the command reports it is already enabled and makes no changes
- **VERIFIES** [[spec.P-enable]]

### Requirement: Explain subcommand
The system SHALL provide an `ah explain <topic>` subcommand that prints playbook guidance for a finding kind or suggested action.

#### Scenario: Explain a finding kind
- **WHEN** a user runs `ah explain no-toml`
- **THEN** the command prints markdown guidance for the `no-toml` finding kind
- **VERIFIES** [[spec.P-explain]]

#### Scenario: Explain a suggested action
- **WHEN** a user runs `ah explain run_ah_scenario_new`
- **THEN** the command prints markdown guidance for the `run_ah_scenario_new` suggested action
- **VERIFIES** [[spec.P-explain]]

#### Scenario: Explain a general topic
- **WHEN** a user runs `ah explain workflow`
- **THEN** the command prints markdown guidance for the general `workflow` topic
- **VERIFIES** [[spec.P-explain]]

#### Scenario: Explain with JSON output
- **WHEN** a user runs `ah explain no-toml --json`
- **THEN** the command emits a JSON object with fields: `topic`, `summary`, `when`, `do`, `human_approval`, `related_topics`, `hints`
- **AND** each `hints` item contains `kind` and `message` string fields
- **VERIFIES** [[spec.P-explain]]

#### Scenario: List all topics
- **WHEN** a user runs `ah explain --list`
- **THEN** the command prints all available topic identifiers, one per line
- **VERIFIES** [[spec.P-explain]]

#### Scenario: Unknown topic is an error
- **WHEN** a user runs `ah explain no-such-topic`
- **THEN** the command exits non-zero
- **AND** prints either `Run ah explain --list` or the sorted list of available topic identifiers
- **VERIFIES** [[spec.P-explain]]

### Requirement: Coverage report command
The system SHALL provide `ah report` to display a conformance coverage matrix across all deployed specs and archetype tiers, modeled on the OpenTelemetry per-language compliance matrix pattern.

#### Scenario: Report coverage by spec and archetype
- **WHEN** a user runs `ah report`
- **THEN** the command prints a table showing each spec as a row and each archetype as a column
- **AND** each cell shows covered/missing/failing counts
- **VERIFIES** [[spec.P-report]]

#### Scenario: Report exits zero when coverage is complete
- **GIVEN** every deployed scenario has a valid, passing contract
- **WHEN** a user runs `ah report`
- **THEN** the command exits zero
- **VERIFIES** [[spec.P-report]]

#### Scenario: Report exits non-zero when scenarios are missing contracts
- **GIVEN** at least one deployed scenario has no sidecar contract
- **WHEN** a user runs `ah report`
- **THEN** the command exits non-zero
- **VERIFIES** [[spec.P-report]]

#### Scenario: Report JSON output
- **WHEN** a user runs `ah report --json`
- **THEN** the command emits a JSON conformance matrix consumable by CI dashboards and agent harnesses
- **VERIFIES** [[spec.P-report]]

### Requirement: Recommendation findings
The system SHALL emit `recommendation` findings when `ah doctor` detects capabilities that are available but not yet configured.

#### Scenario: Recommendation finding carries enable command
- **GIVEN** `ah doctor` detects an available framework not yet configured
- **WHEN** the output is inspected (JSON or text)
- **THEN** a `recommendation` finding is present with `suggested_action = enable_capability`
- **AND** `apply_command` contains the `ah doctor --enable <capability>` invocation
- **VERIFIES** [[spec.P-recommendation]]

#### Scenario: Recommendation finding is a finding kind, not a log line
- **GIVEN** `ah doctor` detects an available but unconfigured framework
- **WHEN** the output is requested as JSON (`ah doctor --json`)
- **THEN** the finding appears in the `findings` array with `kind = recommendation`
- **AND** it carries a `playbook_command` field
- **VERIFIES** [[spec.P-recommendation]]

### Requirement: Spec Lint Command
The system SHALL provide `ah lint` to statically analyze OpenSpec scenario files for quality findings without modifying files or running tests.

#### Scenario: Lint deployed specs
- **WHEN** a user runs `ah lint`
- **THEN** the command analyzes all spec files under `openspec/specs/`
- **AND** emits lint findings to stdout
- **AND** exits zero when only warning-severity findings are present
- **VERIFIES** [[spec.P-lint]]

#### Scenario: Lint a change overlay
- **WHEN** a user runs `ah lint --changes add-parser`
- **THEN** the command analyzes deployed specs plus the `add-parser` change spec overlay
- **VERIFIES** [[spec.P-lint]]

#### Scenario: Lint a single check category
- **WHEN** a user runs `ah lint --check vague-qualifier`
- **THEN** the command runs only the `vague-qualifier` check and skips all others
- **VERIFIES** [[spec.P-lint]]

#### Scenario: Lint JSON output
- **WHEN** a user runs `ah lint --json`
- **THEN** the command emits a JSON object in the same schema as `ah check --json`
- **AND** each finding includes `kind`, `severity`, `spec_path`, `scenario_id`, `message`, and `suggestion`
- **VERIFIES** [[spec.P-lint]]

#### Scenario: Clean spec exits zero with empty findings
- **GIVEN** all analyzed spec files pass all lint checks
- **WHEN** a user runs `ah lint`
- **THEN** the command exits zero
- **AND** emits no findings
- **VERIFIES** [[spec.P-lint]]
