---
tags: [pipeline-run:tdd-ro5-2026-11-25-lint-bridge-dir-mode-spk-lint-per-root-espectacular-hcz, pipeline-step:quality-ledger]
---

QUALITY LEDGER (espectacular-hcz):
Changed:
- src/lint/bridge/invoke.rs — invoke_dir (one spk lint per root), SpkIssue.file field, shared spawn/decode seam
- src/lint/bridge/relay.rs — relay_with_root orchestrator (partition_dual / unavailable_findings / per_file_relay / relay_attributed / relay_unmatched), DirEnvelope bundle, fallback_finding + unmatched_issue_finding advisories
- src/lint/bridge.rs — re-exports (invoke_dir, issue_matches_path, SpkIssue)
- src/lint/engine.rs — threads specs_dir; lint_findings extraction; merge_outputs iterator chain
- src/lint/bridge/dir_mode_tests.rs — 4 spawn-counting/parity tests + real-spk e2e (moved in)
- src/lint/bridge/tests.rs — gate hygiene: ok/opt/relayed helpers, real-spk fn relocated
- src/sync/run.rs — collect_parsed via shared seam; per_file_pass fallback; SyncCtx/TableInput bundles
Verified:
- cargo test lint:: 82/82; sync:: 12/12; bridge dir_mode 4/4; real-spk e2e exercises dir-mode with actual spk
- just test (testaruda) exit 0; pretender --diff-only gate exit 0; ah check green; spk lint green
Review:
- RO5U: no criticals/highs; 2 verified-parity mediums, 3 lows — see RO5U record
Risks:
- real-spike unverified at true consumer scale (bogart 41-spec corpus measurement was manual probes, not this binary); attribute via file-field requires spk >= version emitting it — older pins take the named fallback path (correct, slower)
Next:
- ship-close: update ticket, commit wai artifacts, human approval
