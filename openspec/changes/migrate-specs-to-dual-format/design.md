# Design: migrate-specs-to-dual-format

## Decision 1 — deployed specs are converted even though `openspec archive` strips them

The archive round-trip documented in `docs/src/dual-format-authoring.md` says
`openspec archive` merges the openspec half of a delta into the deployed spec
and **strips the specodelic half**. Converting deployed specs therefore risks
churn: the next archive touching that capability silently reverts the file to
plain openspec.

Options considered:

| Option | Verdict |
|--------|---------|
| Deployed specs stay plain; only deltas convert | Rejected — user asked for full migration; deployed specs are where `ah check` runs daily |
| Patch `openspec archive` behavior in-repo | Rejected — `openspec` is an external binary (v0.19.0); shimming it violates minimal footprint |
| Convert everything + re-derivation recipe + upstream issue | **Chosen** |

**Chosen approach**: convert all 9 files now. When a future archive strips a
file's specodelic half, the half is re-derived from the archived delta (which
retains frontmatter + tables verbatim in `openspec/changes/archive/`) — a
mechanical copy of the frontmatter block and tables, plus a re-run of the
gates. A justfile target `spec-lint` makes the drift visible immediately
(lefthook/CI fail on the stripped file), so re-derivation is never silent. An
upstream issue asks openspec to preserve the half natively; once fixed, the
recipe becomes a no-op.

This is reversible: stripping is the status quo behavior, so the worst case of
abandoning this migration is exactly today's state.

## Decision 2 — enforcement flips to mandated, scoped to this repo only

The pilot change deliberately made enforcement opt-in (`C-opt-in`) — that was
about espectacular's behavior toward *adopters* and stays untouched. This
repo's own lefthook chain and CI can still mandate `spk lint` for its own
corpus; that is dogfooding, not a product change. `spk hooks` wires the gate
into the existing lefthook managed block (it never claims `core.hooksPath`).

## Decision 3 — one delta, for a new `spec-authoring` capability

Behavioral constraint: spec-representation changes require an openspec
proposal — this document is that proposal. The openspec halves of all 9 files
must remain **textually identical** before and after migration (enforced by
the section-sync mirror and `ah check`) — but `openspec validate --strict`
rejects a delta-less change (verified; correction to the original plan, see
task 1.1). The mandate itself is real repo practice worth capturing: one ADDED
requirement, "Dual-Format Spec Corpus", under a **new** `spec-authoring`
capability. It lives outside the product capabilities (gate/cli/...) precisely
so the adopter-facing `C-opt-in` advisory is untouched, and its delta is
authored in dual format as the corpus-wide living example. Its scenarios need
TOML contracts under `.espectacular/spec-authoring/` at apply time.

## Per-file authoring rules (the migration recipe, fixed for this change)

For each file, in order:

1. Frontmatter: `id: spec`, `kind: intent`, one EARS `SHALL` statement
   condensing the file's requirements.
2. Constraints: one `invariant` row per MUST the requirements imply; advisory
   rows where the spec states non-binding guidance. `traces_to: [[spec]]`
   (self-contained deltas) or resolved within the file.
3. Model: states + transitions covering the scenarios.
4. Properties: one `unit` Property per constraint (coverage rule).
5. Openspec half unchanged — deltas keep `## ADDED Requirements` mirrored by
   `## Requirements`; deployed specs keep only `## Requirements`.

Wiki-refs resolve only within the file; domain semantics are cited by prose
path, never wiki-link.

## Gate per file (red → green)

Red: `spk lint <file>` fails (missing frontmatter/tables). Green: all four
pass — `spk lint <file>`, `openspec validate --all --strict`, section sync,
`ah check` (deltas additionally under `--changes <change-id>`).