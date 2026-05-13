# Design: Deterministic spec-test correspondence gate

## Product model

`ah` is an OpenSpec companion tool for AI coding harnesses. It adds deterministic friction by requiring every OpenSpec scenario to have an explicit, runnable test contract.

For this change, "deterministic friction" means deterministic CLI failures with machine-readable JSON findings that an AI coding harness can act on without chat history.

## Implementation context

Implement `ah` as a Rust CLI in this repository:

```text
Cargo.toml
src/main.rs              # CLI entrypoint
src/check.rs             # correspondence gate
src/config.rs            # .espectacular/config.toml schema and loading
src/contracts.rs         # scenario TOML schema and loading
src/openspec.rs          # OpenSpec markdown discovery
src/runner.rs            # test command execution
src/init.rs              # ah init and managed blocks
src/doctor.rs            # ah doctor
src/archetypes.rs        # embedded archetype catalog
src/archive.rs           # ah archive
src/upgrade.rs           # ah upgrade
schemas/config.schema.json
schemas/scenario-contract.schema.json
schemas/check-output.schema.json
tests/                   # integration tests for CLI behavior
```

Build and test commands:

```bash
cargo build
cargo test
cargo run -- check
```

Use `clap` for CLI argument parsing, `serde`/`serde_json` for JSON, and a TOML parser compatible with Serde for `.toml` files.

The tool answers only mechanical questions:

1. Which scenarios exist?
2. Does each scenario have exactly one sidecar TOML contract?
3. Does each contract point at an existing scenario?
4. Does the scenario slug, TOML filename, and TOML `id` match?
5. Does each contract declare at least one test?
6. Can each declared test command run, and does it pass?

It deliberately does not decide whether a test is meaningful. The value is that scenario-to-test correspondence becomes visible, named, runnable, and hard to accidentally omit.

## Directory layout

Deployed OpenSpec scenarios live under `openspec/specs/<spec>/spec.md`.

`ah` mirrors that structure under `.espectacular/`:

```text
.espectacular/
  config.toml
  AGENTS.md
  <spec>/
    <scenario-id>.toml
  changes/
    <change-id>/
      <spec>/
        <scenario-id>.toml
```

Examples:

```text
openspec/specs/compiler/spec.md
.espectacular/compiler/empty-input-rejected.toml

openspec/changes/add-parser/specs/compiler/spec.md
.espectacular/changes/add-parser/compiler/empty-input-rejected.toml
```

## Scenario discovery and ids

`ah` discovers scenarios by scanning OpenSpec markdown for `#### Scenario:` headings.

Scenario ids are auto-slugified from the heading text:

- lowercase
- alphanumeric words joined by `-`
- repeated separators collapsed
- leading/trailing separators removed

If two scenarios in the same spec slugify to the same id, `ah check` fails with a structural finding. Scenarios are append-only: do not rename or delete scenario headings to change behavior. Add a new scenario and supersede the old one instead.

Requirement headings remain useful for output grouping, but the correspondence unit is always the scenario.

## Scenario contract schema

Each scenario has one TOML sidecar:

```toml
id = "empty-input-rejected"
description = "Empty input is rejected before parsing."
archetype = "PF"
status = "active" # active | superseded
superseded_by = ""
authored_with = "0.1.0"

[[tests.unit]]
flags = "tests/compiler/test_parser.py::test_empty_input_rejected"
timeout_seconds = 60

[[tests.pbt]]
flags = "tests/compiler/test_parser.py::test_no_empty_input_is_accepted"

[[tests.shell]]
command = "ah --version | grep -q 'ah '"
timeout_seconds = 10
```

Normative schema: `schemas/scenario-contract.schema.json`.

| Field | Required | Type | Rule |
| --- | --- | --- | --- |
| `id` | yes | string | MUST equal discovered scenario slug and TOML filename stem |
| `description` | yes | string | MAY be empty |
| `archetype` | yes | string | MAY be empty; warn in `ah doctor` when non-empty unknown |
| `status` | yes | enum | `active` or `superseded` |
| `superseded_by` | yes | string | MUST be non-empty when `status = "superseded"`; MAY be empty otherwise |
| `authored_with` | yes | string | installed `ah` version used when contract was created |
| `tests.<type>` | yes | array | At least one test entry across all test types |
| `tests.<type>[].flags` | non-shell only | string | Required for non-shell test entries |
| `tests.shell[].command` | shell only | string | Required for shell test entries |
| `tests.<type>[].timeout_seconds` | no | integer | Positive integer; defaults to 60 |

The contract is valid only when all three identifiers match:

- the discovered scenario slug
- the TOML filename stem
- the TOML `id` value

`status` is an enum: `active` or `superseded`. Unknown statuses fail schema validation. A `superseded` contract must set `superseded_by` to a non-empty scenario id, and its tests still run.

`description` and `archetype` are advisory. They help AI agents and reviewers understand intent, but `ah check` does not enforce archetype-specific edge cases or assertion quality.

A scenario contract must declare at least one test entry. Empty test sets fail.

## Runner configuration

`.espectacular/config.toml` configures the project:

```toml
tool_version = "0.1.0"

[paths]
specs = "openspec/specs"
changes = "openspec/changes"

[runners]
unit = ["uv", "run", "pytest"]
pbt = ["uv", "run", "pytest"]
```

Normative schema: `schemas/config.schema.json`.

| Field | Required | Type | Rule |
| --- | --- | --- | --- |
| `tool_version` | yes | string | pinned `ah` version for compatibility mode |
| `paths.specs` | yes | string | deployed OpenSpec specs path, default created by `ah init` is `openspec/specs` |
| `paths.changes` | yes | string | OpenSpec changes path, default created by `ah init` is `openspec/changes` |
| `runners.<type>` | no | array of strings | argv tokens for non-shell test types; required for each non-shell `tests.<type>` used |

For non-shell tests, `ah` composes the configured runner argv with each entry's `flags` appended as one final argv token:

```text
["uv", "run", "pytest", "tests/compiler/test_parser.py::test_empty_input_rejected"]
```

Non-shell runners are executed without a shell. No glob expansion, shell variable expansion, or shell quoting is applied to runner argv or `flags`.

For `tests.shell`, `ah` runs the entry's `command` directly through `/bin/sh -c`. This supports CLI-level assertions without wrapping them in a unit-test framework.

Execution defaults are deterministic:

- working directory: repository root
- environment: inherit the parent process environment unchanged
- execution order: sequential by `(spec_path, scenario_id, test_type, declaration_index)`
- timeout: 60 seconds per declared test command unless `timeout_seconds` is set on the test entry
- stdout/stderr capture: retain the final 8 KiB of each stream in JSON output
- shell mode: only `tests.shell` entries use `/bin/sh -c`; non-shell entries are executed as configured runner command plus flags

`ah` treats every timeout or non-zero command exit as `test-failing`. It does not try to distinguish a missing selector from a real assertion failure because that distinction is runner-specific. Missing runner configuration and malformed test entries are structural findings.

Multiple scenario contracts may reference the same selector. The schema keeps that duplication local and self-describing. `ah` may deduplicate identical command invocations at runtime, but deduplication is not part of the contract shape.

## `ah check`

Default scope:

```text
ah check
```

validates deployed specs only: `openspec/specs/` plus `.espectacular/<spec>/`.

Change scope:

```text
ah check --changes add-parser
ah check --changes add-parser --changes update-cli
```

validates the post-merge overlay: deployed specs plus selected OpenSpec change deltas, with corresponding contracts under `.espectacular/changes/<change>/`. V1 change overlays support added scenarios and staged contract metadata updates for existing deployed scenarios. Removed scenarios are not supported by the gate; behavior removal must be represented by adding a new scenario and superseding the old one. If multiple selected changes claim the same new scenario id for the same spec, `ah check` fails loudly.

`ah check` reports all findings in stable `(spec_path, scenario_id)` order and exits `0` only when `findings` is empty. It exits non-zero for any finding. JSON output includes the checked scope and enough context for an AI agent to act without an additional lookup, including spec path, scenario title, and scenario body markdown. Scenario body markdown is the markdown after the `#### Scenario:` heading until the next heading whose level is `####` or higher. If the scenario has no body lines, `body_markdown` is an empty string.

Normative schema: `schemas/check-output.schema.json`.

Minimal success JSON shape:

```json
{
  "scope": { "deployed": true, "changes": [] },
  "summary": { "structural": 0, "execution": 0, "passed": 3 },
  "findings": []
}
```

Minimal failure JSON shape:

```json
{
  "scope": { "deployed": true, "changes": ["add-parser"] },
  "summary": { "structural": 1, "execution": 1, "passed": 3 },
  "findings": [
    {
      "kind": "no-tests-declared",
      "category": "structural",
      "spec": "compiler",
      "spec_path": "openspec/specs/compiler/spec.md",
      "scenario": {
        "id": "empty-input-rejected",
        "title": "Empty input rejected",
        "body_markdown": "- **WHEN** input is empty\n- **THEN** parsing fails"
      }
    },
    {
      "kind": "test-failing",
      "category": "execution",
      "spec": "compiler",
      "spec_path": "openspec/specs/compiler/spec.md",
      "scenario": {
        "id": "empty-input-rejected",
        "title": "Empty input rejected",
        "body_markdown": "- **WHEN** input is empty\n- **THEN** parsing fails"
      },
      "test": {
        "type": "unit",
        "command": "uv run pytest tests/compiler/test_parser.py::test_empty_input_rejected",
        "exit_code": 1,
        "timed_out": false,
        "stdout_tail": "",
        "stderr_tail": "AssertionError: ..."
      }
    }
  ]
}
```

Finding kinds are an enum:

| Kind | Category | Meaning |
| --- | --- | --- |
| `no-toml` | structural | scenario has no matching contract |
| `orphan-toml` | structural | contract has no matching scenario |
| `slug-collision` | structural | multiple scenarios in one spec slugify to the same id |
| `id-mismatch` | structural | scenario slug, TOML filename stem, and TOML `id` do not match |
| `invalid-status` | structural | `status` is not `active` or `superseded` |
| `no-tests-declared` | structural | contract declares no test entries |
| `missing-runner` | structural | non-shell test type has no configured runner |
| `malformed-contract` | structural | TOML is unreadable, invalid, missing required fields, or has wrong field types |
| `missing-replacement` | structural | superseded contract points to a replacement scenario absent from scope |
| `overlay-conflict` | structural | selected changes define the same new scenario id for the same spec |
| `test-failing` | execution | declared test command timed out or exited non-zero |

Findings are grouped conceptually into:

- structural failures: finding kinds whose category is `structural`
- execution failures: finding kinds whose category is `execution`

Both categories fail the command. The distinction lets AI agents recognize TDD red-phase execution failures separately from broken wiring.

## OpenSpec change lifecycle

In-flight scenarios live in OpenSpec change proposals. `ah` supports them with explicit scope and staging:

- `ah scenario new <change> <spec> --requirement "<requirement>" "<heading>"` appends a scenario to `openspec/changes/<change>/specs/<spec>/spec.md` under the named `### Requirement:` and creates the matching TOML under `.espectacular/changes/<change>/<spec>/`.
- `ah scenario supersede <spec> <old-id> --with=<new-id> --in-change=<change>` stages a copy of the old deployed contract under `.espectacular/changes/<change>/<spec>/<old-id>.toml`, marks it `superseded`, and links it to the new scenario id.
- `ah archive <change>` moves staged `.espectacular/changes/<change>/<spec>/*.toml` files into `.espectacular/<spec>/` after the matching OpenSpec change is archived.

`ah scenario new` requires the target OpenSpec change spec file and requirement heading to exist. It fails without creating files if either is missing. This keeps OpenSpec proposal structure explicit and avoids guessing where a scenario belongs. It appends this markdown skeleton:

```markdown
#### Scenario: <heading>
- **WHEN** [describe the action or condition]
- **THEN** [describe the expected observable result]
```

It creates this initial TOML skeleton, where `<id>` is the slugified heading and `<tool-version>` is the installed `ah` version:

```toml
id = "<id>"
description = ""
archetype = ""
status = "active"
superseded_by = ""
authored_with = "<tool-version>"
```

The generated contract intentionally fails `ah check` with `no-tests-declared` until a test entry is added.

`ah scenario supersede` requires `<new-id>` to exist in the deployed-plus-selected-change scope for `<spec>`. It fails without modifying files when the replacement id is missing.

`ah archive` first verifies that every staged contract's scenario id exists in deployed `openspec/specs/` after `openspec archive <change>` has run. It fails without moving files if any staged contract would become orphaned. It also fails if a destination contract already exists, except when the staged contract has the same id and `status = "superseded"`; that case replaces the deployed contract as the explicit supersession metadata update.

`ah` does not invoke `openspec validate` as part of `ah check`. Scenario heading discovery is structural and independent. `ah doctor` may report whether `openspec` is available and recommend validation.

## Initialization and health

`ah init` is idempotent. It fails without creating `.espectacular/` when no `openspec/` directory exists. When `openspec/` exists, it:

- requires an `openspec/` directory
- creates `.espectacular/config.toml` if missing
- writes `.espectacular/AGENTS.md`
- creates top-level `AGENTS.md` and `CLAUDE.md` if absent
- refreshes managed `ah` blocks in top-level instruction files
- stubs TOML files with empty test sets for existing deployed scenarios
- detects supported hook frameworks in this order: `lefthook`, then `prek`
- installs an `ah check` pre-commit integration when a supported hook framework is detected
- reports a concern if no supported hook framework is present and does not write raw `.git/hooks/pre-commit`

`ah doctor` validates installation health: config schema, configured paths, tool-version compatibility, managed blocks, supported hook integration, slug collisions, orphan contracts, and known archetype names. Unknown archetype names are warnings because archetypes are advisory; slug collisions and orphan contracts are errors.

`ah upgrade` compatibility changes include config schema version changes, execution default changes, archetype additions, and archetype deprecations.

Local pre-commit is the convenience gate. CI MUST run `ah check` as the enforcement gate; `git commit --no-verify` is accepted as a local escape hatch and does not need special tracking.

## Archetypes

Archetypes are shipped with the `ah` tool rather than stored in each project. `config.toml` pins the tool version a project was authored against. `ah` runs in compatibility mode for pinned versions and upgrades are explicit through `ah upgrade`.

Archetypes are append-only. A newer tool version may deprecate an archetype but does not remove it. Scenario contracts record `authored_with` so upgrade reports can explain version drift. `ah upgrade` updates `.espectacular/config.toml` only; it does not rewrite existing scenario contracts or their `authored_with` values.

`ah type` lists archetypes with one-line descriptions. `ah type <archetype>` prints the full documentation for that archetype. Archetype names are advisory tags for AI guidance and reviewer scanning; they do not drive deterministic edge-case enforcement in v1.

## Risks and mitigations

### RISK-001: Vacuous tests pass the gate

A test can exist and pass while asserting nothing useful.

**Mitigation:** Make this an explicit non-goal. `ah` verifies correspondence and execution only. Review, test-design practice, and future optional analyzers handle test quality.

### RISK-002: Full-project pre-commit can become slow

`ah check` runs every declared test in scope.

**Mitigation:** Start with whole-project correctness. If scale demands it, add an explicit staged scope later without changing the contract schema.

### RISK-003: Scenario id changes break correspondence

Ids derive from scenario headings.

**Mitigation:** Treat scenarios as append-only. Rename-by-supersession rather than editing headings. Slug collisions fail deterministically.
