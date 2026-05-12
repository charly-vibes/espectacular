## M0: Tracer bullet — one scenario, end-to-end

Goal: prove the full pipeline works for one hardcoded case before building the general-purpose parser.

- [ ] 0.1 Pick one real WHEN/THEN scenario from an existing openspec spec (wai or fabbro)
- [ ] 0.2 Hardcode the parser: extract that scenario into the assertion IR (JSON)
- [ ] 0.3 Hardcode the emitter: generate a Go (or Rust) **Structural Frame** from the IR
- [ ] 0.4 Manually implement the frame — verify it compiles and runs
- [ ] 0.5 Simulate drift: change the code so the test fails; verify the drift checker flags it using test results (JUnit XML)
- [ ] 0.6 Validate convention-based mapping works for the target repo's test structure
- [ ] 0.7 Document: write the assertion IR JSON Schema and the event format for cross-tool signals

Validates: RISK-001 (parseability), RISK-002 (convention mapping), overall pipeline viability.

## M1: Compiler — parser + IR + emitters

Goal: handle the full openspec scenario format and produce **Structural Frames** for Go and Rust.

- [ ] 1.1 Implement spec parser: extract requirements and scenarios from openspec `spec.md` files
- [ ] 1.2 Handle archetype tags (e.g. `[PF]`) and metadata blocks in scenarios
- [ ] 1.3 Handle ADDED/MODIFIED/REMOVED delta format from change specs
- [ ] 1.4 Resolve capability references across `specs/` directory
- [ ] 1.5 Stabilize assertion IR JSON Schema (id, archetype, when, then, source location, archive date)
- [ ] 1.6 Build IR generator from parsed specs with incremental updates (only changed specs)
- [ ] 1.7 Define template-based emitter system (e.g. Handlebars)
- [ ] 1.8 Implement Go emitter (generates `Test*` structural frames with TODOs and metadata tags)
- [ ] 1.9 Implement Rust emitter (generates `#[test]` structural frames with TODOs and metadata tags)
- [ ] 1.10 Measure parse success rate across existing specs in wai, fabbro, fotos — target ≥80%

## M2: Drift detection

Goal: detect when code no longer matches compiled assertions via result ingestion.

- [ ] 2.1 Define drift report format (which assertions pass/fail/orphaned, structured JSON + human-readable)
- [ ] 2.2 Implement result parser for JUnit XML and TAP formats
- [ ] 2.3 Implement convention-based assertion-to-code mapping (test function names ↔ assertion IDs) and define ID normalization rules for test symbol generation
- [ ] 2.4 Detect orphaned assertions (spec references code that no longer exists in test results)
- [ ] 2.5 Detect failing assertions (test results show failure for a mapped assertion)
- [ ] 2.6 Support optional annotation-based mapping as override for convention failures

## M3: Integration

Goal: wire the `ah` CLI into the openspec lifecycle and cross-tool feedback loops.

- [ ] 3.1 Hook into `openspec archive` — auto-generate/update assertions when a change is archived
- [ ] 3.2 Git hook adapter — run drift check when spec-referenced files change
- [ ] 3.3 CI integration — drift check as a pipeline step (exit code for pass/fail)
- [ ] 3.4 Cross-tool signals — emit structured events for wai (drift → stale artifact), dont (drift → ungrounded claim), pretender (drift pattern → constraint)

## M4: Distribution and documentation

Goal: ship `ah` as the standalone CLI for espectacular in the charly ecosystem.

- [ ] 4.1 Release workflow (GitHub Actions, homebrew-charly formula, scoop-charly manifest)
- [ ] 4.2 CLI interface: `ah compile`, `ah drift`, `ah report`
- [ ] 4.3 Usage examples and ecosystem integration guide in README
