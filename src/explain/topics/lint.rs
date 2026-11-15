//! Spec-linter finding topics (`ah lint`, espectacular-eia task 6.1).
//!
//! Responsibilities:
//! - one topic per `LintCheckKind`: what the check flags and how to fix it
//! - advisory framing — every lint finding is warning severity and never gates
//!
//! Rationale: each check ships with its own remediation prose, and the exhaustive match in `kinds` guarantees no check kind is left undocumented

use super::super::model::{Hint, TopicEntry};

pub(crate) static LINT_VAGUE_QUALIFIER: TopicEntry = TopicEntry {
    slug: "vague-qualifier",
    summary: "A qualitative term (fast, scalable, user-friendly, …) carries no numeric bound.",
    body: "## vague-qualifier — Unbound qualitative term

A requirement body or scenario step uses a qualitative term without an
adjacent numeric measurement, so any implementation can claim compliance.

**Why it matters**: \"fast\" has no test; \"p95 < 200 ms\" does. Unbound
qualifiers let AI agents implement correct-by-gate but wrong-by-intent
behavior.

**How to fix**: bind the term to a measurement.

```
The system SHALL respond fast (p95 < 200 ms)
```",
    when: "A qualifier word appears without a digit or unit on the same line.",
    do_action: "Add a measurable response condition (e.g. within 200 ms) next to the qualifier.",
    human_approval: false,
    related_topics: &["imperative-step", "conjunctive-bloat", "lint"],
    hints: &[Hint {
        kind: "tip",
        message: "Only the seven known qualifier terms are matched: fast, scalable, user-friendly, easy, simple, intuitive.",
    }],
};

pub(crate) static LINT_IMPERATIVE_STEP: TopicEntry = TopicEntry {
    slug: "imperative-step",
    summary: "A WHEN/THEN step describes UI mechanics instead of business intent.",
    body: "## imperative-step — UI mechanics in a scenario step

A WHEN/THEN/AND step references presentation mechanics (clicks, fills,
navigates to a URL, CSS selectors, buttons, modals), coupling the spec to
the implementation layer.

**Why it matters**: the contract archetype owns layering. A PF scenario
describes user-visible outcomes; UI mechanics belong in the test
implementation, not the spec.

**How to fix**: rephrase the step to describe the business action.

```
- **WHEN** the user submits the form     (business intent)
- **WHEN** the user clicks the Submit button   (UI mechanic — flagged)
```",
    when: "A step line contains a UI-mechanic marker (click, navigates to, fills, button, modal, css, selector, hover, scroll).",
    do_action: "Rephrase to describe the business action (e.g. submits the form).",
    human_approval: false,
    related_topics: &["entangled-spec", "vague-qualifier", "lint"],
    hints: &[],
};

pub(crate) static LINT_CONJUNCTIVE_BLOAT: TopicEntry = TopicEntry {
    slug: "conjunctive-bloat",
    summary: "A scenario chains more AND steps than the configured maximum.",
    body: "## conjunctive-bloat — Over-chained scenario

One scenario chains more AND-linked steps than the configured maximum
(default 5), which usually means it tests several behaviors at once.

**Why it matters**: a multi-behavior scenario fails for any one of them and
hides which behavior broke. Focused scenarios localize failures.

**How to fix**: split the scenario into focused single-behavior scenarios.

Configure the threshold:

```toml
# .espectacular/config.toml
[lint]
max_and_steps = 3
```",
    when: "A scenario has more AND steps than `[lint] max_and_steps`.",
    do_action: "Split the scenario into focused single-behavior scenarios.",
    human_approval: false,
    related_topics: &["missing-negative-scenario", "lint"],
    hints: &[],
};

pub(crate) static LINT_MISSING_NEGATIVE_SCENARIO: TopicEntry = TopicEntry {
    slug: "missing-negative-scenario",
    summary: "A requirement has no scenario exercising an error, rejection, or boundary.",
    body: "## missing-negative-scenario — Happy-path-only requirement

Every scenario under the requirement describes a successful outcome; none
exercises rejection, an error condition, or a boundary violation.

**Why it matters**: incomplete behavioral specification — the failure mode
is the part agents invent freely without spec guidance.

**How to fix**: add a scenario for the corresponding failure mode.

```
#### Scenario: Rejected login
- **WHEN** the user submits invalid credentials
- **THEN** the command exits non-zero and no session is created
```",
    when: "A requirement's scenarios contain no negative markers (exits non-zero, reject, error, invalid, fail, denied, violation, exceed).",
    do_action: "Add a scenario for the corresponding failure mode.",
    human_approval: false,
    related_topics: &["conjunctive-bloat", "vague-qualifier", "lint"],
    hints: &[],
};

pub(crate) static LINT_MISSING_NON_GOALS: TopicEntry = TopicEntry {
    slug: "missing-non-goals",
    summary: "A spec file has no Non-Goals / Out-of-Scope section.",
    body: "## missing-non-goals — Missing Non-Goals section

The spec capability file declares no `## Non-Goals` (or `## Out of Scope`)
section.

**Why it matters**: without explicit exclusions, scope creep during
AI-assisted implementation is silent — anything not forbidden gets built.

**How to fix**: add a Non-Goals section listing explicit exclusions.

```
## Non-Goals

- OAuth and federated identity flows
- Password recovery
```",
    when: "A spec.md has no heading matching Non-Goals / Non Goals / non-goals / Out of Scope.",
    do_action: "Add a `## Non-Goals` section listing explicit exclusions.",
    human_approval: false,
    related_topics: &["unresolved-ambiguity", "lint"],
    hints: &[],
};

pub(crate) static LINT_UNRESOLVED_AMBIGUITY: TopicEntry = TopicEntry {
    slug: "unresolved-ambiguity",
    summary: "A `[NEEDS CLARIFICATION` marker was left in the spec.",
    body: "## unresolved-ambiguity — Deferred decision not resolved

A requirement body or scenario step still contains a `[NEEDS CLARIFICATION:`
marker — an authoring-time decision deferred but never resolved.

**Why it matters**: the marker instructs implementers to guess. Whatever the
agent picks becomes the de-facto spec without review.

**How to fix**: resolve the deferred decision, write the chosen behavior
into the spec, and remove the marker.

```
[NEEDS CLARIFICATION: which auth provider?]  →  the session uses OIDC via the platform IdP
```",
    when: "The text `[NEEDS CLARIFICATION` appears in a requirement body or scenario step.",
    do_action: "Resolve the deferred decision and remove the marker.",
    human_approval: false,
    related_topics: &["missing-non-goals", "review_and_apply", "lint"],
    hints: &[],
};

pub(crate) static LINT_ENTANGLED_SPEC: TopicEntry = TopicEntry {
    slug: "entangled-spec",
    summary: "A PF/SA scenario references presentation-layer primitives.",
    body: "## entangled-spec — Layer-entangled scenario

The scenario's contract archetype is `PF` (presentation-free) or `SA`
(state/assertion), but its text references presentation primitives
(buttons, modals, banners, colors, CSS selectors, hover, scroll). The
archetype declares the scenario belongs to a layer where UI mechanics must
not appear.

**Why it matters**: the archetype is the layering contract. Entangled
scenarios make the compiled contract test presentation details while
claiming domain behavior.

**How to fix**: specify the domain event or state instead of the visual
primitive.

```
- **THEN** the delivery receipt is recorded      (domain state)
- **THEN** a green notification banner is displayed   (visual — flagged)
```

Domain-legitimate language is excluded by the matcher (e.g. \"event
routing\"), and `BP` scenarios are never flagged — transport mechanics are
legitimate at a boundary seam.",
    when: "A PF/SA scenario line matches a presentation primitive without a domain-legitimate exclusion.",
    do_action: "Specify the domain event or state instead of the visual primitive.",
    human_approval: false,
    related_topics: &["imperative-step", "archetypes", "lint"],
    hints: &[Hint {
        kind: "tip",
        message: "The check is archetype-gated: it only fires for scenarios whose contract declares PF or SA.",
    }],
};

pub(crate) static LINT_SPK_FRONTMATTER_MISMATCH: TopicEntry = TopicEntry {
    slug: "spk-frontmatter-mismatch",
    summary: "A file carries specodelic frontmatter the lint bridge would otherwise skip.",
    body: "## spk-frontmatter-mismatch — Skipped specodelic frontmatter\n\nThe file carries `id: spec` specodelic frontmatter, but the lint bridge\nwould not have picked it up under its file-selection rules (task 5.1 of\nderive-contracts-from-specodelic closes the bare-`continue` gap). Content\nthat declares itself specodelic must always reach `spk lint`.\n\n**Why it matters**: a silently skipped dual-format file never gets\nspecodelic validation, so its Properties tables can rot unchecked.\n\n**How to fix**: make sure the file lives under the configured specs root\nand is named `spec.md`, or drop the `id: spec` frontmatter if the file is\nnot meant to be specodelic.",
    when: "A file declares specodelic frontmatter but is excluded from the spk lint relay.",
    do_action: "Move the file into the specs root (or remove the frontmatter if it is not specodelic).",
    human_approval: false,
    related_topics: &["lint", "contract-stale"],
    hints: &[],
};

pub(crate) static LINT_VERIFIES_DANGLING: TopicEntry = TopicEntry {
    slug: "verifies-dangling",
    summary: "A scenario's VERIFIES bullet links to a property id that does not exist.",
    body: "## verifies-dangling — Dangling VERIFIES link\n\nThe scenario carries a `- **VERIFIES** [[spec.P-x]]` bullet, but no\nProperties row with id `P-x` exists in the file. The link is the coverage\nconvention for property-derived contracts (design D1); a dangling link\ncovers nothing.\n\n**Why it matters**: the scenario appears property-covered while it is not\n— the drift signal is silently disarmed.\n\n**How to fix**: correct the property id in the bullet, or add the missing\nProperties row, or remove the bullet and give the scenario its own\ncontract .toml.",
    when: "A VERIFIES bullet cites a property id absent from the file's Properties table.",
    do_action: "Fix the property id in the VERIFIES bullet (or add the missing Properties row).",
    human_approval: false,
    related_topics: &["lint", "no-tests-declared", "contract-stale"],
    hints: &[],
};
pub(crate) static LINT_SCENARIO_UNLINKED: TopicEntry = TopicEntry {
    slug: "scenario-unlinked",
    summary: "A scenario in a dual-format file with Properties rows carries no VERIFIES link.",
    body: "## scenario-unlinked — Scenario without a VERIFIES link\n\nThe file declares Properties rows, but this scenario carries no\n`- **VERIFIES** [[spec.P-id]]` bullet. Under design D1 it falls back to\nits own contract toml — legitimate, but it is not covered by a\nproperty-derived contract.\n\n**Why it matters**: the coverage convention is how specodelic properties\nbecome executable contracts; a scenario outside the convention needs\nmanual contract maintenance that sync cannot refresh.\n\n**How to fix**: add a `- **VERIFIES** [[spec.P-id]]` bullet linking the\nscenario to the property it verifies (then run `ah sync`), or keep the\nscenario's own contract if the fallback is deliberate.",
    when: "A dual-format file with Properties rows contains a scenario without a VERIFIES bullet.",
    do_action: "Add a `- **VERIFIES** [[spec.P-id]]` bullet to the scenario, then run `ah sync` to derive the property's contract.",
    human_approval: false,
    related_topics: &["verifies-dangling", "run_ah_sync", "property-untraced"],
    hints: &[],
};

pub(crate) static LINT_PROPERTY_UNTRACED: TopicEntry = TopicEntry {
    slug: "property-untraced",
    summary: "A Properties row is cited by no scenario's VERIFIES bullet.",
    body: "## property-untraced — Property no scenario verifies\n\nThe Properties row exists, but no scenario in the file carries a\n`- **VERIFIES** [[spec.P-id]]` bullet naming it. The property is\nuntestable-as-written: its derived contract declares no covering\nscenario.\n\n**Why it matters**: an untraced property's contract cannot map to real\nbehavior — the specodelic row is dead weight unless a scenario adopts it.\n\n**How to fix**: add a VERIFIES bullet to a covering scenario, or\nsupersede/remove the row if it is no longer testable.",
    when: "A dual-format file with Properties rows contains a property no scenario cites.",
    do_action: "Add a `- **VERIFIES** [[spec.P-id]]` bullet to a covering scenario, or supersede the row.",
    human_approval: false,
    related_topics: &["verifies-dangling", "scenario-unlinked"],
    hints: &[],
};

pub(crate) static CONTRACT_STALE: TopicEntry = TopicEntry {
    slug: "contract-stale",
    summary: "A derived contract's derived_from hash no longer matches its specodelic source row.",
    body: "## contract-stale — Derived contract drifted from its source row\n\nA contract carrying `derived_from = \"<property-id>@<hash>\"` no longer\nmatches the canonical serialization of its specodelic Properties row. The\nrow was edited after the contract was derived (or the property disappeared\nfrom the table), so the contract's derived fields (id, description,\narchetype, falsifiability_class, derived_from) are outdated.\n\n**Why it matters**: the derived contract is the machine-checked test\nintent for the property; letting it drift defeats the point of deriving\nit.\n\n**How to fix**: run `ah sync` to refresh the derived fields. Human-owned\nfields (tests, status, superseded_by) are never overwritten. If the\nproperty was deleted deliberately, delete the contract too.",
    when: "A contract's derived_from hash differs from its source property row.",
    do_action: "Run `ah sync` to refresh derived fields from the specodelic row.",
    human_approval: false,
    related_topics: &[
        "run_ah_sync",
        "verifies-dangling",
        "review_and_apply",
    ],
    hints: &[Hint {
        kind: "tip",
        message: "ah sync --check reports drift without writing anything — useful in CI.",
    }],
};

pub(crate) static RUN_AH_SYNC: TopicEntry = TopicEntry {
    slug: "run_ah_sync",
    summary: "Run `ah sync` to create or refresh property-derived contracts.",
    body: "## run_ah_sync — Refresh derived contracts\n\nRun `ah sync` in the repository root. Sync derives one contract per\nspecodelic Properties row from `spk parse --json`, refreshing only the\nderived fields (id, description, archetype, falsifiability_class,\nderived_from) and never touching human-owned fields (tests, status,\nsuperseded_by).\n\n```\nah sync          # create or refresh derived contracts\nah sync --check  # CI mode: report drift, write nothing\n```\n\n**When to use**: after editing a Properties row, after adding VERIFIES\nlinks, or when a contract-stale finding is reported.",
    when: "Derived contracts are missing or drifted from their specodelic source rows.",
    do_action: "Run `ah sync` (or `ah sync --check` in CI).",
    human_approval: false,
    related_topics: &["contract-stale", "workflow"],
    hints: &[],
};

pub(crate) static LINT: TopicEntry = TopicEntry {
    slug: "lint",
    summary: "`ah lint` statically analyzes spec files for authoring-quality findings.",
    body: "## lint — The spec quality linter

`ah lint` statically analyzes OpenSpec scenario files for authoring defects
that cause AI agents to implement correct-by-gate but wrong-by-intent
behavior.

**Findings are advisory**: every lint finding is warning severity in v1 and
never changes the exit code. `ah lint` never modifies files.

```
ah lint                      # lint deployed specs
ah lint --changes <id>       # also lint a change overlay
ah lint --check <kind>       # run a single check category
ah lint --json               # machine-readable envelope
```

Check kinds (each has its own topic):

- `vague-qualifier` — unbound qualitative terms
- `imperative-step` — UI mechanics in scenario steps
- `conjunctive-bloat` — over-chained AND steps
- `missing-negative-scenario` — happy-path-only requirements
- `missing-non-goals` — no Non-Goals / Out-of-Scope section
- `unresolved-ambiguity` — leftover [NEEDS CLARIFICATION markers
- `entangled-spec` — presentation primitives in PF/SA scenarios

Dual-format spec files (with `id: spec` frontmatter) additionally relay
specodelic findings as `spk.<rule_id>` findings via the spk bridge. When
the spk binary is missing, one `spk-unavailable` advisory finding is
emitted instead.

Configure the AND-step threshold:

```toml
# .espectacular/config.toml
[lint]
max_and_steps = 5
```",
    when: "Reviewing spec quality before implementation or during review.",
    do_action: "Run `ah lint` and address findings by editing the spec files.",
    human_approval: false,
    related_topics: &[
        "vague-qualifier",
        "imperative-step",
        "conjunctive-bloat",
        "missing-negative-scenario",
        "missing-non-goals",
        "unresolved-ambiguity",
        "entangled-spec",
    ],
    hints: &[],
};
