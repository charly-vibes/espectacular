## Context
Tambor's 329 vitest bindings carry `flags = "--testNamePattern=p_<scenario>"` —
scenario-scoped regex patterns with no file notion, and the anti-goal forbids
new TOML fields. So the ticket sketch's "group by (runner, bound test file)"
is impossible; the design axis is what to batch on when bindings are
pattern-scoped. Grounded 2026-10-10 from tambor `.espectacular/` and
src/runner.rs.

## Goals / Non-Goals
- Goals: order-of-magnitude spawn reduction on pattern-scoped corpora without
  weakening per-contract falsification semantics.
- Non-Goals: parallelizing spawns; new TOML fields; batching cargo/shell;
  changing attribution for unbatched runners.

## Decisions
- Decision: **adaptive OR-joined batch (option B)** — when a runner type's
  binding count exceeds a fixed threshold (8), compose one invocation with
  `--testNamePattern=(p_a|p_b|...)` plus the structured reporter; attribute
  each contract from the structured output.
  - Alternatives: (A) superset run without pattern filter — rejected, it
    executes unbound tests and conflicts with Layer 2 testaruda pruning
    (a pruned-down run would waste a full-suite execution); (D) group by
    identical flags — only ~2–3× win on measured data.
- Decision: threshold = constant 8. Justified from measurement: ~1s vitest
  startup per spawn, so 8 spawns ≈ the batched startup amortization
  crossover. Not a config knob for now (minimal footprint).
- Decision: attribution rule — a contract passes only when **every** test
  matched by its pattern reports `passed`; ≥1 matched test failed →
  `test-failing`; pattern matches nothing → `no-tests-ran`; skipped/todo
  matched → `no-tests-ran` (falsification did not run; reward-hack guard).
- Decision: eligibility guards (all conservative, per-binding fallback):
  - runner type must have structured-reporter support in ah's adapter layer
    (cargo/shell/pytest keep per-binding spawns);
  - binding count ≤ 8 → per-binding spawns;
  - pattern containing JS-only regex constructs (lookahead/backreferences) →
    per-binding spawn (ah re-matches patterns with the Rust regex engine);
  - unparseable output, invocation error, or timed-out batch → named
    (non-silent) fallback signal + per-binding re-runs.
- Decision: batched timeout = max(timeout_seconds) across batched entries;
  a timed-out batch reports `timed_out` and falls back.
- Decision: batched invocation exit code never produces a verdict by itself —
  it only selects the fallback path. matched-zero survives batching.

## Risks / Trade-offs
- Rust-regex vs JS-regex divergence → eligibility guard above.
- Batched JSON can be megabytes → attribution parses the full captured stdout
  (already in memory via wait_with_output); TestResult tails stay 8 KiB
  findings-only; sanity cap on the parse buffer.
- Silent regressions if fallbacks fire constantly → fallbacks are named
  signals surfaced in output (Layer 2 conservative-guard philosophy).

## Migration Plan
Phase 2 TDD on src/runner.rs behind the existing execution path; spawn-count
e2e is the RED test. No consumer migration: bindings, TOML, and JSON schema
unchanged. Rollback = revert the batching planner; per-binding path is the
default below threshold.

## Open Questions
- None blocking; phase 2 may surface vitest JSON schema drift (parse
  tolerantly, fallback on mismatch).