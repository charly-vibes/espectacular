# Command Reference

**In brief:** `ah` is the espectacular CLI. The primary commands are `ah check` (run in CI to enforce specs) and `ah init` (run once per repo to set up). Use `ah explain <topic>` to get guidance on any finding or error.

---

## `ah init`

Create or refresh `.espectacular/` files and hook integration.

```
ah init
```

Idempotent: safe to re-run after updating specs or changing hook frameworks. Stubs contract files for any scenarios that have no existing contract. Installs `ah check` into `lefthook` or `prek` if detected.

**Exit codes:** 0 on success, non-zero if the OpenSpec directory is missing.

---

## `ah check`

Validate deployed specs and run declared tests. Prints a stable JSON envelope to stdout.

```
ah check [--changes <id>]
```

**Flags:**

| Flag | Description |
|------|-------------|
| `--changes <id>` | Include one or more staged change overlays (repeat for multiple) |

**Exit codes:** 0 when `findings` contains no structural or execution findings; 1 otherwise. Quality findings (`quality-*`) never cause a non-zero exit.

**Example — clean run:**

```bash
ah check
```

```json
{
  "scope": { "deployed": true, "changes": [] },
  "summary": { "structural": 0, "execution": 0, "passed": 3, "counts_by_kind": {} },
  "findings": []
}
```

**Example — with a staged change:**

```bash
ah check --changes add-parser-validation
```

```json
{
  "scope": { "deployed": true, "changes": ["add-parser-validation"] },
  "summary": { "structural": 0, "execution": 0, "passed": 4, "counts_by_kind": {} },
  "findings": []
}
```

**Example — with a finding:**

```json
{
  "scope": { "deployed": true, "changes": [] },
  "summary": { "structural": 1, "execution": 0, "passed": 0, "counts_by_kind": { "no-toml": 1 } },
  "findings": [
    {
      "kind": "no-toml",
      "category": "structural",
      "spec": "parser",
      "spec_path": "openspec/specs/parser/spec.md",
      "scenario": { "id": "empty-input-is-rejected", "title": "Empty input is rejected" },
      "suggested_action": "run_ah_scenario_new",
      "playbook_command": "ah explain run_ah_scenario_new"
    }
  ]
}
```

Run the `playbook_command` from any finding to get step-by-step guidance:

```bash
ah explain run_ah_scenario_new
```

**Finding kinds:**

| Kind | Category | Meaning |
|------|----------|---------|
| `no-toml` | structural | scenario has no matching contract file |
| `orphan-toml` | structural | contract exists without a matching scenario |
| `slug-collision` | structural | two scenarios in one spec have the same id |
| `id-mismatch` | structural | scenario slug, filename, and TOML `id` disagree |
| `no-tests-declared` | structural | contract has no runnable test entries |
| `missing-runner` | structural | a non-shell test type has no configured runner |
| `malformed-contract` | structural | TOML cannot be parsed or validated |
| `missing-replacement` | structural | superseded contract points to an absent replacement |
| `overlay-conflict` | structural | selected changes define conflicting staged scenarios |
| `test-failing` | execution | a declared test timed out or exited non-zero |
| `quality-mutation` | quality | mutation score meets threshold (informational) |
| `quality-property` | quality | property-based tests passing (informational) |
| `quality-snapshot` | quality | snapshot tests passing (informational) |

---

## `ah doctor`

Detect configured frameworks and diagnose config, path, hook, and archetype issues.

```
ah doctor [--enable <capability>]
```

**Flags:**

| Flag | Description |
|------|-------------|
| `--enable <capability>` | Write the config block for a detected-but-unconfigured capability |

**Capabilities for `--enable`:** `pytest`, `cargo`, `vitest`, `mutation`, `property`, `snapshot`

**Example output:**

```text
framework: pytest (configured)
recommendation: vitest detected via manifest — run: ah doctor --enable vitest
```

**Example — enable vitest:**

```bash
ah doctor --enable vitest
```

Writes the `[runners.vitest]` entry to `.espectacular/config.toml`. If already configured, exits 0 with an "already enabled" message.

**Exit codes:** 0 when no problems are found (recommendations do not affect exit code); 1 when structural problems exist. `--enable` exits 0 on success, non-zero for unknown capabilities.

---

## `ah explain`

Print guidance for a finding kind or suggested action.

```
ah explain [<topic>] [--list] [--json]
```

**Arguments:**

| Argument | Description |
|----------|-------------|
| `<topic>` | Finding kind or action slug to explain |

**Flags:**

| Flag | Description |
|------|-------------|
| `--list` | List all available topics |
| `--json` | Emit the topic list as JSON (use with `--list`) |

**Example — explain a finding:**

```bash
ah explain no-toml
```

```
## no-toml — Missing contract file

A scenario declared in a spec file has no corresponding contract .toml file
in .espectacular/<component>/.

How to fix: run `ah scenario new` with the spec, scenario id, and heading
to generate the contract stub, then populate the test entries.

    ah scenario new <change> <spec> --requirement <scenario-id> <heading>
```

**Example — list all topics:**

```bash
ah explain --list
```

**Exit codes:** 0 on success; 1 for unknown topics (includes "did you mean" suggestions).

---

## `ah signals`

Read [dont](https://github.com/charly-vibes/dont) rejection events and emit drift signals as JSON.

```
ah signals
```

`dont` is a companion tool that tracks epistemic claims made by AI agents. When an agent makes a claim that is later rejected, `dont` records it as an event. `ah signals` reads those events from `.dont/events/*.json` and re-emits them as structured `DriftSignal` JSON that CI or the [wai](https://github.com/charly-vibes/wai) project-context tool can consume to surface spec-behavior gaps.

**Exit codes:** always 0; returns an empty JSON array if no events are found.

---

## `ah type`

List built-in archetypes, or print full documentation for one.

```
ah type [<code>]
```

**Example — list all:**

```bash
ah type
```

```
PF — Pure Functional: Deterministic behavior where outputs are a function of explicit inputs.
SA — Stateful API: Behavior involving state transitions, persisted data, or ordered operations.
BP — Boundary Protocol: Behavior at an external boundary or protocol seam.
CE — Contract/Event: Behavior expressed as emitted events, messages, claims, or cross-tool signals.
NR — Non-Regression: Behavior asserting existing guarantees remain true while nearby changes land.
```

**Example — full docs for one archetype:**

```bash
ah type PF
```

```
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
```

**Exit codes:** 0 on success; 1 for unknown archetype codes (includes "did you mean" suggestions).

---

## `ah scenario new`

Append a new scenario to a spec and stage its TOML contract.

```
ah scenario new <change> <spec> --requirement "<scenario-id>" "<heading>"
```

**Arguments:**

| Argument | Description |
|----------|-------------|
| `<change>` | Change id (the change directory must exist under `openspec/changes/`) |
| `<spec>` | Spec name (e.g. `parser`) |
| `--requirement` | Scenario id slug — must match a `### Requirement:` block already in the spec |
| `<heading>` | Human-readable scenario heading to append |

Appends the scenario under the named requirement block and creates the contract stub at `.espectacular/changes/<change>/<spec>/<scenario-id>.toml`.

**Exit codes:** 0 on success; 1 if the change or spec is missing, or the requirement block is absent.

---

## `ah scenario supersede`

Stage a supersession update for an existing contract.

```
ah scenario supersede <spec> <old-id> --with=<new-id> --in-change=<change>
```

Marks the old contract `status = "superseded"` and sets `superseded_by` to the new scenario id. The replacement scenario must already exist in the named change overlay.

**Exit codes:** 0 on success; 1 if the deployed contract or replacement is missing.

---

## `ah archive`

Move staged change contracts into deployed `.espectacular/` locations.

```
ah archive <change>
```

Run after a change merges. Moves TOML files from `.espectacular/changes/<change>/` into `.espectacular/<spec>/`, failing if any collision would overwrite an active contract without a supersession in place.

**Exit codes:** 0 on success; 1 on collision or missing staged change.

---

## `ah upgrade`

Report tool-version drift and update `.espectacular/config.toml`.

```
ah upgrade
```

Compares `tool_version` in `.espectacular/config.toml` with the running binary version. Updates the config if they differ, then exits non-zero so CI can detect compatibility changes.

**Exit codes:** 0 when versions match; 1 when drift is detected (even after updating the config).
