use serde::Serialize;

pub struct TopicEntry {
    pub slug: &'static str,
    pub summary: &'static str,
    pub body: &'static str,
    pub when: &'static str,
    pub do_action: &'static str,
    pub human_approval: bool,
    pub related_topics: &'static [&'static str],
    pub hints: &'static [Hint],
}

pub struct Hint {
    pub kind: &'static str,
    pub message: &'static str,
}

#[derive(Serialize)]
pub struct TopicJson {
    pub topic: &'static str,
    pub summary: &'static str,
    pub when: &'static str,
    #[serde(rename = "do")]
    pub do_action: &'static str,
    pub human_approval: bool,
    pub related_topics: &'static [&'static str],
    pub hints: Vec<HintJson>,
}

#[derive(Serialize)]
pub struct HintJson {
    pub kind: &'static str,
    pub message: &'static str,
}

// COMPILE-TIME ENFORCEMENT: exhaustive match over these enums ensures
// every variant has a topic body. Adding a variant without a match arm
// causes a compile error.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum FindingKind {
    NoToml,
    OrphanToml,
    SlugCollision,
    IdMismatch,
    InvalidStatus,
    NoTestsDeclared,
    MissingRunner,
    MissingAdapter,
    MalformedContract,
    MissingReplacement,
    OverlayConflict,
    TestFailing,
    NoTestsRan,
    Recommendation,
    UnknownAction,
    QualityMutation,
    QualityProperty,
    QualitySnapshot,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum SuggestedAction {
    RunAhInit,
    RunAhScenarioNew,
    RunAhScenarioSupersede,
    EditCodeNotScenario,
    EnableCapability,
    ReviewAndApply,
    HumanReviewRequired,
}

const ALL_FINDING_KINDS: &[FindingKind] = &[
    FindingKind::NoToml,
    FindingKind::OrphanToml,
    FindingKind::SlugCollision,
    FindingKind::IdMismatch,
    FindingKind::InvalidStatus,
    FindingKind::NoTestsDeclared,
    FindingKind::MissingRunner,
    FindingKind::MissingAdapter,
    FindingKind::MalformedContract,
    FindingKind::MissingReplacement,
    FindingKind::OverlayConflict,
    FindingKind::TestFailing,
    FindingKind::NoTestsRan,
    FindingKind::Recommendation,
    FindingKind::UnknownAction,
    FindingKind::QualityMutation,
    FindingKind::QualityProperty,
    FindingKind::QualitySnapshot,
];

const ALL_SUGGESTED_ACTIONS: &[SuggestedAction] = &[
    SuggestedAction::RunAhInit,
    SuggestedAction::RunAhScenarioNew,
    SuggestedAction::RunAhScenarioSupersede,
    SuggestedAction::EditCodeNotScenario,
    SuggestedAction::EnableCapability,
    SuggestedAction::ReviewAndApply,
    SuggestedAction::HumanReviewRequired,
];

fn finding_kind_entry(kind: FindingKind) -> &'static TopicEntry {
    match kind {
        FindingKind::NoToml => &NO_TOML,
        FindingKind::OrphanToml => &ORPHAN_TOML,
        FindingKind::SlugCollision => &SLUG_COLLISION,
        FindingKind::IdMismatch => &ID_MISMATCH,
        FindingKind::InvalidStatus => &INVALID_STATUS,
        FindingKind::NoTestsDeclared => &NO_TESTS_DECLARED,
        FindingKind::MissingRunner => &MISSING_RUNNER,
        FindingKind::MissingAdapter => &MISSING_ADAPTER,
        FindingKind::MalformedContract => &MALFORMED_CONTRACT,
        FindingKind::MissingReplacement => &MISSING_REPLACEMENT,
        FindingKind::OverlayConflict => &OVERLAY_CONFLICT,
        FindingKind::TestFailing => &TEST_FAILING,
        FindingKind::NoTestsRan => &NO_TESTS_RAN,
        FindingKind::Recommendation => &RECOMMENDATION,
        FindingKind::UnknownAction => &UNKNOWN_ACTION,
        FindingKind::QualityMutation => &QUALITY_MUTATION,
        FindingKind::QualityProperty => &QUALITY_PROPERTY,
        FindingKind::QualitySnapshot => &QUALITY_SNAPSHOT,
    }
}

fn suggested_action_entry(action: SuggestedAction) -> &'static TopicEntry {
    match action {
        SuggestedAction::RunAhInit => &RUN_AH_INIT,
        SuggestedAction::RunAhScenarioNew => &RUN_AH_SCENARIO_NEW,
        SuggestedAction::RunAhScenarioSupersede => &RUN_AH_SCENARIO_SUPERSEDE,
        SuggestedAction::EditCodeNotScenario => &EDIT_CODE_NOT_SCENARIO,
        SuggestedAction::EnableCapability => &ENABLE_CAPABILITY,
        SuggestedAction::ReviewAndApply => &REVIEW_AND_APPLY,
        SuggestedAction::HumanReviewRequired => &HUMAN_REVIEW_REQUIRED,
    }
}

const GENERAL_TOPICS: &[&TopicEntry] = &[
    &WORKFLOW,
    &SUPERSESSION,
    &ARCHETYPES,
    &PROGRESSIVE_ENABLEMENT,
];

const ADAPTER_TOPICS: &[&TopicEntry] = &[&ADAPTER_PYTEST, &ADAPTER_CARGO, &ADAPTER_VITEST];

pub fn all_topics() -> Vec<&'static TopicEntry> {
    let mut entries: Vec<&'static TopicEntry> = vec![];
    for kind in ALL_FINDING_KINDS {
        entries.push(finding_kind_entry(*kind));
    }
    for action in ALL_SUGGESTED_ACTIONS {
        entries.push(suggested_action_entry(*action));
    }
    entries.extend(GENERAL_TOPICS);
    entries.extend(ADAPTER_TOPICS);
    entries.sort_by_key(|t| t.slug);
    entries
}

pub fn lookup(slug: &str) -> Option<&'static TopicEntry> {
    all_topics().into_iter().find(|t| t.slug == slug)
}

pub fn did_you_mean(input: &str) -> Vec<&'static str> {
    all_topics()
        .into_iter()
        .filter(|t| t.slug.contains(input) || input.contains(t.slug))
        .map(|t| t.slug)
        .collect()
}

pub fn run_explain(topic: Option<&str>, list: bool, json: bool) -> anyhow::Result<()> {
    if list {
        let mut slugs: Vec<&str> = all_topics().into_iter().map(|t| t.slug).collect();
        slugs.sort_unstable();
        for slug in slugs {
            println!("{slug}");
        }
        return Ok(());
    }

    let Some(slug) = topic else {
        let slugs: Vec<&str> = all_topics().into_iter().map(|t| t.slug).collect();
        eprintln!("usage: ah explain <topic>");
        eprintln!("topics: {}", slugs.join(", "));
        std::process::exit(1);
    };

    let Some(entry) = lookup(slug) else {
        let suggestions = did_you_mean(slug);
        if suggestions.is_empty() {
            eprintln!("unknown topic: {slug}");
        } else {
            eprintln!(
                "unknown topic: {slug}. Did you mean: {}?",
                suggestions.join(", ")
            );
        }
        std::process::exit(1);
    };

    if json {
        let output = TopicJson {
            topic: entry.slug,
            summary: entry.summary,
            when: entry.when,
            do_action: entry.do_action,
            human_approval: entry.human_approval,
            related_topics: entry.related_topics,
            hints: entry
                .hints
                .iter()
                .map(|h| HintJson {
                    kind: h.kind,
                    message: h.message,
                })
                .collect(),
        };
        println!("{}", serde_json::to_string_pretty(&output)?);
    } else {
        println!("{}", entry.body);
    }

    Ok(())
}

// ── Finding kind topics ───────────────────────────────────────────────────────

static NO_TOML: TopicEntry = TopicEntry {
    slug: "no-toml",
    summary: "A scenario in a spec file has no matching contract .toml file.",
    body: "## no-toml — Missing contract file

A scenario declared in a spec file has no corresponding contract `.toml` file
in `.espectacular/<component>/`.

**Why it appears**: the scenario was added to the spec but `ah scenario new`
was never run (or the generated file was deleted).

**How to fix**: run `ah scenario new` with the spec, scenario id, and heading
to generate the contract stub, then populate the test entries.

```
ah scenario new <change> <spec> --requirement <scenario-id> <heading>
```",
    when: "A spec scenario exists but its contract .toml file is absent.",
    do_action: "Run `ah scenario new` to create the missing contract file.",
    human_approval: false,
    related_topics: &["run_ah_scenario_new", "orphan-toml"],
    hints: &[Hint {
        kind: "tip",
        message:
            "Use `ah scenario new` — it names the file correctly and pre-fills required fields.",
    }],
};

static ORPHAN_TOML: TopicEntry = TopicEntry {
    slug: "orphan-toml",
    summary: "A contract .toml file has no matching scenario in any spec.",
    body: "## orphan-toml — Orphaned contract file

A contract `.toml` file exists in `.espectacular/<component>/` but no spec
scenario matches its slug.

**Why it appears**: the scenario was removed from the spec (or renamed) without
deleting the corresponding contract file.

**How to fix**: either restore the scenario in the spec, or delete the orphan
contract file. If the behavior is still required, keep the contract and re-add
the scenario.",
    when: "A .toml contract file exists but its scenario was removed from the spec.",
    do_action: "Delete the orphan .toml file or restore the matching spec scenario.",
    human_approval: true,
    related_topics: &["no-toml", "review_and_apply"],
    hints: &[],
};

static SLUG_COLLISION: TopicEntry = TopicEntry {
    slug: "slug-collision",
    summary: "Two scenarios in a spec share the same slug.",
    body: "## slug-collision — Duplicate scenario slug

Two scenarios within the same spec file produce the same slug (the filename
stem used for the contract `.toml`).

**Why it appears**: two headings differ only in punctuation or capitalization,
or were copy-pasted and not renamed.

**How to fix**: rename one of the scenarios so their slugs are unique, then
re-run `ah scenario new` for the renamed one.",
    when: "Two spec scenarios produce the same contract filename.",
    do_action: "Rename one of the conflicting scenarios so slugs are unique.",
    human_approval: false,
    related_topics: &["no-toml", "review_and_apply"],
    hints: &[],
};

static ID_MISMATCH: TopicEntry = TopicEntry {
    slug: "id-mismatch",
    summary: "The `id` field inside a contract .toml does not match its filename.",
    body: "## id-mismatch — Contract id/filename mismatch

The `id` field declared inside the contract `.toml` file does not match the
file's stem (the filename without `.toml`).

**Why it appears**: the file was renamed manually without updating the `id`
field, or the `id` was edited by hand.

**How to fix**: open the `.toml` and set `id` to match the filename stem
exactly.",
    when: "A contract file's `id` field differs from its filename.",
    do_action: "Edit the .toml to set `id` equal to the filename stem.",
    human_approval: false,
    related_topics: &["malformed-contract", "review_and_apply"],
    hints: &[],
};

static INVALID_STATUS: TopicEntry = TopicEntry {
    slug: "invalid-status",
    summary: "A contract has a `status` value that is not `active` or `superseded`.",
    body: "## invalid-status — Unknown contract status

The `status` field in a contract `.toml` holds an unrecognised value.
Valid values are `active` and `superseded`.

**Why it appears**: a typo, or the file was edited by hand with an incorrect
value.

**How to fix**: set `status` to `active` (behavior is in use) or `superseded`
(behavior replaced by a newer contract).",
    when: "A contract's `status` field is neither `active` nor `superseded`.",
    do_action: "Set `status` to `active` or `superseded` in the .toml file.",
    human_approval: false,
    related_topics: &["malformed-contract", "review_and_apply"],
    hints: &[],
};

static NO_TESTS_DECLARED: TopicEntry = TopicEntry {
    slug: "no-tests-declared",
    summary: "A contract declares no test entries in its `[tests]` table.",
    body: "## no-tests-declared — Empty test table

A contract `.toml` exists and is structurally valid, but its `[tests]` table
is empty. An empty test table means the scenario is never verified.

**Why it appears**: the contract stub was generated but test entries were not
added yet.

**How to fix**: add at least one test entry under `[tests]`. Choose a test
type that matches your test runner (`unit`, `pytest`, `cargo`, `vitest`, or
`shell`).",
    when: "A contract's `[tests]` table has no entries.",
    do_action: "Add test entries to the contract's `[tests]` table.",
    human_approval: false,
    related_topics: &["missing-runner", "edit_code_not_scenario"],
    hints: &[Hint {
        kind: "example",
        message: r#"[tests]
unit = [{flags = "crate::tests::my_scenario"}]"#,
    }],
};

static MISSING_RUNNER: TopicEntry = TopicEntry {
    slug: "missing-runner",
    summary: "A test entry references a runner that is not configured.",
    body: "## missing-runner — Unconfigured test runner

A contract test entry declares a test type (e.g. `unit`, `integration`) for
which no runner is configured in `.espectacular/config.toml`.

**Why it appears**: the test type was added to the contract but the
corresponding `[runners.<type>]` entry was not added to the config.

**How to fix**: add a `runners` entry to `.espectacular/config.toml`:

```toml
[runners]
unit = [\"cargo\", \"test\", \"--\"]
```

Or use a known adapter type (`pytest`, `cargo`, `vitest`) that is detected
automatically.",
    when: "A contract references a test type with no matching runner or adapter.",
    do_action: "Add the runner to `[runners]` in `.espectacular/config.toml`.",
    human_approval: false,
    related_topics: &[
        "no-tests-declared",
        "missing-adapter",
        "edit_code_not_scenario",
    ],
    hints: &[],
};

static MISSING_ADAPTER: TopicEntry = TopicEntry {
    slug: "missing-adapter",
    summary: "A test entry uses an adapter type but the adapter was not detected.",
    body: "## missing-adapter — Adapter not detected

A contract test entry declares an adapter-backed test type (`pytest`, `cargo`,
`vitest`) but `ah` could not detect the adapter in the repository.

**Why it appears**: the toolchain is not installed, no manifest was found, or
the adapter is not configured.

**How to fix**: install the toolchain, add the manifest (e.g. `pyproject.toml`,
`Cargo.toml`, `package.json`), or explicitly configure the runner in
`.espectacular/config.toml`.",
    when: "An adapter-backed test type is declared but the adapter is not present.",
    do_action: "Install the toolchain or configure the adapter explicitly.",
    human_approval: false,
    related_topics: &[
        "missing-runner",
        "adapter-pytest",
        "adapter-cargo",
        "adapter-vitest",
    ],
    hints: &[],
};

static MALFORMED_CONTRACT: TopicEntry = TopicEntry {
    slug: "malformed-contract",
    summary: "A contract .toml file fails structural validation.",
    body: "## malformed-contract — Invalid contract file

A contract `.toml` file could not be parsed or failed validation (missing
required fields, invalid field types, constraint violations).

**Why it appears**: the file was hand-edited and a required field was removed
or given an invalid value. The `message` field in the finding contains the
specific error (e.g. `missing field 'id'` or `unknown variant 'bad-status'`).

**How to fix**: read the `message` field to identify which field is wrong,
then open the `.toml` and fix it. Required fields are: `id`, `description`,
`archetype`, `status`, `superseded_by` (empty string when active), and
`authored_with`. The `[tests]` section is optional — a missing tests section
produces a `no-tests-declared` finding instead.",
    when: "A contract .toml is malformed or a required field is missing/invalid.",
    do_action: "Read the message field for the specific error, then fix the .toml.",
    human_approval: false,
    related_topics: &["id-mismatch", "invalid-status", "review_and_apply"],
    hints: &[],
};

static MISSING_REPLACEMENT: TopicEntry = TopicEntry {
    slug: "missing-replacement",
    summary: "A superseded contract's `superseded_by` target does not exist.",
    body: "## missing-replacement — Supersession target missing

A contract has `status = \"superseded\"` and names a replacement via
`superseded_by`, but no contract with that id exists.

**Why it appears**: the replacement contract was not created, was renamed, or
was deleted.

**How to fix**: create the replacement contract via `ah scenario new`, or
update `superseded_by` to point to the correct existing contract id.",
    when: "A superseded contract's replacement contract does not exist.",
    do_action: "Create the replacement contract or correct the `superseded_by` value.",
    human_approval: false,
    related_topics: &[
        "run_ah_scenario_supersede",
        "supersession",
        "review_and_apply",
    ],
    hints: &[],
};

static OVERLAY_CONFLICT: TopicEntry = TopicEntry {
    slug: "overlay-conflict",
    summary: "Two change overlays modify the same scenario simultaneously.",
    body: "## overlay-conflict — Simultaneous change conflict

Two change overlays (passed via `--changes`) both modify the same scenario.
Overlays cannot be merged automatically.

**Why it appears**: two in-flight changes both add or modify the same spec
scenario, creating a conflict that must be resolved manually.

**How to fix**: apply one change at a time, or merge the conflicting changes
into a single overlay before running `ah check`.",
    when: "Two `--changes` overlays modify the same scenario.",
    do_action: "Apply one change at a time or merge the conflicting overlays.",
    human_approval: true,
    related_topics: &["review_and_apply", "workflow"],
    hints: &[],
};

static TEST_FAILING: TopicEntry = TopicEntry {
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

static NO_TESTS_RAN: TopicEntry = TopicEntry {
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

static RECOMMENDATION: TopicEntry = TopicEntry {
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

static UNKNOWN_ACTION: TopicEntry = TopicEntry {
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

static QUALITY_MUTATION: TopicEntry = TopicEntry {
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

static QUALITY_PROPERTY: TopicEntry = TopicEntry {
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

static QUALITY_SNAPSHOT: TopicEntry = TopicEntry {
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

// ── Suggested action topics ───────────────────────────────────────────────────

static RUN_AH_INIT: TopicEntry = TopicEntry {
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

static RUN_AH_SCENARIO_NEW: TopicEntry = TopicEntry {
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

static RUN_AH_SCENARIO_SUPERSEDE: TopicEntry = TopicEntry {
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

static EDIT_CODE_NOT_SCENARIO: TopicEntry = TopicEntry {
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

static ENABLE_CAPABILITY: TopicEntry = TopicEntry {
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

static REVIEW_AND_APPLY: TopicEntry = TopicEntry {
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

static HUMAN_REVIEW_REQUIRED: TopicEntry = TopicEntry {
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

// ── General topics ────────────────────────────────────────────────────────────

static WORKFLOW: TopicEntry = TopicEntry {
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

static SUPERSESSION: TopicEntry = TopicEntry {
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

static ARCHETYPES: TopicEntry = TopicEntry {
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

static PROGRESSIVE_ENABLEMENT: TopicEntry = TopicEntry {
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

// ── Adapter topics ────────────────────────────────────────────────────────────

static ADAPTER_PYTEST: TopicEntry = TopicEntry {
    slug: "adapter-pytest",
    summary: "Python pytest adapter: detection, configuration, and test entry format.",
    body: "## adapter-pytest — Python pytest adapter

The pytest adapter runs Python tests via `pytest`. It is detected automatically
when any of the following are present (in priority order):

1. `runners.pytest` configured in `.espectacular/config.toml`
2. `pyproject.toml` with `[tool.pytest.ini_options]`
3. `pytest` executable on `PATH`
4. A `.py` file with `import pytest`

**Test entry format** (in contract `.toml`):
```toml
[tests]
pytest = [{flags = \"tests/test_module.py::TestClass::test_method\"}]
```

**Configuring explicitly**:
```toml
[runners]
pytest = [\"python\", \"-m\", \"pytest\"]
```

**Failure normalization**: exit code 1 → `test-failing`; exit code 2–5 →
`test-failing` with stderr captured; `ImportError` in stdout → `test-failing`
with import path hint.",
    when: "You are configuring or debugging the pytest adapter.",
    do_action: "Check detection order and add `runners.pytest` to config if needed.",
    human_approval: false,
    related_topics: &[
        "missing-adapter",
        "missing-runner",
        "adapter-cargo",
        "adapter-vitest",
    ],
    hints: &[Hint {
        kind: "tip",
        message: "Use `pyproject.toml` with `[tool.pytest.ini_options]` for automatic detection.",
    }],
};

static ADAPTER_CARGO: TopicEntry = TopicEntry {
    slug: "adapter-cargo",
    summary: "Rust cargo adapter: detection, configuration, and test entry format.",
    body: "## adapter-cargo — Rust cargo adapter

The cargo adapter runs Rust tests via `cargo test`. It is detected
automatically when `Cargo.toml` is present.

**Test entry format** (in contract `.toml`):
```toml
[tests]
cargo = [{flags = \"crate::module::tests::test_name\"}]
```

**Configuring explicitly**:
```toml
[runners]
cargo = [\"cargo\", \"test\", \"--\"]
```

**Failure normalization**: non-zero exit → `test-failing`; build errors are
classified as `test-failing` with the compiler message captured; test panics
include the test name and location.",
    when: "You are configuring or debugging the cargo adapter.",
    do_action: "Ensure `Cargo.toml` is present; add `runners.cargo` to config if needed.",
    human_approval: false,
    related_topics: &[
        "missing-adapter",
        "missing-runner",
        "adapter-pytest",
        "adapter-vitest",
    ],
    hints: &[],
};

static ADAPTER_VITEST: TopicEntry = TopicEntry {
    slug: "adapter-vitest",
    summary: "TypeScript vitest adapter: detection, configuration, and test entry format.",
    body: "## adapter-vitest — TypeScript vitest adapter

The vitest adapter runs TypeScript/JavaScript tests via `vitest`. It is
detected automatically when `package.json` contains a `vitest` dependency.

**Test entry format** (in contract `.toml`):
```toml
[tests]
vitest = [{flags = \"src/module.test.ts\"}]
```

**Configuring explicitly**:
```toml
[runners]
vitest = [\"npx\", \"vitest\", \"run\"]
```

**Failure normalization**: non-zero exit → `test-failing`; transform errors
(TypeScript compilation) are classified as `test-failing` with the error
location; import resolution failures include the unresolved module path.",
    when: "You are configuring or debugging the vitest adapter.",
    do_action: "Ensure `package.json` has a vitest dependency; configure runner if needed.",
    human_approval: false,
    related_topics: &[
        "missing-adapter",
        "missing-runner",
        "adapter-pytest",
        "adapter-cargo",
    ],
    hints: &[],
};

#[cfg(test)]
mod tests {
    use super::*;

    // 9.3 — every FindingKind variant has a topic body
    #[test]
    fn every_finding_kind_has_a_topic() {
        for kind in ALL_FINDING_KINDS {
            let entry = finding_kind_entry(*kind);
            assert!(
                !entry.slug.is_empty(),
                "FindingKind {:?} has empty slug",
                kind
            );
            assert!(
                !entry.body.is_empty(),
                "FindingKind {:?} has empty body",
                kind
            );
        }
    }

    // 9.3 — every SuggestedAction variant has a topic body
    #[test]
    fn every_suggested_action_has_a_topic() {
        for action in ALL_SUGGESTED_ACTIONS {
            let entry = suggested_action_entry(*action);
            assert!(
                !entry.slug.is_empty(),
                "SuggestedAction {:?} has empty slug",
                action
            );
            assert!(
                !entry.body.is_empty(),
                "SuggestedAction {:?} has empty body",
                action
            );
        }
    }

    // 9.3 — quality finding kinds present
    #[test]
    fn quality_finding_kinds_present() {
        assert!(lookup("quality-mutation").is_some());
        assert!(lookup("quality-property").is_some());
        assert!(lookup("quality-snapshot").is_some());
    }

    // 9.5 — general topics present
    #[test]
    fn general_topics_present() {
        assert!(lookup("workflow").is_some());
        assert!(lookup("supersession").is_some());
        assert!(lookup("archetypes").is_some());
        assert!(lookup("progressive-enablement").is_some());
    }

    // 9.5 — general topic bodies are non-empty
    #[test]
    fn general_topic_bodies_non_empty() {
        for slug in &[
            "workflow",
            "supersession",
            "archetypes",
            "progressive-enablement",
        ] {
            let entry = lookup(slug).unwrap_or_else(|| panic!("missing general topic: {slug}"));
            assert!(
                !entry.body.is_empty(),
                "general topic {slug} has empty body"
            );
        }
    }

    // 9.7 — --json fields present on all topics
    #[test]
    fn json_fields_present_on_all_topics() {
        for entry in all_topics() {
            let output = TopicJson {
                topic: entry.slug,
                summary: entry.summary,
                when: entry.when,
                do_action: entry.do_action,
                human_approval: entry.human_approval,
                related_topics: entry.related_topics,
                hints: entry
                    .hints
                    .iter()
                    .map(|h| HintJson {
                        kind: h.kind,
                        message: h.message,
                    })
                    .collect(),
            };
            let json = serde_json::to_string(&output).expect("serialization failed");
            assert!(json.contains("\"topic\""));
            assert!(json.contains("\"summary\""));
            assert!(json.contains("\"when\""));
            assert!(json.contains("\"do\""));
            assert!(json.contains("\"hints\""));
        }
    }

    // 9.9 — --list returns stable sorted output
    #[test]
    fn list_is_sorted() {
        let topics = all_topics();
        let slugs: Vec<&str> = topics.iter().map(|t| t.slug).collect();
        let mut sorted = slugs.clone();
        sorted.sort_unstable();
        assert_eq!(slugs, sorted, "all_topics() is not sorted");
    }

    // 9.9 — unknown topic returns None
    #[test]
    fn unknown_topic_returns_none() {
        assert!(lookup("totally-unknown-xyzzy").is_none());
    }

    // 9.9 — did_you_mean returns suggestions for partial match
    #[test]
    fn did_you_mean_returns_suggestions() {
        let suggestions = did_you_mean("test");
        assert!(!suggestions.is_empty(), "expected suggestions for 'test'");
        assert!(suggestions.contains(&"test-failing"));
    }

    // 9.11 — adapter topics present
    #[test]
    fn adapter_topics_present() {
        assert!(lookup("adapter-pytest").is_some());
        assert!(lookup("adapter-cargo").is_some());
        assert!(lookup("adapter-vitest").is_some());
    }

    // 9.11 — adapter topic bodies are non-empty
    #[test]
    fn adapter_topic_bodies_non_empty() {
        for slug in &["adapter-pytest", "adapter-cargo", "adapter-vitest"] {
            let entry = lookup(slug).unwrap_or_else(|| panic!("missing adapter topic: {slug}"));
            assert!(!entry.body.is_empty());
        }
    }

    // 9.13 — no duplicate slugs in registry
    #[test]
    fn no_duplicate_slugs() {
        let topics = all_topics();
        let mut seen = std::collections::HashSet::new();
        for entry in &topics {
            assert!(
                seen.insert(entry.slug),
                "duplicate topic slug: {}",
                entry.slug
            );
        }
    }

    // 9.13 — topic count matches expected
    #[test]
    fn topic_count_is_complete() {
        let topics = all_topics();
        // 18 finding kinds + 7 suggested actions + 4 general + 3 adapter = 32
        assert_eq!(
            topics.len(),
            32,
            "expected 32 topics (18 finding + 7 action + 4 general + 3 adapter), got {}",
            topics.len()
        );
    }

    // hint shape
    #[test]
    fn hints_have_kind_and_message() {
        for entry in all_topics() {
            for hint in entry.hints {
                assert!(!hint.kind.is_empty());
                assert!(!hint.message.is_empty());
            }
        }
    }
}
