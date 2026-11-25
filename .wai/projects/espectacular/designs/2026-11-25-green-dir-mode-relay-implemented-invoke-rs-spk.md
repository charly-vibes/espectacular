---
tags: [pipeline-run:tdd-ro5-2026-11-25-lint-bridge-dir-mode-spk-lint-per-root-espectacular-hcz, pipeline-step:green]
---

GREEN: dir-mode relay implemented — invoke.rs: SpkIssue gains file:Option<String>+Clone, spawn_lint/decode_issues extracted, invoke_dir spawns 'spk lint <root>' once; relay.rs: relay_with_root partitions dual specs in-root (dir spawn) vs outside (per-file, keeps /virtual test seams), named fallbacks: Broken→per-file re-run (parse isolation), missing-file-field→spk-bridge-fallback advisory+per-file re-run, Unavailable→per-spec advisory parity; issues partitioned by normalized path match, unmatched issues surfaced by path (never dropped), envelope warnings never become findings; engine.rs threads specs_dir. Evidence: cargo test lint::bridge 20/20 (incl. real-spk e2e dir-mode), cargo test lint:: 82/82, just test ingested 542 results green (26 unmatched-name warning is pre-existing adapter ingestion quirk, unrelated).
