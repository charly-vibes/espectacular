# Design: --base/--head range selection

## Context
`run_check_with_selection(repo_root, changes, run_tests, changed_files,
all_tests)` already accepts any repo-relative changed-file set; selection
(espectacular-0k8 Layer 1) maps files to capabilities via `affected_capability`
(specs tree, changes tree, contracts tree). Only the *source* of the set is
hard-wired to the worktree diff in `src/main.rs`. The change therefore stays
almost entirely in CLI arg plumbing plus one new git invocation.

## Decisions

### D1 — range semantics follow testaruda-jdw5
`git diff --name-only <base> <head>` — a two-dot snapshot diff, not
`base...head`; chosen deliberately because over-selection (deletions and
divergent bases still map to capabilities) is the safe direction and
three-dot merge-base resolution adds failure modes. In-range changes are
changed by
definition (the store/HEAD already contains them — a worktree-fingerprint
comparison cannot detect them). Precedent keeps the two tools' CLI contracts
aligned so lefthook configs read the same way.

### D2 — empty range / failed ref resolve to FULL RUN, not exit-20
testaruda's exit-20 EMPTY is only meaningful when selection comes from a
fingerprinted store: EMPTY there means "computed, and nothing is affected".
A git range is different: empty output is what a broken ref, an unborn
branch, or an already-pushed HEAD all look like. The gate must never pass
with zero executed tests because of a degenerate input it cannot
distinguish, so every degenerate range degrades to the same conservative
default as today's no-signal path: run every declared contract. This keeps
the `selection_absent_changed_files_run_everything` invariant as the single
fallback story.

### D3 — precedence is total and documented
1. `--all-tests` → selection off (everything runs).
2. `--files <paths>` → explicit set wins (pre-commit/agent supply).
3. `--base <ref>` (head defaults `HEAD`) → range diff.
4. else → worktree diff (unchanged).
Range mode only applies when `--run-tests` is set, matching the existing
worktree-diff gating condition. Supplying `--head` without `--base` is a
clap-level usage error (exit 2) — a head without a base is meaningless.

### D4 — no output-shape changes
Selection reporting (`report.selection`), finding kinds, exit codes, and the
JSON envelope are untouched. Range mode only changes which contracts run.

### D5 — residual: docs-only commits still run everything (review CORR-001,
verified)
The existing Layer-1 fallback runs every declared contract when the
changed set maps to no capability (src/check.rs doc comment). A range
touching only README/prose maps to no capability, so docs-only pushes keep
today's full-run cost even with `--base`. Accepted: the conservative
fallback is the single fallback story (D2), and exempting prose from the
gate would weaken falsifiability. The README wiring note (task 3.2) and
the GH#42 comment must set that expectation explicitly.

### D6 — `--head` without `--base`
Rejected at parse via clap `requires = "base"` on the `--head` arg —
exit 2, matching the invalid-subcommand convention.

## Test shape (inputs/outputs before implementation)
E2E via `tests/check_cli.rs` fixture repos (init → commit base → commit
change per capability → run `target/debug/ah check --run-tests --base <sha>`):

1. base = pre-change commit → only the changed capability's contract runs;
   other capability's contract is absent from `selection.selected`.
2. head omitted → defaults to HEAD (same result as explicit HEAD).
3. `--files` + `--base` → files win; range ignored.
4. `--all-tests` + `--base` → everything runs.
5. base == HEAD (empty range) → every declared contract runs.
6. base = `not-a-ref` → every declared contract runs (no crash, exit per
   findings).
7. no `--base` → worktree behavior byte-identical (regression harness).
8. `--base` without `--run-tests` → range ignored; structural gate
   unchanged (tasks-only coverage; delta scenario 8 asserts it against
   P-range-precedence).

## Anti-goals
- No new finding kinds, no JSON schema edits.
- No `--via testaruda` delegation (deep integration stays a follow-up).
- No change to contract TOML format, IR, or `ah sync` behavior.
