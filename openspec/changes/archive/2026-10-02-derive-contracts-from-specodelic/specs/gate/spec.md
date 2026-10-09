---
id: gate
kind: intent
statement: "WHEN a dual-format spec whose specodelic half is lint-clean carries Properties rows THE gate SHALL derive one scenario contract per row, SHALL cover every scenario that carries a VERIFIES link to that property, and SHALL detect drift between a derived contract and its source row."
---

# property-derived contracts

Today the specodelic half of a dual-format spec is inert decoration: the
Properties rows that carry the strongest test intent never feed the gate, and
teams re-declare the same behavior as hand-authored contract TOMLs that drift
silently. This change makes Properties rows authoritative for test intent in
files that opt in, with scenarios as human prose linked to properties via
`**VERIFIES**` bullets.

Derivation consumes the `spk parse --json` structured IR (never re-parses
specodelic markdown) and is refused for files whose `spk lint` is not clean.
Contracts record a canonical-JSON hash of their source row so `ah sync` can
refresh derived fields while human-owned fields (`tests`, `status`,
`superseded_by`) are never overwritten.

## Constraints

| id | kind | expr | traces_to |
|----|------|------|-----------|
| C-property-contract | invariant | in a dual-format spec whose specodelic half is spk lint-clean, each Properties row derives exactly one scenario contract whose id is the slugified property id | [[gate]] |
| C-verifies-covers | invariant | a scenario carrying a VERIFIES bullet linking to an existing property id is covered by that property's derived contract and emits no no-tests-declared finding | [[gate]] |
| C-unlinked-unchanged | invariant | a scenario without a VERIFIES link keeps the per-scenario contract requirement and may still emit no-tests-declared | [[gate]] |
| C-derived-stale | invariant | a derived contract whose derived_from hash does not match the canonical serialization of its source property row emits a contract-stale structural finding | [[gate]] |
| C-sync-ownership | invariant | ah sync refreshes only derived fields (id, description, archetype, falsifiability_class, derived_from) and never overwrites human-owned fields (tests, status, superseded_by) | [[gate]] |
| C-plain-unaffected | advisory | plain openspec specs and dual-format specs without Properties rows discover, gate, and check byte-identically to before this change | [[gate]] |

## Model

### States

- `idle`
- `deriving`
- `matching`
- `reported`

### Transitions

| id | from | to | guard |
|----|------|----|-------|
| t-derive | idle | deriving | [[gate.C-property-contract]] |
| t-match | deriving | matching | [[gate.C-verifies-covers]] |
| t-stale | matching | reported | [[gate.C-derived-stale]] |
| t-own | matching | reported | [[gate.C-sync-ownership]] |
| t-unlinked | deriving | reported | [[gate.C-unlinked-unchanged]] |

## Properties

| id | kind | derives_from | generator | predicate |
|----|------|--------------|-----------|-----------|
| P-derive | unit | [[gate.C-property-contract]] | a lint-clean dual-format file with N property rows | the contract set contains exactly N derived contracts with slugified ids |
| P-covers | unit | [[gate.C-verifies-covers]] | a scenario carrying a VERIFIES link to an existing property id | no-tests-declared is suppressed for that scenario |
| P-unlinked | unit | [[gate.C-unlinked-unchanged]] | a scenario without a VERIFIES link | the gate requires its own contract exactly as before this change |
| P-stale | unit | [[gate.C-derived-stale]] | a derived contract whose source row predicate is edited after sync | contract-stale is emitted and ah sync --check exits non-zero without writing |
| P-ownership | unit | [[gate.C-sync-ownership]] | a derived contract with hand-filled tests and status, re-synced after a predicate edit | derived fields change; tests, status, and superseded_by are byte-identical |
| P-unaffected | unit | [[gate.C-plain-unaffected]] | a corpus of plain openspec specs and Properties-less dual-format specs | discovery and check output is byte-identical to pre-change behavior |

## Purpose

Make the specodelic Properties layer the single source of test intent for
opt-in dual-format files: contracts are derived, hash-pinned to their source
rows, and refreshed by `ah sync` without ever clobbering human test bindings.

## ADDED Requirements

### Requirement: Property-Derived Contracts
The system SHALL derive one scenario contract per Properties row in a dual-format spec whose specodelic half is lint-clean, using the slugified property id as the contract id, and SHALL treat a scenario carrying a `**VERIFIES** [[gate.P-...]]` link as covered by that property's contract.

#### Scenario: Derive contracts from properties
- **GIVEN** a dual-format spec is `spk lint`-clean and carries three Properties rows
- **WHEN** `ah sync` runs
- **THEN** exactly three derived contracts exist with slugified property ids
- **AND** each records `derived_from = "<property-id>@<hash>"`

#### Scenario: Cover linked scenario
- **GIVEN** a scenario carries `- **VERIFIES** [[gate.P-not-shipped]]` and the property's derived contract exists
- **WHEN** `ah check` validates the spec
- **THEN** the scenario emits no `no-tests-declared` finding

#### Scenario: Unlinked scenario unchanged
- **GIVEN** a scenario carries no `VERIFIES` link
- **WHEN** `ah check` validates the spec
- **THEN** the scenario requires its own contract exactly as before this change

#### Scenario: Plain specs unaffected
- **GIVEN** a corpus of plain openspec specs and dual-format specs without Properties rows
- **WHEN** discovery and check run over the corpus
- **THEN** their output is byte-identical to pre-change behavior

### Requirement: Derived Contract Drift
The system SHALL record `derived_from = "<property-id>@<hash>"` in each derived contract, emit a `contract-stale` structural finding when the hash no longer matches the source row's canonical serialization, and refresh only derived fields during `ah sync` — never `tests`, `status`, or `superseded_by`.

#### Scenario: Detect stale contract
- **GIVEN** a property row's predicate is edited after its contract was derived
- **WHEN** `ah check` validates the spec
- **THEN** a `contract-stale` structural finding is emitted
- **AND** `ah sync --check` exits non-zero without writing

#### Scenario: Refresh preserves owned fields
- **GIVEN** a derived contract has hand-filled `tests` and `status`
- **WHEN** the source predicate is edited and `ah sync` re-runs
- **THEN** derived fields are updated to match the row
- **AND** `tests`, `status`, and `superseded_by` are byte-identical to before

## Requirements

### Requirement: Property-Derived Contracts
The system SHALL derive one scenario contract per Properties row in a dual-format spec whose specodelic half is lint-clean, using the slugified property id as the contract id, and SHALL treat a scenario carrying a `**VERIFIES** [[gate.P-...]]` link as covered by that property's contract.

#### Scenario: Derive contracts from properties
- **GIVEN** a dual-format spec is `spk lint`-clean and carries three Properties rows
- **WHEN** `ah sync` runs
- **THEN** exactly three derived contracts exist with slugified property ids
- **AND** each records `derived_from = "<property-id>@<hash>"`

#### Scenario: Cover linked scenario
- **GIVEN** a scenario carries `- **VERIFIES** [[gate.P-not-shipped]]` and the property's derived contract exists
- **WHEN** `ah check` validates the spec
- **THEN** the scenario emits no `no-tests-declared` finding

#### Scenario: Unlinked scenario unchanged
- **GIVEN** a scenario carries no `VERIFIES` link
- **WHEN** `ah check` validates the spec
- **THEN** the scenario requires its own contract exactly as before this change

#### Scenario: Plain specs unaffected
- **GIVEN** a corpus of plain openspec specs and dual-format specs without Properties rows
- **WHEN** discovery and check run over the corpus
- **THEN** their output is byte-identical to pre-change behavior

### Requirement: Derived Contract Drift
The system SHALL record `derived_from = "<property-id>@<hash>"` in each derived contract, emit a `contract-stale` structural finding when the hash no longer matches the source row's canonical serialization, and refresh only derived fields during `ah sync` — never `tests`, `status`, or `superseded_by`.

#### Scenario: Detect stale contract
- **GIVEN** a property row's predicate is edited after its contract was derived
- **WHEN** `ah check` validates the spec
- **THEN** a `contract-stale` structural finding is emitted
- **AND** `ah sync --check` exits non-zero without writing

#### Scenario: Refresh preserves owned fields
- **GIVEN** a derived contract has hand-filled `tests` and `status`
- **WHEN** the source predicate is edited and `ah sync` re-runs
- **THEN** derived fields are updated to match the row
- **AND** `tests`, `status`, and `superseded_by` are byte-identical to before