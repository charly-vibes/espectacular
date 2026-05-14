# Change: Add quality measurement and language adapters to `ah`

Change-id: `add-quality-measurement-and-adapters`
Status: draft
Builds on: `add-spec-assertions`

## Why

`add-spec-assertions` establishes `ah` as a deterministic spec-test correspondence gate, but deliberately accepts RISK-001: a vacuous test that asserts nothing still satisfies the gate. For an AI coding harness that optimizes directly for a green gate, correspondence without a quality signal is a checkbox.

This change adds quality *measurement* (not enforcement) and the language adapter layer that makes the gate real across Python, Rust, and TypeScript projects. It also locks the agent-facing contract — the JSON finding schema and the `ah explain` playbook mechanism — so the harness has a stable, compile-enforced surface to consume.

## What Changes

- Add a language adapter layer with three first-class adapters: Python (pytest), Rust (cargo test), TypeScript (vitest). Adapters normalize test execution into the shared finding schema.
- Add a hybrid adapter model: bundled adapters for curated frameworks plus a `[runners.custom.<name>]` plugin protocol for the long tail.
- Add progressive enablement: `ah doctor` detects available frameworks and recommends them; `ah doctor --enable <capability>` writes the config block that turns a capability on. Never silent.
- Add quality measurement as stateless, per-run, opt-in capabilities: mutation testing, property-based testing, snapshot testing. Measurement is surfaced via `ah doctor`; it is not a hard gate in v1.
- Define the v1 JSON finding schema, including agent-targeted fields (`scenario_prose`, `suggested_action`, `apply_command`, `playbook_command`) and deterministic ordering.
- Replace the embedded-markdown playbook with an `ah explain <topic>` subcommand. The playbook ships in the binary, compile-enforced against the finding/action enums. AGENTS.md shrinks to a single meta-instruction.
- Keep v1 a pure function: no persistent state. Features that require cross-run history (flake detection, test-impact caching, mutation trends) defer to v0.2+.

## Non-Goals

- `ah` still does not judge whether a test is *meaningful*. Mutation/property/snapshot scores are measured and surfaced; v1 does not fail the gate on them.
- v1 does not persist state. No flake history, no dependency-graph cache, no trend baselines.
- v1 does not ship adapters beyond pytest, cargo test, and vitest. jest is a v0.2 fast-follow.
- No project-local playbook override in v1. The playbook is binary-shipped and version-bound.
- No hermetic execution mode in v1.

## Impact

- Affected specs: `cli` (new `ah doctor --enable`, `ah explain` requirements), `gate` (finding schema extension, quality measurement)
- New specs: `adapters` (language adapter layer and plugin protocol), `explain` (`ah explain` subcommand)
- Affected code: `src/adapters/` module tree, `src/explain.rs`, `src/doctor.rs` (detection + `--enable`), finding schema types, `schemas/check-output.schema.json`
- **BREAKING**: none. Projects opt into adapters and capabilities; the baseline gate behavior from `add-spec-assertions` is unchanged.

## Open Questions

- Adapter detection precedence is defined (manifest → environment → source import), but the exact per-language signals need pinning — e.g., does a Rust property adapter look for `proptest` in `Cargo.toml` dev-dependencies only, or also workspace dependencies?
- `ah explain --json` field set is provisional; the `hints` field shape is undefined until the first context-aware hint lands (likely v0.2 with state).
- Per-archetype default mutation thresholds are unset. v1 ships with thresholds absent (measure-only, no warning) until real-world scores inform defaults.
- **Blocker**: `[runners.custom.<name>]` JSON envelope is named but not yet specified. Nothing in the plugin path can be built or tested until its shape is fixed. This must be resolved before implementation begins.
