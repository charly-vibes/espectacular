use super::*;

const FIXTURE: &str = "tests/fixtures/simple/openspec/specs";
const COLLISION_FIXTURE: &str = "tests/fixtures/collision/openspec/specs";

#[test]
fn missing_dir_error_names_the_path() {
    // espectacular-iq4 / GH#28.2: a bare "No such file or directory (os
    // error 2)" cost the reporter an hour — any IO failure from this
    // function must name the directory it was reading.
    let err = discover_scenarios("/nonexistent/ah-iq4-specs").unwrap_err();
    let msg = format!("{err:#}");
    assert!(
        msg.contains("/nonexistent/ah-iq4-specs"),
        "error must name the specs dir; got: {msg}"
    );
}

#[test]
fn discovers_scenarios_from_headings() {
    let scenarios = discover_scenarios(FIXTURE).unwrap();
    assert_eq!(scenarios.len(), 2);
}

#[test]
fn scenario_id_is_slugified_heading() {
    let scenarios = discover_scenarios(FIXTURE).unwrap();
    let ids: Vec<&str> = scenarios.iter().map(|s| s.id.as_str()).collect();
    assert!(ids.contains(&"empty-input-rejected"));
    assert!(ids.contains(&"null-bytes-rejected"));
}

#[test]
fn scenario_heading_preserved_verbatim() {
    let scenarios = discover_scenarios(FIXTURE).unwrap();
    let found = scenarios
        .iter()
        .find(|s| s.id == "empty-input-rejected")
        .unwrap();
    assert_eq!(found.heading, "Empty input rejected");
}

#[test]
fn scenario_spec_path_is_relative_spec_name() {
    let scenarios = discover_scenarios(FIXTURE).unwrap();
    let found = scenarios
        .iter()
        .find(|s| s.id == "empty-input-rejected")
        .unwrap();
    assert_eq!(found.spec_path, "compiler");
}

#[test]
fn scenario_source_line_is_one_based() {
    let scenarios = discover_scenarios(FIXTURE).unwrap();
    let found = scenarios
        .iter()
        .find(|s| s.id == "empty-input-rejected")
        .unwrap();
    // heading is at line 7 in the fixture
    assert_eq!(found.source_line, 7);
}

#[test]
fn scenario_body_contains_given_when_then() {
    let scenarios = discover_scenarios(FIXTURE).unwrap();
    let found = scenarios
        .iter()
        .find(|s| s.id == "empty-input-rejected")
        .unwrap();
    assert!(found.body.contains("GIVEN"));
    assert!(found.body.contains("WHEN"));
    assert!(found.body.contains("THEN"));
}

#[test]
fn slugify_lowercases_and_hyphens() {
    assert_eq!(slugify("Empty input rejected"), "empty-input-rejected");
}

#[test]
fn slugify_strips_non_alphanumeric() {
    assert_eq!(slugify("Hello, World!"), "hello-world");
}

#[test]
fn slugify_collapses_repeated_separators() {
    assert_eq!(slugify("foo  --  bar"), "foo-bar");
}

#[test]
fn slugify_trims_leading_trailing_separators() {
    assert_eq!(slugify(" -- foo -- "), "foo");
}

// 1.4 RED: collision detection
#[test]
fn no_collisions_in_clean_fixture() {
    let scenarios = discover_scenarios(FIXTURE).unwrap();
    let collisions = detect_slug_collisions(&scenarios);
    assert!(collisions.is_empty());
}

#[test]
fn detects_slug_collision() {
    let scenarios = discover_scenarios(COLLISION_FIXTURE).unwrap();
    let collisions = detect_slug_collisions(&scenarios);
    // Both scenarios share the same slug, so both are flagged
    assert_eq!(collisions.len(), 2);
}

#[test]
fn collision_tuple_contains_spec_and_id() {
    let scenarios = discover_scenarios(COLLISION_FIXTURE).unwrap();
    let collisions = detect_slug_collisions(&scenarios);
    let (spec, id, _heading) = &collisions[0];
    assert_eq!(spec, "compiler");
    assert_eq!(id, "empty-input-rejected");
}

// adopt-dual-format-specs 1.1 RED: mirror deduplication
#[test]
fn mirror_identical_id_and_body_yields_one_scenario() {
    let content = "\
## ADDED Requirements

### Requirement: Sample
#### Scenario: Empty input rejected
- **GIVEN** empty input
- **WHEN** submitted
- **THEN** rejected

## Requirements

### Requirement: Sample
#### Scenario: Empty input rejected
- **GIVEN** empty input
- **WHEN** submitted
- **THEN** rejected
";
    let scenarios = parse_scenarios_from_spec(content, "compiler");
    assert_eq!(
        scenarios.len(),
        1,
        "mirrored identical (id, body) must dedupe"
    );
    assert_eq!(scenarios[0].id, "empty-input-rejected");
}

#[test]
fn mirror_dedupe_keeps_first_occurrence_source_line() {
    let content = "\
## ADDED Requirements

#### Scenario: Empty input rejected
- **GIVEN** empty input
- **WHEN** submitted
- **THEN** rejected

## Requirements

#### Scenario: Empty input rejected
- **GIVEN** empty input
- **WHEN** submitted
- **THEN** rejected
";
    let scenarios = parse_scenarios_from_spec(content, "compiler");
    assert_eq!(scenarios.len(), 1);
    // heading is on line 3 (first occurrence)
    assert_eq!(scenarios[0].source_line, 3);
}

#[test]
fn same_id_different_body_still_collides() {
    let content = "\
#### Scenario: Empty input rejected
- **GIVEN** empty input
- **WHEN** submitted
- **THEN** rejected

#### Scenario: Empty input rejected
- **GIVEN** null bytes
- **WHEN** submitted
- **THEN** rejected
";
    let scenarios = parse_scenarios_from_spec(content, "compiler");
    assert_eq!(scenarios.len(), 2, "distinct bodies are distinct scenarios");
    let collisions = detect_slug_collisions(&scenarios);
    assert_eq!(
        collisions.len(),
        2,
        "both sides of a distinct-body collision are flagged"
    );
}

#[test]
fn frontmatter_and_table_rows_never_yield_scenarios() {
    let content = "\
---
id: spec
statement: \"#### Scenario: not-a-scenario\"
---

## Constraints

| id | kind | expr | traces_to |
|----|------|------|-----------|
| C1 | invariant | #### Scenario: table-row-fake | |

## Requirements

#### Scenario: Empty input rejected
- **GIVEN** empty input
- **WHEN** submitted
- **THEN** rejected
";
    let scenarios = parse_scenarios_from_spec(content, "compiler");
    let ids: Vec<&str> = scenarios.iter().map(|s| s.id.as_str()).collect();
    assert_eq!(
        ids,
        vec!["empty-input-rejected"],
        "only real headings count"
    );
}

#[test]
fn plain_openspec_discovery_unchanged() {
    // P-optin: a file with no frontmatter and no specodelic tables parses
    // exactly as before (both scenarios discovered).
    let content = "\
## Requirements

#### Scenario: Empty input rejected
- **GIVEN** empty input
- **WHEN** submitted
- **THEN** rejected

#### Scenario: Null bytes rejected
- **GIVEN** null bytes
- **WHEN** submitted
- **THEN** rejected
";
    let scenarios = parse_scenarios_from_spec(content, "compiler");
    assert_eq!(scenarios.len(), 2);
}
// ---- property-derived contract bindings (DDL migration, design D6) ----

#[test]
fn p_discovery() {
    discovers_scenarios_from_headings();
    detects_slug_collision();
    no_collisions_in_clean_fixture();
    mirror_identical_id_and_body_yields_one_scenario();
    mirror_dedupe_keeps_first_occurrence_source_line();
    frontmatter_and_table_rows_never_yield_scenarios();
    plain_openspec_discovery_unchanged();
}
