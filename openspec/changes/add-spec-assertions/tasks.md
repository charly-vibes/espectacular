## M0: Tracer bullet — one scenario, end-to-end

Goal: prove the full pipeline works for one hardcoded case before building the general-purpose parser.

- [ ] 0.1 Pick one real WHEN/THEN scenario from an existing openspec spec (wai or fabbro)
- [ ] 0.2 Hardcode the parser: extract that scenario into the assertion IR (YAML)
- [ ] 0.3 Hardcode the emitter: generate a Go (or Rust) test stub from the IR that compiles
- [ ] 0.4 Run the generated test stub — verify it compiles and the assertion is meaningful
- [ ] 0.5 Simulate drift: remove or change the code the scenario describes, verify the drift checker flags it
- [ ] 0.6 Validate convention-based mapping works for the target repo's test structure
- [ ] 0.7 Document: write the assertion IR schema (even if minimal) and the event format for cross-tool signals

Validates: RISK-001 (parseability), RISK-002 (convention mapping), overall pipeline viability.

## M1: Compiler — parser + IR + emitters

Goal: handle the full openspec scenario format and produce compilable test stubs for Go and Rust.

- [ ] 1.1 Implement spec parser: extract requirements and scenarios from openspec `spec.md` files
- [ ] 1.2 Handle ADDED/MODIFIED/REMOVED delta format from change specs
- [ ] 1.3 Resolve capability references across `specs/` directory
- [ ] 1.4 Validate parsed scenarios match WHEN/THEN format; report unparseable scenarios as warnings
- [ ] 1.5 Stabilize assertion IR schema (id, when, then, source location, archive date, capability)
- [ ] 1.6 Build IR generator from parsed specs with incremental updates (only changed specs)
- [ ] 1.7 Define emitter interface (IR → language-specific test code)
- [ ] 1.8 Implement Go emitter (generates `Test*` functions with assertion comments and source traceability)
- [ ] 1.9 Implement Rust emitter (generates `#[test]` stubs with assertion comments and source traceability)
- [ ] 1.10 Measure parse success rate across existing specs in wai, fabbro, fotos — target ≥80%

## M2: Drift detection

Goal: detect when code no longer matches compiled assertions.

- [ ] 2.1 Define drift report format (which assertions pass/fail/orphaned, structured JSON + human-readable)
- [ ] 2.2 Implement convention-based assertion-to-code mapping (test file naming ↔ spec capability)
- [ ] 2.3 Detect orphaned assertions (spec references code that no longer exists)
- [ ] 2.4 Detect failing assertions (code behavior diverged from spec)
- [ ] 2.5 Support optional annotation-based mapping (`#[spec("id")]`) as override for convention failures

## M3: Integration

Goal: wire espectacular into the openspec lifecycle and cross-tool feedback loops.

- [ ] 3.1 Hook into `openspec archive` — auto-generate/update assertions when a change is archived
- [ ] 3.2 Git hook adapter — run drift check when spec-referenced files change
- [ ] 3.3 CI integration — drift check as a pipeline step (exit code for pass/fail)
- [ ] 3.4 Cross-tool signals — emit structured events for wai (drift → stale artifact), dont (drift → ungrounded claim), pretender (drift pattern → constraint)

## M4: Distribution and documentation

Goal: ship espectacular as a standalone tool in the charly ecosystem.

- [ ] 4.1 Release workflow (GitHub Actions, homebrew-charly formula, scoop-charly manifest)
- [ ] 4.2 CLI interface: `espectacular compile`, `espectacular drift`, `espectacular report`
- [ ] 4.3 Usage examples and ecosystem integration guide in README
