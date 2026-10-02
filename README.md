> *"Porque mis armas son los pinceles y mis municiones son las pinturas"*
> — Peter de los Polvorines

# espectacular

> **Why:** AI-written behavior needs adversarial evaluation, not just tests —
> espectacular is the behavioral verification layer of the charly ecosystem,
> stress-testing agent outputs against hostile scenarios and stakeholder
> evaluations before they ship.
> **Status:** [beta](docs/src/status.md) · adversarial evaluation workflows shipped · [Motivation & design](docs/src/index.md) · [charly-vibes Tool Ecosystem](https://charly-vibes.github.io/dulce-de-leche/ecosystem-map.html)
[![tracked with wai](https://img.shields.io/badge/tracked%20with-wai-blue)](https://github.com/charly-vibes/wai)

Behavioral verification layer for the charly AI development ecosystem.

## Installation

### Cargo (crates.io)

```bash
cargo install espectacular
```

Installs the `ah` binary.

### Homebrew (macOS & Linux)

```bash
brew tap charly-vibes/charly
brew install ah
```

### Scoop (Windows)

```powershell
scoop bucket add charly https://github.com/charly-vibes/scoop-charly.git
scoop install ah
```

### From source

```bash
git clone https://github.com/charly-vibes/espectacular
cd espectacular
cargo build --release
```

## Tooling

This repo is configured to use:

- `wai` — project context, reasoning, handoffs
- `bd` (beads) — issue tracking and dependencies
- `openspec` — specs and change proposals
- `dont` — epistemic claim tracking and evidence grounding

## Quick start

```bash
just prime
just status
just validate
```

Or run the tools directly:

```bash
wai status
bd ready
openspec list
dont prime --plain
```

## `ah` workflow

`ah` is the CLI that enforces spec-test correspondence.

### Local pre-commit vs CI

- Local hooks are a convenience layer. `ah init` installs `ah check` into the lefthook `pre-commit` **and** `pre-push` stages when it finds `lefthook.yml`; for `prek.toml` it appends the `ah check` integration. Husky repos get a manual-wiring concern (detection only, wiring is lefthook-only).
- CI is the enforcement gate. Run `ah check` in CI and fail the job on any non-zero exit.
- `ah doctor` helps explain setup drift; it is not the enforcement command.

### Commands

| Command | Purpose |
| --- | --- |
| `ah init` | Create or refresh `.espectacular/` files and hook integration |
| `ah doctor` | Detect configured frameworks and diagnose config, path, hook, collision, orphan, and archetype issues |
| `ah doctor --enable <capability>` | Write the config block for a detected-but-unconfigured capability (`pytest`, `cargo`, `vitest`, `mutation`, `property`, `snapshot`) |
| `ah check` | Run fast structural analysis (spec/contract correspondence); use `--run-tests` to execute declared tests |
| `ah check --run-tests` | Run structural analysis plus declared contract tests |
| `ah check --changes <id>` | Validate deployed specs plus one or more staged change overlays |
| `ah lint [root]` | Heuristic spec-quality checks (vague qualifiers, non-imperative steps, conjunctive bloat, missing negative scenarios/non-goals, unresolved ambiguity, entangled specs, scenario-to-property trace gaps) plus a specodelic (`spk`) bridge |
| `ah sync` | Derive property contracts from lint-clean specodelic Properties rows (`--check` for CI drift mode) |
| `ah report` | Generate a spec-to-contract coverage matrix (covered/missing/failing per spec) |
| `ah explain [topic]` | Print guidance for a finding kind or suggested action; omit topic for a list |
| `ah explain --list` | List all explainable topics |
| `ah explain --json` | Emit the topic list as JSON |
| `ah signals` | Read `dont` rejection events and emit drift signals as JSON |
| `ah type` | List built-in archetypes |
| `ah type <code>` | Print full built-in documentation for one archetype |
| `ah scenario new <change> <spec> --requirement "<requirement>" "<heading>"` | Append a new scenario and stage its TOML contract |
| `ah scenario supersede <spec> <old-id> --with=<new-id> --in-change=<change>` | Stage a supersession update for an existing contract |
| `ah archive <change>` | Move staged change contracts into deployed `.espectacular/` locations |
| `ah feedback <kind>` | File an issue against the upstream repo via `gh` (`--dry-run` to preview) |
| `ah completions <shell>` | Generate shell completions (bash, zsh, fish, powershell, elvish) |
| `ah upgrade` | Report tool-version drift and update `.espectacular/config.toml` only |

`ah upgrade` exits non-zero when it detects drift, even after rewriting `tool_version`, so automation can notice compatibility changes.

## `.espectacular/` layout

```text
.espectacular/
├── AGENTS.md
├── config.toml
├── <spec>/
│   └── <scenario-id>.toml
└── changes/
    └── <change>/
        └── <spec>/
            └── <scenario-id>.toml
```

- `.espectacular/<spec>/<scenario-id>.toml` stores deployed scenario contracts.
- `.espectacular/changes/<change>/<spec>/<scenario-id>.toml` stores staged change overlays.
- `openspec/specs/<spec>/spec.md` is the deployed spec source.
- `openspec/changes/<change>/specs/<spec>/spec.md` is the staged change spec source.

## Schemas and file formats

Normative schema files:

- `schemas/check-output.schema.json` — `ah check` JSON envelope
- `schemas/config.schema.json` — `.espectacular/config.toml`
- `schemas/scenario-contract.schema.json` — scenario TOML contracts
- `schemas/custom-runner.schema.json` — custom runner JSON envelope

### `.espectacular/config.toml`

```toml
tool_version = "0.1.0"

[paths]
specs = "openspec/specs"
changes = "openspec/changes"

[runners]
pytest = ["pytest"]
cargo = ["cargo", "test"]
vitest = ["vitest", "run"]

[quality.mutation]
enabled = true
threshold = 0.80
command = ["/bin/sh", "{}"]

[capabilities.property]
enabled = true

[capabilities.snapshot]
enabled = true
```

Required fields:

- `tool_version`: pinned `ah` version for compatibility mode
- `paths.specs`: deployed OpenSpec root
- `paths.changes`: staged OpenSpec change root
- `runners.<type>`: argv array used to execute non-shell test entries

Optional quality fields:

- `quality.mutation.enabled`: activate mutation quality signal
- `quality.mutation.threshold`: minimum mutation score (default `0.80`)
- `quality.mutation.command`: argv template; `{}` is replaced with the runner script path. Mutation scoring is owned by **pretender** (suite decision `espectacular-djw`): the supported wiring is `command = ["pretender", "mutation", "--format", "json"]`, which consumes pretender's report; raw-mutator commands are deprecated and flagged by `ah doctor`
- `capabilities.property.enabled`: activate property-based testing quality signal
- `capabilities.snapshot.enabled`: activate snapshot testing quality signal

### Scenario contract TOML

```toml
id = "empty-input-rejected"
description = "Empty input is rejected before parsing."
archetype = "PF"
status = "active"
superseded_by = ""
authored_with = "0.1.0"

[[tests.unit]]
flags = "tests/compiler/test_parser.py::test_empty_input_rejected"
timeout_seconds = 60
```

Required top-level fields:

- `id`
- `description`
- `archetype`
- `status` (`active` or `superseded`)
- `superseded_by` (non-empty when `status = "superseded"`)
- `authored_with`
- `falsifiability_class` (optional; `"safety"` or `"liveness"` — see below)
- `tests`

Test entry rules:

- `[[tests.shell]]` entries use `command`
- non-shell `[[tests.<type>]]` entries use `flags`
- `timeout_seconds` is optional but must be positive when present

#### Scenario-scoped test entries

Each `[[tests.*]]` entry runs once per contract that declares it. An entry that includes a whole test file (or an entire suite) re-executes everything in it for every contract that references any scenario in that file — N contracts means N full runs, and a failure in any single test blurs attribution across every scenario sharing the file.

Scope each entry to the single test that verifies the scenario, using the runner's selection mechanism:

- **cargo**: substring filter — `flags = "scenario_id"` matches test names containing the scenario id (this is how the property-derived `p-*` contracts bind)
- **pytest**: node id or keyword — `flags = "tests/unit/test_message.py::test_defaults"` or `flags = "-k test_defaults"`
- **vitest**: `flags = "-t message_defaults"`
- **Frameworks without test filtering** (e.g. Julia's `Test`): dispatch on the scenario id inside the test file — pass it as an argument or environment variable and select the matching `@testset` to run:

```toml
[[tests.shell]]
command = "julia --project=. test/unit/message_test.jl message_defaults"
timeout_seconds = 60
```

```julia
# test/unit/message_test.jl
using REPLy, Test
const scenario = get(ARGS, 1, "all")
if scenario in ("all", "message_defaults")
    @testset "message defaults" begin
        # ...
    end
end
```

If a framework offers no selection mechanism at all and dispatching is not feasible, split scenarios into separate test files so one include runs one scenario's assertions only. See `ah explain scenario-scoped-tests`.

### `falsifiability_class`

Contracts may declare an optional `falsifiability_class`:

- `"safety"` — the scenario asserts a safety claim: nothing bad ever happens; falsified by a single deterministic failing run.
- `"liveness"` — the scenario asserts a liveness claim: a good state eventually occurs. Liveness claims are only falsifiable under bounded execution, so if every test entry omits `timeout_seconds`, `ah check` emits a `missing-liveness-timeout` **warning** suggesting an explicit timeout. Warnings do not fail the gate.

The field is optional and absent by default — contracts without it validate exactly as before. Invalid values emit an `invalid-falsifiability-class` structural finding and fail the gate.

## Append-only authoring workflow

Scenarios are append-only.

- Do not rewrite or delete a deployed scenario to change intent.
- Add a new scenario under the targeted requirement with `ah scenario new ...`.
- If the old scenario is replaced, stage a supersession with `ah scenario supersede ...`.
- Once the change is accepted, run `ah archive <change>` to move staged TOML files into deployed `.espectacular/` paths.

Requirement targeting is explicit: `ah scenario new` appends under the named `### Requirement:` block and fails if that requirement is missing.

## `ah check` JSON output

`ah check` always prints a stable JSON envelope to stdout.

Success shape (envelope wrapper shown; examples below show the `data` field only):

```json
{
  "ok": true,
  "envelope_version": "0.1",
  "cli_version": "0.4.0",
  "envelope_kind": "ok",
  "data": {
    "scope": { "deployed": true, "changes": [] },
    "summary": { "structural": 0, "execution": 0, "passed": 1, "counts_by_kind": {} },
    "findings": []
  },
  "warnings": [],
  "hints": [],
  "meta": { "duration_ms": 0, "tx": null, "request_id": null, "author": null }
}
```

`data` fields:

- `scope.deployed`: always `true` in v1
- `scope.changes`: selected change ids, sorted and deduplicated
- `summary.structural`: count of structural findings
- `summary.execution`: count of execution findings
- `summary.passed`: count of passing declared tests
- `summary.counts_by_kind`: map of finding kind → count (all categories, including quality)
- `findings`: sorted by `(spec_path, scenario.id, kind, test)`

Each finding includes:

- `kind`: finding kind string (see table below)
- `category`: `"structural"`, `"execution"`, or `"quality"`
- `spec_path`: spec path or synthetic path (e.g. `"(quality/mutation)"`)
- `suggested_action`: slug for `ah explain <suggested_action>`
- `playbook_command`: pre-formed command string, e.g. `"ah explain fix_tool_invocation"`

### `ah check` finding kinds

| Kind | Category | Meaning |
| --- | --- | --- |
| `no-toml` | structural | scenario has no matching `.espectacular/.../*.toml` contract |
| `orphan-toml` | structural | contract exists without a matching OpenSpec scenario |
| `invalid-falsifiability-class` | structural | `falsifiability_class` value is not `safety` or `liveness`; the scenario's tests do not run |
| `slug-collision` | structural | two scenarios in one spec slugify to the same id |
| `id-mismatch` | structural | scenario slug, TOML filename, and TOML `id` disagree |
| `no-tests-declared` | structural | contract has no runnable test entries |
| `missing-runner` | structural | a non-shell test type has no configured runner |
| `malformed-contract` | structural | TOML cannot be parsed or validated |
| `missing-replacement` | structural | a superseded contract points to a replacement scenario that is absent from scope |
| `overlay-conflict` | structural | selected changes define conflicting staged scenarios or staged contract updates |
| `test-failing` | execution | a declared test timed out or exited non-zero |
| `quality-mutation` | quality | mutation testing score meets or exceeds threshold |
| `quality-composability` | quality | composability tool (vampiro) reported findings |
| `quality-cost` | quality | cost analysis tool (crua) reported findings |
| `quality-boundary-coverage` | quality | boundary coverage tool (livin) reported findings |
| `quality-property` | quality | property-based testing is active and passing |
| `quality-snapshot` | quality | snapshot testing is active and passing |

| `missing-liveness-timeout` | warning | liveness-tagged contract has no `timeout_seconds` on any test entry — liveness claims are only falsifiable under bounded execution |
| `no-tests-ran` | execution | a runner matched zero tests (cargo-style `0 passed; N filtered out`) — a binding that executes nothing is not a pass |
| `contract-stale` | structural | a derived contract's `derived_from` hash no longer matches its specodelic source Properties row — run `ah sync` to refresh |

`test-failing` findings include test execution details: `type`, `command`, `exit_code`, `timed_out`, `stdout_tail`, and `stderr_tail`.

Quality findings (`quality-*`) are informational — they appear in `counts_by_kind` but do not cause `ah check` to exit non-zero.

Warning findings (`missing-liveness-timeout`) are non-gating — they set the finding `severity` to `"warning"` and `ah check` still exits 0. All other findings carry `severity = "error"`.

## Language adapter dispatch

`ah check` maps each `[[tests.<type>]]` entry to a runner:

- `shell` entries use the `command` field directly.
- Named types (`pytest`, `cargo`, `vitest`) look up `runners.<type>` in `config.toml` and prepend those argv entries to the `flags` value.
- `custom` entries invoke the configured runner and parse its JSON envelope (see Custom runner protocol below).

Detection precedence (highest to lowest):

1. Explicitly configured in `[runners]`
2. Detected via project manifest (e.g. `pytest.ini`, `Cargo.toml`, `package.json` with vitest)
3. Detected via binary on `$PATH`

`ah doctor` reports each detected framework and its source. Detected-but-unconfigured frameworks appear as recommendations:

```text
framework: pytest (configured)
recommendation: vitest detected via manifest — run: ah doctor --enable vitest
```

Run `ah doctor --enable <capability>` to write the corresponding config block automatically.

## Custom runner protocol

A custom runner is any executable that prints a JSON envelope to stdout:

```json
{
  "exit_code": 0,
  "passed": true,
  "findings": []
}
```

- `exit_code`: the runner's exit code (informational)
- `passed`: `true` when the test suite passed; `false` triggers a `test-failing` finding
- `findings`: additional findings to surface (empty when `passed` is `true`)

Schema: `schemas/custom-runner.schema.json`. The `findings` array reuses the finding shape from `schemas/check-output.schema.json`.

Configure a custom runner in `config.toml`:

```toml
[runners]
my-tool = ["./scripts/my-runner.sh"]
```

Then reference it in a scenario contract:

```toml
[[tests.custom]]
flags = "--suite integration"
timeout_seconds = 120
```

## Quality signals

Quality signals surface test-suite health metrics as findings. They are enabled via `config.toml` and never block `ah check` (exit code is unaffected).

### Mutation testing

```toml
[quality.mutation]
enabled = true
threshold = 0.80
command = ["/bin/sh", "{}"]
```

`{}` in `command` is replaced with a generated runner script path. When the mutation score meets or exceeds `threshold`, a `quality-mutation` finding appears. If the tool exits non-zero, a `test-failing` finding is emitted instead.

**Mutation ownership (suite decision `espectacular-djw`, option A):**
**pretender owns mutation execution and scoring.** Espectacular gates on the
score but does not re-run mutation tooling. Supported wiring:

```toml
[quality.mutation]
enabled = true
threshold = 0.80
command = ["pretender", "mutation", "--format", "json"]
```

- A raw-mutator command (e.g. `cargo-mutants`, `mutmut`, `stryker`, `pit`)
  is deprecated and flagged as a `mutation-ownership` doctor check: it risks
  a duplicate mutation run and a competing threshold alongside pretender.
- The `{}` generated-runner template is kept for backward compatibility but
  deprecated for mutation-owning setups; wire pretender instead.

### Suite-trio quality signals (vampiro / crua / livin)

```toml
[quality.composability]
enabled = true
command = ["vampiro", "check", "--format", "json"]

[quality.cost]
enabled = true
command = ["crua", "check", "--format", "json"]

[quality.boundary-coverage]
enabled = true
command = ["livin", "check", "--format", "json"]
```

Each tool must emit a custom-runner JSON envelope on stdout with an optional
`findings` array. When findings are present, a `quality-*` finding is emitted.
If the tool exits non-zero, a tool error is reported. These signals are
informational only (do not cause `ah check` to exit non-zero).

Absence of these tools is a graceful no-op — no error is raised when the
config section is missing.

### Property-based testing

```toml
[capabilities.property]
enabled = true
```

When a scenario has a `[[tests.property]]` entry and the run passes, a `quality-property` finding is emitted.

### Snapshot testing

```toml
[capabilities.snapshot]
enabled = true
```

When a scenario has a `[[tests.snapshot]]` entry and the run passes, a `quality-snapshot` finding is emitted.

## `ah doctor` diagnostics

`ah doctor` exits zero with:

```text
healthy: all checks passed
```

Otherwise it exits non-zero and emits diagnostics such as:

- `bad-config`
- `version-drift`
- `missing-path`
- `collision`
- `orphan-contract`
- `unknown-archetype`
- `missing-managed-block`
- `hook-absent`
- `hook-wired` (lefthook detected but a stage does not run `ah check`)
- `mutation-ownership` (a raw mutator — cargo-mutants, mutmut, stryker, pit — is wired into `quality.mutation.command`; pretender owns mutation execution and scoring)

## Archetypes

Use `ah type` to list current archetypes and `ah type <code>` for full guidance.

Current catalog:

- `PF` — Pure Functional
- `SA` — Stateful API
- `BP` — Boundary Protocol
- `CE` — Contract/Event
- `NR` — Non-Regression

The catalog is embedded, append-only, and version-addressable so older pinned projects remain readable in compatibility mode.

## Non-goals

v1 does **not**:

- judge whether a test is meaningful
- inspect test internals for assertion quality
- detect prose drift inside a scenario body

Review and supersession discipline handle those concerns; `ah` only checks deterministic correspondence and execution.

## Notes

- `wai` state lives in `.wai/`
- beads state lives in `.beads/`
- OpenSpec files live in `openspec/`
- `dont` state lives in `.dont/`


---

## A note on authorship

All the code in this repository was generated by a large language model. This is not a confession, nor an apology. It's a fact, like the one that says water boils at a hundred degrees at sea level: neutral, technical, and with consequences one discovers later.

What the human did is what tends to happen before and after things come into existence: thinking. Reviewing requirements, arguing about edge cases, understanding what needs to be built and why, deciding how the system should behave when reality —which is capricious and does not read documentation— confronts it with situations nobody anticipated. The hours of planning, of design, of reading specifications until exhaustion dissolves the boundary between understanding and hallucination.

The LLM writes. The human knows what it should say.

There is a distinction, even if looking at the commit history makes it hard to find. The distinction is that a machine can produce correct code without understanding anything, the same way a calculator can solve an integral without knowing what time is. Understanding what that integral is *for*, whether it actually solves the problem, whether the problem was the right problem to begin with — that remains human territory. For now.

*[Leer en español](https://charly-vibes.github.io/charly-vibes/)*
