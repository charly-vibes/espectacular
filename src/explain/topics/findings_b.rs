//! Execution and quality `ah check` findings.
//!
//! Responsibilities:
//! - failing, empty, and degraded test runs
//! - recommendations for available quality capabilities
//! - mutation, property, and snapshot measurement results
//!
//! Rationale: these findings come from executing tests and quality tools, not from inspecting the spec tree, so they carry different remediation advice

use super::super::model::{Hint, TopicEntry};

pub(crate) static TEST_FAILING: TopicEntry = TopicEntry {
    slug: "test-failing",
    summary: "A test ran but exited non-zero — the scenario behavior is not met.",
    body: "## test-failing — Failing test

A test declared in a contract ran but exited with a non-zero exit code,
meaning the scenario's behavioral requirement is not currently satisfied by
the code.

**Why it appears**: the code does not implement the behavior described in the
scenario, or the test is broken.

**How to fix**: read the scenario prose in the finding output to understand
what behavior is required, then fix the implementation. Do not modify the
scenario or contract — the test describes the contract, not the code.",
    when: "A test executed and returned a non-zero exit code.",
    do_action: "Fix the implementation to satisfy the failing scenario. Do not edit the contract.",
    human_approval: false,
    related_topics: &["edit_code_not_scenario", "no-tests-declared"],
    hints: &[Hint {
        kind: "warning",
        message: "Read the scenario prose in the finding — it describes the required behavior.",
    }],
};

pub(crate) static NO_TESTS_RAN: TopicEntry = TopicEntry {
    slug: "no-tests-ran",
    summary: "A shell test exited 0 but the test filter matched no tests.",
    body: "## no-tests-ran — Test filter matched nothing

A `[[tests.shell]]` command exited successfully (exit code 0) but the test
runner output shows that zero tests were actually executed. The contract
appears green but exercises nothing.

**Why it appears**: the test function was renamed or deleted after the
contract was wired, leaving the filter stale. `cargo test` exits 0 when a
filter matches no tests.

**How to fix**: update the test filter in the contract to match an existing
test function, or add the missing test function to the codebase.",
    when: "A shell test exits 0 and stdout contains `test result: ok. 0 passed`.",
    do_action: "Update the test filter in the contract or add the missing test function.",
    human_approval: false,
    related_topics: &["test-failing", "edit_code_not_scenario"],
    hints: &[Hint {
        kind: "example",
        message: "Run `cargo test -- --list` to see available test names and update the filter.",
    }],
};

pub(crate) static BATCH_FALLBACK: TopicEntry = TopicEntry {
    slug: "batch-fallback",
    summary: "A batched runner invocation degraded; its contracts re-ran per-binding instead.",
    body: "## batch-fallback — Batched invocation degraded, per-binding re-runs used\n\nAbove the batching threshold, contracts bound to the same runner normally\nshare one invocation whose structured output is attributed per contract.\nThis finding means that batched invocation degenerated (unparseable or\noversized structured output, invocation error, or timeout) and every\naffected binding was re-run individually with plain exit-code verdicts.\n\n**Why it matters**: correctness is unaffected — the per-binding re-runs\nare the authoritative verdicts — but the push gate lost its batching\nspeedup for this run, and a persistently degenerate runner (e.g. a JSON\nreporter emitting non-JSON) is worth fixing.\n\n**How to fix**: check the runner's structured reporter (the message names\nthe reason). If the degradation is transient (timeout under load),\nnothing to do; if it repeats, fix the reporter or the hanging tests.",
    when: "A batched invocation's structured output could not be attributed.",
    do_action: "Inspect the runner's structured reporter; per-binding verdicts already carry the truth.",
    human_approval: false,
    related_topics: &["no-tests-ran", "test-failing", "human_review_required"],
    hints: &[],
};

pub(crate) static RECOMMENDATION: TopicEntry = TopicEntry {
    slug: "recommendation",
    summary: "ah detected a quality capability that is available but not yet enabled.",
    body: "## recommendation — Quality capability available

`ah` detected that a quality measurement capability (mutation testing,
property-based testing, or snapshot testing) is available in this repository
but not yet enabled in `.espectacular/config.toml`.

**Why it appears**: the toolchain or framework was detected but the capability
is not yet configured.

**How to fix**: review the recommendation and enable the capability if
appropriate. Use `ah doctor --enable <capability>` to apply the suggested
configuration, or edit `.espectacular/config.toml` directly.",
    when: "A quality capability is available but not enabled.",
    do_action: "Review and optionally enable via `ah doctor --enable <capability>`.",
    human_approval: true,
    related_topics: &[
        "enable_capability",
        "progressive-enablement",
        "quality-mutation",
        "quality-property",
        "quality-snapshot",
    ],
    hints: &[],
};

pub(crate) static UNKNOWN_ACTION: TopicEntry = TopicEntry {
    slug: "unknown-action",
    summary: "A finding contains an unrecognised `suggested_action` value.",
    body: "## unknown-action — Unrecognised suggested action

A finding in the check output contains a `suggested_action` value that is not
in the known set of actions. This usually indicates a version mismatch between
the tool generating the finding and the tool consuming it.

**Why it appears**: the finding was generated by a newer version of `ah` with
a new action type, or by a custom runner with a non-standard suggested_action.

**How to fix**: upgrade `ah` to the latest version, or treat the finding as
requiring human review.",
    when: "A finding's `suggested_action` is not a recognised enum value.",
    do_action: "Upgrade `ah` or treat as `human_review_required`.",
    human_approval: true,
    related_topics: &["human_review_required"],
    hints: &[],
};

pub(crate) static QUALITY_MUTATION: TopicEntry = TopicEntry {
    slug: "quality-mutation",
    summary: "A mutation testing quality measurement result.",
    body: "## quality-mutation — Mutation testing score

This finding records the result of a mutation testing run. Mutation testing
injects small code changes (mutants) and checks whether the test suite detects
them. A higher mutation score means the tests are more sensitive to code
changes.

**Score interpretation**:
- This is a measurement finding, not a gate failure in v1
- A low score suggests tests may pass even when the code is wrong
- Enable mutation testing incrementally — start with a low threshold

**How to enable**: add a `[quality.mutation]` entry to
`.espectacular/config.toml`.",
    when: "Mutation testing is enabled and a run completes.",
    do_action: "Review the score; improve tests if the score is below your threshold.",
    human_approval: false,
    related_topics: &[
        "quality-property",
        "quality-snapshot",
        "progressive-enablement",
        "enable_capability",
    ],
    hints: &[Hint {
        kind: "info",
        message: "In v1, mutation testing is a measurement only — it does not fail the gate.",
    }],
};

pub(crate) static QUALITY_PROPERTY: TopicEntry = TopicEntry {
    slug: "quality-property",
    summary: "A property-based testing quality measurement result.",
    body: "## quality-property — Property-based testing result

This finding records the result of a property-based testing run. Property-based
tests generate many inputs automatically to search for counterexamples to
stated invariants.

**Why it matters**: a property test that passes a hundred examples may still
fail on an input the generator hasn't tried. High coverage from property tests
indicates stronger behavioral guarantees.

**How to enable**: add a `[quality.property]` entry to
`.espectacular/config.toml`.",
    when: "Property-based testing is enabled and a run completes.",
    do_action: "Review results; add property tests for untested invariants.",
    human_approval: false,
    related_topics: &[
        "quality-mutation",
        "quality-snapshot",
        "progressive-enablement",
    ],
    hints: &[],
};

pub(crate) static QUALITY_SNAPSHOT: TopicEntry = TopicEntry {
    slug: "quality-snapshot",
    summary: "A snapshot testing quality measurement result.",
    body: "## quality-snapshot — Snapshot testing result

This finding records the result of a snapshot testing run. Snapshot tests
compare the current output of a function or command against a stored baseline.

**Why it matters**: snapshot tests catch unintended changes to output formats,
JSON shapes, or rendered content.

**How to enable**: add a `[quality.snapshot]` entry to
`.espectacular/config.toml`.

**Updating snapshots**: when output changes intentionally, update the snapshots
and commit the new baseline.",
    when: "Snapshot testing is enabled and a run completes.",
    do_action: "Review changed snapshots; update the baseline if the change is intentional.",
    human_approval: false,
    related_topics: &[
        "quality-mutation",
        "quality-property",
        "progressive-enablement",
    ],
    hints: &[],
};
