## 1. Batching planner (RED→GREEN)
- [x] 1.1 RED: spawn-counting e2e — fixture corpus with N vitest-bound contracts sharing one runner runs exactly 1 runner invocation when N > threshold (assert execute_command invocations ≤ file count; today 1 per contract)
- [x] 1.2 RED: below-threshold corpus (≤ 8 bindings) keeps per-binding spawn count
- [x] 1.3 GREEN: batch planner groups eligible bindings per runner type, composes OR-joined pattern + structured reporter flags, max(timeout_seconds) invocation timeout
- [x] 1.4 GREEN: JS-only-regex patterns (lookahead/backrefs) excluded from batching, run per-binding

## 2. Attribution (RED→GREEN)
- [ ] 2.1 RED: batched JSON output — contract passes iff every test matched by its pattern reports passed; matched+failed → test-failing; matched-nothing → no-tests-ran even when invocation exits zero; skipped/todo matched → no-tests-ran
- [ ] 2.2 RED: unparseable/error/timed-out batched invocation → named fallback signal + per-binding re-runs with exit-code verdicts
- [ ] 2.3 GREEN: attribution parses full captured stdout (sanity-capped); TestResult tails stay 8 KiB findings-only
- [ ] 2.4 GREEN: cargo/shell/pytest and any ineligible binding — check output byte-identical to pre-change behavior (regression harness)

## 3. Gates & hygiene
- [ ] 3.1 File-header criterion: reshaped src/runner.rs (+ any new module) carries Purpose/Responsibilities/Rationale header
- [ ] 3.2 Full suite green (535 baseline) + ah check --run-tests on the espectacular corpus green; runner.rs no-tests-ran guard tests pass unmodified
- [ ] 3.3 clippy --all-targets -D warnings, fmt, ah check clean
- [ ] 3.4 bd export + ticket notes; GH#40 comment with measured spawn counts