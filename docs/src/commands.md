# Command Reference

## `ah init`

Create or refresh `.espectacular/` files and hook integration.

```
ah init
```

Idempotent — safe to re-run after updating specs or changing hook frameworks. Stubs contract files for any scenarios that have no existing contract. Installs `ah check` into `lefthook` or `prek` if detected.

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

**Example — deployed specs only:**

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

Use `ah explain <kind>` for guidance on any finding kind.

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

Writes the `[runners.vitest]` entry to `.espectacular/config.toml`. If the capability is already configured, exits 0 with an "already enabled" message.

**Exit codes:** 0 when no problems are found (framework recommendations do not affect exit code); 1 when structural problems exist. `--enable` exits 0 on success, non-zero for unknown capabilities.

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

Read `dont` rejection events and emit drift signals as JSON.

```
ah signals
```

Reads `.dont/events/*.json`, filters for ungrounded claim rejections, and emits a JSON array of `DriftSignal` objects. Used by `wai` to surface spec-behavior gaps.

**Exit codes:** always 0; returns an empty array if no events are found.

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
| `<change>` | Change id (must exist under `openspec/changes/`) |
| `<spec>` | Spec name (e.g. `parser`) |
| `--requirement` | Scenario id slug (must match a `### Requirement:` block in the spec) |
| `<heading>` | Human-readable scenario heading |

Appends the scenario under the named requirement block and creates the contract stub at `.espectacular/changes/<change>/<spec>/<scenario-id>.toml`.

**Exit codes:** 0 on success; 1 if the change or spec is missing, or the requirement block is absent.

---

## `ah scenario supersede`

Stage a supersession update for an existing contract.

```
ah scenario supersede <spec> <old-id> --with=<new-id> --in-change=<change>
```

Marks the old contract `status = "superseded"` and sets `superseded_by` to the new scenario id. The replacement scenario must exist in the named change overlay.

**Exit codes:** 0 on success; 1 if the deployed contract or replacement is missing.

---

## `ah archive`

Move staged change contracts into deployed `.espectacular/` locations.

```
ah archive <change>
```

Runs after a change is merged. Moves TOML files from `.espectacular/changes/<change>/` into `.espectacular/<spec>/`, failing if any collision would overwrite an active contract without a supersession.

**Exit codes:** 0 on success; 1 on collision or missing staged change.

---

## `ah upgrade`

Report tool-version drift and update `.espectacular/config.toml`.

```
ah upgrade
```

Compares `tool_version` in `.espectacular/config.toml` with the running binary version. Updates the config if they differ, then exits non-zero so CI can detect compatibility changes.

**Exit codes:** 0 when versions match; 1 when drift is detected (even after updating the config).
