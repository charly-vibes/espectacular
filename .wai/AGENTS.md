# wai Workflow Reference

> This file is managed by `wai init`. Do not edit manually.
> Changes will be overwritten on the next init.

## When to Use What

| Need | Tool | Example |
|------|------|---------|
| Record reasoning/research | wai | `wai add research "findings"` |
| Capture design decisions | wai | `wai add design "architecture choice"` |
| Session context transfer | wai | `wai handoff create <project>` |
| Track work items/bugs | `bd` | `bd create --title="..." --type=task` |
| Find available work | `bd` | `bd ready` |
| Manage dependencies | `bd` | `bd dep add <blocked> <blocker>` |
| Propose system changes | openspec | Read `openspec/AGENTS.md` |
| Define requirements | openspec | `openspec validate --strict` |

Key distinction:
- **wai** = *why* decisions were made (reasoning, context, handoffs)
- **`bd`** (beads) = *what* needs to be done (concrete tasks, status tracking)
- **openspec** = *what the system should look like* (specs, requirements, proposals)

## Starting a Session

1. Run `wai sync` to ensure all agent tools and skills are correctly projected.
2. Run `wai status` to see active projects, current phase, and suggestions.
3. Run `bd ready` to find available work items.
   Before claiming: read the relevant source files to confirm
   the issue is not already implemented.
4. Check `openspec list` for active change proposals.
5. Check the phase — it tells you what kind of work is expected:
   - **research** → gather information, explore options
   - **design** → make architectural decisions
   - **plan** → break work into tasks
   - **implement** → write code, guided by research/plans
   - **review** → validate against plans
   - **archive** → wrap up
6. Read existing artifacts with `wai search "<topic>"` before starting new work.

## Capturing Work

Record the reasoning behind your work, not just the output:

```bash
wai add research "findings"         # What you learned, trade-offs
wai add plan "approach"             # How you'll implement, why
wai add design "decisions"          # Architecture choices, rationale
wai add research --file notes.md    # Import longer content
```

Use `--project <name>` if multiple projects exist. Otherwise wai picks the first one.

Phases are a guide, not a gate. Use `wai phase show` / `wai phase next`.

## Tracking Work Across Tools

When beads and openspec are both active, keep them in sync:
- When creating a beads ticket for an openspec task, include the task
  reference in the description (format: `<change-id>:<phase>.<task>`,
  e.g. `add-why-command:7.1`)
- When closing a beads ticket linked to a task, also check the box
  (`[x]`) in the change's `tasks.md`

## Ending a Session

Before saying "done", run this checklist:

```
[ ] wai handoff create <project>   # capture context for next session
[ ] bd close <id>                  # close completed issues; also close parent epic if last sub-task
[ ] openspec tasks.md — mark completed tasks [x]
[ ] openspec list — archive any ✓ Complete changes (`openspec archive <id> --yes`)
[ ] wai reflect                    # update CLAUDE.md with project patterns (every ~5 sessions)
[ ] git add <files> && git commit  # commit code + handoff
```

If beads needs any extra follow-up beyond `bd close`, run `bd` and use the
commands your installed version offers. Do not assume a hard-coded sync
subcommand.

### Autonomous Loop

One task per session. The resume loop:

1. `wai prime` — orient (shows ⚡ RESUMING if mid-task)
2. Work on the single task
3. `wai close` — capture state (run this before every `/clear`)
4. `git add <files> && git commit`
5. `/clear` — fresh context

→ Next session: `wai prime` shows RESUMING with exact next steps.

When context reaches ~40%: stop and tell the user — responses degrade past
this point. Recommend `wai close` then `/clear` to resume cleanly.
Do NOT skip `wai close` — it enables resume detection.

## Quick Reference

### wai
```bash
wai status                    # Project status and next steps
wai add research "notes"      # Add research artifact
wai add plan "plan"           # Add plan artifact
wai add design "design"       # Add design artifact
wai add skill <name>          # Scaffold a new agent skill
wai search "query"            # Search across artifacts
wai search --tag <tag>        # Filter by tag (repeatable)
wai search --latest           # Most recent match only
wai why "why use TOML?"       # Ask why (LLM-powered oracle)
wai why src/config.rs         # Explain a file's history
wai reflect                   # Synthesize project patterns into CLAUDE.md
wai close                     # Session handoff + pending-resume signal
wai phase show                # Current phase
wai doctor                    # Workspace health
wai pipeline list             # List pipelines
wai pipeline start <n> --topic=<t>  # Start a run; set WAI_PIPELINE_RUN=<id>
wai pipeline next             # Advance to next step
```

### beads (CLI: `bd`)
```bash
bd ready                     # Available work
bd show <id>                 # Issue details
bd create --title="..."      # New issue
bd update <id> --status=in_progress
bd close <id>                # Complete work
```

### openspec
Read `openspec/AGENTS.md` for full instructions.
```bash
openspec list              # Active changes
openspec list --specs      # Capabilities
```

## Structure

The `.wai/` directory organizes artifacts using the PARA method:
- **projects/** — active work with phase tracking and dated artifacts
- **areas/** — ongoing responsibilities (no end date)
- **resources/** — reference material, agent configs, templates
- **archives/** — completed or inactive items

Do not edit `.wai/config.toml` directly. Use `wai` commands instead.
