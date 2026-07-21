# Agent Workflow: `ah check` as Specification Gate

This document describes how AI agents discover and use `ah check` — the
espectacular verification gate — in the standard development cycle. It is
extracted from observed patterns across the charly-vibes project ecosystem.

---

## How Agents Discover `ah check`

Agents learn about `ah check` through the **`ah:managed` block** in a
project's `AGENTS.md` (or `CLAUDE.md`). This block is deployed by `ah init`
and refreshed by `ah init`.

```markdown
<!-- ah:managed:start -->
## espectacular

Run `ah check` to verify spec-test correspondence before committing.

- `ah check` — validate all deployed specs
- `ah check --changes <name>` — validate with a change overlay
- `ah init` — set up or refresh espectacular project files
- `ah doctor` — diagnose setup issues
- `ah explain <topic>` — playbook guidance for finding kinds
- `ah doctor --enable <adapter>` — write adapter config into config.toml
- `ah signals` — emit dont drift signals
<!-- ah:managed:end -->
```

**Key design decisions:**

- The block is compact (fits in ~15 lines of agent context) — agents scan it
  early in a session and recognize `ah check` as a commit gate.
- It lists only the most common commands — `ah explain` provides the full
  playbook for less frequent tasks.
- The `ah:managed` delimiter signals to agents that this section is
  auto-generated and authoritative.

---

## Typical Usage Patterns

### 1. Pre-commit gate (most common)

The standard loop when working on OpenSpec projects:

```
edit → ah check → fix → ah check → commit
```

Concrete example:

> This pattern was observed in testaruda pi sessions, where `ah check` was
> the primary feedback loop during contract wiring and spec changes.
> Source sessions: `2026-07-08T15-35-34-510Z`, `2026-07-16T01-38-10-034Z`.

```bash
# 1. Make changes to spec or code
# 2. Run the gate
ah check

# 3. If findings appear, fix them
# 4. Re-run to verify
ah check

# 5. Commit when green
git add -p
git commit -m "fix: ..."
```

**When to run `ah check`:**

| Situation | Whether to run |
|-----------|---------------|
| Modified spec files | Always |
| Modified contract TOML files | Always |
| Modified production code tracked by contracts | Always |
| Edited tests only | Optional (contracts pass/fail) |
| Edited docs only | Usually not needed |
| Changed a change overlay | `ah check --changes <name>` |

### 2. Change-overlay validation

When a staged OpenSpec change is in progress:

```bash
ah check --changes add-parser-validation
```

This validates the base spec plus the overlay's additional scenarios, without
requiring the overlay to be archived first.

Multiple overlays can be validated together by repeating the flag:

```bash
ah check --changes add-parser-validation --changes fix-dangling-contracts
```

Useful when interdependent changes are being developed simultaneously.

### 3. Fresh project setup

```bash
ah init            # Scaffold .espectacular/, stub contracts
ah doctor          # Verify setup is complete
ah doctor --enable cargo   # Configure runner
ah check           # Run initial validation
```

### 4. Post-`ah init` verification

After bootstrapping a new project, `ah init` stubs contracts for every
scenario missing one. The pattern is:

```bash
ah init            # Stub missing contracts
ah check           # Verify — expect structural findings for no-tests-declared
# Fill in test commands for each stub contract, then:
ah check           # Green — all contracts wired
git add -A
git commit -m "chore: init espectacular and stub contracts"
```

---

## How Findings Are Interpreted

`ah check` emits a JSON envelope with a `findings` array. Each finding has a
`kind`, `category`, and `suggested_action` that agents can act on.

### Finding categories

| Category | Meaning | Exit code impact |
|----------|---------|-----------------|
| `structural` | Spec/contract mismatch (missing contract, orphan, duplicate ID) | Non-zero |
| `execution` | Contract test failed or timed out | Non-zero |
| `quality-*` | Mutation score below threshold, property coverage gap | Zero (advisory) |

### Common findings and agent responses

| Finding kind | What it means | Agent action |
|-------------|---------------|--------------|
| `missing-contract` | A scenario has no contract file | `ah explain run_ah_scenario_new` → `ah scenario new` |
| `orphan-contract` | A contract file has no matching scenario | Remove or archive the contract |
| `no-tests-declared` | Contract exists but has no test commands | Fill in `[[tests.shell]]` or `[[tests.flags]]` |
| `collision` | Two specs declare the same scenario ID | Rename one scenario heading |
| `test-failing` | A contract test exited non-zero | Inspect test output, fix code |
| `no-tests-ran` | Shell test ran no tests (exit 0, no output) | Run `ah check --json` to inspect the full captured output. The contract's `[[tests.shell]]` command may not match any test files, or the adapter may not be detecting the test framework. Check config and test command. |
| `duplicate-id` | Same scenario ID appears in two specs | Deduplicate |
| `mutation-below-threshold` | Mutation kill rate too low | Add or improve test cases |

Before starting work, ensure `ah` is available in PATH:

```bash
which ah || cargo install --git git@cv:charly-vibes/espectacular.git
```

### Quick reference: interpreting output

```json
{
  "summary": { "structural": 0, "execution": 0, "passed": 3, "counts_by_kind": {} },
  "findings": []
}
```
→ **Green.** All checks pass. Ready to commit.

```json
{
  "summary": { "structural": 1, "execution": 0, "passed": 0, "counts_by_kind": { "missing-contract": 1 } },
  "findings": [ { "kind": "missing-contract", "category": "structural", ... } ]
}
```
→ **Structural finding.** A scenario has no contract file. Run `ah scenario new`
   to create one, then `ah check` again.

```json
{
  "summary": { "structural": 0, "execution": 1, "passed": 2, "counts_by_kind": { "test-failing": 1 } },
  "findings": [ { "kind": "test-failing", "category": "execution", ... } ]
}
```
→ **Execution finding.** A contract test failed. Fix the code, then `ah check`
   again.

For full guidance on any finding kind:

```bash
ah explain <finding-kind>
```

### CI/script usage

In CI pipelines or automated scripts, use `--json` for machine-readable
output:

```bash
ah check --json
# Exit code: 0 if no structural/execution findings, 1 otherwise
```

The `--run-tests` flag forces contract test execution even when no spec
changes are detected (useful in CI when dependencies may have changed).

### Forcing a commit through findings

On a WIP branch or when findings are acceptable, you can commit despite
non-zero exit. This is common during staged change development:

```bash
ah check --changes my-change
# If findings are scoped to the change and expected, proceed
git commit -m "wip: my change"
```

For CI-only enforcement, run `ah check` in CI and allow local commits to
pass — the managed block in AGENTS.md should still instruct agents to
resolve findings.

---

## Commit Workflow with `ah check` as Gate

### Standard workflow

```
1. bd claim <ticket>        # Claim work
2. git pull --rebase        # Sync
3. [edit code / specs]      # Make changes
4. ah check                 # Verify spec-test correspondence
5. [fix findings]           # Iterate until green
6. git add <files>          # Stage specific files (never git add -A)
7. git commit -m "..."      # Commit with descriptive message
8. ah check                 # Verify pre-push (catches regressions from merge)
9. bd close <ticket>        # Close ticket
```

### When findings are expected

Some tickets produce findings that are *expected* and *resolved* within the
same ticket. Example: a ticket that adds a new scenario creates a
`missing-contract` finding until the contract file is written.

In those cases, the workflow is:

```
1. ah check                 # Baseline — note existing findings
2. [edit code / specs]      # Make changes
3. ah check                 # Verify — expected findings should match ticket scope
4. [fix unexpected findings]
5. ah check                 # Green — all expected findings resolved
6. git add && git commit
```

### Pre-push hooks (optional)

Projects with `lefthook` or `prek` can integrate `ah check` as a pre-push
hook. This is configured by `ah init` and runs automatically:

```yaml
# .lefthook.yml (auto-generated by ah init)
pre-push:
  commands:
    ah-check:
      run: ah check
      skip: false  # Block push on failure
```

If you want `ah check` to run but not block the push (advisory mode), set
`skip: true`. This lets you push while still seeing the output.

The actual config generated by `ah init` uses `skip: false` by default.
```

---

## Agent Workflow Diagram

```
┌─────────────────────────────────────────────────────────┐
│                    Session Start                         │
│                                                          │
│  1. Agent reads AGENTS.md → discovers ah:managed block   │
│  2. Agent runs `ah check` to establish baseline           │
│  3. Agent reads any existing findings                     │
└─────────────────────┬───────────────────────────────────┘
                      │
                      ▼
┌─────────────────────────────────────────────────────────┐
│                    Development Loop                       │
│                                                          │
│  ┌──────────┐    ┌──────────┐    ┌──────────┐           │
│  │  Edit    │───▶│ah check  │───▶│  Fix     │           │
│  │  code/   │    │          │    │ findings │──┐        │
│  │  specs   │    └──────────┘    └──────────┘  │        │
│  └──────────┘         ▲                       │        │
│                       │  findings remain       │        │
│                       └────────────────────────┘        │
│                                │                         │
│                          green │                         │
│                                ▼                         │
│                      ┌──────────────────┐               │
│                      │  ah check passes  │               │
│                      └────────┬──────────┘               │
└─────────────────────────────────┬───────────────────────┘
                                  │
                                  ▼
┌─────────────────────────────────────────────────────────┐
│                    Commit                                 │
│                                                          │
│  1. git add <specific files>                              │
│  2. git commit -m "fix: ..."                              │
│  3. (optional) ah check pre-push                        │
│  4. bd close <ticket>                                     │
└─────────────────────────────────────────────────────────┘
```

---

## Adding `ah check` to a New Project

For agent operators who want to replicate this pattern:

```bash
# 1. Install ah
cargo install --git git@cv:charly-vibes/espectacular.git

# 2. Ensure the project has an openspec/ directory
#    (the root AGENTS.md or CLAUDE.md will be updated
#     automatically by ah init)

# 3. Run init
ah init

# 4. Verify
ah doctor    # Should show healthy
ah check     # Should have findings from stub contracts

# 5. Verify the ah:managed block was written to AGENTS.md
#    (ah init does this automatically — no manual edits needed)
```

## Sources

This document is extracted from observed agent workflow patterns in the
following pi sessions:

- `2026-07-08T15-35-34-510Z` — Heavy `ah check` usage during contract wiring
  for the v0.2.0 release (espectacular project)
- `2026-07-16T01-38-10-034Z` — Recent workflow showing the
  edit → ah check → fix → ah check → commit cycle (testaruda project)

Both sessions demonstrate the `ah check` feedback loop in practice.
Testaruda was the first OpenSpec project to adopt `ah check` as a regular
development gate, and its patterns were replicated across the charly-vibes
ecosystem.