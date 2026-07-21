# Audit: Spec-Validation Patterns Across Charly Projects

**Ticket:** `espectacular-892`
**Scope:** All charly-vibes OpenSpec projects (2026-04 through 2026-07)
**Source session:** `019f72d1-e2bf-7d6f-a707-403228d4ee3b`

---

## Executive Summary

Before `ah check` was deployed to all 13 charly projects on 2026-07-20,
agents had **no standard tool** for spec-test correspondence validation.
They relied on a mix of `openspec validate --strict` (structure-only),
`cargo test` / `just test` (test-only), and manual inspection. This audit
documents those patterns as evidence for the pi extension ticket
(`espectacular-c9v`).

---

## Patterns Observed

### Pattern 1: `openspec validate --strict` (primary pre-espectacular tool)

Used consistently from April through July 2026 across all projects with
OpenSpec specs. Validates spec structure (headings, scenarios, IDs) but
**does not verify contracts or tests**.

```bash
openspec validate --strict
openspec validate --all --strict
openspec validate --specs --strict
openspec validate add-change-name --strict
```

**Found in:** 22+ log entries across 10 projects (atril, fabbro, fotos,
khipu, miblioteca, nayra, paranoid, pretender, wai, tRAGar).

**Pain points:**
- `--strict` flag required every invocation (easy to forget)
- No test execution integration — agents still needed to run `cargo test`
  separately
- No contract file validation — only spec structural checks
- No JSON output for CI/script consumption
- Exit code behavior was unclear

### Pattern 2: `cargo test` / `just test` as spec-validation proxy

Agents frequently used test runners as a _proxy_ for spec verification,
often paired with `openspec validate`:

```bash
just test && openspec validate --all --strict
```

**Found in:** 15+ log entries across 7 projects.

**Pain points:**
- Two separate commands for one validation intent
- No feedback that tests *correspond* to specs — only that tests pass
- Projects without `openspec validate` integration just ran `cargo test`
  and assumed spec alignment

### Pattern 3: Manual spec-to-test reading (no tool)

Agents read spec files and test files manually to verify correspondence:

```
"read spec" → "read test file" → "compare scenarios" → "fix" → "commit"
```

**Found in:** Occasional entries where agents reviewed scenarios
alongside test files (2026-04-20 atril session, 2026-04-30 REPLy.jl).

**Pain points:**
- Entirely manual — no automation
- High cognitive load for contract wiring
- Easy to miss scenarios or leave orphans

### Pattern 4: `ah check` (current — espectacular)

After espectacular v0.2.0, the intended tool:

```bash
ah check                    # Fast static validation
ah check --changes <name>   # With staged change overlay
ah check --json             # Machine-readable
```

**Deployment history:**
- 2026-06-18: First dogfooded in espectacular itself
- 2026-07-08: v0.2.0 — all espectacular contracts wired, 0 findings
- 2026-07-15: v0.2.2 — human-readable output (silent JSON before)
- 2026-07-20: Deployed to all 13 openspec projects
- 2026-07-21: Performance fix (`--run-tests` flag, fast default)

**Gap (pre-deployment):** Only 2/27 projects had `ah check` available.
Agents didn't use it because it wasn't there to discover.

---

## Pain Points (Evidence for `espectacular-c9v`)

| # | Pain Point | Before | After | Severity |
|---|-----------|--------|-------|----------|
| 1 | **Two-step validation** | `openspec validate --strict` + `cargo test` | `ah check` does both | HIGH |
| 2 | **No contract verification** | `openspec validate` only checked spec structure | `ah check` validates contracts + runs tests | HIGH |
| 3 | **No CI integration** | `openspec validate` JSON output unclear | `ah check --json` has stable schema | MEDIUM |
| 4 | **Agent discovery** | No AGENTS.md block → agents didn't know to validate | `ah:managed` block in AGENTS.md | HIGH |
| 5 | **Manual spec-test mapping** | Agents read spec + test files by hand | `ah check` reports missing/orphan contracts | MEDIUM |
| 6 | **Forgotten `--strict`** | Easy to validate without the right flags | `ah check` defaults to comprehensive | LOW |
| 7 | **No test output in findings** | `openspec validate` had no test execution | `ah check` captures test output in findings | MEDIUM |

---

## Timeline

```
April 2026
  └─ openspec validate --strict  (sole tool, all projects)
  └─ just test / cargo test      (as spec proxy)

June 2026
  └─ ah check dogfooded in espectacular
  └─ openspec validate still used elsewhere

July 10
  └─ Last openspec validate usage (tRAGar project)
  └─ ah check v0.2.0 in espectacular

July 15
  └─ ah check v0.2.2 (human-readable output)

July 18
  └─ espectacular-892 ticket created (this audit)
  └─ Deployment begins

July 20
  └─ ah check deployed to all 13 openspec projects

July 21
  └─ Performance fix, --run-tests flag
  └─ This audit completed
```

---

## Recommendations for Pi Extension (`espectacular-c9v`)

1. **Discovery is the primary gap.** After deployment, agents still only use
   `ah check` if the `ah:managed` block is in their context window. A pi
   custom tool makes discovery unconditional.

2. **Pain point #1 (two-step validation)** is the strongest motivator.
   Agents ran two commands (`openspec validate` + `cargo test`) where one
   should suffice. `ah check` already solves this — the pi extension just
   needs to expose it.

3. **Pain point #4 (agent discovery)** is the blocker. The `ah:managed`
   block works when an agent reads it, but agents don't always read
   `AGENTS.md` in every session. A pi tool bypasses this entirely.

4. **Manual spec-test mapping (#5)** is the rarest but most expensive
   pattern — it wastes agent context on mechanical comparison.
