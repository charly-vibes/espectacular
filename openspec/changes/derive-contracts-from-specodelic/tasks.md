# Tasks: derive-contracts-from-specodelic

## 0. Spikes (resolved — recorded answers, no remaining work)

- [x] 0.1 Zero-match `cargo test` filter exits 0 ("0 passed; 1 filtered out") → the zero-tests-ran guard is **required** (lands in `src/runner.rs`)
- [x] 0.2 `openspec validate --strict` accepts a `**VERIFIES**` bullet inside a scenario body → keep the link in the bullets
- [x] 0.3 `spk lint --json` envelope fields confirmed (`ok` / `data.issues` / `warnings` / `hints`); pin `spk` 0.2.x in `.espectacular/config.toml` when the parse release lands
- [x] 0.4 Superseded by upstream change `add-parse-command` in specodelic (structured IR export; the lib already serializes `Spec`)

## 1. Upstream dependency (specodelic)

- [x] 1.1 `spk parse --json` implemented, released, and pinned: specodelic `add-parse-command` shipped in specodelic 0.3.0 (crates.io); CI pin moved from git-main to `--version 0.3.0`; `specodelic = "0.3.0"` added to `versions.ddl.toml`; spk pin recorded in `.espectacular/config.toml` `[spk]` (consumer lands with group 2); archive round-trip re-validated with `spk archive-companion --dry-run` (resolves cli-core, gate, lint restore plan; Open question 5)

## 2. Derivation core (TDD)

- [x] 2.1 Failing tests: `spk parse --json` IR deserialization (properties with `id|kind|derives_from|generator|predicate` cells), tolerant of unknown envelope fields and `data: null`
- [x] 2.2 Failing tests: row canonical-JSON hash stability (whitespace reordering does not change the hash; editing a cell does)
- [x] 2.3 Failing tests: archetype / falsifiability inference per design D5 table
- [x] 2.4 Implement the derivation module; add optional `derived_from` to `schemas/scenario-contract.schema.json`

## 3. `ah sync`

- [x] 3.1 Register `sync` in the `Command` enum in `src/main.rs` per `cli-core` conventions (this delta's cli-core requirement)
- [x] 3.2 Failing tests: refuses to write when `spk lint` fails or `spk` is missing; exits non-zero with a reason (`spk-unavailable` for missing binary)
- [x] 3.3 Implement create-or-refresh preserving `tests`, `status`, `superseded_by`
- [x] 3.4 `--check` mode: no writes, non-zero exit on drift or missing contracts
- [x] 3.5 Resolve the overrides question (design Open question 3) and record the decision in the design doc

## 4. Gate integration

- [x] 4.1 Failing tests: scenario with `**VERIFIES** [[spec.P-x]]` yields no `no-tests-declared`; unlinked scenario still does
- [x] 4.2 `contract-stale` structural finding from hash mismatch
- [x] 4.3 Zero-tests-ran guard in `src/runner.rs`: a `flags` binding matching zero tests fails instead of passing
- [x] 4.4 Add `ah explain` topics: `contract-stale`, `spk-frontmatter-mismatch`, `verifies-dangling`
- [x] 4.5 Regression: plain openspec specs and dual-format specs without Properties discover and check byte-identically to before (all 8 deployed specs have Properties rows — this is repo-wide)

## 5. Lint

- [x] 5.1 `spk-frontmatter-mismatch` for `kind: intent` files the bridge would otherwise skip (bare `continue` in `relay_with`)
- [x] 5.2 Relay `spk graph` typing violations and dangling refs as `spk.graph.*` warnings
- [x] 5.3 Trace checks: `verifies-dangling`, advisory `scenario-unlinked` and `property-untraced`; none affect exit code

## 6. Docs, governance, pilot

- [x] 6.1 **Governance decision (human-confirmed)**: settle the derived-contract commit policy per design D8 — amend the "don't commit `.espectacular/` state" rule with the committed-artifact carve-out before any sync writes TOMLs
- [ ] 6.2 Extend `docs/src/dual-format-authoring.md`: the `VERIFIES` convention, derived vs owned fields, test naming rule
- [ ] 6.3 Pilot on the `lint` capability: add links, `ah sync`, bind tests, delete redundant contracts
- [ ] 6.4 CI: run `ah sync --check` and `spk lint`/`spk graph` directly alongside `ah check`

## 7. Validation

- [ ] 7.1 `spk lint openspec/changes/derive-contracts-from-specodelic/specs` exits zero
- [ ] 7.2 `openspec validate derive-contracts-from-specodelic --strict` exits zero
- [ ] 7.3 `ah check --changes derive-contracts-from-specodelic` reports no mirror-duplication conflicts
- [ ] 7.4 Archive round-trip (Open question 5). Known issue: spk 0.3.0's widened mirror rule is unsatisfiable for dual-format files carrying both ADDED and MODIFIED deltas — filed upstream as specodelic#8; `just spec-lint` stays red on the archived adopt-genesis cli file until the fix (extended mirror committed here is the correct mirror for the per-requirement-containment fix): validate against plain `openspec archive` now; re-validate with `spk archive-companion` after group 1