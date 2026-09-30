# Tasks: migrate-specs-to-dual-format

Every conversion task follows the per-ticket pipeline (TDD → ro5u → fix →
commit): the "red" state is `spk lint <file>` failing; "green" is all four
gates passing (`spk lint <file>`, `openspec validate --all --strict`,
section sync, `ah check`). One ticket per file, one concern per PR.

## 1. Tooling enablement

- [x] 1.1 Done during proposal: `openspec validate --strict` rejects delta-less changes (ERROR: "Change must have at least one delta") — resolved by the `spec-authoring` delta, see design.md Decision 3
- [x] 1.2 `spk init` — SPECODELIC managed block written into `AGENTS.md` (Revision 8)
- [x] 1.3 `spk hooks install` — `spk lint openspec` wired into lefthook pre-commit (regression gate for dual files), plus a `spec-corpus-staged` command (frontmatter mandate over staged `spec.md` files — catches an archive strip at commit time; corpus-wide gate stays out of pre-commit so the migration backlog doesn't block every commit)
- [x] 1.4 `spec-lint` justfile target: `spk lint openspec` + frontmatter mandate over all non-archived `spec.md` files (red = the 9-file migration backlog, by design). CI spec-gate job deferred to 4.3. Install-source correction vs EDGE-001 fix: specodelic publishes on **crates.io** (`cargo install specodelic`), not git@cv
- [x] 1.5 Gates proven: red test — staged plain `spec.md` fails pre-commit with "specodelic frontmatter missing"; green test — real staged set passes pre-commit (exit 0); `just spec-lint` reports exactly the 9-file backlog

## 2. Change deltas (4 files — dual-format is the documented delta practice)

- [x] 2.1 `openspec/changes/adopt-genesis/specs/cli/spec.md` (3 reqs, 4 scenarios) + `ah check --changes adopt-genesis` byte-identical to pre-migration
- [x] 2.2 `openspec/changes/add-contract-property-class/specs/gate/spec.md` (1 req, 9 scenarios) + `ah check --changes add-contract-property-class` byte-identical to pre-migration; known-expected residue: overlay-conflicts where the overlay model cannot express MODIFIED requirement text (beads `espectacular-hct`), mirroring the pilot's task 2.4 pattern
- [x] 2.3 `openspec/changes/upgrade-genesis/specs/config/spec.md` (1 req, 3 scenarios)
- [x] 2.4 `openspec/changes/upgrade-genesis/specs/cli-core/spec.md` (1 req, 2 scenarios)

## 3. Deployed specs (5 files — one ticket each, smallest first)

- [x] 3.1 `openspec/specs/explain/spec.md` (7 reqs, 15 scenarios)
- [x] 3.2 `openspec/specs/adapters/spec.md` (6 reqs, 25 scenarios)
- [x] 3.3 `openspec/specs/lint/spec.md` (9 reqs, 26 scenarios)
- [x] 3.4 `openspec/specs/gate/spec.md` (13 reqs, 60 scenarios)
- [x] 3.5 `openspec/specs/cli/spec.md` (13 reqs, 47 scenarios)

## 4. Docs and upstream

- [x] 4.1 Update `docs/src/dual-format-authoring.md`: enforcement mandated in this repo; archive-strip + re-derivation recipe (frontmatter from archived delta, surviving rows from git history, rows for merged requirements, re-run gates)
- [x] 4.2 File upstream issue — corrected audience per HITL: **specodelic** (owner of the dual-format protocol), not openspec — filed as [specodelic#7](https://github.com/charly-vibes/specodelic/issues/7) via `spk feedback`
- [x] 4.3 CI spec-gate job added to `.github/workflows/ci.yml` (`cargo install specodelic --locked`, `just spec-lint`) and `spec-lint` wired into the `just ci` pipeline; full gate: `spk lint` (11 files), `openspec validate --all --strict`, `just spec-lint` all exit zero