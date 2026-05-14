## Prerequisites

- [ ] P.1 `add-spec-assertions` deployed and `ah check` baseline is green

## 0. Custom runner envelope schema (gates sections 3–6)

- [ ] 0.1 Red: add failing test asserting `schemas/custom-runner.schema.json` exists and is valid JSON Schema
- [ ] 0.2 Green: author `schemas/custom-runner.schema.json` — define envelope fields: `exit_code` (integer, required), `passed` (boolean, required), `findings` (array of full finding objects, required); document the schema inline
- [ ] 0.3 Red: add failing test asserting an empty `findings` array with `exit_code: 0` is a valid envelope (pass case)
- [ ] 0.4 Green: confirm schema accepts the empty-findings pass case
- [ ] 0.5 Refactor: cross-reference the envelope schema from `schemas/check-output.schema.json` so both schemas share the finding object definition

## 1. Finding schema extension

- [ ] 1.1 Red: add failing tests asserting every finding carries `suggested_action`, `playbook_command`, and (when applicable) `scenario_prose`
- [ ] 1.2 Green: extend finding schema types to include agent-action fields; update `schemas/check-output.schema.json`
- [ ] 1.3 Red: add failing snapshot tests for deterministic `(spec_path, scenario_id, kind)` sort order
- [ ] 1.4 Green: sort findings in the JSON emitter
- [ ] 1.5 Red: add failing test for `summary.counts_by_kind` presence and accuracy
- [ ] 1.6 Green: compute and emit counts-by-kind in the envelope summary
- [ ] 1.7 Refactor: centralize finding construction so agent-action fields cannot be omitted

## 2. Adapter trait and dispatch

- [ ] 2.1 Red: add failing tests for the `Adapter` trait interface (detect, invoke, normalize)
- [ ] 2.2 Green: define `Adapter` trait in `src/adapters/mod.rs` with detection precedence chain
- [ ] 2.3 Red: add failing tests for adapter dispatch selecting the correct adapter from config
- [ ] 2.4 Green: implement adapter dispatch in the gate runner
- [ ] 2.5 Refactor: isolate detection from invocation in the trait

## 3. Python pytest adapter

- [ ] 3.1 Red: add failing tests for pytest detection via `pyproject.toml`, environment, and source import
- [ ] 3.2 Green: implement detection in `src/adapters/python.rs`
- [ ] 3.3 Red: add failing tests for pytest invocation, exit-code normalization, and bounded tail capture
- [ ] 3.4 Green: implement invocation and normalization
- [ ] 3.5 Refactor: share tail-capture logic across adapters

## 4. Rust cargo test adapter

- [ ] 4.1 Red: add failing tests for cargo detection via `Cargo.toml`
- [ ] 4.2 Green: implement detection in `src/adapters/rust.rs`
- [ ] 4.3 Red: add failing tests for cargo invocation and exit-code normalization
- [ ] 4.4 Green: implement invocation and normalization
- [ ] 4.5 Refactor: align normalization path with pytest adapter

## 5. TypeScript vitest adapter

- [ ] 5.1 Red: add failing tests for vitest detection via `package.json`
- [ ] 5.2 Green: implement detection in `src/adapters/typescript.rs`
- [ ] 5.3 Red: add failing tests for vitest invocation and exit-code normalization
- [ ] 5.4 Green: implement invocation and normalization
- [ ] 5.5 Refactor: confirm tail-capture reuse from step 3.5

## 6. Custom runner plugin protocol

- [ ] 6.1 Red: add failing tests for custom runner envelope parsing against `schemas/custom-runner.schema.json`
- [ ] 6.2 Green: implement envelope parsing and normalization in `src/adapters/custom.rs`
- [ ] 6.3 Red: add failing tests for non-zero exit without valid envelope producing `test-failing`
- [ ] 6.4 Green: implement error path
- [ ] 6.5 Red: add failing test proving no custom runner runs without explicit config
- [ ] 6.6 Green: guard invocation behind config presence check

## 7. `ah doctor` detection and `--enable`

- [ ] 7.1 Red: add failing tests for framework detection reporting in `ah doctor` output (pytest, cargo, vitest, PBT tools)
- [ ] 7.2 Green: implement detection reporting in `src/doctor.rs`
- [ ] 7.3 Red: add failing tests for `recommendation` finding emitted when available framework is not configured
- [ ] 7.4 Green: emit recommendation findings with `apply_command` set to the `--enable` invocation
- [ ] 7.5 Red: add failing tests for `ah doctor --enable <capability>` writing config block
- [ ] 7.6 Green: implement `--enable` flag writing config block to `.espectacular/config.toml`
- [ ] 7.7 Red: add failing tests for unknown capability error and already-enabled no-op
- [ ] 7.8 Green: implement error and no-op paths
- [ ] 7.9 Refactor: share config-write path between `ah init` and `ah doctor --enable`

## 8. Quality measurement capabilities

- [ ] 8.1 Red: add failing tests for mutation finding emitted when `tests.mutation = true` and tool is configured
- [ ] 8.2 Green: implement mutation dispatch in `src/quality.rs`
- [ ] 8.3 Red: add failing tests proving mutation is skipped in pre-commit scope without explicit flag
- [ ] 8.4 Green: implement pre-commit scope guard
- [ ] 8.5 Red: add failing tests for `tests.property` and `tests.snapshot` finding emission
- [ ] 8.6 Green: implement property and snapshot capability dispatch
- [ ] 8.7 Red: add failing test proving quality findings do not cause non-zero exit
- [ ] 8.8 Green: ensure gate exit code is unaffected by quality finding severity
- [ ] 8.9 Refactor: unify quality finding construction

## 9. `ah explain` subcommand

- [ ] 9.1 Red: add failing build test proving missing variant body causes compile failure
- [ ] 9.2 Green: implement compile-time enforcement via proc macro or build.rs assertion
- [ ] 9.3 Red: add failing tests for markdown output for each `FindingKind` variant (including `quality-mutation`, `quality-property`, `quality-snapshot`) and each `SuggestedAction` variant
- [ ] 9.4 Green: implement topic bodies in `src/explain.rs` — quality finding kinds must have bodies that explain the score, how to enable the capability, and when the finding appears
- [ ] 9.5 Red: add failing tests for general topics (workflow, supersession, archetypes, progressive-enablement)
- [ ] 9.6 Green: implement general topic bodies
- [ ] 9.7 Red: add failing tests for `--json` output shape against provisional schema
- [ ] 9.8 Green: implement `--json` serialization
- [ ] 9.9 Red: add failing tests for `--list` stable output and unknown topic error
- [ ] 9.10 Green: implement listing and error path
- [ ] 9.11 Red: add failing test for each adapter contributing its own topic
- [ ] 9.12 Green: wire adapter topics into the explain registry
- [ ] 9.13 Refactor: ensure explain registry is the single source of topic truth

## 10. AGENTS.md update

- [ ] 10.0 Red: add failing test asserting that `ah init --refresh` produces a `.espectacular/AGENTS.md` containing exactly the single meta-instruction paragraph (no other content)
- [ ] 10.1 Shrink `.espectacular/AGENTS.md` to a single meta-instruction: run `playbook_command` on every finding before acting
- [ ] 10.2 Green: update `ah init` managed-block refresh to write the new AGENTS.md content; confirm test 10.0 passes

## 11. Integration and validation

- [ ] 11.1 Add end-to-end integration test: Python project with pytest, `ah check` produces zero findings
- [ ] 11.2 Add end-to-end integration test: Rust project with cargo test, `ah check` produces zero findings
- [ ] 11.3 Add end-to-end integration test: TypeScript project with vitest, `ah check` produces zero findings
- [ ] 11.4 Add end-to-end integration test: `ah explain missing-contract` exits zero with non-empty output
- [ ] 11.5 Run `openspec validate add-quality-measurement-and-adapters --strict` and resolve all issues
