---
id: explain
kind: intent
statement: "WHEN a user runs ah explain THE playbook SHALL serve compile-enforced topics for every FindingKind, SuggestedAction, quality finding kind, and compiled-in adapter capability, with stable sorted listing, machine-readable JSON, and non-zero exit for unknown topics."

---

# explain Specification

## Constraints

| id | kind | expr | traces_to |
|----|------|------|-----------|
| C-compile-enforced | invariant | the build fails if any `FindingKind` or `SuggestedAction` enum variant lacks a playbook topic body, and succeeds enumerating all topics when complete | [[explain]] |
| C-topic-coverage | invariant | `ah explain` provides topics for every `FindingKind` value, every `SuggestedAction` value, and a set of general topics | [[explain]] |
| C-json-schema | invariant | `ah explain <topic> --json` emits a valid JSON object with `topic`, `summary`, `when`, `do` (string array), `human_approval` (boolean), `related_topics` (string array), and `hints` (objects with `kind` and `message`), for every valid topic | [[explain]] |
| C-listing-stable | invariant | `ah explain --list` prints all topic identifiers one per line, sorted alphabetically, identical across runs | [[explain]] |
| C-unknown-topic | invariant | an unknown topic exits non-zero with either the sorted topic list or a pointer to `ah explain --list` | [[explain]] |
| C-quality-kinds | invariant | topics exist for the quality finding kinds `quality-mutation`, `quality-property`, `quality-snapshot`, and are subject to compile enforcement like all `FindingKind` values | [[explain]] |
| C-adapter-topics | invariant | progressive-enablement capability topics are included when their adapter modules are compiled in; two adapter modules registering the same topic identifier fail the build | [[explain]] |

## Model

### States

- `compiling`
- `serving`
- `errored`

### Transitions

| id | from | to | guard |
|----|------|----|-------|
| t-enforce | compiling | compiling | [[explain.C-compile-enforced]] AND [[explain.C-adapter-topics]] (duplicate registration) |
| t-serve | compiling | serving | [[explain.C-topic-coverage]] AND [[explain.C-quality-kinds]] |
| t-json | serving | serving | [[explain.C-json-schema]] |
| t-list | serving | serving | [[explain.C-listing-stable]] |
| t-unknown | serving | errored | [[explain.C-unknown-topic]] |

## Properties

| id | kind | derives_from | generator | predicate |
|----|------|--------------|-----------|-----------|
| P-enforced | unit | [[explain.C-compile-enforced]] | a build with one variant stripped of its topic body vs the complete set | the stripped build fails naming the missing topic; the complete build succeeds and `--list` enumerates every variant |
| P-coverage | unit | [[explain.C-topic-coverage]] | every `FindingKind` and `SuggestedAction` value plus a general topic | each resolves via `ah explain <topic>` printing guidance and exiting zero |
| P-json | unit | [[explain.C-json-schema]] | every valid topic with `--json` | output is a valid object with exactly the required fields and typed `hints` entries |
| P-listing | unit | [[explain.C-listing-stable]] | two successive `--list` runs | outputs are byte-identical and alphabetically sorted |
| P-unknown | unit | [[explain.C-unknown-topic]] | `ah explain no-such-topic` | exits non-zero with the topic list or the `--list` pointer |
| P-quality | unit | [[explain.C-quality-kinds]] | `ah explain quality-mutation`, `quality-property`, `quality-snapshot` | each prints kind-specific guidance (mutation meaning/enablement) and exits zero |
| P-adapter | unit | [[explain.C-adapter-topics]] | a build with the pytest adapter compiled in vs two modules registering one topic id | the first serves `ah explain pytest` enablement guidance; the second fails the build naming the conflict |

## Purpose
TBD - created by archiving change add-quality-measurement-and-adapters. Update Purpose after archive.

## Non-Goals

- Serving playbook content from runtime or external files — topics are embedded in the binary at compile time
- Authoring playbooks for upstream tools' own findings (genesis, specodelic) — only espectacular's kinds and actions
- Localizing or theming playbook text — English-only output

## Requirements

### Requirement: Playbook is compile-enforced
The system SHALL fail to build if any `FindingKind` or `SuggestedAction` enum variant lacks a corresponding `ah explain` topic body.

#### Scenario: Missing topic body is a compile error
- **GIVEN** a `FindingKind` or `SuggestedAction` variant has no associated playbook body
- **WHEN** the project is built with `cargo build`
- **THEN** the build fails with an error identifying the missing topic
- **VERIFIES** [[explain.P-enforced]]

#### Scenario: All variants have topics at build time
- **GIVEN** all enum variants have associated playbook bodies
- **WHEN** the project is built
- **THEN** the build succeeds and `ah explain --list` enumerates them all
- **VERIFIES** [[explain.P-coverage]]

### Requirement: Topic coverage
The system SHALL provide `ah explain` topics for every `FindingKind` value, every `SuggestedAction` value, and a set of general topics.

#### Scenario: Finding kind topic exists
- **WHEN** a user runs `ah explain no-toml`
- **THEN** the command prints guidance for the `no-toml` finding kind and exits zero
- **VERIFIES** [[explain.P-coverage]]

#### Scenario: Suggested action topic exists
- **WHEN** a user runs `ah explain run_ah_scenario_new`
- **THEN** the command prints guidance for the `run_ah_scenario_new` action and exits zero
- **VERIFIES** [[explain.P-coverage]]

#### Scenario: General topic exists
- **WHEN** a user runs `ah explain workflow`
- **THEN** the command prints general workflow guidance and exits zero
- **VERIFIES** [[explain.P-coverage]]

#### Scenario: Topic with no kind or action is refused
- **WHEN** a user runs `ah explain` with a topic identifier that corresponds to no `FindingKind`, `SuggestedAction`, or general topic
- **THEN** the command exits non-zero and prints no guidance for it
- **VERIFIES** [[explain.P-unknown]]

### Requirement: Structured JSON output
The system SHALL support `--json` output for `ah explain` that emits a machine-readable object.

#### Scenario: JSON output has required fields
- **WHEN** a user runs `ah explain no-toml --json`
- **THEN** the output is a valid JSON object containing: `topic` (string), `summary` (string), `when` (string), `do` (array of strings), `human_approval` (boolean), `related_topics` (array of strings), `hints` (array of objects)
- **AND** each `hints` item contains `kind` (string) and `message` (string)
- **VERIFIES** [[explain.P-json]]

#### Scenario: JSON output is valid for every topic
- **GIVEN** any valid topic identifier
- **WHEN** `ah explain <topic> --json` is run
- **THEN** the output passes JSON schema validation
- **VERIFIES** [[explain.P-json]]

#### Scenario: JSON output is not emitted for an unknown topic
- **WHEN** a user runs `ah explain no-such-topic --json`
- **THEN** the command exits non-zero and emits a plain-text error, never a malformed JSON object
- **VERIFIES** [[explain.P-unknown]]

### Requirement: Topic listing
The system SHALL enumerate all available topics on demand.

#### Scenario: List enumerates all topics
- **WHEN** a user runs `ah explain --list`
- **THEN** the command prints all topic identifiers, one per line, and exits zero
- **VERIFIES** [[explain.P-listing]]

#### Scenario: List is stable across runs
- **WHEN** `ah explain --list` is run twice in succession
- **THEN** the output is identical (topics are sorted alphabetically)
- **VERIFIES** [[explain.P-listing]]

#### Scenario: Invalid extra argument does not corrupt the listing
- **WHEN** `ah explain --list` is run with a stray positional argument
- **THEN** the command still prints the full sorted topic list and exits zero
- **VERIFIES** [[explain.P-listing]]

### Requirement: Unknown topic handling
When a user requests an unknown `ah explain` topic, the system SHALL exit non-zero and print either `Run ah explain --list` or the sorted list of available topic identifiers.

#### Scenario: Unknown topic exits non-zero
- **WHEN** a user runs `ah explain no-such-topic`
- **THEN** the command exits non-zero
- **AND** the error message lists available topics or directs the user to `ah explain --list`
- **VERIFIES** [[explain.P-unknown]]

### Requirement: Quality finding kind topics
The system SHALL provide `ah explain` topics for every quality finding kind introduced by this change: `quality-mutation`, `quality-property`, `quality-snapshot`. These are `FindingKind` values and are therefore subject to the compile-enforcement requirement.

#### Scenario: quality-mutation topic exists
- **WHEN** a user runs `ah explain quality-mutation`
- **THEN** the command prints guidance explaining what the mutation score means, how to enable mutation testing, and when the finding appears
- **VERIFIES** [[explain.P-quality]]

#### Scenario: quality-property topic exists
- **WHEN** a user runs `ah explain quality-property`
- **THEN** the command prints guidance for the `quality-property` finding kind and exits zero
- **VERIFIES** [[explain.P-quality]]

#### Scenario: quality-snapshot topic exists
- **WHEN** a user runs `ah explain quality-snapshot`
- **THEN** the command prints guidance for the `quality-snapshot` finding kind and exits zero
- **VERIFIES** [[explain.P-quality]]

#### Scenario: Unknown quality topic is refused
- **WHEN** a user runs `ah explain quality-nonexistent`
- **THEN** the command exits non-zero and prints no quality guidance
- **VERIFIES** [[explain.P-unknown]]

### Requirement: Adapter topics ship with adapters
The system SHALL include `ah explain` topics for progressive-enablement capabilities when their adapter modules are compiled in.

#### Scenario: Pytest adapter contributes topic
- **GIVEN** the pytest adapter is compiled into the binary
- **WHEN** a user runs `ah explain pytest`
- **THEN** the command prints guidance for enabling and using the pytest adapter
- **VERIFIES** [[explain.P-adapter]]

#### Scenario: Duplicate topic registration is a compile error
- **GIVEN** two adapter modules attempt to register the same topic identifier
- **WHEN** the project is built
- **THEN** the build fails identifying the conflicting topic name
- **VERIFIES** [[explain.P-enforced]]
