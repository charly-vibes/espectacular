use std::fs;

#[test]
fn llm_txt_documents_scenario_as_four_hash_headings() {
    let content = fs::read_to_string("llm.txt").expect("llm.txt should exist at project root");
    // The Scenario definition must correctly identify #### Scenario: as the discovery heading,
    // not ### Requirement: which is only a grouping label.
    let scenario_line = content
        .lines()
        .find(|l| l.starts_with("- **Scenario**"))
        .expect("llm.txt should have a Scenario definition entry");
    assert!(
        scenario_line.contains("#### Scenario:"),
        "Scenario definition should reference '#### Scenario:' as the discovery heading, got: {scenario_line}"
    );
    // ### Requirement: may appear as the parent grouping reference, but the
    // scenario itself must NOT be described as a "### Requirement:" heading.
    assert!(
        !scenario_line.contains("one `### Requirement:` heading"),
        "Scenario definition should NOT describe the scenario as a '### Requirement:' heading, got: {scenario_line}"
    );
}

#[test]
fn llm_txt_documents_spec_as_requirement_grouping() {
    let content = fs::read_to_string("llm.txt").expect("llm.txt should exist at project root");
    // The Spec definition should mention ### Requirement: as a grouping label
    let spec_line = content
        .lines()
        .find(|l| l.starts_with("- **Spec**"))
        .expect("llm.txt should have a Spec definition entry");
    assert!(
        spec_line.contains("### Requirement:"),
        "Spec definition should reference '### Requirement:' as a grouping, got: {spec_line}"
    );
}
