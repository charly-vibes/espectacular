# Authoring Dual-Format Specs

espectacular supports the **specodelic dual-format protocol** as an *opt-in*
authoring mode: a single `spec.md` can carry both the openspec grammar
(`## ADDED|MODIFIED|REMOVED Requirements` + `#### Scenario:` headings) and
the specodelic four-layer grammar (frontmatter intent, Constraints, Model,
Properties), each linted by its own tool.

- openspec half → validated by `openspec validate <change-id> --strict`
- specodelic half → linted by `spk lint <file>`
- espectacular's gate (`ah check`) reads only the openspec half; the
  specodelic layers are inert decoration it happens to understand.

A worked example lives in
`openspec/changes/adopt-dual-format-specs/specs/gate/spec.md` — this page
explains its anatomy.

## Anatomy of a dual-format file

```markdown
---
id: spec                      ← 1. YAML frontmatter (intent layer)
kind: intent
statement: "WHEN … THE … SHALL …"
---

# <capability heading>

Prose (optional). Never parsed by the gate.

## Constraints                ← 2. Constraints table (invariant layer)

| id | kind | expr | traces_to |
|----|------|------|-----------|
| C-mirror-dedupe | invariant | two `#### Scenario:` headings … | |

## Model                      ← 3. Model (states + transitions)

### States
- `scanning`

### Transitions

| id | from | to | guard |
|----|------|----|-------|
| t-scan | scanning | deduplicating | [[spec.C-tables-inert]] |

## Properties                 ← 4. Properties table

| id | kind | derives_from | generator | predicate |
|----|------|--------------|-----------|-----------|
| P-dedupe | unit | [[spec.C-mirror-dedupe]] | … | … |

## MODIFIED Requirements      ← openspec half (unchanged grammar)

### Requirement: Scenario Discovery
…

#### Scenario: Reject duplicate scenario ids
- **GIVEN** …
- **WHEN** …
- **THEN** …
```

### Section-sync mirror

In change deltas, the specodelic protocol mirrors requirement text into a
trailing `## Requirements` section so deployed-spec tooling sees a stable
section spelling. The mirror **must be textually identical** to the delta
section (specodelic section-sync).

The gate understands this: two `#### Scenario:` headings in the same file
with the **same slugified id and identical bodies** are treated as one
logical scenario and discovered exactly once. It emits no slug-collision
finding for the mirror.

### Table derivation

Scenario prose may be derived into structured tables (e.g. a Properties
`generator`/`predicate` column quoting a scenario). This is safe by
construction: the gate only discovers scenarios from lines that **start**
with `#### Scenario: `. YAML frontmatter, Constraints rows, Model states
and transitions, and Properties rows never yield scenarios — no matter
what scenario-like text they quote.

## Enforcement is mandated in this repo (opt-in for adopters)

espectacular deliberately does **not** make its product require the
two-grammar format: plain openspec repos remain fully valid, and `ah check`
treats both grammars as valid (`C-adopters-unaffected`).

This repository, however, dogfoods the mandate (change
`migrate-specs-to-dual-format`, capability `spec-authoring`):

- **pre-commit** — `spk lint openspec` (deep lint of dual-format files) plus
  a staged-file frontmatter mandate (`spec-corpus-staged`) that catches an
  archive strip at commit time;
- **corpus gate** — `just spec-lint` fails while any non-archived `spec.md`
  lacks the specodelic half, and runs in CI;
- **CI** — the spec-gate job installs specodelic from crates.io and runs the
  same mandate.

The migration recipe for converting a plain file lives in
`openspec/changes/migrate-specs-to-dual-format/design.md`.

## In-gate section-sync drift signal

Because mirrored sections must be textually identical, a **same-id,
different-body** collision in a dual-format repo is not just a naming
accident — it is section-sync drift: one of the two mirrored copies was
edited without the other. `ah check` catches this as a slug-collision
structural finding **without** running `spk lint`, which makes the gate a
cheap first-line drift detector for dual-format repos. (`spk lint` remains
the authoritative section-sync linter; the gate's signal is a byproduct of
collision detection, not a replacement.)

## Linting both halves

```bash
# specodelic half
spk lint openspec/changes/<change-id>/specs/<capability>/spec.md

# openspec half
openspec validate <change-id> --strict

# gate (both grammars in scope)
ah check --changes <change-id>
```

All three must exit zero before the change is deployed; the gate additionally
proves every discovered scenario has a contract.

## Archive round-trip

`openspec archive` merges the openspec half of a delta into the deployed
spec but **strips the specodelic half** (frontmatter and four-layer tables
do not survive). The strip is not silent: `just spec-lint` (pre-commit
staged mandate, corpus gate, and CI) fails until the half is re-derived.

**Re-derivation recipe** (not a blind copy — archive *merges*):

1. Copy the frontmatter block from the archived delta
   (`openspec/changes/archive/<change>/specs/<capability>/spec.md`).
2. Carry over the surviving constraint/property rows from the pre-archive
   deployed spec (git history).
3. Add rows for any newly merged requirements.
4. Re-run the gates: `spk lint`, `openspec validate --all --strict`,
   section sync, `ah check`.

An issue is filed against **specodelic** (specodelic#7) asking for the
dual-format corpus to survive the archive round-trip natively; once fixed,
the recipe becomes a no-op.
