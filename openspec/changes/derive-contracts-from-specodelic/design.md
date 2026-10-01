# Design: derive-contracts-from-specodelic

## Context

Today a dual-format spec has two independent test-intent declarations:
Properties rows (specodelic) and `#### Scenario:` headings with contract TOMLs
(espectacular). `src/lint/bridge.rs` shells out to `spk lint <file> --json` and
never reimplements specodelic rules; this change keeps that principle and
extends it: `ah` never re-parses specodelic markdown tables either.

Grounding corrections folded in from the review: the integration boundary is
the `spk parse --json` IR (Option B); the archive round-trip answer is
unreleased upstream; the zero-tests-ran guard lands in `src/runner.rs`.

## Decisions

### D1 — Precedence: specodelic authoritative per property, scenarios as prose
When a file has Properties rows and passes `spk lint`, contracts are derived from
rows. A scenario is *covered* iff it carries `- **VERIFIES** [[spec.P-id]]`.
Unlinked scenarios fall back to today's rule (they need their own contract).
Rationale: incremental adoption; no flag day; plain openspec untouched.

### D2 — `ah` consumes the `spk parse --json` IR; validity stays with `spk`
`ah sync` runs **two chained subprocess invocations per file**: `spk lint`
first (refusal gate — a lint-dirty file is never derived from), then
`spk parse --json` (structured `Spec` IR: intent, constraints, states,
transitions, properties, links — all `Serialize`-derived in specodelic's
lib crate). Chosen over embedding lint status in `parse` because it keeps
each tool's envelope single-purpose and matches the bridge's invocation
pattern. If `spk` is missing, `ah sync` fails with `spk-unavailable`; `ah
check` degrades to advisory as today. The consumer tolerates unknown envelope
fields and `data: null` (serde defaults, same as the bridge) so pre-1.0
envelope evolution cannot crash the gate. If a future spk release adds a
structured export with lint status, the chaining can collapse to one call.

### D3 — Derived vs human-owned fields
Derived (rewritten by `ah sync`): `id`, `description` (from predicate),
`archetype`, `falsifiability_class`, `derived_from`.
Human-owned (never overwritten): `tests`, `status`, `superseded_by`.
Rationale: the only thing a human must supply is the test binding; everything
else is a function of the spec row.

### D4 — Drift via canonical-JSON hash
`derived_from = "<property-id>@<sha256-prefix>"` where the hash input is the
canonical JSON serialization of the parsed property `Row` (`id`, `kind`,
`derives_from`, `generator`, `predicate` — BTreeMap cell order makes it
deterministic). Editing any cell changes the hash; reordering whitespace does
not. Mismatch → `contract-stale` (structural). `ah sync --check` exits
non-zero on drift without writing (CI mode).

### D5 — Initial archetype / falsifiability inference
| Condition on the property's source constraint | archetype | falsifiability_class |
|---|---|---|
| kind `effect`, or cited by a state `emits` | CE | safety |
| cited by a transition guard | SA | safety |
| property kind `law` | PF | safety |
| otherwise | PF | safety |
Liveness is never inferred (not expressible in the current grammar); authors
needing it edit the generated contract, which makes it stale — see Open
question on overrides.

### D6 — Test-body ownership
`spk compile` output (`<spec>_props.rs`) quotes generator/predicate and
contains `todo!()` stubs; it is regenerated on every compile and must not be
edited. Derived contracts bind to a *human-owned* test named after the
property id (`p_not_shipped`) via `tests.cargo.flags`. `ah check` flags a
missing binding structurally; `--run-tests` executes it.

### D7 — Model-check out of scope
`.check.json` outcomes were all `exploration_only`. Gating on them would imply
verification that does not exist.

### D8 — Derived contracts are committed spec artifacts (governance)
`ah sync` writes derived contract TOMLs alongside the contract corpus the gate
reads (`src/check.rs` resolves `contracts_dir = .espectacular/`). That collides
with the repo rule "don't commit tool-managed `.espectacular/` state" unless
the policy is explicit. Resolution (task 6.1, human-confirmed): derived
contracts are **committed artifacts refreshed by sync** — like lockfiles —
not disposable tool state; the rule is amended to name the carve-out
(`.espectacular/**/contracts/**` committed; runtime state not). Alternatives
rejected: deriving contracts in-memory at check time (human-owned `tests`
fields still need a committed home — splits the artifact); moving contracts
out of `.espectacular/` (flag-day churn for existing adopters).

## Alternatives considered

- **Generate scenarios from properties** (reverse direction): rejected — scenario
  prose is for humans; generating it loses wording and breaks openspec review.
- **Make specodelic mandatory when present**: rejected — flag day for adopters.
- **Have spk emit ah contracts**: viable long-term, but couples spk to ah's schema.
- **Crate dependency on specodelic** (Option C): strongest typing, but couples
  releases pre-1.0, obsoletes `spk-unavailable` semantics (needs a lint-capability
  delta), and crates.io 0.1.0 is stale vs main. Deferred to post-1.0.

## Migration

Opt-in per file. Pilot on this repo's `lint` capability: add `VERIFIES` links,
run `ah sync`, fill in test bindings, then remove the now-redundant hand-written
contracts. All 8 deployed specs carry Properties rows; the remaining seven
follow the same recipe, gated on the byte-identical regression task.

## Open questions

1. ~~Runner semantics of `flags`.~~ **Resolved by spike**: `cargo test`
   exits 0 when the filter matches zero tests ("0 passed; 1 filtered out").
   The zero-tests-ran guard is required; it lands in `src/runner.rs`.
2. ~~`openspec validate --strict` and the `**VERIFIES**` bullet.~~ **Resolved
   by spike**: accepted inside a scenario body (exit 0); keep the link in the
   bullets.
3. **Overrides.** Hand-edited derived fields make a contract stale. Options:
   an `override_*` namespace honored by sync, or a Properties-row annotation.
   Decide during task group 3.
4. ~~`id: spec` constraint.~~ **Non-issue**: all deployed capability files are
   `id: spec`, and `spk parse` exports per-file IR anyway.
5. **Archive round-trip.** `spk archive-companion` (deploys the dual-format
   layer verbatim, fail-closed) resolves the design — but it is on specodelic
   main only, not in any release (spk 0.2.0 rejects it). Until the release +
   pin bump, validate round-trip behavior against plain `openspec archive`;
   re-validate with archive-companion after task group 1.
6. **Commit policy for derived contracts** (D8): needs explicit human
   confirmation in task 6.1 before the pilot writes any derived TOMLs.