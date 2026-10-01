# Proposal: derive-contracts-from-specodelic

Status: proposed (2026-10-01) — revised after implementation-grounding review
(Rule-of-5, converged Stage 4; TypeSafe-verified HIGH findings folded in)

## Why

`adopt-dual-format-specs` let one `spec.md` carry both grammars, but the two
halves are still decoupled: `ah check` reads only the openspec half, and the
specodelic Constraints / Model / Properties layers are "inert decoration"
(`C-tables-inert`). Only `ah lint` touches the specodelic side, and only to relay
`spk lint` warnings.

The consequence is duplicated intent. A team that writes a Property row
(`generator` + `predicate`, derived from a Constraint) must then re-declare the
same behavior as a `#### Scenario:` heading plus a hand-authored contract TOML.
The two can drift silently, and the stronger artifact (a typed, traceable
Property) never feeds the gate.

Findings from grounding against the current implementation (`ah` 0.8.0,
`spk` 0.2.0) — all verified, none assumed:

- `ah lint` relays `spk lint` findings, but the bridge activates **only** for
  frontmatter `id: spec`. A file with any other id is skipped with no finding
  and no `spk-unavailable` advisory — a silent miss (`src/lint/bridge.rs`,
  bare `continue` in `relay_with`).
- A transition guard citing an `advisory` constraint passes `spk lint` and is
  reported only by `spk graph`, which the bridge never runs.
- `spk 0.2.0` has no structured export of Properties rows; `spk compile --json`
  embeds generated artifact *text* only. The structured `Spec` IR exists as a
  lib API in specodelic — the upstream ask reduces to exposing it
  (`spk parse --json`, specodelic change `add-parse-command`).
- The contract schema already has `falsifiability_class` (safety | liveness) and
  `tests` entries of the form `flags` XOR `command` (+ `timeout_seconds`), so
  derivation needs no schema change beyond an optional `derived_from` field.
- Spike answers (see design Open questions): `cargo test` exits 0 on a
  zero-match filter, so a zero-tests-ran guard is **required**;
  `openspec validate --strict` accepts a `**VERIFIES**` bullet inside a
  scenario body.

## What Changes

- **gate capability (ADDED)**: in a dual-format spec whose specodelic half is
  `spk lint`-clean, each Properties row derives one scenario contract
  (`id` = slugified property id). Scenarios carrying a `**VERIFIES**
  [[spec.P-...]]` link are covered by that property's contract; unlinked
  scenarios keep today's per-scenario contract requirement.
- **gate capability (ADDED)**: derived contracts record a hash of their source
  row (`derived_from = "<property-id>@<sha>"`); a mismatch yields a
  `contract-stale` structural finding. `ah sync` refreshes derived fields only
  and never overwrites human-owned fields (`tests`, `status`, `superseded_by`).
- **cli-core capability (ADDED)**: new command `ah sync` (with `--check` for
  CI) stages and refreshes derived contracts; refuses when `spk lint` fails or
  `spk` is missing (`spk-unavailable`).
- **lint capability (ADDED)**: the bridge never skips a `kind: intent` file
  silently (new `spk-frontmatter-mismatch` finding), also relays `spk graph`
  findings, and reports scenario-to-property trace gaps (advisory).
- **Integration boundary (Option B)**: `ah` consumes the `spk parse --json`
  structured IR instead of re-parsing specodelic markdown tables. The fleet's
  versioned-binary decoupling and the bridge's degrade-advisory semantics are
  preserved; a lib-crate dependency is deferred to a post-1.0 consolidation.
- **Governance**: derived contracts are committed spec artifacts, not
  tool-managed `.espectacular/` state — the commit policy is decided in
  task 6.1 before the pilot (design D8 / Open question on governance).

## Capabilities

### Added requirements
- `gate` — Property-Derived Contracts; Derived Contract Drift
- `lint` — Specodelic Relay Completeness; Scenario Property Trace
- `cli-core` — Sync Command

## Non-Goals

- Not running or gating on specodelic model-check output (every outcome observed
  was `exploration_only`; revisit when spk predicates are executable).
- Not translating Property predicates into Rust. Test bodies stay human-owned.
- Not changing the contract TOML schema beyond the optional `derived_from`
  field, archetypes, or runner adapters.
- Not requiring the specodelic half anywhere outside this repo's own dogfooding.
- Not making specodelic a compile-time dependency of `ah` (post-1.0 option).

## Impact

- `src/check.rs` (coverage against derived contracts, `contract-stale`),
  new derivation module (deserializes `spk parse --json` IR; no markdown-table
  re-parsing), `src/lint/` (bridge completeness, graph relay, trace checks),
  `src/main.rs` (Command enum gains `sync` — note: there is no `src/cli/`),
  `src/runner.rs` (zero-tests-ran guard for `flags` bindings — implied by
  Open question 1's answer, previously missing from Impact),
  `schemas/scenario-contract.schema.json` (optional `derived_from` field),
  `src/explain.rs` (new topics), docs.
- Adopters: opt-in by presence of Properties rows and `VERIFIES` links. All 8
  deployed capability specs carry Properties rows, so the pilot choice of
  `lint` is scoping, not default behavior; task on byte-identical regression
  covers the rest.
- Upstream dependency: specodelic `add-parse-command` implemented, released,
  and pinned (see tasks group 1). `spk archive-companion` is on specodelic
  main but **not in any release** (verified: unrecognized by spk 0.2.0) — the
  archive round-trip task validates against plain `openspec archive` until the
  pin bump lands.

## Risks and dependencies

- specodelic is pre-1.0 (format revision 12). Mitigation: pin the tested `spk`
  version in `.espectacular/config.toml`; derive only from lint-clean specs.
- Free-text predicates limit what can be generated: derivation yields contract
  *structure and a test binding*, not assertions.
- Release coordination across the fleet: `spk parse` and `spk archive-companion`
  must ship and be pinned before groups 2+ can run in CI.
- Derived-contract commit policy must be settled (task 6.1) before the pilot
  writes contract TOMLs into `.espectacular/`.