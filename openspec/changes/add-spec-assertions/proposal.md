# Epic: Spec-to-assertion compilation and drift detection

## Why

The charly ecosystem verifies AI-generated code structurally (pretender) and epistemically (dont), but nothing verifies **behavioral correctness** — whether the code actually does what the spec says. OpenSpec specs already contain testable invariants in the form of requirements with scenarios (`WHEN`/`THEN` pairs), but these are never compiled into executable checks. Meanwhile, archived specs silently drift from reality as code evolves.

## What Changes

- **Spec-to-assertion compiler**: Parse openspec `spec.md` files and generate executable test assertions from `WHEN`/`THEN` scenarios. Output is language-specific test code (initially Rust and Go to match wai and fabbro).
- **Drift detector**: Compare archived spec invariants against current code state. Flag divergence as potential openspec proposals or alerts.
- **Integration hooks**: Wire into the openspec archive workflow — when a change is archived, automatically generate/update assertions. Periodically (or on relevant file changes) run drift checks.
- **CLI identity**: Define the standalone CLI command as `ah`, with `compile`, `drift`, and `report` subcommands.

## Milestones

| Milestone | Outcome | Depends on |
|-----------|---------|------------|
| **M0: Tracer bullet** | One hardcoded scenario parsed → IR → Go test stub → compiles → drift check detects removal | Nothing |
| **M1: Compiler** | Parser handles full openspec scenario format, IR is stable, Go + Rust emitters produce compilable test stubs | M0 |
| **M2: Drift detection** | Convention-based code mapping, orphan + failure detection, structured drift report | M1 |
| **M3: Integration** | Hooks into `openspec archive`, git hooks, CI step, cross-tool signals to dont/pretender/wai | M2 |
| **M4: Distribution** | Release workflow, homebrew-charly formula, `ah` CLI interface, documentation | M3 |

See `tasks.md` for per-milestone work items.

## Impact

- Affected specs: all repos using openspec (wai, pretender, nayra, fotos, etc.)
- Affected code: `ah` companion binary, openspec archive integration points, CI pipelines (new verification step)
- **BREAKING**: None — purely additive. Existing openspec workflows are unchanged.
