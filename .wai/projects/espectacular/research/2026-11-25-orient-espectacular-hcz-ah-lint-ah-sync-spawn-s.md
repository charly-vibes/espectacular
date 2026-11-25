---
tags: [pipeline-run:tdd-ro5-2026-11-25-lint-bridge-dir-mode-spk-lint-per-root-espectacular-hcz, pipeline-step:orient]
---

ORIENT: espectacular-hcz — ah lint/ah sync spawn spk lint per spec file (relay.rs:22 loop, sync/run.rs:89); each invocation rescans corpus context (bogart: 10.2s/file, 41 files ≈ 420s; dir-mode once = 10.1s). Fix: one dir-mode spawn per linted root + partition issues by per-issue file field. Guarded by existing invariant C-dual-bridge (findings semantics preserved, linting continues on failure) — no openspec proposal needed (execution-internal, no contract/IR change). Task selected: espectacular-hcz (claimed, branch lint-bridge-dir-mode-spk).
