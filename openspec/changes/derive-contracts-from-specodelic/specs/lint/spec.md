---
id: spec
kind: intent
statement: "WHEN the lint bridge inspects kind-intent files THE linter SHALL never skip a specodelic file silently, SHALL relay spk graph findings, and SHALL report scenario-to-property trace gaps as advisory findings that never affect the exit code."
---

# specodelic relay completeness and scenario property trace

The dual-format lint bridge activates only when frontmatter `id: spec` matches
the filename stem; any other `kind: intent` file is skipped by a bare
`continue` with no finding and no `spk-unavailable` advisory — a silent miss
(verified in `src/lint/bridge.rs::relay_with`). The bridge also never runs
`spk graph`, so typing violations and dangling references surface nowhere in
`ah lint`, and the new `VERIFIES` trace convention has no checker at all.

This change closes the skip gap, extends the relay to `spk graph`, and adds
advisory-only trace findings for the scenario-to-property linkage introduced
by property-derived contracts.

## Constraints

| id | kind | expr | traces_to |
|----|------|------|-----------|
| C-no-silent-skip | invariant | a kind-intent file whose frontmatter id does not match its filename stem emits an spk-frontmatter-mismatch finding instead of being skipped silently | [[spec]] |
| C-graph-relay | invariant | spk graph typing violations and dangling references for dual-format files are relayed as spk.graph.* warning findings | [[spec]] |
| C-trace-advisory | advisory | verifies-dangling, scenario-unlinked, and property-untraced are reported at advisory severity and never affect the lint exit code | [[spec]] |
| C-plain-untouched | advisory | plain openspec files never trigger an spk invocation and produce no specodelic findings | [[spec]] |

## Model

### States

- `collecting`
- `relaying`
- `tracing`
- `reported`

### Transitions

| id | from | to | guard |
|----|------|----|-------|
| t-relay | collecting | relaying | [[spec.C-no-silent-skip]] |
| t-graph | relaying | tracing | [[spec.C-graph-relay]] |
| t-trace | tracing | reported | [[spec.C-graph-relay]] |
| t-plain | relaying | reported | [[spec.C-no-silent-skip]] |

## Properties

| id | kind | derives_from | generator | predicate |
|----|------|--------------|-----------|-----------|
| P-mismatch | unit | [[spec.C-no-silent-skip]] | a kind-intent file whose frontmatter id differs from its filename stem | lint emits spk-frontmatter-mismatch naming the file and the expected id |
| P-graph | unit | [[spec.C-graph-relay]] | a dual-format corpus containing a typing violation and a dangling reference | both are relayed as spk.graph.* warnings |
| P-trace | unit | [[spec.C-trace-advisory]] | a dual-format corpus with a dangling VERIFIES link, an unlinked scenario, and an untraced property | all three appear as advisory findings; the exit code is zero |
| P-plain | unit | [[spec.C-plain-untouched]] | a corpus of plain openspec spec files | lint output contains no specodelic findings and no spk invocation occurs |

## Purpose

Make the specodelic relay complete — no silent skips, graph findings included —
and give the scenario-to-property trace convention its own advisory checker.

## ADDED Requirements

### Requirement: Specodelic Relay Completeness
The linter SHALL emit an `spk-frontmatter-mismatch` finding for every `kind: intent` file whose frontmatter id does not match its filename stem instead of skipping it silently, and SHALL relay `spk graph` typing violations and dangling references as `spk.graph.*` warning findings.

#### Scenario: Mismatched id is reported
- **GIVEN** a `kind: intent` file whose frontmatter id differs from its filename stem
- **WHEN** `ah lint` runs over it
- **THEN** an `spk-frontmatter-mismatch` finding names the file and the expected id
- **AND** no specodelic file is skipped without a finding

#### Scenario: Graph findings relayed
- **GIVEN** a dual-format corpus containing a typing violation and a dangling reference
- **WHEN** `ah lint` runs over the corpus
- **THEN** both are relayed as `spk.graph.*` warnings

#### Scenario: Clean dual-format file unchanged
- **GIVEN** a dual-format file that passes `spk lint` and `spk graph`
- **WHEN** `ah lint` runs over it
- **THEN** no specodelic findings are emitted for the file

### Requirement: Scenario Property Trace
The linter SHALL report advisory trace findings — `verifies-dangling`, `scenario-unlinked`, and `property-untraced` — for dual-format files with Properties rows, without affecting the exit code.

#### Scenario: Dangling verifies link
- **GIVEN** a scenario carries `**VERIFIES** [[spec.P-missing]]` and no such property exists
- **WHEN** `ah lint` runs over the file
- **THEN** a `verifies-dangling` advisory finding is emitted

#### Scenario: Unlinked scenario and untraced property
- **GIVEN** a dual-format file with Properties rows where one scenario has no `VERIFIES` link and one property is cited by no scenario
- **WHEN** `ah lint` runs over the file
- **THEN** `scenario-unlinked` and `property-untraced` advisory findings are emitted
- **AND** the exit code is zero

#### Scenario: Plain files untouched
- **GIVEN** a corpus of plain openspec spec files
- **WHEN** `ah lint` runs over the corpus
- **THEN** no specodelic findings are emitted and no spk invocation occurs

## Requirements

### Requirement: Specodelic Relay Completeness
The linter SHALL emit an `spk-frontmatter-mismatch` finding for every `kind: intent` file whose frontmatter id does not match its filename stem instead of skipping it silently, and SHALL relay `spk graph` typing violations and dangling references as `spk.graph.*` warning findings.

#### Scenario: Mismatched id is reported
- **GIVEN** a `kind: intent` file whose frontmatter id differs from its filename stem
- **WHEN** `ah lint` runs over it
- **THEN** an `spk-frontmatter-mismatch` finding names the file and the expected id
- **AND** no specodelic file is skipped without a finding

#### Scenario: Graph findings relayed
- **GIVEN** a dual-format corpus containing a typing violation and a dangling reference
- **WHEN** `ah lint` runs over the corpus
- **THEN** both are relayed as `spk.graph.*` warnings

#### Scenario: Clean dual-format file unchanged
- **GIVEN** a dual-format file that passes `spk lint` and `spk graph`
- **WHEN** `ah lint` runs over it
- **THEN** no specodelic findings are emitted for the file

### Requirement: Scenario Property Trace
The linter SHALL report advisory trace findings — `verifies-dangling`, `scenario-unlinked`, and `property-untraced` — for dual-format files with Properties rows, without affecting the exit code.

#### Scenario: Dangling verifies link
- **GIVEN** a scenario carries `**VERIFIES** [[spec.P-missing]]` and no such property exists
- **WHEN** `ah lint` runs over the file
- **THEN** a `verifies-dangling` advisory finding is emitted

#### Scenario: Unlinked scenario and untraced property
- **GIVEN** a dual-format file with Properties rows where one scenario has no `VERIFIES` link and one property is cited by no scenario
- **WHEN** `ah lint` runs over the file
- **THEN** `scenario-unlinked` and `property-untraced` advisory findings are emitted
- **AND** the exit code is zero

#### Scenario: Plain files untouched
- **GIVEN** a corpus of plain openspec spec files
- **WHEN** `ah lint` runs over the corpus
- **THEN** no specodelic findings are emitted and no spk invocation occurs