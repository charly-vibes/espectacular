//! Workflow and quality themes not tied to a single finding.
//!
//! Responsibilities:
//! - the spec-driven development loop and supersession policy
//! - archetypes, scenario-scoped tests, and progressive quality enablement
//!
//! Rationale: these are the conceptual topics readers reach for by name, and they change far less often than per-finding remediation

use super::super::model::{Hint, TopicEntry};

pub(crate) static SCENARIO_SCOPED_TESTS: TopicEntry = TopicEntry {
    slug: "scenario-scoped-tests",
    summary: "Scope each contract's test entries to only the tests that verify it.",
    body: "## scenario-scoped-tests — Scope test entries to the scenario\n\nEach `[[tests.*]]` entry runs once per contract that declares it. An entry\nthat includes a whole test file (or an entire suite) re-executes everything\nin it for every contract that references any scenario in that file — N\ncontracts means N full runs, and a failure in any single test blurs\nattribution across every scenario sharing the file.\n\n**Recommended pattern**: scope each entry to the single test that verifies\nthe scenario, using the runner's selection mechanism:\n\n- `cargo`: substring filter — `flags = \"scenario_id\"` matches test names\n  containing the scenario id (this is how property-derived `p-*` contracts\n  bind). A filter that matches nothing fails the gate via `no-tests-ran`, so\n  a typo in the binding cannot silently pass.\n- `pytest`: node id or keyword — `flags = \"tests/unit/test_message.py::test_defaults\"`\n  or `flags = \"-kmessage_defaults\"`.\n- `vitest`: `flags = \"--testNamePattern=message_defaults\"`.\n- Frameworks without test filtering (e.g. Julia's `Test`): dispatch on the\n  scenario id inside the test file — pass it as an argument or environment\n  variable and select the matching `@testset` to run.\n\n`flags` reaches the runner as a **single argument** — prefer one-token forms\n  (node ids, `--flag=value`, `-k<expr>`). When you need multiple arguments, use\n  a `[[tests.shell]]` entry instead (`command` is passed to `/bin/sh -c` and\n  tokenizes normally; requires a POSIX shell).\n\nIf a framework offers no selection mechanism at all and dispatching is not\nfeasible, split scenarios into separate test files so one include runs one\nscenario's assertions only.",
    when: "A contract's test entries run more than the scenario they verify.",
    do_action: "Scope each test entry to the single test that verifies the scenario (filter flag, node id, or dispatch argument).",
    human_approval: false,
    related_topics: &["no-tests-declared", "no-tests-ran", "workflow"],
    hints: &[Hint {
        kind: "example",
        message: r#"[[tests.cargo]]
flags = "message_defaults"
timeout_seconds = 30

# filter-less frameworks: dispatch on the scenario id
[[tests.shell]]
command = "julia --project=. test/unit/message_test.jl message_defaults"
timeout_seconds = 60"#,
    }],
};

pub(crate) static WORKFLOW: TopicEntry = TopicEntry {
    slug: "workflow",
    summary: "Overview of the espectacular spec-driven development workflow.",
    body: "## workflow — espectacular workflow

espectacular (`ah`) enforces spec-driven development: specifications drive
tests, tests drive code.

**Core loop**:
1. Write or update a scenario in a spec file
2. Run `ah scenario new` to generate the contract stub
3. Add test entries to the contract
4. Write failing tests that exercise the behavior
5. Implement the behavior until tests pass
6. Run `ah check` to verify all scenarios pass

**Key commands**:
- `ah init` — set up the project
- `ah check` — verify all scenarios
- `ah check --changes <name>` — verify with an in-flight change
- `ah scenario new` — create a contract stub
- `ah scenario supersede` — retire a changed contract
- `ah explain <topic>` — get guidance on any finding or action
- `ah type <code>` — look up an archetype

**Finding flow**: when `ah check` emits a finding, read `suggested_action`
and run `ah explain <suggested_action>` for guidance.",
    when: "You want an overview of how to use espectacular.",
    do_action: "Follow the core loop: spec → contract → test → implement → check.",
    human_approval: false,
    related_topics: &["archetypes", "supersession", "progressive-enablement"],
    hints: &[],
};

pub(crate) static SUPERSESSION: TopicEntry = TopicEntry {
    slug: "supersession",
    summary: "How to retire and replace contracts when behavior changes.",
    body: "## supersession — Retiring and replacing contracts

When the behavior described by a scenario changes, do not edit the existing
contract. Instead, supersede it and create a replacement.

**Why**: superseded contracts form an audit trail. An agent or reviewer can
trace the history of behavioral decisions without reading git history.

**How to supersede**:
```
ah scenario supersede <spec> <old-id> --with <new-id> --in-change <change>
```

This sets `status = \"superseded\"` and `superseded_by = <new-id>` on the
old contract. The old contract is preserved but excluded from active checks.

**When to supersede vs. edit**:
- Behavior changes → supersede
- Test entry typo or runner fix → edit in place (no behavior change)
- Scenario renamed → supersede the old slug, create new",
    when: "A scenario's required behavior changes and the contract must be updated.",
    do_action: "Use `ah scenario supersede` to retire the old contract and create a replacement.",
    human_approval: false,
    related_topics: &[
        "workflow",
        "run_ah_scenario_supersede",
        "missing-replacement",
    ],
    hints: &[],
};

pub(crate) static ARCHETYPES: TopicEntry = TopicEntry {
    slug: "archetypes",
    summary: "Scenario archetypes classify behavioral patterns for consistent test shapes.",
    body: "## archetypes — Scenario archetypes

Archetypes classify the behavioral pattern a scenario describes. The archetype
informs the appropriate test shape.

**List archetypes**: `ah type`
**Look up an archetype**: `ah type <code>`

**Available archetypes**:
- `PF` — Pure Functional: deterministic, input → output
- `SA` — Stateful API: state transitions, ordered operations
- `BP` — Boundary Protocol: CLI, HTTP, filesystem, serialization
- `CE` — Contract/Event: structured outputs, events, machine-readable signals
- `NR` — Non-Regression: guardrails, compatibility, migration safety

**When to choose an archetype**: set `archetype` in the contract `.toml`
when creating a new contract. Use the archetype that best describes how the
behavior should be verified.",
    when: "You want guidance on choosing the right test shape for a scenario.",
    do_action: "Run `ah type <code>` to read the archetype description.",
    human_approval: false,
    related_topics: &["workflow", "no-tests-declared"],
    hints: &[],
};

pub(crate) static PROGRESSIVE_ENABLEMENT: TopicEntry = TopicEntry {
    slug: "progressive-enablement",
    summary: "How to incrementally adopt quality measurement capabilities.",
    body: "## progressive-enablement — Incremental quality adoption

Quality measurement capabilities (mutation, property, snapshot testing) are
opt-in. Enable them incrementally to avoid overwhelming teams with findings.

**Recommended sequence**:
1. Run `ah check` with no quality capabilities — fix all structural findings
2. Enable one capability at a time: start with snapshot testing (fast, low
   overhead), then property testing, then mutation (slowest)
3. Set initial threshold to `0` to measure without gating
4. Review the scores over several runs, then raise the threshold gradually

**Configuring a capability**:
```toml
[quality.snapshot]
enabled = true
threshold = 0   # raise this once the score stabilises
```

**Pre-commit note**: mutation testing is automatically skipped in pre-commit
hooks — it is too slow for interactive use. Run it in CI instead.",
    when: "You want to add quality measurement without disrupting the team.",
    do_action: "Enable one capability at a time, starting with threshold = 0.",
    human_approval: false,
    related_topics: &[
        "enable_capability",
        "quality-mutation",
        "quality-property",
        "quality-snapshot",
    ],
    hints: &[Hint {
        kind: "tip",
        message: "Snapshot testing has the lowest overhead — enable it first.",
    }],
};
