# Change: Adopt genesis

## Why

espectacular is the suite's architecture-drift gate but lacks the shared
envelope format, self-healing error suggestions, and the feedback subcommand.
It already ships managed-block injector mechanics (via `ah:managed`). Adopting
genesis closes the gaps: structured `--json` output through the shared
envelope, typed `Suggestion` footers on errors, and a `feedback` subcommand
wrapping `genesis::feedback` for filing issues upstream.

## What Changes

- Add `genesis` git dependency (pinned by tag `v0.1.0`) to `Cargo.toml`.
- Route `--json` output through `genesis::envelope` (check, doctor, report,
  signals).
- Adopt `genesis::suggestions` for typo detection and fix-footers on
  espectacular's command surface (`check`/`doctor`/`init`/`report`/`signals`).
- Source the `<!-- ah:managed:start/end -->` injector mechanics from
  `genesis::managed_block`; espectacular keeps its block *content*
  (the command reference and adapter listing).
- Add an `espectacular feedback [KIND]` subcommand wrapping `genesis::feedback`.
  espectacular owns the command surface; genesis owns the machinery.
- Keep all domain logic (spec-test correspondence, adapter config, finding
  kinds, drift signals). The genesis boundary rule protects this.

## Impact

- Affected specs: `espectacular-cli-core` (MODIFIED — envelope + suggestions +
  managed_block + feedback).
- Affected code: `Cargo.toml`, `src/main.rs` (new `Feedback` variant + error
  footer), `json.rs` (envelope wrapping), `src/managed_block.rs` (thin
  wrapper over genesis).
- Blocked by: genesis tagging `v0.1.0` (envelope/suggestions/managed_block/
  feedback stable).
- No user-visible behavior change except `--json` envelopes are now identical
  in shape to wai/dont/pretender/testaruda.