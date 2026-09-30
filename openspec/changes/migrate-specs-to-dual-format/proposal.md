# Proposal: migrate-specs-to-dual-format

Status: draft

## Why

espectacular's mission is detecting spec-code drift, and the specodelic
four-layer grammar tightens the *spec* side of that detection: constraints
become machine-linted invariants, models expose state machines, and properties
tie each constraint to a testable derivation. The `adopt-dual-format-specs`
change (archived 2026-09-29) proved the dual-format protocol works end to end —
discovery deduplicates mirrors, structured layers stay inert, and the pilot
delta lints clean in both tools — but adoption stopped at a single pilot file.

This change migrates **every** spec file in the repo to dual format: all 5
deployed capability specs (48 requirements, 173 scenarios) and all 4 change
deltas (6 requirements, 18 scenarios). It also flips this repo's own
enforcement from opt-in to mandated: `spk lint` joins the lefthook hook chain
and CI, so a spec file can no longer regress to plain openspec here.

This is repo dogfooding, not tool behavior: espectacular remains a
language-agnostic, opt-in tool for adopters (`C-opt-in` from the pilot change
is untouched), so **no spec delta is required** — only spec-file authoring,
tooling wiring, and docs.

## What Changes

- **Spec files (9)**: every file under `openspec/specs/` and every delta under
  active `openspec/changes/*/specs/` gains the specodelic half (frontmatter
  `id: spec` / `kind: intent` / EARS statement, Constraints, Model,
  Properties) while its openspec half stays textually identical.
- **Tooling**: `spk init` writes the SPECODELIC managed block into `AGENTS.md`;
  `spk hooks` wires the dual-format gate into lefthook; CI runs `spk lint` on
  the spec corpus.
- **Docs**: `docs/src/dual-format-authoring.md` flips from "enforcement is
  opt-in" to "enforcement is mandated in this repo", and documents the archive
  strip + re-derivation recipe (see design.md).
- **Upstream**: an issue is filed against openspec asking `openspec archive` to
  preserve frontmatter and four-layer tables on deploy.

## Capabilities

None — no deployed requirement changes. Deliberately: the gate spec's
`C-opt-in` advisory (plain openspec specs remain valid) is a product promise to
adopters and MUST NOT be converted into a mandate.

## Impact

- 9 spec files under `openspec/` (authoring only; no code).
- `AGENTS.md` (tool-managed SPECODELIC block), `lefthook.yml`, CI workflow, `justfile`.
- `docs/src/dual-format-authoring.md`.
- One upstream openspec issue.
- All gates must stay green per file: `spk lint <file>`, `openspec validate
  --all --strict`, section sync, and `ah check` (with the matching change
  overlay for deltas).