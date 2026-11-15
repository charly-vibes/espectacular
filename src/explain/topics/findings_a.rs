//! Structural `ah check` findings — spec and contract correspondence.
//!
//! Responsibilities:
//! - missing, orphaned, colliding, or malformed contract files
//! - status, falsifiability, and supersession field violations
//! - test-declaration gaps and overlay conflicts
//!
//! Rationale: these findings describe the spec/contract relationship, so their remediation is mostly file surgery the reader can name in one place

use super::super::model::{Hint, TopicEntry};

pub(crate) static NO_TOML: TopicEntry = TopicEntry {
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

pub(crate) static ORPHAN_TOML: TopicEntry = TopicEntry {
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

pub(crate) static SLUG_COLLISION: TopicEntry = TopicEntry {
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

pub(crate) static ID_MISMATCH: TopicEntry = TopicEntry {
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

pub(crate) static INVALID_STATUS: TopicEntry = TopicEntry {
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

pub(crate) static INVALID_FALSIFIABILITY_CLASS: TopicEntry = TopicEntry {
    slug: "invalid-falsifiability-class",
    summary: "A contract has a `falsifiability_class` that is not `safety` or `liveness`.",
    body: "## invalid-falsifiability-class — Unknown falsifiability class

The optional `falsifiability_class` field in a contract `.toml` holds an
unrecognised value. Valid values are `safety` (bad states never occur — one
deterministic test can falsify the claim) and `liveness` (good states
eventually occur — only falsifiable under bounded execution).

**Why it appears**: a typo, or a value borrowed from another taxonomy.

**How to fix**: set `falsifiability_class` to `safety` or `liveness`, or
remove the field entirely — an absent field is always valid.",
    when: "A contract's `falsifiability_class` field is neither `safety` nor `liveness`.",
    do_action: "Set `falsifiability_class` to `safety` or `liveness`, or delete the field.",
    human_approval: false,
    related_topics: &["invalid-status", "review_and_apply"],
    hints: &[],
};

pub(crate) static MISSING_LIVENESS_TIMEOUT: TopicEntry = TopicEntry {
    slug: "missing-liveness-timeout",
    summary: "A `liveness` contract declares no `timeout_seconds` on any test entry.",
    body: "## missing-liveness-timeout — Unbounded liveness claim

A contract tagged `falsifiability_class = \"liveness\"` declares a liveness
claim: good states eventually occur. Such a claim is only falsifiable under
bounded execution — without a timeout, a hanging or looping test never
produces a falsifying result, so the scenario reads as verified while it is
merely un-executed. This is a warning: it does not fail the gate.

**Why it appears**: the contract was tagged liveness but its test entries
rely on the runner's default timeout behavior (or none).

**How to fix**: declare `timeout_seconds` on at least one test entry in the
contract. That single explicit bound is enough to suppress the warning.",
    when: "A `liveness` contract's test entries all omit `timeout_seconds`.",
    do_action: "Add `timeout_seconds` to at least one test entry in the contract .toml.",
    human_approval: false,
    related_topics: &["invalid-falsifiability-class", "review_and_apply"],
    hints: &[Hint {
        kind: "example",
        message: r#"[[tests.unit]]
flags = "crate::tests::my_scenario"
timeout_seconds = 30"#,
    }],
};

pub(crate) static NO_TESTS_DECLARED: TopicEntry = TopicEntry {
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

pub(crate) static MISSING_RUNNER: TopicEntry = TopicEntry {
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

pub(crate) static MISSING_ADAPTER: TopicEntry = TopicEntry {
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

pub(crate) static MALFORMED_CONTRACT: TopicEntry = TopicEntry {
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

pub(crate) static MISSING_REPLACEMENT: TopicEntry = TopicEntry {
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

pub(crate) static OVERLAY_CONFLICT: TopicEntry = TopicEntry {
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
