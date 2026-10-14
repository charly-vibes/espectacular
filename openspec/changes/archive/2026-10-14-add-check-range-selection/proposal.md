# Change: add --base/--head range selection to ah check

## Why
`ah check --run-tests` selects contract tests from the **worktree diff only**
(`src/main.rs` `changed_files_from_git` hard-wires `git diff --name-only
HEAD`). This is correct for pre-commit/IDE use, but pathological for
pre-push gating: at push time everything is committed, so the worktree diff
is always empty and **every push runs every declared contract**.

Measured impact (GH#42, tambor `.gate-timing.log`, 3 pushes on 2026-10-09):

| gate | mean |
|---|---|
| `ah check --run-tests` | **841s (~14 min)** — ~97% of pre-push wall time |
| `testaruda-exec` (unit tests, range-based selection) | 27s |
| `pretender-gate-full` | 0.4s |

Consumer telemetry (charly-fx4): tambor pure-`git push` median 642s, wanna
197s — both gate on `ah check --run-tests`. espectacular's own pre-push
runs in 23s median because its test gate already uses range-based selection.

## What Changes
- Add `--base <ref>` / `--head <ref>` to `ah check`, mirroring testaruda's
  proven CLI contract (`ChangeSet::from_diff`, ticket testaruda-jdw5):
  `git diff --name-only <base> <head>` supplies the changed-file set;
  `--head` defaults to `HEAD` when omitted.
- Precedence stays explicit and unchanged otherwise: `--files` wins over the
  range, the range wins over the worktree diff, and `--all-tests` overrides
  all of them. Range mode only applies with `--run-tests`.
- Conservative fallback: a range that yields no files (e.g. rebase onto
  origin) or refs that fail to resolve degrades to running **every** declared
  contract — never a silent vacuous green, never exit-20.
- no-tests-ran falsifiability is untouched: it applies per selected contract
  exactly as before.
- Lefthook wiring (consumer-side, documented in the espectacular README's
  CLI docs):
  `ah check --run-tests --base @{push} --head HEAD`, with the documented
  fallback `--base origin/main` (or `--all-tests`) when `@{push}` is
  undefined (first push of a branch).
- Residual (accepted, documented): commits whose range diff maps to no
  capability (e.g. README/prose-only) still run every declared contract —
  the existing conservative fallback (see design D5). Range selection
  makes code-touching pushes proportional, not all pushes.

## Impact
- Affected specs: gate (ADDED requirement "Range-Based Test Selection";
  selection was previously unspecified in the deployed corpus)
- Affected code: `src/main.rs` (`Command::Check` flags + a range variant of
  `changed_files_from_git`),
  `src/check.rs` (unchanged — it already accepts an arbitrary changed-file
  set); `tests/check_cli.rs` RED→GREEN e2e coverage
- Governance: contract TOML format and JSON Schema IR untouched; selection
  is a scope computation feeding the existing pipeline.
- Related: GH#40 / batch-runner-spawns (complementary — batching shrinks the
  all-run cost, range selection fixes the clean-worktree pathology).
- Ticket: to be filed after proposal approval (HITL gate).

## Edge cases decided
- **Empty range** (base == head or no diff): full run. Rationale: unlike
  testaruda's store-driven selection — where EMPTY (exit-20) means "no code
  affected" — a git-derived empty range is indistinguishable from a
  degenerate ref situation, so the gate degrades to the documented safe
  default instead of passing with nothing executed.
- **Unresolvable ref** (`git diff` non-zero): same safe fallback, full run.
- **`@{push}` semantics**: per-remote/per-branch ref, resolved by git at
  hook invocation; per-worktree pushes each carry their own `@{push}` —
  documented, not special-cased in ah.
