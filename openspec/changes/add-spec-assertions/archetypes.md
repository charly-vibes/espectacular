# Test Archetypes as Structural Frames

To enable AI agents to implement test assertions reliably, `espectacular` emits **Structural Frames**. These are skeletal test stubs that define the "where" and "what" without prescribing the "how". This keeps the tool language-agnostic and robust.

## 1. Pure Functional (PF) [Frame]
- **Description:** A structure for testing deterministic logic.
- **Focus:** Mapping inputs to outputs.
- **Stub Goal:** Provide a function signature and clear `WHEN`/`THEN` comment blocks.

### Rust PF Frame
```rust
#[test]
fn test_scenario_id() {
    // [espectacular:PF]
    // WHEN: user clicks the calendar toggle
    // THEN: the display switches to Holocene Era format

    // TODO: Implement functional assertion
}
```

## 2. Stateful API (SA) [Frame]
- **Description:** A structure for testing state transitions or API calls.
- **Focus:** Setup, Action, and Verification.
- **Stub Goal:** Organize the test into logical phases for an LLM to fill.

### Go SA Frame
```go
func TestScenarioID(t *testing.T) {
    // [espectacular:SA]

    // 1. SETUP
    // TODO: Initialize state

    // 2. WHEN: user clicks the calendar toggle
    // TODO: Perform action

    // 3. THEN: the display switches to Holocene Era format
    // TODO: Assert state change
}
```

## 3. Boundary / Protocol (BP) [Frame]
- **Description:** A structure for testing external interfaces or strict contracts.
- **Focus:** Mocking and Contract verification.
- **Implementation Hint:** In Rust, use `mockall`. In Go, use interfaces and mocks.

## 4. CLI / E2E (CE) [Frame]
- **Description:** A structure for testing full tool invocations or binary behavior.
- **Focus:** Command execution, stdout/stderr, and exit codes.
- **Stub Goal:** Provide a setup for running the binary and asserting on its output.

### Rust CE Frame
```rust
#[test]
fn test_scenario_id() {
    // [espectacular:CE]
    let mut cmd = Command::cargo_bin("ah").unwrap();

    // WHEN: user runs 'ah status'
    let assert = cmd.arg("status").assert();

    // THEN: the output contains 'Ready'
    assert.success().stdout(predicate::str::contains("Ready"));
}
```

## 5. Property-Based / Fuzzing [Hint]
If a scenario is tagged with `[pbt]` or `[fuzz]`, the frame includes metadata hints for the LLM to choose a PBT/Fuzzing library.

- **PF + [pbt]:** Use `proptest` (Rust) or `testing/quick` (Go).
- **SA + [fuzz]:** Use `cargo-fuzz` (Rust) or `go test -fuzz` (Go).
