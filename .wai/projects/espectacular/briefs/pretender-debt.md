# Brief: fix pretender gate debt — espectacular (charly-1df)

## Why
pretender 0.10.0 tightened defaults (new `min_assertions` unit metric; new file-level checks: `unwraps`, `mut_ratio`, `mock_count`, `lazy_cluster`, `void_mutator`). This repo fails `pretender check --mode gate`. Delegated to a subagent: the fix is wide but mostly mechanical; the lead orchestrates only.

## Known debt (scan 2026-11-04)
191 unit violations: min_assertions 60, function_lines 73, abc 26. File-level debt not yet counted — enumerate.

## Task
1. Enumerate everything: `pretender check --mode gate` (add `--show-all` if helpful; `pretender explain <metric>` for definitions).
2. Fix violations in code:
   - min_assertions → add real assertions to tests (assert actual behavior; never `assert!(true)`).
   - function_lines / cyclomatic / cognitive / abc / nesting_max / params → behavior-preserving refactors: extract helpers, split functions.
   - unwraps → non-test code: proper error handling per repo idiom; test code: `expect("why")` or config-compliant alternative (check `pretender.toml` for scope).
   - mut_ratio / mock_count / lazy_cluster / void_mutator → usually: remove unused muts/mocks, merge or delete dead tests, assert more per test.
3. HARD CONSTRAINTS: do NOT edit `pretender.toml` thresholds; do NOT use `--adopt`/baselines; do NOT disable or bypass checks; do NOT push.
4. Keep the repo green: run its gates (see justfile / AGENTS.md — typically `just` or `cargo test` / `pytest`) before committing.
5. Commit atomically with conventional messages (e.g. `fix(quality): ...`, `refactor(quality): ...`). Add ONLY files you touched — `git add <files>`, never `git add -A`/`git add .`. Pre-existing dirty/untracked files are someone else's WIP: leave them unstaged. Also never stage `.pretender/history/` state noise.
6. If a specific violation needs architectural rework (est. >30 min or behavior-risky), skip that one, note it in your final report, fix the rest.

## Completion criteria (each must exit 0)
- `pretender check --mode gate`
- repo gates/test suite (justfile default or language runner)
- `git status --porcelain` shows no NEW modifications beyond pre-existing WIP listed above

## Spawn model
- Model: inherit orchestrator (openrouter/z-ai/glm-5.3-flash). Context ceiling ≤ 60%.
- Wall-clock budget: 60 min, enforced by orchestrator `timeout`.
- Session name: subagent:charly-1df:implement — resumable via `pi -c -p -n "charly-1df-resume"`.
- Final report: ≤200 words — violations fixed per category, files touched, commits made, anything skipped and why.
