## 1. Dependency
- [ ] 1.1 Add `genesis = { git = "https://github.com/charly-vibes/genesis", tag = "v0.1.0" }` to `Cargo.toml`.
- [ ] 1.2 Verify build with envelope/suggestions/managed_block/feedback modules stable.

## 2. Adopt shared envelope
- [ ] 2.1 Route `check`/`doctor`/`report`/`signals` `--json` through `genesis::envelope`.
- [ ] 2.2 Test: top-level keys match the shared shape.

## 3. Adopt suggestions
- [ ] 3.1 Register espectacular's command list with `genesis::suggestions::SuggestionEngine`.
- [ ] 3.2 Wire `main.rs` error sink to emit `genesis::suggestions` fix-footers.
- [ ] 3.3 Regression: `ah doctr` (typo) prints "Did you mean 'doctor'?".

## 4. Source managed_block from genesis
- [ ] 4.1 Source the `<!-- ah:managed:start/end -->` injector from `genesis::managed_block`.
- [ ] 4.2 Keep espectacular's block content (command reference, adapter listing).
- [ ] 4.3 Regression: `ah init` / `ah doctor --fix` still inject/refresh the block.

## 5. Add `feedback` subcommand (wraps `genesis::feedback`)
- [ ] 5.1 Add `Feedback` variant to the `Commands` enum with `KIND` + flags (per agent-issue-reporting playbook §2).
- [ ] 5.2 Read espectacular's error scratch (`$XDG_CACHE_HOME/espectacular/errors.jsonl`) for `--from-last-error`; never shadow the real error.
- [ ] 5.3 Default target repo = espectacular's `Cargo.toml` `repository`; labels from playbook §8.
- [ ] 5.4 Wire the error-footer hook: non-zero exits with no `genesis::suggestions::Fix` print `Feedback: ah feedback bug --from-last-error`.
- [ ] 5.5 Regression: `ah feedback bug --dry-run` prints body + exact `gh` line; redactor strips a `https://<pat>@…` remote.

## 6. Clean up
- [ ] 6.1 Remove dead local code; `cargo clippy -- -D warnings` clean.
- [ ] 6.2 Verify tool-craft (genesis `.wai` research) Appendix A.3 espectacular row; file a charly-monorepo ticket if inaccurate.