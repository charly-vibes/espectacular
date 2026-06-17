# Changelog

All notable changes to `ah` are documented here.

Format: [Keep a Changelog](https://keepachangelog.com/en/1.1.0/).
Versioning: [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

---

## [Unreleased]

---

## [0.1.0] — 2026-06-17

Initial stable release. Covers two deployed change proposals:
`add-spec-assertions` and `add-quality-measurement-and-adapters`.

### Added

#### Core gate (`add-spec-assertions`)

- `ah check` — validate deployed specs and run all declared tests; emits stable JSON to stdout.
- `ah check --changes <id>` — validate with one or more staged change overlays.
- `ah init` — create or refresh `.espectacular/` directory, config, AGENTS.md managed block, and pre-commit hook integration (lefthook, prek).
- `ah doctor` — diagnose config, path, hook, collision, orphan, archetype, and managed-block issues.
- `ah scenario new` — append a new scenario to a staged change spec and create its TOML contract stub.
- `ah scenario supersede` — stage a supersession update for an existing deployed contract.
- `ah archive <change>` — promote staged change contracts into deployed `.espectacular/` paths.
- `ah upgrade` — detect and repair `tool_version` drift in `.espectacular/config.toml`.
- `ah type` / `ah type <code>` — list or describe built-in archetypes (PF, SA, BP, CE, NR).
- Versioned, append-only archetype catalog embedded in the binary.
- Stable JSON output schema at `schemas/check-output.schema.json`.
- Scenario contract TOML schema at `schemas/scenario-contract.schema.json`.
- Config TOML schema at `schemas/config.schema.json`.

#### Language adapters and quality signals (`add-quality-measurement-and-adapters`)

- **Language adapter dispatch** — `ah check` auto-detects and routes test execution through the correct adapter based on config and project manifest:
  - Python: pytest (via `pyproject.toml`, `pytest.ini`, `setup.cfg`, environment import, or explicit config)
  - Rust: cargo test (via `Cargo.toml` or explicit config)
  - TypeScript: vitest (via `package.json` dependency or explicit config)
  - Custom: arbitrary runner via JSON envelope protocol (see below)
- **Custom runner protocol** — configure any test runner under `[runners.custom.<name>]`; the runner must emit a JSON envelope (`exit_code`, `passed`, `findings[]`) on stdout. Schema at `schemas/custom-runner.schema.json`.
- **`ah explain <topic>`** — print playbook guidance for any finding kind or suggested action; supports `--json` and `--list`.
- **`ah doctor --enable <adapter>`** — detect adapter readiness and write the corresponding runner or quality config block into `.espectacular/config.toml`.
- **Quality signals** (opt-in, emitted as `quality-*` findings when checks pass):
  - `quality-mutation` — mutation kill rate measured against a configured threshold.
  - `quality-property` — property-based test suite present and passing.
  - `quality-snapshot` — snapshot test suite present and passing.
- **`ah signals`** — read `.dont/events/*.json` and emit drift signal JSON for integration with the `dont` evidence layer.
- All findings now carry `suggested_action` and `playbook_command` fields for agent-consumable remediation.
- `summary.counts_by_kind` added to `ah check` JSON output.

### Finding kinds

| Kind | Category |
| --- | --- |
| `no-toml` | structural |
| `orphan-toml` | structural |
| `slug-collision` | structural |
| `id-mismatch` | structural |
| `no-tests-declared` | structural |
| `missing-runner` | structural |
| `malformed-contract` | structural |
| `missing-replacement` | structural |
| `overlay-conflict` | structural |
| `test-failing` | execution |
| `quality-mutation` | quality |
| `quality-property` | quality |
| `quality-snapshot` | quality |

---

[Unreleased]: https://github.com/charly-vibes/espectacular/compare/v0.1.0...HEAD
[0.1.0]: https://github.com/charly-vibes/espectacular/releases/tag/v0.1.0
