---
tags: [pipeline-run:tdd-ro5-2026-11-25-lint-bridge-dir-mode-spk-lint-per-root-espectacular-hcz, pipeline-step:fix-review]
---

RO5U: rule-of-5 pass on db3ac08..HEAD (7 files, +696/-128). CRITICAL: none. HIGH: none. MEDIUM: (1) non-utf8 corpus paths degrade via lossy dir arg -> Broken -> per-file fallback which skips non-utf8 paths — identical to pre-hcz behavior, verified parity; (2) unmatched-issue advisory uses full file path as spec_path — schema-inconsistent with logical spec names, intentional+documented for unwalked files. LOW: timeout concern resolved (spawn has none — plain output()+ETXTBSY retry); graph relay stays per-file ~2ms/file measured; relay doc covers all four paths. C-dual-bridge invariant preserved per-path; Path::starts_with component-wise; merge_outputs recompute == sum. Verdict: PRODUCTION READY.
