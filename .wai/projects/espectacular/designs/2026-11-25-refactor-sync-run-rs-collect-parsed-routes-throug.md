---
tags: [pipeline-run:tdd-ro5-2026-11-25-lint-bridge-dir-mode-spk-lint-per-root-espectacular-hcz, pipeline-step:refactor]
---

REFACTOR: sync/run.rs collect_parsed routes through shared invoke_dir seam — one spk lint spawn per sync pass; per_file_pass preserves tri-state (C-sync-refusal); DirCollect/TableInput param bundles; gate-forced splits (collect_parsed 53→~30, sync_property 6→4 params, build_table 5→3). Commits: 6675f6e (GREEN), 18bf6e6 (REFACTOR). All pretender checks green on touched files; just test exit 0.
