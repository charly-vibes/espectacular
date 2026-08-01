<!-- OPENSPEC:START -->
# OpenSpec Instructions

These instructions are for AI assistants working in this project.

Always open `@/openspec/AGENTS.md` when the request:
- Mentions planning or proposals (words like proposal, spec, change, plan)
- Introduces new capabilities, breaking changes, architecture shifts, or big performance/security work
- Sounds ambiguous and you need the authoritative spec before coding

Use `@/openspec/AGENTS.md` to learn:
- How to create and apply change proposals
- Spec format and conventions
- Project structure and guidelines

Keep this managed block so 'openspec update' can refresh the instructions.

<!-- OPENSPEC:END -->

<!-- DONT:START -->
# DONT MANAGED BLOCK — DO NOT EDIT

This project uses `dont` for grounded-claim workflow.

**When to use:** When making a design or correctness assertion during development, run `dont conclude "<assertion>"` before asserting it. For documented facts, use `dont ground "<fact>" --file <path> --lines <N>`.

At session start run `dont prime --json`.

Canonical agent instructions: `.dont/AGENTS.md`.

Edits inside this managed block will be overwritten by `dont doctor --fix`.
<!-- DONT:END -->

<!-- WAI:START -->
## PRIMARY OBJECTIVE

Build and maintain **espectacular** — the behavioral verification layer that
compiles openspec specifications into executable assertions and detects drift
between specs and code. Every action should trace back to: does this make
espectacular more reliable at detecting drift, better at integrating with
openspec, or easier to adopt in a new project?

# Workflow Tools

This project uses **wai** to track the *why* behind decisions — research,
reasoning, and design choices that shaped the code. Run `wai status` first
to orient yourself.

Detected workflow tools:
- **wai** — research, reasoning, and design decisions
- **beads** — issue tracking (tasks, bugs, dependencies). CLI command: **`bd`** (not `beads`)
- **openspec** — specifications and change proposals (see `openspec/AGENTS.md`)

> **CRITICAL**: Apply TDD and Tidy First throughout — not just when writing code:
> - **Planning/task creation**: each ticket should map to a red→green→refactor cycle; refactoring tasks must be separate tickets from feature tasks.
> - **Design**: define the test shape (inputs/outputs) before designing the implementation.
> - **Implementation**: write the failing test first, then make it pass, then tidy in a separate commit.

> **When beginning research or creating a ticket**: run `wai search "<topic>"` to check for existing patterns before writing new content.

## Quick Start

1. `wai sync` — ensure agent tools are projected
2. `wai status` — see active projects, phase, and suggestions
3. `bd ready` — find available work items

When context reaches ~40%: stop and tell the user — responses degrade past
this point. Recommend `wai close` then `/clear` to resume cleanly.
Do NOT skip `wai close` — it enables resume detection.



## Detailed Instructions

Full workflow reference — session lifecycle, capturing work, command cheat
sheets, cross-tool sync, and PARA structure — lives in **`.wai/AGENTS.md`**.
Read it at the start of your first session or when you need detailed guidance.

## PRIMARY OBJECTIVE (echo)

Build and maintain **espectacular** — the behavioral verification layer that
compiles openspec specifications into executable assertions and detects drift
between specs and code. Every action should trace back to: does this make
espectacular more reliable at detecting drift, better at integrating with
openspec, or easier to adopt in a new project?

Keep this managed block so `wai init` can refresh the instructions.

<!-- WAI:END -->

## Behavioral Constraints

These constraints are **persistent** — they live outside the WAI managed
block so they survive `wai init`. Do not remove or edit them without
deliberate intent.

### Prohibited (DON'T)

- **DON'T** change the scenario contract format or JSON Schema IR without an openspec proposal
- **DON'T** push directly to main — all changes go through feature branches with PR review
- **DON'T** skip `ah check` before committing — spec-test correspondence is espectacular's reason to exist
- **DON'T** break backward compatibility of emitted test archetypes without a deprecation cycle
- **DON'T** modify managed blocks (`<!-- WAI: -->`, `<!-- OPENSPEC: -->`, `<!-- DONT: -->`, `<!-- ah:managed: -->`)
- **DON'T** introduce new parallel types for patterns that genesis already provides — file a genesis change first
- **DON'T** commit generated artifacts or `.espectacular/` state files that are tool-managed

### Stop and Ask

Pause and request human input when any of these triggers fire:
1. **Ambiguity** — the ticket text itself is contradictory or underspecified
2. **Scope uncertainty** — the ticket is clear but the change naturally touches code or features not mentioned in it
3. **Irreversibility** — breaking changes to scenario contracts, existing adapter APIs
4. **Secrets/credentials** — any external service, API key, or credential not yet authorized
5. **Test failure persistence** — unresolved test failure after two repair attempts, or the same failure across 3 different approaches
6. **Push/release** — pushing to remote, creating a release, or deploying
7. **Context saturation** — context approaching ~40%; recommend `wai close` then `/clear`

### Minimal Footprint

- Prefer small, focused changes over large refactors — one ticket, one concern
- Delete unused code, don't leave commented-out code behind
- Keep PRs under 400 lines changed. If you cannot, split the work into multiple PRs before proceeding.
- Use existing abstractions (genesis, wai patterns) before introducing new ones
- espectacular is a verification tool — prefer compile-time guarantees over runtime checks

### Drift Detection

Proceed without routine confirmation when the next step is clear.
Do not ask to continue, fix, or commit — just do it. After each major
action (edit, test run, commit), pause and self-check:
1. **ALIGNMENT** — does this still serve detecting spec-code drift?
2. **SCOPE** — did I stay within the ticket scope or did I expand into unticketed work?
3. **FOOTPRINT** — did I leave dead code, debug prints, or unnecessary changes?
4. **GOVERNANCE** — did I follow openspec workflow for spec changes?

If any check fails: undo the last change (`git checkout -- <files>` for
uncommitted edits, `git revert HEAD` for committed) before proceeding,
or open a follow-up ticket.

<!-- WAI:REFLECT:REF:START -->
## Accumulated Project Patterns

Project-specific conventions, gotchas, and architecture notes live in
`.wai/resources/reflections/`. Run `wai search "<topic>"` to retrieve relevant
context before starting research or creating tickets.

> **Before research or ticket creation**: always run `wai search "<topic>"` to
> check for known patterns. Do not rediscover what is already documented.

<!-- WAI:REFLECT:REF:END -->


# Agent Instructions

This project uses **bd** (beads) for issue tracking. Run `bd prime` for full workflow context.

## Quick Reference

```bash
bd ready              # Find available work
bd show <id>          # View issue details
bd update <id> --claim  # Claim work atomically
bd close <id>         # Complete work
bd dolt push          # Push beads data to remote
```

## Non-Interactive Shell Commands

**ALWAYS use non-interactive flags** with file operations to avoid hanging on confirmation prompts.

Shell commands like `cp`, `mv`, and `rm` may be aliased to include `-i` (interactive) mode on some systems, causing the agent to hang indefinitely waiting for y/n input.

**Use these forms instead:**
```bash
# Force overwrite without prompting
cp -f source dest           # NOT: cp source dest
mv -f source dest           # NOT: mv source dest
rm -f file                  # NOT: rm file

# For recursive operations
rm -rf directory            # NOT: rm -r directory
cp -rf source dest          # NOT: cp -r source dest
```

**Other commands that may prompt:**
- `scp` - use `-o BatchMode=yes` for non-interactive
- `ssh` - use `-o BatchMode=yes` to fail instead of prompting
- `apt-get` - use `-y` flag
- `brew` - use `HOMEBREW_NO_AUTO_UPDATE=1` env var

<!-- BEGIN BEADS INTEGRATION v:1 profile:minimal hash:ca08a54f -->
## Beads Issue Tracker

This project uses **bd (beads)** for issue tracking. Run `bd prime` to see full workflow context and commands.

### Quick Reference

```bash
bd ready              # Find available work
bd show <id>          # View issue details
bd update <id> --claim  # Claim work
bd close <id>         # Complete work
```

### Rules

- Use `bd` for ALL task tracking — do NOT use TodoWrite, TaskCreate, or markdown TODO lists
- Run `bd prime` for detailed command reference and session close protocol
- Use `bd remember` for persistent knowledge — do NOT use MEMORY.md files

## Session Completion

**When ending a work session**, you MUST complete ALL steps below. Work is NOT complete until `git push` succeeds.

**MANDATORY WORKFLOW:**

1. **File issues for remaining work** - Create issues for anything that needs follow-up
2. **Run quality gates** (if code changed) - Tests, linters, builds
3. **Update issue status** - Close finished work, update in-progress items
4. **PUSH TO REMOTE** - This is MANDATORY:
   ```bash
   git pull --rebase
   bd dolt push
   git push
   git status  # MUST show "up to date with origin"
   ```
5. **Clean up** - Clear stashes, prune remote branches
6. **Verify** - All changes committed AND pushed
7. **Hand off** - Provide context for next session

**CRITICAL RULES:**
- Work is NOT complete until `git push` succeeds
- NEVER stop before pushing - that leaves work stranded locally
- NEVER say "ready to push when you are" - YOU must push
- If push fails, resolve and retry until it succeeds
<!-- END BEADS INTEGRATION -->

<!-- ah:managed:start -->
## espectacular

Run `ah check` to verify spec-test correspondence before committing.

- `ah check` — validate all deployed specs
- `ah check --changes <name>` — validate with a change overlay
- `ah init` — set up or refresh espectacular project files
- `ah doctor` — diagnose setup issues
- `ah explain <topic>` — playbook guidance for finding kinds and suggested actions
- `ah doctor --enable <adapter>` — write adapter config into .espectacular/config.toml
- `ah signals` — emit dont drift signals
<!-- ah:managed:end -->

## `ah` Binary Availability

Verified: `ah` is available in pi's bash execution context.
- `which ah` → `~/.cargo/bin/ah`
- `ah --version` → `ah 0.3.0`
- `~/.cargo/bin` is in pi's `PATH`

If `ah` is not found in a session, ensure `~/.cargo/bin` is on the `PATH`.
Install with: `cargo install --git git@cv:charly-vibes/espectacular.git`

## Git & Workflow Discipline

- **Never use `git add -A`** — always stage specific files with explicit paths
- **Per-ticket pipeline**: always follow `TDD → ro5u → fix → commit → next ticket`
