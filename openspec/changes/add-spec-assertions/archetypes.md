# Archetype catalog seed

Archetypes are built into the `ah` tool and surfaced through `ah type` and `ah type <name>`. They are advisory tags for AI guidance and reviewer scanning. `ah check` records and reports archetype names, but does not enforce archetype-specific edge cases or test meaningfulness in v1.

Archetypes are append-only. A future tool version may deprecate an archetype, but existing scenario contracts remain readable in compatibility mode.

## PF — Pure Functional

Deterministic behavior where outputs are a function of explicit inputs.

Use for:
- parsers
- formatters
- validators
- pure transformations
- deterministic calculations

Typical test shapes:
- unit examples for representative inputs
- property-based tests for invariants
- boundary input examples

## SA — Stateful API

Behavior involving state transitions, persisted data, or ordered operations.

Use for:
- create/update/delete flows
- session state
- caches
- workflow state machines
- idempotency rules

Typical test shapes:
- unit or integration tests for state before/after
- repeated-operation tests
- invalid transition tests

## BP — Boundary Protocol

Behavior at an external boundary or protocol seam.

Use for:
- HTTP APIs
- CLI invocation
- filesystem effects
- network calls
- serialization contracts

Typical test shapes:
- shell tests for command behavior
- integration tests against test doubles
- golden input/output fixtures

## CE — Contract/Event

Behavior expressed as emitted events, messages, claims, or cross-tool signals.

Use for:
- structured JSON outputs
- event logs
- report formats
- machine-readable diagnostics
- integration payloads

Typical test shapes:
- schema validation tests
- golden JSON comparisons
- event presence/absence checks
