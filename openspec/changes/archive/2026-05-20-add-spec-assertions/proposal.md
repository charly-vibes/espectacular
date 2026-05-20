# Change: Add deterministic spec-test correspondence gate

## Why

AI coding harnesses can implement behavior that is only loosely connected to the OpenSpec scenarios they were asked to satisfy. The project needs deterministic friction: a tool that raises missing wiring, missing tests, and failing scenario checks back to the AI before code is committed.

## What Changes

- Replace speculative spec-to-test compilation with a deterministic correspondence gate exposed as `ah check`.
- Add `.espectacular/` as the sidecar contract directory that mirrors OpenSpec specs and change proposals.
- Require one TOML contract per OpenSpec scenario; each contract declares the scenario id, description, archetype tag, status, and the test commands/selectors that cover the scenario.
- Require the discovered scenario slug, TOML filename stem, and TOML `id` to match.
- Run declared tests through project-configured runners and emit stable JSON findings for missing contracts, orphan contracts, invalid contract wiring, and failing tests.
- Add authoring and lifecycle commands: `ah init`, `ah doctor`, `ah type`, `ah scenario new`, `ah scenario supersede`, `ah archive`, and `ah upgrade`.
- Install local pre-commit integration during `ah init` when a supported hook framework is present, and document CI as the enforcement gate.

## Non-Goals

- `ah` does not judge whether a test is meaningful, sufficient, or semantically aligned with scenario prose.
- `ah` does not parse natural language `WHEN`/`THEN` text into assertions.
- `ah` does not generate language-specific test code, maintain an assertion IR, or provide emitters.
- `ah` does not inspect test internals such as fixtures, mocks, assertions, or setup/teardown.
- `ah check` does not detect prose drift inside a scenario body; reviewers must require append-only supersession for substantive intent changes.

## Impact

- Affected specs: `cli`, `gate`
- Affected code: new Rust `ah` CLI (`Cargo.toml`, `src/*.rs`), normative schemas under `schemas/`, `.espectacular/` project files, pre-commit/CI setup, OpenSpec-adjacent scenario lifecycle commands
- **BREAKING**: none for existing OpenSpec projects; projects opt in with `ah init`.
