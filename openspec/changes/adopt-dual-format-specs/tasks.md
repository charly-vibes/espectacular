# Tasks: adopt-dual-format-specs

## 1. Mirror deduplication in scenario discovery (TDD)

- [ ] 1.1 Write failing tests in `src/openspec.rs`: identical `(id, body)` headings in one file discover exactly one scenario; same id with different body still collides; frontmatter/table lines never yield scenarios
- [ ] 1.2 Implement mirror deduplication in `parse_scenarios_from_spec` / `detect_slug_collisions`; keep `extract_body` boundaries verbatim
- [ ] 1.3 Add scenario contracts for `deduplicate-mirrored-delta-sections` and `parse-dual-format-deployed-spec` under `.espectacular/gate/`
- [ ] 1.4 Update `.espectacular/gate/reject-duplicate-scenario-ids.toml` so its collision fixture uses headings with the same slug but **different bodies** (behavior change from the MODIFIED Scenario Discovery requirement)

## 2. Dual-format delta gates (pilot proof)

- [ ] 2.1 `spk lint openspec/changes/adopt-dual-format-specs/specs/gate/spec.md` exits zero
- [ ] 2.2 `openspec validate adopt-dual-format-specs --strict` exits zero
- [ ] 2.3 After 1.x: `ah check --changes adopt-dual-format-specs` reports **no** mirror-duplication conflicts and **no** no-toml findings
- [ ] 2.4 Known-expected residue: 2 overlay-conflicts for `discover-deployed-scenario` and `reject-duplicate-scenario-ids` — the overlay model cannot express MODIFIED requirement text (see beads `espectacular-hct`); they persist until that gap is resolved and are not a defect of this change

## 3. Authoring guidance

- [ ] 3.1 Document dual-format authoring (frontmatter block, table derivation, section-sync mirror) in `docs/` with this delta as the example
- [ ] 3.2 Record the opt-in enforcement decision (no CI mandate) in `docs/`
- [ ] 3.3 Document the same-id-different-body collision as an in-gate section-sync drift signal for dual-format repos

## 4. Archive round-trip (before archiving this change)

- [ ] 4.1 Verify `openspec archive` preserves dual-format files (frontmatter + four-layer tables) in deployed specs, or document that it strips the specodelic half and confirm C-tables-inert keeps discovery correct
