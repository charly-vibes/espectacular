use crate::openspec::{discover_scenarios, slugify};
use anyhow::Context;
use std::path::Path;

#[derive(Debug)]
pub struct ScenarioNewResult {
    pub scenario_path: String,
    pub contract_path: String,
}

#[derive(Debug)]
pub struct ScenarioSupersededResult {
    pub contract_path: String,
}

pub fn run_scenario_new(
    repo_root: &Path,
    change: &str,
    spec: &str,
    requirement: &str,
    heading: &str,
) -> anyhow::Result<ScenarioNewResult> {
    let spec_path = repo_root
        .join("openspec/changes")
        .join(change)
        .join("specs")
        .join(spec)
        .join("spec.md");
    anyhow::ensure!(
        spec_path.exists(),
        "change spec not found: openspec/changes/{change}/specs/{spec}/spec.md"
    );

    let content = std::fs::read_to_string(&spec_path)
        .with_context(|| format!("cannot read {}", spec_path.display()))?;

    let req_needle = format!("### Requirement: {requirement}");
    let req_pos = content
        .find(&req_needle)
        .ok_or_else(|| anyhow::anyhow!("requirement '{requirement}' not found in spec"))?;

    // Find the end of this requirement's section (next ### or EOF)
    let section_start = req_pos + req_needle.len();
    let section_end = content[section_start..]
        .find("\n### ")
        .map(|p| section_start + p)
        .unwrap_or(content.len());

    let section = &content[section_start..section_end];
    let slug = slugify(heading);
    let collision_needle = format!("#### Scenario: {heading}");
    anyhow::ensure!(
        !section.contains(&collision_needle),
        "scenario '{heading}' already exists under requirement '{requirement}'"
    );

    let skeleton = format!(
        "\n\n#### Scenario: {heading}\n- **WHEN** [describe the action or condition]\n- **THEN** [describe the expected observable result]"
    );
    let new_content = format!(
        "{}{}{}",
        &content[..section_end],
        skeleton,
        &content[section_end..]
    );
    std::fs::write(&spec_path, &new_content)
        .with_context(|| format!("cannot write {}", spec_path.display()))?;

    let contract_dir = repo_root
        .join(".espectacular/changes")
        .join(change)
        .join(spec);
    std::fs::create_dir_all(&contract_dir)?;

    let version = env!("CARGO_PKG_VERSION");
    let toml_content = format!(
        "id = \"{slug}\"\ndescription = \"\"\narchetype = \"\"\nstatus = \"active\"\nsuperseded_by = \"\"\nauthored_with = \"{version}\"\n"
    );
    let contract_file = contract_dir.join(format!("{slug}.toml"));
    std::fs::write(&contract_file, &toml_content)
        .with_context(|| format!("cannot write {}", contract_file.display()))?;

    let scenario_rel = format!("openspec/changes/{change}/specs/{spec}/spec.md");
    let contract_rel = format!(".espectacular/changes/{change}/{spec}/{slug}.toml");
    Ok(ScenarioNewResult {
        scenario_path: scenario_rel,
        contract_path: contract_rel,
    })
}

pub fn run_scenario_supersede(
    repo_root: &Path,
    spec: &str,
    old_id: &str,
    new_id: &str,
    change: &str,
) -> anyhow::Result<ScenarioSupersededResult> {
    // Validate new_id exists in deployed-plus-change scope
    let deployed_specs_dir = repo_root.join("openspec/specs");
    let change_specs_dir = repo_root
        .join("openspec/changes")
        .join(change)
        .join("specs");

    let mut all_ids: Vec<String> = Vec::new();
    if deployed_specs_dir.exists() {
        if let Ok(scenarios) = discover_scenarios(deployed_specs_dir.to_str().unwrap_or("")) {
            all_ids.extend(
                scenarios
                    .into_iter()
                    .filter(|s| s.spec_path == spec)
                    .map(|s| s.id),
            );
        }
    }
    if change_specs_dir.exists() {
        if let Ok(scenarios) = discover_scenarios(change_specs_dir.to_str().unwrap_or("")) {
            all_ids.extend(
                scenarios
                    .into_iter()
                    .filter(|s| s.spec_path == spec)
                    .map(|s| s.id),
            );
        }
    }

    let new_exists = all_ids.iter().any(|id| id == new_id);
    anyhow::ensure!(
        new_exists,
        "replacement scenario '{new_id}' not found in deployed or change scope for '{spec}'"
    );

    // Find deployed contract
    let deployed_contract = repo_root
        .join(".espectacular")
        .join(spec)
        .join(format!("{old_id}.toml"));
    anyhow::ensure!(
        deployed_contract.exists(),
        "deployed contract not found: .espectacular/{spec}/{old_id}.toml"
    );

    let text = std::fs::read_to_string(&deployed_contract)
        .with_context(|| format!("cannot read {}", deployed_contract.display()))?;
    let mut value: toml::Value =
        toml::from_str(&text).with_context(|| "invalid TOML in deployed contract")?;

    if let Some(table) = value.as_table_mut() {
        table.insert("status".into(), toml::Value::String("superseded".into()));
        table.insert(
            "superseded_by".into(),
            toml::Value::String(new_id.to_string()),
        );
    }

    let dest_dir = repo_root
        .join(".espectacular/changes")
        .join(change)
        .join(spec);
    std::fs::create_dir_all(&dest_dir)?;

    let dest_file = dest_dir.join(format!("{old_id}.toml"));
    let out = toml::to_string_pretty(&value)?;
    std::fs::write(&dest_file, &out)
        .with_context(|| format!("cannot write {}", dest_file.display()))?;

    let contract_rel = format!(".espectacular/changes/{change}/{spec}/{old_id}.toml");
    Ok(ScenarioSupersededResult {
        contract_path: contract_rel,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    fn make_repo_with_change(change: &str, spec: &str, requirement: &str) -> TempDir {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path();

        let spec_dir = root
            .join("openspec/changes")
            .join(change)
            .join("specs")
            .join(spec);
        fs::create_dir_all(&spec_dir).unwrap();

        let spec_md = format!(
            "# Capability: {spec}\n\n## ADDED Requirements\n\n### Requirement: {requirement}\n"
        );
        fs::write(spec_dir.join("spec.md"), &spec_md).unwrap();
        dir
    }

    fn make_repo_with_deployed_scenario(spec: &str, scenario_id: &str) -> TempDir {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path();

        // Deployed spec
        let deployed_spec_dir = root.join("openspec/specs").join(spec);
        fs::create_dir_all(&deployed_spec_dir).unwrap();
        let heading = scenario_id.replace('-', " ");
        let spec_md = format!(
            "# Capability: {spec}\n\n## Requirements\n\n### Requirement: Core\n\n#### Scenario: {heading}\n- **WHEN** action\n- **THEN** result\n"
        );
        fs::write(deployed_spec_dir.join("spec.md"), &spec_md).unwrap();

        // Deployed contract
        let contract_dir = root.join(".espectacular").join(spec);
        fs::create_dir_all(&contract_dir).unwrap();
        let contract = format!(
            "id = \"{scenario_id}\"\ndescription = \"Test scenario.\"\narchetype = \"PF\"\nstatus = \"active\"\nsuperseded_by = \"\"\nauthored_with = \"0.2.2\"\n\n[[tests.unit]]\nflags = \"tests/test_foo.py\"\ntimeout_seconds = 60\n"
        );
        fs::write(contract_dir.join(format!("{scenario_id}.toml")), &contract).unwrap();
        dir
    }

    // ── 4.9 RED: ah scenario new happy-path ──────────────────────────────────

    #[test]
    fn new_scenario_creates_markdown_skeleton() {
        let dir = make_repo_with_change("s5", "compiler", "Input validation");
        let result = run_scenario_new(
            dir.path(),
            "s5",
            "compiler",
            "Input validation",
            "Null bytes rejected",
        )
        .unwrap();

        let spec_path = dir.path().join(&result.scenario_path);
        let content = fs::read_to_string(&spec_path).unwrap();
        assert!(
            content.contains("#### Scenario: Null bytes rejected"),
            "skeleton heading missing"
        );
        assert!(
            content.contains("- **WHEN** [describe the action or condition]"),
            "WHEN placeholder missing"
        );
        assert!(
            content.contains("- **THEN** [describe the expected observable result]"),
            "THEN placeholder missing"
        );
    }

    #[test]
    fn new_scenario_creates_staged_contract() {
        let dir = make_repo_with_change("s5", "compiler", "Input validation");
        let result = run_scenario_new(
            dir.path(),
            "s5",
            "compiler",
            "Input validation",
            "Null bytes rejected",
        )
        .unwrap();

        let contract_path = dir.path().join(&result.contract_path);
        assert!(contract_path.exists(), "contract file not created");
        assert_eq!(
            result.contract_path,
            ".espectacular/changes/s5/compiler/null-bytes-rejected.toml"
        );
    }

    #[test]
    fn new_scenario_toml_has_correct_defaults() {
        let dir = make_repo_with_change("s5", "compiler", "Input validation");
        let result = run_scenario_new(
            dir.path(),
            "s5",
            "compiler",
            "Input validation",
            "Null bytes rejected",
        )
        .unwrap();

        let contract_path = dir.path().join(&result.contract_path);
        let text = fs::read_to_string(&contract_path).unwrap();
        assert!(text.contains("id = \"null-bytes-rejected\""), "id wrong");
        assert!(text.contains("status = \"active\""), "status wrong");
        assert!(text.contains("superseded_by = \"\""), "superseded_by wrong");
        assert!(text.contains("description = \"\""), "description wrong");
        assert!(text.contains("archetype = \"\""), "archetype wrong");
        assert!(
            text.contains("authored_with = \"0.2.2\""),
            "authored_with wrong"
        );
    }

    #[test]
    fn new_scenario_appends_after_existing_scenarios() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path();
        let spec_dir = root.join("openspec/changes/s5/specs/compiler");
        fs::create_dir_all(&spec_dir).unwrap();
        let initial = "# Capability: compiler\n\n## ADDED Requirements\n\n### Requirement: Input validation\n\n#### Scenario: Empty input rejected\n- **WHEN** a\n- **THEN** b\n\n### Requirement: Other\n";
        fs::write(spec_dir.join("spec.md"), initial).unwrap();

        run_scenario_new(
            root,
            "s5",
            "compiler",
            "Input validation",
            "Null bytes rejected",
        )
        .unwrap();

        let content = fs::read_to_string(spec_dir.join("spec.md")).unwrap();
        let pos_empty = content.find("Empty input rejected").unwrap();
        let pos_null = content.find("Null bytes rejected").unwrap();
        let pos_other = content.find("### Requirement: Other").unwrap();
        assert!(pos_empty < pos_null, "new scenario before existing");
        assert!(pos_null < pos_other, "new scenario after Other section");
    }

    #[test]
    fn new_scenario_returns_correct_paths() {
        let dir = make_repo_with_change("s5", "compiler", "Input validation");
        let result = run_scenario_new(
            dir.path(),
            "s5",
            "compiler",
            "Input validation",
            "Null bytes rejected",
        )
        .unwrap();

        assert_eq!(
            result.scenario_path,
            "openspec/changes/s5/specs/compiler/spec.md"
        );
        assert_eq!(
            result.contract_path,
            ".espectacular/changes/s5/compiler/null-bytes-rejected.toml"
        );
    }

    // ── 4.11 RED: non-destructive failure ────────────────────────────────────

    #[test]
    fn new_scenario_fails_when_change_spec_missing() {
        let dir = tempfile::tempdir().unwrap();
        let err = run_scenario_new(
            dir.path(),
            "s5",
            "compiler",
            "Input validation",
            "Null bytes rejected",
        )
        .unwrap_err();
        assert!(
            err.to_string().contains("change spec not found"),
            "wrong error: {err}"
        );
    }

    #[test]
    fn new_scenario_fails_when_requirement_missing() {
        let dir = make_repo_with_change("s5", "compiler", "Input validation");
        let err = run_scenario_new(
            dir.path(),
            "s5",
            "compiler",
            "Nonexistent requirement",
            "Some scenario",
        )
        .unwrap_err();
        assert!(
            err.to_string().contains("not found in spec"),
            "wrong error: {err}"
        );
    }

    #[test]
    fn new_scenario_fails_on_collision() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path();
        let spec_dir = root.join("openspec/changes/s5/specs/compiler");
        fs::create_dir_all(&spec_dir).unwrap();
        let initial = "# Capability\n\n### Requirement: Input validation\n\n#### Scenario: Null bytes rejected\n- **WHEN** a\n- **THEN** b\n";
        fs::write(spec_dir.join("spec.md"), initial).unwrap();

        let err = run_scenario_new(
            root,
            "s5",
            "compiler",
            "Input validation",
            "Null bytes rejected",
        )
        .unwrap_err();
        assert!(
            err.to_string().contains("already exists"),
            "wrong error: {err}"
        );
    }

    #[test]
    fn new_scenario_writes_nothing_on_failure() {
        let dir = tempfile::tempdir().unwrap();
        let _ = run_scenario_new(
            dir.path(),
            "s5",
            "compiler",
            "Input validation",
            "Some heading",
        );
        let contract_dir = dir.path().join(".espectacular/changes/s5/compiler");
        assert!(
            !contract_dir.exists(),
            "contract dir should not be created on failure"
        );
    }

    // ── 4.13 RED: ah scenario supersede ──────────────────────────────────────

    #[test]
    fn supersede_creates_overlay_contract() {
        let dir = make_repo_with_deployed_scenario("compiler", "empty-input-rejected");
        // Add new scenario to deployed spec so new_id is in scope
        let deployed_spec = dir.path().join("openspec/specs/compiler/spec.md");
        let mut content = fs::read_to_string(&deployed_spec).unwrap();
        content.push_str("#### Scenario: Null bytes rejected\n- **WHEN** x\n- **THEN** y\n");
        fs::write(&deployed_spec, &content).unwrap();

        let result = run_scenario_supersede(
            dir.path(),
            "compiler",
            "empty-input-rejected",
            "null-bytes-rejected",
            "s5",
        )
        .unwrap();

        let dest = dir.path().join(&result.contract_path);
        assert!(dest.exists(), "overlay contract not created");
        assert_eq!(
            result.contract_path,
            ".espectacular/changes/s5/compiler/empty-input-rejected.toml"
        );
    }

    #[test]
    fn supersede_sets_status_superseded() {
        let dir = make_repo_with_deployed_scenario("compiler", "empty-input-rejected");
        let deployed_spec = dir.path().join("openspec/specs/compiler/spec.md");
        let mut content = fs::read_to_string(&deployed_spec).unwrap();
        content.push_str("#### Scenario: Null bytes rejected\n- **WHEN** x\n- **THEN** y\n");
        fs::write(&deployed_spec, &content).unwrap();

        let result = run_scenario_supersede(
            dir.path(),
            "compiler",
            "empty-input-rejected",
            "null-bytes-rejected",
            "s5",
        )
        .unwrap();

        let dest = dir.path().join(&result.contract_path);
        let text = fs::read_to_string(&dest).unwrap();
        assert!(
            text.contains("status = \"superseded\""),
            "status not set: {text}"
        );
    }

    #[test]
    fn supersede_sets_superseded_by() {
        let dir = make_repo_with_deployed_scenario("compiler", "empty-input-rejected");
        let deployed_spec = dir.path().join("openspec/specs/compiler/spec.md");
        let mut content = fs::read_to_string(&deployed_spec).unwrap();
        content.push_str("#### Scenario: Null bytes rejected\n- **WHEN** x\n- **THEN** y\n");
        fs::write(&deployed_spec, &content).unwrap();

        let result = run_scenario_supersede(
            dir.path(),
            "compiler",
            "empty-input-rejected",
            "null-bytes-rejected",
            "s5",
        )
        .unwrap();

        let dest = dir.path().join(&result.contract_path);
        let text = fs::read_to_string(&dest).unwrap();
        assert!(
            text.contains("null-bytes-rejected"),
            "superseded_by not set: {text}"
        );
    }

    #[test]
    fn supersede_fails_when_new_id_not_in_scope() {
        let dir = make_repo_with_deployed_scenario("compiler", "empty-input-rejected");
        let err = run_scenario_supersede(
            dir.path(),
            "compiler",
            "empty-input-rejected",
            "nonexistent-scenario",
            "s5",
        )
        .unwrap_err();
        assert!(
            err.to_string()
                .contains("not found in deployed or change scope"),
            "wrong error: {err}"
        );
    }

    #[test]
    fn supersede_fails_when_deployed_contract_missing() {
        let dir = make_repo_with_deployed_scenario("compiler", "empty-input-rejected");
        let deployed_spec = dir.path().join("openspec/specs/compiler/spec.md");
        let mut content = fs::read_to_string(&deployed_spec).unwrap();
        content.push_str("#### Scenario: Null bytes rejected\n- **WHEN** x\n- **THEN** y\n");
        fs::write(&deployed_spec, &content).unwrap();

        let err = run_scenario_supersede(
            dir.path(),
            "compiler",
            "nonexistent-old-scenario",
            "null-bytes-rejected",
            "s5",
        )
        .unwrap_err();
        assert!(
            err.to_string().contains("deployed contract not found"),
            "wrong error: {err}"
        );
    }

    #[test]
    fn supersede_writes_nothing_on_failure() {
        let dir = make_repo_with_deployed_scenario("compiler", "empty-input-rejected");
        let _ = run_scenario_supersede(
            dir.path(),
            "compiler",
            "empty-input-rejected",
            "nonexistent-scenario",
            "s5",
        );
        let dest_dir = dir.path().join(".espectacular/changes/s5/compiler");
        assert!(
            !dest_dir.exists(),
            "overlay dir should not exist on failure"
        );
    }

    #[test]
    fn supersede_finds_new_id_in_change_overlay() {
        let dir = make_repo_with_deployed_scenario("compiler", "empty-input-rejected");

        // Add new scenario only in change overlay (not deployed)
        let change_spec_dir = dir.path().join("openspec/changes/s5/specs/compiler");
        fs::create_dir_all(&change_spec_dir).unwrap();
        let change_spec = "# Capability\n\n### Requirement: Core\n\n#### Scenario: Brand new scenario\n- **WHEN** x\n- **THEN** y\n";
        fs::write(change_spec_dir.join("spec.md"), change_spec).unwrap();

        let result = run_scenario_supersede(
            dir.path(),
            "compiler",
            "empty-input-rejected",
            "brand-new-scenario",
            "s5",
        )
        .unwrap();

        let dest = dir.path().join(&result.contract_path);
        assert!(dest.exists(), "overlay contract not created");
    }
}
