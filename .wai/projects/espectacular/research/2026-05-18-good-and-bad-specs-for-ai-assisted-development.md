# Good and Bad Specs for AI-Assisted Development: A Field Guide for Solo Devs and Small Teams

**Bottom line:** For solo devs and small teams using Claude Code, Cursor, Copilot, or Kiro, the spec patterns that consistently outperform "vibe coding" are short (1–5 page), behavior-oriented, technology-agnostic documents written in EARS or Given-When-Then, with explicitly numbered functional requirements (FR-001…), explicit non-goals, measurable success criteria, and edge-case sections — split into requirements/design/tasks only when complexity justifies it. The two reference templates worth copying today are **GitHub spec-kit's `spec-template.md`** and **Amazon Kiro's three-file pattern** (`requirements.md` / `design.md` / `tasks.md`); both are open, both encode the same lessons, and both are usable with any agent.

## TL;DR

- **Good specs are behavioral, numbered, and testable.** They use EARS ("WHEN X, THE SYSTEM SHALL Y") or Given-When-Then acceptance criteria, mark unknowns with `[NEEDS CLARIFICATION]`, list non-goals, and define success in measurable terms. They are tech-agnostic in the requirements layer and push implementation detail into a separate plan/design doc.
- **Bad specs leak implementation into requirements, omit edge cases, use vague qualifiers ("fast", "user-friendly"), and drift from code.** AI agents will faithfully implement the wrong thing if you let them; they will also pad a 3-point story into 16 acceptance criteria if you let *them* write the spec unchecked.
- **For solo devs, prefer one short spec; for teams of 2–8, adopt spec-kit or Kiro's 3-file split.** Treat the spec as a *living, version-controlled artifact* that agents read on every session — not a Confluence page you write once.

## Key Findings

1. **Two open templates now dominate the AI-era SDD conversation:** GitHub `spec-kit` (released publicly Sept 2025; CLI scaffolds Markdown templates and slash-commands like `/speckit.specify`, `/speckit.plan`, `/speckit.tasks`, `/speckit.analyze` into any of ~30 agents) and Amazon **Kiro** (the standalone IDE, replacing Amazon Q Developer, that codifies a `requirements.md`/`design.md`/`tasks.md` workflow with EARS notation baked in).
2. **EARS is the dominant requirements grammar for AI-assisted specs.** Originated by Alistair Mavin, Philip Wilkinson, Adrian Harwood, and Mark Novak at Rolls-Royce PLC in their paper "EARS (Easy Approach to Requirements Syntax)" at the 17th IEEE International Requirements Engineering Conference (RE'09), Atlanta, 2009. Per alistairmavin.com, EARS has since been adopted by Airbus, Bosch, Dyson, Honeywell, Intel, NASA, and Siemens. Its five patterns (Ubiquitous, Event-driven, State-driven, Unwanted behavior, Optional) map cleanly onto AI-generatable test types and are explicitly the format Kiro generates.
3. **Sean Grove's "The New Code" (AI Engineer World's Fair 2025) is the conceptual anchor.** Grove, a member of OpenAI's technical staff, argues that code is a "lossy projection" of intent. His exemplar is the OpenAI Model Spec — a Markdown document with stable clause IDs (e.g., `sy73`, `#avoid_sycophancy`), per-clause challenge prompts that double as unit tests, and a "chain of command" priority structure.
4. **The leading critique** comes from Birgitta Böckeler, Distinguished Engineer and AI-assisted delivery expert at Thoughtworks, in "Understanding Spec-Driven-Development: Kiro, spec-kit, and Tessl" (martinfowler.com, 15 October 2025): SDD tools vary on whether the spec is *spec-first* (write once, then code), *spec-anchored* (kept alongside code), or *spec-as-source* (only spec is edited; code is generated and marked `// GENERATED FROM SPEC - DO NOT EDIT`, as Tessl is exploring). Most spec-kit and Kiro practice today is spec-first; spec-anchored is the durable win.
5. **"Vibe coding" failure modes are well-documented and predictable.** Teams hit a "3-month wall" of architectural drift, context loss across long chat threads, and spaghetti chat histories that nobody can audit; structured specs are the documented antidote.
6. **AI agents over-elaborate when given vague briefs.** Real-world Kiro users report a small bug producing "4 user stories with 16 acceptance criteria"; the corrective is to constrain scope explicitly and use shorter specs for small changes (Kiro now offers Quick Plan, spec-kit offers Plan-Mode review).
7. **Rubric convergence:** Bill Wake's **INVEST** (Independent, Negotiable, Valuable, Estimable, Small, Testable) for user stories and **SMART** (Specific, Measurable, Achievable, Relevant, Time-bound) for acceptance criteria — both from 2003 — remain the most cited shared evaluation framework, now bolted onto AI-specific checks (`[NEEDS CLARIFICATION]` markers, executable success criteria, agent-readable structure).

## Details

### Concrete spec examples — the structures to copy

**1. GitHub spec-kit `spec-template.md`** (single-file functional spec; ~1–3 pages):

```
## User Scenarios & Testing
### User Story 1 - <name> (Priority: P1) 🎯 MVP
  Independent Test: ...
  Acceptance Scenarios: Given … When … Then …
## Edge Cases
## Requirements (Functional)
  - FR-001: System MUST <specific capability>
  - FR-006: System MUST authenticate users via [NEEDS CLARIFICATION: SSO/OAuth/email?]
## Key Entities
## Success Criteria
  - SC-001: Users complete account creation in under 2 minutes
## Out of Scope
```

What makes this good: explicit numbering (FR-001…), an explicit ambiguity marker (`[NEEDS CLARIFICATION:…]`), priority-tagged user stories, an "Independent Test" line per story, measurable success criteria, and an Out-of-Scope section. A real example in the wild is the `viral2viral` repo (Iurii D., committed Nov 24, 2025), whose `specs/001-ugc-video-generator/spec.md` contains FR-001 through FR-039 grouped under sub-headings like "Video Management" and "Error Handling and Validation" — readable in one sitting, traceable to tasks. Representative excerpt:

> **User Story 1 - Video Upload and Analysis (Priority: P1) 🎯 MVP**
> A marketing manager uploads a successful UGC advertisement video…
> **Independent Test**: Can be fully tested by uploading a video file, receiving AI analysis results displayed on screen, and verifying the user can view and edit the analysis data.
> - **FR-001**: System MUST accept video file uploads in MP4, MOV, and AVI formats up to 100MB in size
> - **FR-002**: System MUST store uploaded source videos in cloud storage (AWS S3) with unique identifiers immediately upon successful upload

**2. Kiro three-file pattern**: `requirements.md` uses EARS; `design.md` adds architecture, sequence diagrams, components; `tasks.md` is a dependency-ordered, numbered execution plan that Kiro builds into a wave-based dependency graph. An open example from a Restaurant Tracker project (remotesynth/Kiro-Restaurant-Tracker on GitHub):

> User Story: As a user, I want to add reviews to restaurants I've visited…
> WHEN a user marks a restaurant as visited THEN the system SHALL enable review functionality.
> WHEN a user has already provided a star rating THEN the system SHALL allow them to adjust it on subsequent visits.

What makes this good: every acceptance criterion is a verifiable EARS statement (testable in isolation), the user story carries the "why," and tasks.md is mechanically derived. The same pattern handles bug specs (`bugfix.md`), where Kiro additionally documents an "unchanged behavior" section — "WHEN [condition] THEN the system SHALL CONTINUE TO [existing behavior]" — generating regression tests automatically.

**3. Sean Grove's exemplar — the OpenAI Model Spec** (Model Spec 2025/10/27, CC0). The document is structured as: Overview → Definitions → Chain of Command → Stay in Bounds → Seek the Truth Together → Do the Best Work → Use Appropriate Style. Each section tag indicates **authority level** (Root, System, Developer, User, Guideline). Every clause has a stable anchor ID (e.g., `#avoid_sycophancy`, `#do_not_lie`); Grove notes in his talk that "every clause in the model spec has an ID … using that ID, you can find another file in the repository sy73.markdown … that contains one or more challenging prompts for this exact clause." The companion files are effectively per-clause unit tests — what Grove calls making the spec "executable."

A representative rule excerpt:

> "Comply with applicable laws. The model should not promote, facilitate, or engage in illegal activity."

What makes the Model Spec the gold standard for Grove's argument: it is Markdown (human + machine readable), version-controlled with date stamps (`2024-05-08` → `2025-12-18`), multi-stakeholder-editable, has unique IDs per clause, has paired adversarial test prompts per ID, and is shipped as both a *behavior* doc and an *eval* dataset. Most SDD specs you'll write should imitate three of those five properties.

**4. RFCs and design docs as the lineage.** The **Rust RFC template** (Summary → Motivation → Guide-level explanation → Reference-level explanation → Drawbacks → Rationale and alternatives → Prior art → Unresolved questions → Future possibilities) is widely copied. **GitLab's design-doc template** is leaner: Summary, Motivation, Goals, Non-Goals, Proposal, Design and implementation details, Alternative Solutions. Stripe, GitLab, Uber, LinkedIn, Spotify, and Airbnb all run RFC/design-doc cultures, per Gergely Orosz's catalog of openly-shared templates.

### Patterns of good specs

| Element | Good practice (with attestation) |
|---|---|
| Requirements grammar | EARS or Given-When-Then, one behavior per line, with concrete units ("within 1 second," not "quickly") (RequireKit) |
| Numbering | FR-001…, SC-001… so tasks and tests can back-reference (spec-kit) |
| Tech-stack placement | Out of `requirements.md`; lives in `plan.md`/`design.md` (Kiro and spec-kit both warn against this) |
| Ambiguity | Explicit `[NEEDS CLARIFICATION: …]` tokens that block downstream `/plan` until resolved (spec-kit's `/speckit.clarify`) |
| Non-goals/Out-of-scope | Explicit section; prevents agent scope creep (GitLab design-doc template, spec-kit template) |
| Edge cases | Dedicated section enumerated up front; Kiro auto-generates these from clarification dialogs |
| Success metrics | Measurable: "p95 < 200 ms," "auth completes < 2 min" — not "fast" |
| Document split | One file for solo/trivial work; three-file split (req/design/tasks) when complexity warrants |
| Living vs static | Version-controlled in-repo (`.specify/specs/NNN-feature/` or `.kiro/specs/<feature>/`); referenced in every agent session via `#spec` mentions or constitution files |
| Constitution/AGENTS.md | Project-wide invariants (code quality, test coverage, security) live separately, not duplicated per feature (spec-kit `constitution.md`) |
| Authority/precedence | Where rules conflict, declare which wins (Model Spec's chain-of-command idea) |

### Patterns of bad specs (with cited failure modes)

- **Vague qualifiers:** "The system should validate email format" — fix: "The system shall validate email format according to RFC 5322" (RequireKit example).
- **Tech in requirements:** Kiro case study showed a spec that said "the system must use React and Node.js" — wrong layer; requirements collapse if framework changes.
- **Missing acceptance criteria:** A spec for "auth" without password reset, lockout, or concurrent-session rules → security holes and rework (Kiro published case study).
- **Over-decomposition:** A 3-point story expanded by AI into 16 acceptance criteria (Birgitta Böckeler, "Understanding Spec-Driven-Development", martinfowler.com, Oct 2025). Solution: keep specs scoped to the change.
- **Stale/drifting specs:** Code edits without spec edits is the documented #1 SDD failure mode (Isoform analysis quoted in Augment Code guide: "Updating the code is much easier than updating the spec first"). Mitigations: spec-anchored workflows (Tessl), `/speckit.analyze` cross-artifact consistency check, Kiro's `Sync Files`.
- **Implicit assumptions:** Long chat threads encode ordering or invariants that vanish on context reset; one published staging failure traced an async-refactor crash to an implicit ordering assumption lost during AI refactoring (cited in Augment Code analysis).
- **Single bloated doc that is too long to be read:** Addy Osmani's "curse of instructions" — when too many directives are piled into one prompt, model compliance with each drops; split or summarize via an extended TOC.

### EARS notation in modern AI-assisted development

The five patterns map onto generatable tests:

- **Ubiquitous:** `The system shall require passwords ≥ 12 characters.` → property test.
- **Event-driven:** `WHEN a user requests password reset, the system SHALL send a reset link within 30 seconds.` → integration test.
- **State-driven:** `WHILE offline, the system shall queue pending operations.` → state-machine test.
- **Unwanted behavior:** `IF the connection times out, then the system shall retry 3 times.` → error/edge test.
- **Optional:** `WHERE two-factor authentication is enabled, the system shall send a verification code via SMS.` → conditional test.

Kiro's `requirements.md` is EARS-native; spec-kit currently is not, though an open issue (#1356) proposes adding optional EARS sections, and the community is converging on its use because EARS-shaped sentences cause agents to emit cleaner, more isolated test cases.

### How AI changes spec writing (vs writing for humans)

Compared to a human reader, an agent benefits from:

1. **Explicit commands and paths.** GitHub's analysis of over 2,500 agent configuration files (cited by Addy Osmani, "How to Write a Good Spec for AI Agents," O'Reilly Radar, Feb 20, 2026) found the most effective specs include literal commands (`npm test`, `pytest -v`), exact paths (`src/`, `tests/`), and "Never touch" boundaries. "Never commit secrets" was the single most common helpful constraint.
2. **Section delimiters.** Markdown headings or XML-like tags (`<background>`, `<instructions>`, `<output_format>`) — Anthropic's effective-context-engineering guidance.
3. **Test-first ordering.** spec-kit's tasks template enforces contract → integration → e2e → unit test creation *before* implementation; Kiro generates tests aligned to acceptance criteria.
4. **Bounded context per task.** Break work into focused chunks (~500-token specs per task beat 3,000-token monoliths); Claude Code's Plan Mode and Kiro's per-task isolation address this.
5. **Conformance suites.** Embedding "must pass cases in `conformance/api-tests.yaml`" as success criteria — Simon Willison's "tests are the agents' superpower" pattern.

### Iterative vs upfront

The empirical sweet spot reported by practitioners is "**plan upfront, implement iteratively**." Start with a one-paragraph vision; have the agent draft a spec in read-only Plan Mode; review and edit it; lock it; then let the agent decompose into tasks and execute. Don't rewrite the spec mid-task — open a new spec session. Kiro and spec-kit both bake this in via approval gates.

### Synthesized 12-point evaluation rubric

Score each on 0–2 (0 = absent, 1 = present, 2 = excellent). A passing spec scores ≥ 16/24:

1. **Scope clarity** — Explicit goals, non-goals, MVP boundary.
2. **Behavioral grammar** — EARS or Given-When-Then; one behavior per line; concrete units.
3. **Numbering & traceability** — FR-001, SC-001 anchors that tasks/tests can reference.
4. **Tech-agnostic at requirements layer** — implementation lives in plan/design.
5. **Edge cases & error states** — dedicated section, not afterthoughts.
6. **Measurable success** — at least one observable, falsifiable metric.
7. **Ambiguity markers** — explicit `[NEEDS CLARIFICATION]` tokens; no silent assumptions.
8. **NFRs called out** — performance, security, accessibility have their own lines.
9. **Living artifact** — committed in-repo path the agent reads on every session.
10. **Independent testability** — each user story can be released and tested standalone (INVEST-Small/Testable).
11. **Authority/precedence rules** — when constraints conflict, the spec says which wins.
12. **Right size** — short enough to read in one pass; long enough to be actionable.

## Recommendations

**For a solo dev shipping a small project (web app, CLI, library, SDK):**
- Adopt **spec-kit** (`uvx --from git+https://github.com/github/spec-kit specify init`). One spec.md per feature, ~1–3 pages. Use `/speckit.clarify` before `/speckit.plan`. Read the generated `spec.md` line-by-line before approving — this is the highest-leverage 10 minutes you'll spend.
- Write a one-page `constitution.md` (test coverage minimums, lint rules, "never touch" list) once for the project.
- Skip the design.md split until a single feature exceeds ~5 user stories.

**For a 2–8-person team building production software:**
- Adopt **Kiro** if your stack is AWS-heavy and you value the IDE-native experience and EARS enforcement; otherwise adopt spec-kit (agent-agnostic, no IDE lock-in).
- Enforce the three-file split (requirements/design/tasks); make spec edits a required PR step alongside code (the "spec-anchored" discipline from Böckeler).
- Add a PR template checkbox: "Spec updated to reflect this change." Without this, drift is inevitable.
- Run `/speckit.analyze` or Kiro's cross-artifact check before every merge.

**When to *avoid* heavy SDD:** Throwaway scripts, exploratory prototypes, one-off utilities, single-function bug fixes. Use plain Claude Code or Cursor for those. The benchmark for switching to SDD is: "Will this code outlive the current chat session, or will more than one person touch it?" If yes, write a spec.

**Adoption ramp (4 weeks):** Week 1 — run spec-kit on one feature and read the output critically. Week 2 — write your project constitution; rerun a feature with `/speckit.clarify`. Week 3 — adopt EARS for acceptance criteria. Week 4 — gate PRs on spec-update-in-same-commit.

**Stop and reassess if** any of these benchmarks fail after 4 weeks: (a) >30% of generated specs require manual rewrites of >50% content (your constitution is too thin); (b) specs grow past ~8 pages routinely (you're over-specifying); (c) code diverges from spec on >25% of PRs (your spec-update gate isn't enforced).

## Caveats

- **The SDD field is < 12 months old as a named practice.** spec-kit landed publicly in September 2025; Kiro replaced Amazon Q Developer late 2025. Best practices are still emerging; expect templates and slash-commands to churn into 2026.
- **AI-generated specs need human review.** Multiple Kiro and spec-kit walkthroughs report agents inventing requirements, over-elaborating, or burying tech-stack assumptions inside functional sections. Treat the agent as a verbose junior PM, not an authority.
- **"Code is the executable truth" remains a valid counter-argument** (Olmedo, Medium, response to Grove). When production breaks at 3 a.m. you are debugging code, not Markdown. Specs reduce drift, they do not eliminate the need to read code.
- **Pricing surprises with hosted SDD tools are real.** Kiro's "Spec Tax" is literal: spec-mode requests cost $0.20/credit while vibe-mode requests cost $0.04/credit — a 5× differential built into overage pricing (per Kiro's official pricing page and corroborated by dev.to and byteiota.com analyses; The Register described the original tier structure as "a wallet-wrecking tragedy"). spec-kit (BYO agent) has no such tax.
- **Hype is real.** Sean Grove's "spec is the new code" framing has been called "the return of the waterfall model" by critics (36Kr, Medium). The defensible claim is narrower: *for AI-driven implementation, a structured, version-controlled spec measurably reduces rework and drift compared with chat-only prompting.*
- **Spec-as-source (Tessl-style, no human code editing) is experimental.** Don't bet a production codebase on it yet; spec-anchored is the safe-but-real win.

## Completion table

| Spec item | Covered |
|---|---|
| Concrete spec examples (spec-kit, Kiro, Grove/OpenAI Model Spec, RFCs) | ✓ |
| Good-spec patterns | ✓ |
| Bad-spec patterns | ✓ |
| Evaluation rubrics/checklists (INVEST/SMART + synthesized 12-point) | ✓ |
| EARS notation with named originators and adopters | ✓ |
| Small-team/AI-fit patterns | ✓ |
| Actual text excerpts (Model Spec, Kiro, viral2viral) | ✓ |
| Source attribution inline | ✓ |
| Range of software types (web, CLI, library, AWS infra, video AI) | ✓ |
| Sized for small teams | ✓ |
