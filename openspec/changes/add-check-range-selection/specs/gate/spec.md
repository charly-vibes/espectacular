---
id: gate
kind: intent
statement: "WHEN a user runs ah check --run-tests with --base <ref> THE gate SHALL derive the changed-file set from git diff --name-only <base> <head> and SHALL run only the contract tests of affected capabilities, and SHALL run every declared contract when the range yields no files or a ref fails to resolve."
---

# range-based test selection

`ah check --run-tests` derives its changed-file set from the worktree diff
only (`git diff --name-only HEAD`). At pre-push time the worktree is clean,
so the set is empty and every declared contract runs on every push —
measured at ~841s per push on tambor (~97% of the gate wall time). This
change adds `--base <ref>` / `--head <ref>` (mirroring testaruda's
`ChangeSet::from_diff`, testaruda-jdw5) so hooks can select from a commit
range instead, and defines the conservative fallback: a degenerate range
(empty diff or unresolvable ref) runs everything rather than passing with
nothing executed.

## Constraints

| id | kind | expr | traces_to |
|----|------|------|-----------|
| C-range-diff | invariant | when `--base <ref>` is given with `--run-tests` and no explicit file list, the changed-file set is `git diff --name-only <base> <head>`, with `--head` defaulting to `HEAD` | [[gate]] |
| C-range-precedence | invariant | the changed-file source precedence is total: `--all-tests` disables selection, `--files` wins over `--base`, `--base` wins over the worktree diff, and the worktree diff remains the only no-flag source | [[gate]] |
| C-range-safe | invariant | a range that yields no files or whose refs fail to resolve runs every declared contract; the gate never exits green having executed zero tests because of a degenerate range | [[gate]] |
| C-range-unchanged | invariant | with no `--base`/`--head`, selection input, report shape, finding kinds, and exit codes are byte-identical to pre-change behavior | [[gate]] |

## Model

### States
- collecting
- selecting
- executing
- reported

### Transitions

| id | from | to | event | guard |
|----|------|----|-------|-------|
| t-collect | collecting | selecting | the changed-file set resolves from files, range, or worktree diff per the precedence order | [[gate.C-range-precedence]] |
| t-select | selecting | executing | affected capabilities are chosen from the resolved set; a degenerate set selects every capability | [[gate.C-range-safe]] |
| t-report | executing | reported | verdicts emit findings and the selection report is unchanged in shape | [[gate.C-range-unchanged]] |

## Properties

| id | kind | derives_from | generator | predicate |
|----|------|--------------|-----------|-----------|
| P-range-files | unit | [[gate.C-range-diff]] | a fixture repo where one capability's spec changes after the base commit | `ah check --run-tests --base <base-sha>` runs only that capability's contract; the other capability's contract is absent from the selection report |
| P-range-precedence | unit | [[gate.C-range-precedence]] | the same fixture invoked with `--files`, with `--all-tests`, and with no flags | the explicit file list wins over the range; `--all-tests` runs everything; the no-flag run uses the worktree diff |
| P-range-safe | unit | [[gate.C-range-safe]] | the fixture invoked with `--base HEAD` and with `--base not-a-ref` | both invocations run every declared contract and exit per findings, never green with zero tests executed |
| P-range-unchanged | unit | [[gate.C-range-unchanged]] | the fixture invoked with no range flags | check output is byte-identical to pre-change behavior |

## Purpose

Make pre-push gating proportional: a push that touches one capability runs
that capability's contract tests, not the whole corpus, while any doubt
about the range (empty diff, bad ref) falls back to the full run that is
correct today.

## ADDED Requirements

### Requirement: Range-Based Test Selection
The system SHALL derive the `ah check --run-tests` changed-file set from the commit range `git diff --name-only <base> <head>` when `--base` is given, and SHALL run every declared contract when the range yields no files or a ref fails to resolve.

#### Scenario: Select contracts from a commit range
- **GIVEN** a fixture repo with two capabilities whose contracts declare tests
- **AND** a base commit after which only one capability's spec changed
- **WHEN** a user runs `ah check --run-tests --base <base-sha>`
- **THEN** only the changed capability's contract tests run
- **AND** the other capability's contract is absent from the selection report
- **VERIFIES** [[gate.P-range-files]]

#### Scenario: Head defaults to HEAD
- **GIVEN** the same fixture repo and base commit
- **WHEN** a user runs `ah check --run-tests --base <base-sha>` without `--head`
- **THEN** the selection is identical to `ah check --run-tests --base <base-sha> --head HEAD`
- **VERIFIES** [[gate.P-range-files]]

#### Scenario: Explicit files win over the range
- **GIVEN** both `--files <paths>` and `--base <ref>` are given
- **WHEN** a user runs `ah check --run-tests`
- **THEN** the explicit file list supplies the changed-file set and the range is ignored
- **VERIFIES** [[gate.P-range-precedence]]

#### Scenario: All-tests overrides the range
- **GIVEN** both `--all-tests` and `--base <ref>` are given
- **WHEN** a user runs `ah check --run-tests`
- **THEN** every declared contract runs regardless of the range
- **VERIFIES** [[gate.P-range-precedence]]

#### Scenario: Empty range runs everything
- **GIVEN** `--base` resolves to the same commit as `--head`, so the range diff is empty
- **WHEN** a user runs `ah check --run-tests --base HEAD`
- **THEN** every declared contract runs
- **AND** the run exits per its findings, never green with zero tests executed
- **VERIFIES** [[gate.P-range-safe]]

#### Scenario: Unresolvable ref runs everything
- **GIVEN** `--base not-a-ref` fails to resolve, so `git diff` exits non-zero
- **WHEN** a user runs `ah check --run-tests --base not-a-ref`
- **THEN** every declared contract runs
- **AND** the command does not crash and exits per its findings
- **VERIFIES** [[gate.P-range-safe]]

#### Scenario: No range flags keep worktree behavior
- **GIVEN** no `--base` or `--head` is given
- **WHEN** a user runs `ah check --run-tests`
- **THEN** the changed-file set comes from the worktree diff exactly as before
- **AND** the selection report, finding kinds, and exit codes are unchanged
- **VERIFIES** [[gate.P-range-unchanged]]

## Requirements

### Requirement: Range-Based Test Selection
The system SHALL derive the `ah check --run-tests` changed-file set from the commit range `git diff --name-only <base> <head>` when `--base` is given, and SHALL run every declared contract when the range yields no files or a ref fails to resolve.

#### Scenario: Select contracts from a commit range
- **GIVEN** a fixture repo with two capabilities whose contracts declare tests
- **AND** a base commit after which only one capability's spec changed
- **WHEN** a user runs `ah check --run-tests --base <base-sha>`
- **THEN** only the changed capability's contract tests run
- **AND** the other capability's contract is absent from the selection report
- **VERIFIES** [[gate.P-range-files]]

#### Scenario: Head defaults to HEAD
- **GIVEN** the same fixture repo and base commit
- **WHEN** a user runs `ah check --run-tests --base <base-sha>` without `--head`
- **THEN** the selection is identical to `ah check --run-tests --base <base-sha> --head HEAD`
- **VERIFIES** [[gate.P-range-files]]

#### Scenario: Explicit files win over the range
- **GIVEN** both `--files <paths>` and `--base <ref>` are given
- **WHEN** a user runs `ah check --run-tests`
- **THEN** the explicit file list supplies the changed-file set and the range is ignored
- **VERIFIES** [[gate.P-range-precedence]]

#### Scenario: All-tests overrides the range
- **GIVEN** both `--all-tests` and `--base <ref>` are given
- **WHEN** a user runs `ah check --run-tests`
- **THEN** every declared contract runs regardless of the range
- **VERIFIES** [[gate.P-range-precedence]]

#### Scenario: Empty range runs everything
- **GIVEN** `--base` resolves to the same commit as `--head`, so the range diff is empty
- **WHEN** a user runs `ah check --run-tests --base HEAD`
- **THEN** every declared contract runs
- **AND** the run exits per its findings, never green with zero tests executed
- **VERIFIES** [[gate.P-range-safe]]

#### Scenario: Unresolvable ref runs everything
- **GIVEN** `--base not-a-ref` fails to resolve, so `git diff` exits non-zero
- **WHEN** a user runs `ah check --run-tests --base not-a-ref`
- **THEN** every declared contract runs
- **AND** the command does not crash and exits per its findings
- **VERIFIES** [[gate.P-range-safe]]

#### Scenario: No range flags keep worktree behavior
- **GIVEN** no `--base` or `--head` is given
- **WHEN** a user runs `ah check --run-tests`
- **THEN** the changed-file set comes from the worktree diff exactly as before
- **AND** the selection report, finding kinds, and exit codes are unchanged
- **VERIFIES** [[gate.P-range-unchanged]]
