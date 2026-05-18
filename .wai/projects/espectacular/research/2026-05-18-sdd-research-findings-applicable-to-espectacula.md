# SDD Research Findings: Applicable to espectacular

Sources: spec-practices-chatgpt.md, specs-practices-claude.md, specs-practices-gemini.md

## Priority Sequence

Implementation dependencies order the sections below. Detection (Section 1) must exist before generation (Section 2). Generation must exist before drift detection (Section 3). Reporting (Section 4) is last. Quality checks (Section 5) and tooling patterns (Section 6) are cross-cutting and can be addressed in parallel.

1. **Assertion generation** — recognize spec patterns, emit typed assertion stubs
2. **Drift detection** — surface divergence between spec and implementation
3. **Quality checks** — lint specs for atomicity, measurability, completeness
4. **Reporting** — conformance matrix, reproduction cases

## Research Non-Goals

The following patterns appear in the research but are explicitly out of scope for espectacular:

- SDK generation and transform layers (Stainless-style) — espectacular is an assertion/drift tool, not a code generator
- Production Readiness Reviews (KEP PRR) — enforcement of process gates is a workflow concern, not a spec analysis concern
- Full BDD test orchestration (Cucumber/Gherkin runners) — espectacular emits stubs, not a test framework
- API schema linting (Spectral, OpenAPI) — espectacular operates on openspec proposals, not OpenAPI documents

## 1. Assertion Generation

An **archetype** is a typed assertion stub emitted by espectacular that corresponds to a specific spec pattern. Each archetype carries a stable ID traceable to the source spec clause, a test tier (unit / integration / e2e), and a behavioral shape (observable outcome).

**EARS grammar** (Easy Approach to Requirements Syntax, Mavin et al., RE'09) maps one-to-one to archetype types:

| EARS pattern | Form | Archetype tier |
|---|---|---|
| Ubiquitous | `The system shall…` | Property test |
| Event-driven | `WHEN X, THE SYSTEM SHALL Y` | Integration test |
| State-driven | `WHILE [state]…` | State-machine test |
| Unwanted behavior | `IF [condition], THEN [fallback]` | Error/edge test |
| Optional | `WHERE [feature enabled]…` | Conditional test |

espectacular should recognize EARS patterns in openspec proposals and emit the appropriate archetype per pattern.

**OpenAI Model Spec pattern**: each clause carries a stable anchor ID (e.g., `#avoid_sycophancy`) with a companion file of adversarial test prompts. espectacular should emit per-requirement assertion stubs with stable IDs traceable to the spec clause.

**Declarative, not imperative**: archetypes must capture *what* the system does, not *how* — decoupled from UI mechanics so assertions survive refactors. Imperative archetypes (naming specific buttons, routes, CSS classes) are a reliability anti-pattern.

**Independent testability (INVEST-Testable)**: each archetype should be independently runnable. The spec-kit "Independent Test" line per user story is the reference pattern.

**Non-regression archetypes**: Kiro's `bugfix.md` includes `WHEN [condition] THEN the system SHALL CONTINUE TO [existing behavior]`. espectacular should emit non-regression assertions for unchanged-behavior sections.

## 2. Drift Detection

**Spec drift** occurs when implementation silently diverges from spec — auth flows shift, mandatory fields become optional, undocumented constraints appear. This creates security blind spots (widely documented; Wiz, among others, covers the API-drift case). espectacular's continuous drift detection addresses this directly.

**Schemathesis checks** are the best public model for espectacular's assertion categories:

| Check | What it detects |
|---|---|
| `not_a_server_error` | Valid inputs cause 5xx — backend resilience failure |
| `positive_data_acceptance` | Valid payloads rejected — phantom constraints not in spec |
| `negative_data_rejection` | Invalid payloads accepted — validation loopholes |
| `response_schema_conformance` | Response shape diverges from spec — type/field drift |

**NASA coverage feedback loop**: uncovered code signals one of four things — missing requirement, missing test, dead code, or deactivated configuration. espectacular's drift detection should surface this as a structured signal, not just a coverage number.

**OpenTelemetry compliance matrix**: per-language conformance reporting against normative spec. espectacular should produce a similar matrix per emitter/language showing covered vs. missing requirements.

## 3. Quality Checks espectacular Should Enforce in Specs

From NASA, Volere, Kubernetes, and synthesized rubrics:

| Check | Signal | Detection heuristic |
|---|---|---|
| **Atomicity** | One behavior per requirement; no conjunctive step bloat | Flag requirements containing multiple `AND`-chained behaviors |
| **Measurability** | Concrete units ("within 1 second", "p95 < 200ms") | Flag vague qualifiers: *fast, scalable, easy, user-friendly, intuitive, efficient, performant* without accompanying numeric bound |
| **Non-goals present** | Absence is a scope-creep smell | Flag specs with no non-goals or out-of-scope section |
| **Traceability** | FR-001 anchors linking requirements → tests → code | Flag requirements without unique IDs |
| **Ambiguity markers** | `[NEEDS CLARIFICATION]` blocks downstream generation | Flag specs where clarification tokens are absent on ambiguous requirements |
| **Negative testing** | Happy-path-only specs are incomplete | Flag specs lacking at least one error/failure state requirement |
| **Tech-agnostic requirements layer** | Implementation details belong in design, not requirements | Flag requirements naming specific frameworks, libraries, or runtime versions |
| **Observable acceptance criteria** | Each requirement needs at least one falsifiable criterion | Flag requirements without a Given-When-Then or EARS-shaped acceptance statement |

**12-point evaluation rubric** (synthesized from INVEST, SMART, and spec-kit, via specs-practices-claude; score 0–2 per item, pass ≥ 16/24):

| # | Dimension | espectacular check name |
|---|---|---|
| 1 | Scope clarity — explicit goals, non-goals, MVP boundary | `check:scope-boundary` |
| 2 | Behavioral grammar — EARS or Given-When-Then; one behavior per line; concrete units | `check:behavioral-grammar` |
| 3 | Numbering & traceability — FR-001, SC-001 anchors that tasks/tests can reference | `check:traceability-anchors` |
| 4 | Tech-agnostic at requirements layer — implementation lives in plan/design | `check:tech-agnostic` |
| 5 | Edge cases & error states — dedicated section, not afterthoughts | `check:negative-coverage` |
| 6 | Measurable success — at least one observable, falsifiable metric | `check:measurability` |
| 7 | Ambiguity markers — explicit `[NEEDS CLARIFICATION]` tokens; no silent assumptions | `check:ambiguity-markers` |
| 8 | NFRs called out — performance, security, accessibility have their own lines | `check:nfr-present` |
| 9 | Living artifact — committed in-repo path the agent reads on every session | `check:living-artifact` |
| 10 | Independent testability — each user story can be released and tested standalone | `check:invest-testable` |
| 11 | Authority/precedence rules — when constraints conflict, the spec says which wins | `check:conflict-resolution` |
| 12 | Right size — short enough to read in one pass; long enough to be actionable | `check:size-signal` |

## 4. Tooling Patterns espectacular Should Emulate

**Spectral** (JSON/YAML linter): JSONPath expressions as rule targets, regex matching for enforcement, format filters per spec version. espectacular's linting layer should follow this architecture: rules select nodes via path expressions, apply predicates, emit severity-ranked findings.

**gherkin-lint rules worth modeling**:
- `no-duplicate-tags` — prevent test duplication
- `no-empty-background` — no structural dead code
- `no-examples-in-scenarios` — enforce Scenario vs Scenario Outline distinction
- `scenario-size` — max steps per scenario (fights step bloat)

**Schemathesis output model**: reads spec → generates permutations → emits **minimal reproduction cases** (curl commands). espectacular's report output should emit minimal reproduction cases that pinpoint the spec clause and the failing assertion.

**KEP test plan structure** is the gold standard for scaffolding: unit tests, integration tests, e2e tests all explicitly named in the spec before implementation. espectacular should scaffold all three tiers from a single spec with explicit tier labels.
