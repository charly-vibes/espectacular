## 1. Gate core

- [x] 1.1 Red: add failing tests for OpenSpec scenario discovery from `#### Scenario:` headings.
- [x] 1.2 Green: implement deployed-spec scenario discovery.
- [x] 1.3 Refactor: isolate markdown heading parsing and source-location reporting.
- [x] 1.4 Red: add failing tests for slugification, scenario-id collisions, and id/file/slug mismatch.
- [x] 1.5 Green: implement deterministic slugification, collision detection, and contract id matching.
- [x] 1.6 Refactor: centralize scenario identity normalization.
- [x] 1.7 Red: add failing tests for `.espectacular/config.toml` schema validation against `schemas/config.schema.json`.
- [x] 1.8 Green: implement config loading, path defaults, tool-version parsing, and runner argv map validation.
- [x] 1.9 Red: add failing tests for per-scenario TOML schema validation against `schemas/scenario-contract.schema.json`, status enum validation, and superseded metadata.
- [x] 1.10 Green: implement contract loading and schema validation.
- [x] 1.11 Red: add failing tests for missing contracts, orphan contracts, duplicate contracts, and no tests declared.
- [x] 1.12 Green: implement structural correspondence findings.
- [x] 1.13 Refactor: separate structural finding construction from filesystem traversal.

## 2. Test execution and JSON output

- [x] 2.1 Red: add failing tests for composing configured runner commands with TOML test flags.
- [x] 2.2 Green: implement non-shell test command composition and execution.
- [x] 2.3 Red: add failing tests for `tests.shell` command execution.
- [x] 2.4 Green: execute shell tests through the system shell.
- [x] 2.5 Red: add failing tests proving non-zero exits and timeouts become `test-failing` execution findings.
- [x] 2.6 Green: map command exit status, timeout flag, bounded stdout/stderr tails, and test type into execution findings.
- [x] 2.7 Red: add failing tests for repository-root working directory, inherited environment, sequential execution, and `/bin/sh -c` shell mode.
- [x] 2.8 Green: implement deterministic execution defaults.
- [x] 2.9 Red: add failing snapshot/schema tests for `ah check` JSON output against `schemas/check-output.schema.json`, including success output.
- [x] 2.10 Green: emit stable JSON with scope, summary, findings, scenario context, command details, and exit status.
- [x] 2.11 Refactor: keep JSON serialization deterministic and ordered by spec path plus scenario id.

## 3. Change-overlay support

- [x] 3.1 Red: add failing tests for `ah check --changes <name>` using deployed-plus-change overlay validation.
- [x] 3.2 Green: load OpenSpec change scenarios and staged contracts from `.espectacular/changes/<change>/`.
- [x] 3.3 Red: add failing tests for multiple `--changes` flags, deterministic lexicographic overlay resolution, and conflicting new scenario ids.
- [x] 3.4 Green: implement multi-change overlays, sorted change-id normalization, and conflict findings.
- [x] 3.5 Red: add failing tests for staged superseded metadata updates on deployed scenarios, conflicting staged updates for one deployed scenario id, and missing replacement ids.
- [x] 3.6 Green: apply staged superseded contracts over deployed contracts in change scope, reject duplicate staged updates, and reject dangling `superseded_by` values.
- [x] 3.7 Refactor: share deployed and overlay scope resolution.

## 4. Lifecycle commands

- [x] 4.1 Red: add failing tests for idempotent `ah init` file creation, missing-`openspec/` refusal, and managed-block refresh.
- [x] 4.2 Green: implement `ah init` for `.espectacular/config.toml`, `.espectacular/AGENTS.md`, top-level `AGENTS.md`, and `CLAUDE.md`.
- [x] 4.3 Red: add failing tests for `ah init` stubbing empty TOML contracts for existing deployed scenarios.
- [x] 4.4 Green: implement deployed-scenario contract stubbing without overwriting existing contracts.
- [x] 4.5 Red: add failing tests for hook detection precedence: `lefthook`, then `prek`, no raw git hook fallback.
- [x] 4.6 Green: implement supported pre-commit hook integration and missing-framework concern reporting.
- [x] 4.7 Red: add failing tests for `ah doctor` setup checks.
- [x] 4.8 Green: implement `ah doctor` for config, paths, version compatibility, managed blocks, hooks, collisions, orphans, and archetype names.
- [x] 4.9 Red: add failing tests for `ah scenario new <change> <spec> --requirement "<requirement>" "<heading>"` including exact markdown and TOML skeletons.
- [x] 4.10 Green: implement scenario creation under an existing requirement and matching staged contract creation.
- [x] 4.11 Red: add failing tests that scenario creation fails without the target change spec file or requirement heading.
- [x] 4.12 Green: implement non-destructive failure for missing scenario creation targets.
- [x] 4.13 Red: add failing tests for `ah scenario supersede <spec> <old-id> --with=<new-id> --in-change=<change>` and missing replacement ids.
- [x] 4.14 Green: implement staged supersession contract creation and replacement-id validation.
- [x] 4.15 Red: add failing tests for `ah archive <change>` moving staged contracts, refusing collisions, and refusing pre-OpenSpec-archive orphans.
- [x] 4.16 Green: implement archive precondition checks, moves, and allowed superseded-contract replacement.
- [x] 4.17 Red: add failing tests for `ah upgrade` reporting tool-version drift and compatibility changes without rewriting existing contract `authored_with` values.
- [x] 4.18 Green: implement `ah upgrade` reporting and config update only.
- [x] 4.19 Refactor: extract shared managed-file and filesystem-write helpers.

## 5. Archetype documentation commands

- [x] 5.1 Red: add failing tests that `ah type` lists PF, SA, BP, and CE with one-line descriptions.
- [x] 5.2 Green: embed the v1 archetype catalog and implement `ah type`.
- [x] 5.3 Red: add failing tests that `ah type <archetype>` prints full documentation and unknown archetypes fail clearly.
- [x] 5.4 Green: implement detailed archetype lookup and compatibility-mode access.
- [x] 5.5 Refactor: keep archetype catalog append-only and version-addressable.

## 6. Documentation and validation

- [x] 6.1 Document local pre-commit as convenience gate and CI `ah check` as enforcement gate.
- [x] 6.2 Document `.espectacular/` layout, config schema, scenario TOML schema, and schema file paths.
- [x] 6.3 Document append-only scenario workflow, requirement targeting, and supersession rules.
- [x] 6.4 Document `ah check` JSON output schema, success output, and finding kinds.
- [x] 6.5 Document that test meaningfulness and prose-drift detection are non-goals.
- [x] 6.6 Run `openspec validate add-spec-assertions --strict` and fix all validation errors.
