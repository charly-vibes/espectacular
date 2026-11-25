---
tags: [pipeline-run:tdd-ro5-2026-11-25-lint-bridge-dir-mode-spk-lint-per-root-espectacular-hcz, pipeline-step:red]
---

RED: command='cargo test lint::bridge' → 15 passed / 5 failed. The 4 new dir-mode tests fail exactly on the missing behavior: dir_mode_spawns_once (3 spawns vs 1), broken_corpus_falls_back (3 vs 4), missing_file_field (3 vs 4), skipped_warnings (3 vs 1 + 0 findings vs 2); p_bridge fails only via its new bindings. Pre-existing 15 green → scaffold preserved behavior. Failure is behavioral (spawn counting at the shim seam), not setup breakage.
