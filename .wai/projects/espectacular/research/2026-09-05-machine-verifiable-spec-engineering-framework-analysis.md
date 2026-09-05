# Machine-Verifiable Specification Engineering — Findings Applicable to espectacular

Source: `~/Downloads/Verifiable Software Specification Framework.md` (2026-09-05)
Kind: research (maps a specification-engineering framework onto espectacular's current feature surface)

**Caveat:** the source doc renders its formulas as embedded base64 images, so the exact metric equations were unreadable. The surrounding prose, the RCI citation (ref 13: "Improving the clarity of requirements by generating RCI value"), and the conceptual definitions are sufficient to evaluate the ideas; re-derive formulas from the cited papers before implementing anything numeric.

## Framework Summary

The doc proposes a specification stack (formal boundary atop, implementation below) that separates:

1. **Formal state layer** (TLA+, Alloy) — system invariants, safety/liveness properties
2. **Core domain logic** (DDD) — capabilities and invariants with zero transport/UI coupling
3. **Operational contracts + property-based tests** (Pact, Hypothesis) and, as a sibling at the same level, **quality attribute scenarios** (ATAM / ISO 25010) — the source doc bifurcates the middle layer rather than stacking quality attributes beneath contracts

It also defines three quantitative spec-quality metrics — Requirement Clarity Index (RCI), Internal/External Completeness, Volatility Rate — and catalogs boundary blind spots (backpressure, circuit breakers, non-deterministic outputs, eventual consistency, additive-only schema evolution).

**Positioning check:** espectacular occupies layer 3 territory (scenario ↔ test correspondence + gate) and already uses layer 2 vocabulary in its archetypes. The framework is a source of lint rules, metadata dimensions, and signals — not a new architecture for the tool.

## Gap Analysis

| Framework concept | espectacular today | Gap / opportunity |
|---|---|---|
| RCI clarity scoring of requirement text | `ah check` verifies structural correspondence only | Lint scenario bodies for ambiguity markers |
| Internal completeness (unhandled branches) | GIVEN/WHEN/THEN parsed, not semantically checked | Flag scenarios missing THEN, or table examples not covered by tests |
| Volatility Rate from spec history | Append-only specs + git log already exist | Compute supersede frequency per requirement as spec-debt signal |
| Decoupled domain capability (no UI primitives in specs) | Archetype taxonomy exists but no coupling lint | Flag UI/route/CSS references in PF/SA scenarios |
| Safety vs liveness property distinction | No property class in contract metadata | Optional contract field; drives timeout/failure-semantics checks |
| Non-deterministic/probabilistic scenarios | No archetype fits (PF requires determinism; BP is protocol-shaped) | Decision needed: new archetype vs documented BP+custom-runner pattern |
| Backpressure / circuit-breaker / convergence patterns | `ah explain` playbooks are finding-oriented | Add BP test-design playbooks for these failure modes |
| ATAM/ISO 25010 quality-attribute scenarios | Out of scope (matches prior research non-goals) | None — keep out |

## Improvement Opportunities (priority order)

### 1. Spec-quality lint: clarity + completeness checks (`ah check` extensions)

Highest leverage, lowest cost. Extends the existing finding-kinds machinery.

- **`vague-quantifier`** — scenario text containing "fast", "user-friendly", "as appropriate", "etc." (RCI-style ambiguity markers). Note: openspec requirement language uses RFC 2119 modality keywords (MUST/SHALL/SHOULD/MAY), so "should" must be excluded from the matcher to avoid firing on normative SHOULD usage.
- **`passive-scenario`** — THEN clauses with no observable system action
- **`missing-branch`** — a WHEN clause enumerating value cases ("under 200", "over 200") with contracts covering only a subset. (The source doc models this as Gherkin Scenario Outline tables, but espectacular's parser handles GIVEN/WHEN/THEN bullets only — no table parsing exists in `src/` — so the rule must key on enumerated text, not table rows.)
- **`undefined-term`** — domain terms used in scenarios but never defined in the spec's requirement headers (external-completeness proxy). Will fire on ordinary English ("input", "system") without stopword filtering or a capitalized/quoted-term convention.

All four are pure spec-text analysis — no test execution, cheap to unit-test (TDD-friendly), and directly serve "more reliable at detecting drift": ambiguous specs currently pass the gate and defer ambiguity to humans at review time.

**Gate semantics (design constraint):** these must ship as warnings, not gate-failing errors — every existing adopter's prose-heavy specs would otherwise fail `ah check` on day one. Escalate to gate-failing only via per-project config opt-in or a `--strict` flag. Each new finding kind also needs an `ah explain <kind>` entry (playbooks are finding-oriented by design).

### 2. Entanglement lint: UI primitives in non-presentation scenarios

The framework's §2 antipattern table ("clicks the blue modal submission button") maps to a concrete rule: scenarios with archetype `PF` or `SA` whose text references UI primitives (buttons, routes, modals, colors, CSS selectors, component names) get an `entangled-spec` finding. The archetype already tells us the expected layer — this rule just checks the spec text against it. Small matcher list, high signal; also improves assertion longevity (declarative-not-imperative principle already noted in the 2026-05-18 SDD research). Known false-positive risk: "route" appears legitimately in domain contexts ("event routing"), "component" in domain decomposition — needs a vetted matcher list plus a per-contract suppression annotation so reviewers can waive individual findings.

### 3. Volatility Rate as a `dont` signal

Specs are append-only and contracts carry `superseded_by`; git history already records when scenarios were superseded. Volatility Rate (supersedes + additions per requirement per time window) is computable with existing data and would surface spec debt as a drift signal via `ah signals` rather than a new command. Candidate after #1 lands (reuses the spec parser for scenario identity across revisions; git-log walking is new). Aligns with the existing dont drift-signal integration.

### 4. Safety/liveness property class in contracts

Optional `property_class = "safety" | "liveness" | ""` field on contracts. Enforcement value: liveness claims ("every request eventually terminates") require tests with explicit timeout semantics — `ah check` could require `timeout_seconds` on liveness-tagged test entries and warn on liveness claims verified only by deterministic unit tests. Backward compatible if the field defaults to empty (no deprecation cycle needed) — but the scenario contract format is governance-protected, so even this additive change requires an openspec proposal first.

### 5. BP archetype blind-spot playbooks in `ah explain`

The framework's blind-spot catalog (Pact "ignores provider-side stateful persistence"; BDD "shallow state coverage") suggests `ah explain bp` should carry test-design guidance for: circuit-breaker state machines (CLOSED/OPEN/HALF-OPEN transitions), backpressure invariants (bounded queue under arrival > processing rate), eventual-consistency convergence windows, and additive-only evolution (three-phase deprecation). Documentation-only change; zero code risk.

### 6. Non-deterministic scenarios — decision needed, don't rush

Statistical assertions (P(correct) ≥ 1−δ at confidence θ) have no home in the archetype taxonomy. Options: (a) new archetype (breaking taxonomy change → openspec proposal required), (b) documented BP + custom-runner JSON-envelope pattern (no schema change), (c) out of scope. Caveat on (b): the custom-runner envelope carries a pre-computed pass/fail verdict — sample size, confidence, and CI belong in the runner, so the spec side still lacks a home for the statistical claim itself (a generalization of #4's `property_class` idea, e.g. `verification_kind`). LLM-adjacent consumers will ask for this; recommend (b) first, promote to (a) only when two+ adopters need it.

### 7. Agent skills for unstructured spec verification (dont-style)

**Axis the deterministic gate cannot cover:** `ah check` verifies structural correspondence and test passage, but the framework's highest-value checks — is this scenario genuinely testable? are boundary cases enumerated? is the requirement complete against stakeholder expectations? — are judgment calls. Agent skills (SKILL.md workflows, like the ~40 installed in `~/.agents/skills/`) let an LLM do what regex lints can't, and the ecosystem already proves the pattern: dont distributes epistemic discipline via a managed AGENTS.md block + CLI verbs (`conclude`, `ground`, `trust`), and espectacular already has the parallel `ah:managed:` block that `ah init` projects.

**Proposal:** ship an opt-in skill (projected by `ah init --skills` or a `skills/` dir in the repo) that walks an agent through adversarial spec critique per requirement: enumerate failure modes (framework §4's catalog as prompts), hunt ambiguity (RCI criteria as judgment calls, not regex), identify uncovered boundary values, and propose missing scenarios. **Non-negotiable: it must emit machine-checkable artifacts, not prose** — spec gaps become openspec change proposals, findings become bd tickets, contested claims get grounded via `dont conclude/ground` (dont is already a detected workflow tool in this ecosystem). Output that only produces prose would be noise; output that feeds the gate's input formats is a force multiplier.

This complements rather than competes with #1: the lint catches mechanical ambiguity cheaply and continuously; the skill catches semantic ambiguity expensively and on demand.

## Non-Goals (consistent with 2026-05-18 SDD research)

- **TLA+/Alloy model checking** — espectacular verifies spec↔test correspondence, not system correctness proofs. State-space explosion and abstraction cost are the framework's own cited failure modes (refs 18, 8).
- **ATAM/ISO 25010 quality-attribute scenario modeling** — non-functional performance verification is an observability/CI concern, not a spec-correspondence one.
- **Pact/CDC tooling** — contract testing at service boundaries belongs to dedicated tools; espectacular's adapters already delegate runner execution.
- **Natural-language spec rewriting (ReCompGPT-style)** — we lint, we don't generate prose.

## Recommended Next Steps

1. `bd create` tickets for #1 (lint checks, one TDD ticket per finding kind) and #2 (entanglement lint)
2. Open an openspec change proposal covering the new finding kinds + `property_class` schema addition (governance: spec-surface changes need proposals even when additive)
3. `ah explain bp` playbook ticket (#5) as a docs-first PR
4. Decision memo for #6 (archetype taxonomy) before any schema work
5. Prototype skill for #7 — a `spec-review` SKILL.md emitting openspec/bd/dont artifacts; adopt dont's managed-block distribution via `ah init` once proven
6. Defer #3 and #4 until #1's spec-parsing layer exists — reuse, don't duplicate

Every item traces to the primary objective: #1–#2 make drift detection more reliable (specs that pass the gate today can be ambiguous or entangled), #3–#4 add signal surface, #5–#7 ease adoption (and #7 extends verification beyond what deterministic rules can reach).
