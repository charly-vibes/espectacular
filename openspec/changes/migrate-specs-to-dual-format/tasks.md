# Tasks: migrate-specs-to-dual-format

Every conversion task follows the per-ticket pipeline (TDD → ro5u → fix →
commit): the "red" state is `spk lint <file>` failing; "green" is all four
gates passing (`spk lint <file>`, `openspec validate --all --strict`,
section sync, `ah check`). One ticket per file, one concern per PR.

## 1. Tooling enablement

- [ ] 1.1 Verify `openspec validate` accepts this delta-less change (`--strict`); if it requires a delta, record the exact error in design.md as a correction
- [ ] 1.2 `spk init` — write the SPECODELIC managed block into `AGENTS.md`
- [ ] 1.3 `spk hooks` — wire the dual-format gate into `lefthook.yml`
- [ ] 1.4 Add `spec-lint` to `justfile` and run `spk lint` over the corpus in CI
- [ ] 1.5 Gates green: `spk lint` exits zero on existing compliant files, lefthook runs the hook, CI passes

## 2. Change deltas (4 files — dual-format is the documented delta practice)

- [ ] 2.1 `openspec/changes/adopt-genesis/specs/cli/spec.md` (3 reqs, 4 scenarios) + `ah check --changes adopt-genesis` contracts stay green
- [ ] 2.2 `openspec/changes/add-contract-property-class/specs/gate/spec.md` (1 req, 9 scenarios) + `ah check --changes add-contract-property-class` contracts stay green
- [ ] 2.3 `openspec/changes/upgrade-genesis/specs/config/spec.md` (1 req, 3 scenarios)
- [ ] 2.4 `openspec/changes/upgrade-genesis/specs/cli-core/spec.md` (1 req, 2 scenarios)

## 3. Deployed specs (5 files — one ticket each, smallest first)

- [ ] 3.1 `openspec/specs/explain/spec.md` (7 reqs, 15 scenarios)
- [ ] 3.2 `openspec/specs/adapters/spec.md` (6 reqs, 25 scenarios)
- [ ] 3.3 `openspec/specs/lint/spec.md` (9 reqs, 26 scenarios)
- [ ] 3.4 `openspec/specs/gate/spec.md` (13 reqs, 60 scenarios)
- [ ] 3.5 `openspec/specs/cli/spec.md` (13 reqs, 47 scenarios)

## 4. Docs and upstream

- [ ] 4.1 Update `docs/src/dual-format-authoring.md`: enforcement mandated in this repo; archive-strip + re-derivation recipe (copy frontmatter + tables from the archived delta, re-run gates)
- [ ] 4.2 File upstream openspec issue: preserve frontmatter and four-layer tables on `openspec archive`
- [ ] 4.3 Full-corpus gate: `spk lint` (9 files), `openspec validate --all --strict`, `ah check` all exit zero