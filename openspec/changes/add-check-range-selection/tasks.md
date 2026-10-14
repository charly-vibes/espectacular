## 1. Range selection plumbing (RED→GREEN)
- [ ] 1.1 RED: e2e — fixture repo, two capabilities, commit base then change one; `ah check --run-tests --base <sha>` runs only the affected capability's contract (absent from `selection.selected` otherwise); clean-worktree pathology gone
- [ ] 1.2 RED: e2e — `--base <sha>` without `--head` defaults to HEAD (identical selection to explicit `--head HEAD`)
- [ ] 1.3 GREEN: clap flags `--base <ref>` / `--head <ref>`; `--head` without `--base` rejected at parse (exit 2); changed-file source = `git diff --name-only <base> <head>` when `--base` given and `--run-tests` and no `--files` and no `--all-tests`

## 2. Fallback + precedence (RED→GREEN)
- [ ] 2.1 RED: empty range (base == HEAD) runs every declared contract; no exit-20, no vacuous pass
- [ ] 2.2 RED: unresolvable ref (`--base not-a-ref`) runs every declared contract, exits per findings (no crash, no silent green)
- [ ] 2.3 RED: precedence — `--files` beats `--base`; `--all-tests` beats `--base`; `--base` without `--run-tests` leaves the structural gate unchanged
- [ ] 2.4 GREEN: implement fallback + precedence; range mode ignored when `--run-tests` is absent

## 3. Regression + docs
- [ ] 3.1 Regression harness: no `--base` → worktree-diff selection behavior byte-identical (existing selection tests pass unmodified)
- [ ] 3.2 README: lefthook pre-push wiring `ah check --run-tests --base @{push} --head HEAD` + fallback (`--base origin/main` / `--all-tests`) when `@{push}` is undefined
- [ ] 3.3 Wire contracts for the new gate scenarios (`ah sync` + contract TOMLs), `ah check --changes add-check-range-selection` clean apart from expected pre-archive findings
- [ ] 3.4 Full suite green, clippy --all-targets -D warnings, fmt, `ah check --run-tests` green on the espectacular corpus
- [ ] 3.5 bd export + ticket notes; GH#42 comment with measured pre-push wall time after range selection
