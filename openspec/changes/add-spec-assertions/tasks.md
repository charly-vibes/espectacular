## 1. Gate core

- [ ] 1.1 Red: add failing tests for OpenSpec scenario discovery from `#### Scenario:` headings.
- [ ] 1.2 Green: implement deployed-spec scenario discovery.
- [ ] 1.3 Refactor: isolate markdown heading parsing and source-location reporting.
- [ ] 1.4 Red: add failing tests for slugification, scenario-id collisions, and id/file/slug mismatch.
- [ ] 1.5 Green: implement deterministic slugification, collision detection, and contract id matching.
- [ ] 1.6 Refactor: centralize scenario identity normalization.
- [ ] 1.7 Red: add failing tests for `.espectacular/config.toml` schema validation against `schemas/config.schema.json`.
- [ ] 1.8 Green: implement config loading, path defaults, tool-version parsing, and runner argv map validation.
- [ ] 1.9 Red: add failing tests for per-scenario TOML schema validation against `schemas/scenario-contract.schema.json`, status enum validation, and superseded metadata.
- [ ] 1.10 Green: implement contract loading and schema validation.
- [ ] 1.11 Red: add failing tests for missing contracts, orphan contracts, duplicate contracts, and no tests declared.
- [ ] 1.12 Green: implement structural correspondence findings.
- [ ] 1.13 Refactor: separate structural finding construction from filesystem traversal.

## 2. Test execution and JSON output

- [ ] 2.1 Red: add failing tests for composing configured runner commands with TOML test flags.
- [ ] 2.2 Green: implement non-shell test command composition and execution.
- [ ] 2.3 Red: add failing tests for `tests.shell` command execution.
- [ ] 2.4 Green: execute shell tests through the system shell.
- [ ] 2.5 Red: add failing tests proving non-zero exits and timeouts become `test-failing` execution findings.
- [ ] 2.6 Green: map command exit status, timeout flag, bounded stdout/stderr tails, and test type into execution findings.
- [ ] 2.7 Red: add failing tests for repository-root working directory, inherited environment, sequential execution, and `/bin/sh -c` shell mode.
- [ ] 2.8 Green: implement deterministic execution defaults.
- [ ] 2.9 Red: add failing snapshot/schema tests for `ah check` JSON output against `schemas/check-output.schema.json`, including success output.
- [ ] 2.10 Green: emit stable JSON with scope, summary, findings, scenario context, command details, and exit status.
- [ ] 2.11 Refactor: keep JSON serialization deterministic and ordered by spec path plus scenario id.

## 3. Change-overlay support

- [ ] 3.1 Red: add failing tests for `ah check --changes <name>` using deployed-plus-change overlay validation.
- [ ] 3.2 Green: load OpenSpec change scenarios and staged contracts from `.espectacular/changes/<change>/`.
- [ ] 3.3 Red: add failing tests for multiple `--changes` flags and conflicting new scenario ids.
- [ ] 3.4 Green: implement multi-change overlays and conflict findings.
- [ ] 3.5 Red: add failing tests for staged superseded metadata updates on deployed scenarios and missing replacement ids.
- [ ] 3.6 Green: apply staged superseded contracts over deployed contracts in change scope and reject dangling `superseded_by` values.
- [ ] 3.7 Refactor: share deployed and overlay scope resolution.

## 4. Lifecycle commands

- [ ] 4.1 Red: add failing tests for idempotent `ah init` file creation, missing-`openspec/` refusal, and managed-block refresh.
- [ ] 4.2 Green: implement `ah init` for `.espectacular/config.toml`, `.espectacular/AGENTS.md`, top-level `AGENTS.md`, and `CLAUDE.md`.
- [ ] 4.3 Red: add failing tests for `ah init` stubbing empty TOML contracts for existing deployed scenarios.
- [ ] 4.4 Green: implement deployed-scenario contract stubbing without overwriting existing contracts.
- [ ] 4.5 Red: add failing tests for hook detection precedence: `lefthook`, then `prek`, no raw git hook fallback.
- [ ] 4.6 Green: implement supported pre-commit hook integration and missing-framework concern reporting.
- [ ] 4.7 Red: add failing tests for `ah doctor` setup checks.
- [ ] 4.8 Green: implement `ah doctor` for config, paths, version compatibility, managed blocks, hooks, collisions, orphans, and archetype names.
- [ ] 4.9 Red: add failing tests for `ah scenario new <change> <spec> --requirement "<requirement>" "<heading>"` including exact markdown and TOML skeletons.
- [ ] 4.10 Green: implement scenario creation under an existing requirement and matching staged contract creation.
- [ ] 4.11 Red: add failing tests that scenario creation fails without the target change spec file or requirement heading.
- [ ] 4.12 Green: implement non-destructive failure for missing scenario creation targets.
- [ ] 4.13 Red: add failing tests for `ah scenario supersede <spec> <old-id> --with=<new-id> --in-change=<change>` and missing replacement ids.
- [ ] 4.14 Green: implement staged supersession contract creation and replacement-id validation.
- [ ] 4.15 Red: add failing tests for `ah archive <change>` moving staged contracts, refusing collisions, and refusing pre-OpenSpec-archive orphans.
- [ ] 4.16 Green: implement archive precondition checks, moves, and allowed superseded-contract replacement.
- [ ] 4.17 Red: add failing tests for `ah upgrade` reporting tool-version drift and compatibility changes without rewriting existing contract `authored_with` values.
- [ ] 4.18 Green: implement `ah upgrade` reporting and config update only.
- [ ] 4.19 Refactor: extract shared managed-file and filesystem-write helpers.

## 5. Archetype documentation commands

- [ ] 5.1 Red: add failing tests that `ah type` lists PF, SA, BP, and CE with one-line descriptions.
- [ ] 5.2 Green: embed the v1 archetype catalog and implement `ah type`.
- [ ] 5.3 Red: add failing tests that `ah type <archetype>` prints full documentation and unknown archetypes fail clearly.
- [ ] 5.4 Green: implement detailed archetype lookup and compatibility-mode access.
- [ ] 5.5 Refactor: keep archetype catalog append-only and version-addressable.

## 6. Documentation and validation

- [ ] 6.1 Document local pre-commit as convenience gate and CI `ah check` as enforcement gate.
- [ ] 6.2 Document `.espectacular/` layout, config schema, scenario TOML schema, and schema file paths.
- [ ] 6.3 Document append-only scenario workflow, requirement targeting, and supersession rules.
- [ ] 6.4 Document `ah check` JSON output schema, success output, and finding kinds.
- [ ] 6.5 Document that test meaningfulness and prose-drift detection are non-goals.
- [ ] 6.6 Run `openspec validate add-spec-assertions --strict` and fix all validation errors.
