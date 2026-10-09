# Change: batch contract test spawns per runner invocation

## Why
`ah check --run-tests` spawns one runner process per contract test binding
(src/runner.rs `run_declared_tests`). On tambor that is 658 spawns per push,
~93% of the pre-push gate (~790s), dominated by vitest startup (~1s per
spawn). GH#40, ticket espectacular-bm2.

## What Changes
- Batch eligible bindings of one runner type into a single invocation with an
  OR-joined pattern and a structured reporter; attribute per-contract
  pass/fail from the structured output.
- Preserve per-binding semantics exactly: matched-zero still fails, timeout
  bounds and fallback paths stay conservative and named.
- Runners without structured per-test output (cargo/shell/pytest), small
  binding sets, and JS-only-regex patterns keep today's per-binding spawns.
- **BREAKING** (execution semantics only): the gate's "Test Runner Execution"
  requirement is modified — one batched command now produces many per-binding
  verdicts instead of one exit-code verdict per command.

## Impact
- Affected specs: gate (Test Runner Execution requirement, MODIFIED)
- Affected code: src/runner.rs (batch planner + attribution), src/check.rs
  (only if attribution wiring touches it)
- Governance: contract TOML format and JSON Schema IR untouched (anti-goal);
  batching is execution-internal.
- Ticket: espectacular-bm2 (phase 1 design decided 2026-10-10, recorded in
  ticket notes)