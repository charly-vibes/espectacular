# Design: Spec-to-assertion compilation and drift detection

## Core concepts

### Assertion model

An openspec scenario:

```markdown
#### Scenario: User toggles calendar [PF]
- **WHEN** user clicks the calendar toggle
- **THEN** the display switches to Holocene Era format
```

Becomes a **spec assertion** — a structured intermediate representation (IR) defined via **JSON Schema**:

```json
{
  "assertion": {
    "id": "calendar-support/toggle-calendar",
    "archetype": "PF",
    "when": "user clicks the calendar toggle",
    "then": "the display switches to Holocene Era format",
    "source": "specs/calendar-support/spec.md:42",
    "archived": "2026-04-15"
  }
}
```

This IR is language-agnostic. **Emitters** (using templates) translate it into **Structural Frames** — skeletal code blocks that an AI agent or developer can implement.

### Structural Frames (Archetypes)

To help LLMs implement these assertions reliably, we categorize scenarios into **Archetypes**:
- **PF (Pure Functional):** Deterministic, ideal for PBT/Fuzzing.
- **SA (Stateful API):** Side-effecting handlers and state transitions.
- **BP (Boundary Protocol):** External integrations and mocks.

The emitters generate the **frame** (signature, metadata tags, TODOs for WHEN/THEN) but NOT the implementation logic. This keeps the tool simple, robust, and truly language-agnostic.

See `archetypes.md` for full definitions.

### Two directions

### Two directions

| Direction | Trigger | Input | Output |
|-----------|---------|-------|--------|
| **Forward** (spec → frames) | `openspec archive` hook | archived spec.md | assertion IR + Structural Frames |
| **Backward** (drift detection) | git hook, CI, or manual | assertion IR + Test Results (JUnit/TAP) | drift report |

`espectacular` provides a standalone CLI (e.g., `espectacular compile`) that the `openspec` tool triggers via its existing hook system during the archive workflow.

### Drift detection strategy

Drift can mean:
1. **Assertion fails** — code changed behavior, spec is stale (detected via test result ingestion)
2. **Assertion orphaned** — code removed, spec still references it
3. **Code uncovered** — new behavior exists without a spec

**Decision:** Source in openspec archive, generated into project tests as **Structural Frames**. One-way flow (archive → tests). 

To ensure language-agnostic drift detection:
- **Result Ingestion**: Use standard test outputs (JUnit XML, TAP).
- **ID Embedding**: Emitters MUST embed the Assertion ID into the generated test name or metadata block (e.g., `Test_capability_req_scenario`) so the result parser can map failures back to the spec.


### What's the assertion granularity?

- One assertion per scenario (finest grain, most useful)
- One assertion per requirement (groups scenarios, less noise)
- One assertion per capability (too coarse for meaningful drift)

**Decision:** One per scenario, grouped by requirement for reporting.

### How to map assertions to code?

Options:
- **Convention-based**: test file naming matches spec capability (`test_calendar_support.rs` ↔ `specs/calendar-support/`)
- **Annotation-based**: code comments or attributes link to spec IDs (`#[spec("calendar-support/toggle")]`)
- **Heuristic**: grep for keywords from WHEN/THEN clauses in test files

**Decision:** Convention-based for v1 with optional annotations. Heuristics are not reliable enough — defer or drop. The tracer bullet (M0) will validate that convention-based mapping works for at least one real repo before committing to this approach.

### Language / runtime

The implementation behind espectacular could be:
- A Rust binary (consistent with wai, fotos)
- A Python script (consistent with release tooling)
- An openspec subcommand (extension to existing CLI if one exists)

**Decision:** Ship a Rust binary surfaced to users as the `ah` command, distributed via homebrew-charly like the other tools. The assertion IR is YAML/JSON so emitters can be added without recompiling.

## Risks and mitigations

### RISK-001: WHEN/THEN scenarios may not be machine-parseable

OpenSpec scenarios are written in natural language. Parsing "WHEN user clicks the calendar toggle" into a structured assertion requires either strict formatting constraints or NLP. If scenarios are too free-form, the parser becomes unreliable or requires AI assistance.

**Mitigation:** M0 tracer bullet starts with one hardcoded scenario to validate the parsing approach. If free-form parsing fails, introduce a stricter scenario micro-format (e.g., `WHEN <subject> <verb> <object>`) as an openspec convention. Measure parse success rate across existing specs before committing to the full parser in M1.

### RISK-002: Convention-based code mapping may not generalize

Convention-based mapping (`test_calendar_support.rs` ↔ `specs/calendar-support/`) assumes repos follow predictable naming. Repos with different test structures will break the mapping.

**Mitigation:** M0 validates against one real repo. If convention-based mapping covers <80% of test files in target repos (wai, fabbro, fotos), escalate to annotation-based mapping in M1. The emitter already generates source traceability comments — annotations are a small incremental step.

### Boundary: wai-f0dv drift vs. espectacular drift

espectacular drift = spec scenarios vs. runtime behavior (behavioral contracts). wai-f0dv drift = decision artifacts vs. codebase they describe (decision context). These are complementary, not overlapping. espectacular signals can feed into wai's freshness checks but not replace them.

## Integration with feedback loops

When drift is detected, the signal should flow:
- To **wai**: flag that a decision artifact may be stale (connects to wai-f0dv)
- To **dont**: drift in claim-related specs could invalidate grounding (connects to dont-nwck)
- To **pretender**: persistent drift patterns could become structural constraints (connects to pretender-5rk)
