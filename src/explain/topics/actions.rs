//! `suggested_action` topics — what to do next for a finding.
//!
//! Responsibilities:
//! - command-shaped actions (`ah init`, `ah scenario new`, `ah scenario supersede`)
//! - judgment-shaped actions (fix the code, review and apply, human review)
//!
//! Rationale: the finding payload names a `suggested_action`, and `ah explain <action>` must resolve it to concrete guidance

use super::super::model::{Hint, TopicEntry};

pub(crate) static RUN_AH_INIT: TopicEntry = TopicEntry {
    slug: "run_ah_init",
    summary: "Run `ah init` to create or refresh the project's espectacular scaffold.",
    body: "## run_ah_init — Run ah init

Run `ah init` in the repository root to create or refresh the `.espectacular/`
directory structure, `config.toml`, and any missing scaffold files.

```
ah init
```

**When to use**: on a fresh repository, after cloning a project that uses
`ah`, or after the tool version is upgraded.",
    when: "The .espectacular directory or config.toml is missing or outdated.",
    do_action: "Run `ah init` in the repository root.",
    human_approval: false,
    related_topics: &["workflow"],
    hints: &[],
};

pub(crate) static RUN_AH_SCENARIO_NEW: TopicEntry = TopicEntry {
    slug: "run_ah_scenario_new",
    summary: "Run `ah scenario new` to create a contract file for a spec scenario.",
    body: "## run_ah_scenario_new — Create a contract

Run `ah scenario new` to generate the contract `.toml` stub for a scenario
that exists in a spec but has no contract yet.

```
ah scenario new <change> <spec> --requirement <scenario-id> <heading>
```

This creates `.espectacular/<component>/<slug>.toml` with required fields
pre-filled. Edit the `[tests]` table to add test entries.",
    when: "A spec scenario exists but its contract .toml file is absent.",
    do_action: "Run `ah scenario new <change> <spec> --requirement <id> <heading>`.",
    human_approval: false,
    related_topics: &["no-toml", "workflow"],
    hints: &[Hint {
        kind: "tip",
        message: "The command names the file correctly and prevents slug collisions.",
    }],
};

pub(crate) static RUN_AH_SCENARIO_SUPERSEDE: TopicEntry = TopicEntry {
    slug: "run_ah_scenario_supersede",
    summary: "Run `ah scenario supersede` to mark a contract as replaced.",
    body: "## run_ah_scenario_supersede — Supersede a contract

Run `ah scenario supersede` to mark an existing contract as superseded and
link it to its replacement:

```
ah scenario supersede <spec> <old-id> --with <new-id> --in-change <change>
```

This sets `status = \"superseded\"` and `superseded_by = <new-id>` in the old
contract, maintaining the audit trail without deleting history.",
    when: "A scenario's behavior has changed and the old contract should be retired.",
    do_action: "Run `ah scenario supersede` to mark the old contract as replaced.",
    human_approval: false,
    related_topics: &["supersession", "missing-replacement"],
    hints: &[],
};

pub(crate) static EDIT_CODE_NOT_SCENARIO: TopicEntry = TopicEntry {
    slug: "edit_code_not_scenario",
    summary: "Fix the implementation — do not edit the scenario or contract.",
    body: "## edit_code_not_scenario — Fix the code, not the scenario

The failing test describes a behavioral contract. The scenario and contract
are the source of truth for what the code should do. **Do not edit the
scenario or contract** to make the test pass.

**How to fix**: read the scenario prose from the finding output to understand
the required behavior, then modify the implementation until the test passes.

**Why this rule matters**: editing the scenario to match the code defeats the
purpose of spec-driven development — the spec should drive the code, not the
reverse.",
    when: "A test is failing and the scenario describes correct expected behavior.",
    do_action: "Read the scenario prose and fix the implementation.",
    human_approval: false,
    related_topics: &["test-failing", "workflow"],
    hints: &[Hint {
        kind: "warning",
        message: "The scenario prose in the finding describes exactly what the code must do.",
    }],
};

pub(crate) static ENABLE_CAPABILITY: TopicEntry = TopicEntry {
    slug: "enable_capability",
    summary: "Enable a detected quality measurement capability.",
    body: "## enable_capability — Enable a quality capability

A quality measurement capability (mutation testing, property-based testing,
or snapshot testing) was detected in the repository and is recommended.

**How to enable**: run `ah doctor --enable <capability>` to write the
suggested configuration entry, then commit the updated
`.espectacular/config.toml`.

Alternatively, add the entry manually:

```toml
[quality.mutation]
enabled = true
threshold = 0
```

Start with a threshold of `0` and raise it as the score improves.",
    when: "A quality capability is detected but not yet configured.",
    do_action: "Run `ah doctor --enable <capability>` or edit config.toml.",
    human_approval: true,
    related_topics: &["recommendation", "progressive-enablement"],
    hints: &[Hint {
        kind: "tip",
        message: "Start with threshold = 0 to measure without gating, then raise gradually.",
    }],
};

pub(crate) static REVIEW_AND_APPLY: TopicEntry = TopicEntry {
    slug: "review_and_apply",
    summary: "Review the structural issue and apply the appropriate fix manually.",
    body: "## review_and_apply — Review and apply a fix

This finding identifies a structural issue in the spec/contract relationship
that requires human review before applying a fix.

**Common cases**:
- `no-toml`: create the missing contract with `ah scenario new`
- `orphan-toml`: delete the orphan or restore the scenario
- `slug-collision`: rename one of the conflicting scenarios
- `id-mismatch`: fix the `id` field in the `.toml`
- `malformed-contract`: read the `message` field for the specific error, then fix the `.toml`
- `no-tests-declared`: add a `[[tests.*]]` entry to the contract
- `overlay-conflict`: resolve the conflicting changes

Review the `kind` and `message` fields in the finding for the specific issue.",
    when: "A structural spec/contract issue needs manual inspection before fixing.",
    do_action: "Review the finding details and apply the appropriate fix.",
    human_approval: true,
    related_topics: &[
        "no-toml",
        "orphan-toml",
        "slug-collision",
        "id-mismatch",
        "malformed-contract",
    ],
    hints: &[],
};

pub(crate) static HUMAN_REVIEW_REQUIRED: TopicEntry = TopicEntry {
    slug: "human_review_required",
    summary: "This finding requires human judgment — no mechanical fix is available.",
    body: "## human_review_required — Human review needed

This finding cannot be resolved by a mechanical action. It requires human
judgment to determine the correct course of action.

**Why it appears**: the finding falls outside the known action space (possibly
a new finding kind from a newer tool version), or the situation is ambiguous
enough that automation would risk data loss.

**How to proceed**: read the finding's `message` and `scenario_prose` fields,
understand the context, and decide on the appropriate action.",
    when: "A finding needs human judgment to resolve.",
    do_action: "Read the full finding details and apply human judgment.",
    human_approval: true,
    related_topics: &["unknown-action", "review_and_apply"],
    hints: &[],
};
