# Change: Add quality measurement and language adapters to `ah`

Change-id: `add-quality-measurement-and-adapters`
Status: draft
Depends on: `add-spec-assertions` being archived/deployed first. This change modifies `cli` and `gate` requirements introduced by that baseline change and must not be archived before it.

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

## Resolved Clarifications

- Adapter detection precedence is defined as manifest → environment → source import, and the selected `detection_source` is reported in doctor/check output. Per-language v1 signals are defined in the design document.
- `ah explain --json` includes `hints` as an array of objects, where each item contains `kind` and `message`. Richer hint payloads are deferred to v0.2 without changing the v1 minimum shape.
- Per-archetype default mutation thresholds are intentionally absent in v1. Completed quality measurements below user-configured thresholds emit warning/info findings and do not fail the gate; tool execution failures still fail the gate.
- `[runners.custom.<name>]` uses the envelope defined in `schemas/custom-runner.schema.json`; process exit failures override successful envelopes, and envelope failures override process success.
