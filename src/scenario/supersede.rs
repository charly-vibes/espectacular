use crate::openspec::discover_scenarios;
use anyhow::Context;
use std::path::{Path, PathBuf};

#[derive(Debug)]
pub struct ScenarioSupersededResult {
    pub contract_path: String,
}

pub fn run_scenario_supersede(
    repo_root: &Path,
    spec: &str,
    old_id: &str,
    new_id: &str,
    change: &str,
) -> anyhow::Result<ScenarioSupersededResult> {
    ensure_new_id_in_scope(repo_root, spec, change, new_id)?;

    let deployed_contract = deployed_contract_path(repo_root, spec, old_id);
    anyhow::ensure!(
        deployed_contract.exists(),
        "deployed contract not found: .espectacular/{spec}/{old_id}.toml"
    );

    let value = superseded_contract(&deployed_contract, new_id)?;
    let contract_path = write_overlay_contract(repo_root, change, spec, old_id, &value)?;
    Ok(ScenarioSupersededResult { contract_path })
}

/// Validate that `new_id` exists in deployed-plus-change scope.
fn ensure_new_id_in_scope(
    repo_root: &Path,
    spec: &str,
    change: &str,
    new_id: &str,
) -> anyhow::Result<()> {
    let mut all_ids = scoped_scenario_ids(&repo_root.join("openspec/specs"), spec);
    all_ids.extend(scoped_scenario_ids(
        &repo_root
            .join("openspec/changes")
            .join(change)
            .join("specs"),
        spec,
    ));
    anyhow::ensure!(
        all_ids.iter().any(|id| id == new_id),
        "replacement scenario '{new_id}' not found in deployed or change scope for '{spec}'"
    );
    Ok(())
}

fn scoped_scenario_ids(dir: &Path, spec: &str) -> Vec<String> {
    if !dir.exists() {
        return Vec::new();
    }
    match discover_scenarios(dir.to_str().unwrap_or("")) {
        Ok(scenarios) => scenarios
            .into_iter()
            .filter(|s| s.spec_path == spec)
            .map(|s| s.id)
            .collect(),
        Err(_) => Vec::new(),
    }
}

fn deployed_contract_path(repo_root: &Path, spec: &str, old_id: &str) -> PathBuf {
    repo_root
        .join(".espectacular")
        .join(spec)
        .join(format!("{old_id}.toml"))
}

/// Read the deployed contract and flag it as superseded by `new_id`.
fn superseded_contract(path: &Path, new_id: &str) -> anyhow::Result<toml::Value> {
    let text =
        std::fs::read_to_string(path).with_context(|| format!("cannot read {}", path.display()))?;
    let mut value: toml::Value =
        toml::from_str(&text).with_context(|| "invalid TOML in deployed contract")?;

    if let Some(table) = value.as_table_mut() {
        table.insert("status".into(), toml::Value::String("superseded".into()));
        table.insert(
            "superseded_by".into(),
            toml::Value::String(new_id.to_string()),
        );
    }
    Ok(value)
}

fn write_overlay_contract(
    repo_root: &Path,
    change: &str,
    spec: &str,
    old_id: &str,
    value: &toml::Value,
) -> anyhow::Result<String> {
    let dest_dir = repo_root
        .join(".espectacular/changes")
        .join(change)
        .join(spec);
    std::fs::create_dir_all(&dest_dir)?;

    let dest_file = dest_dir.join(format!("{old_id}.toml"));
    let out = toml::to_string_pretty(value)?;
    std::fs::write(&dest_file, &out)
        .with_context(|| format!("cannot write {}", dest_file.display()))?;
    Ok(format!(
        ".espectacular/changes/{change}/{spec}/{old_id}.toml"
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

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
            "id = \"{scenario_id}\"\ndescription = \"Test scenario.\"\narchetype = \"PF\"\nstatus = \"active\"\nsuperseded_by = \"\"\nauthored_with = \"0.3.0\"\n\n[[tests.unit]]\nflags = \"tests/test_foo.py\"\ntimeout_seconds = 60\n"
        );
        fs::write(contract_dir.join(format!("{scenario_id}.toml")), &contract).unwrap();
        dir
    }

    /// Deployed repo whose spec also carries the replacement scenario.
    fn make_repo_with_replacement(spec: &str, old_id: &str) -> TempDir {
        let dir = make_repo_with_deployed_scenario(spec, old_id);
        let deployed_spec = dir.path().join("openspec/specs").join(spec).join("spec.md");
        let mut content = fs::read_to_string(&deployed_spec).unwrap();
        content.push_str("#### Scenario: Null bytes rejected\n- **WHEN** x\n- **THEN** y\n");
        fs::write(&deployed_spec, &content).unwrap();
        dir
    }

    // ── 4.13 RED: ah scenario supersede ──────────────────────────────────────

    #[test]
    fn supersede_creates_overlay_contract() {
        let dir = make_repo_with_replacement("compiler", "empty-input-rejected");

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
        let dir = make_repo_with_replacement("compiler", "empty-input-rejected");

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
        let dir = make_repo_with_replacement("compiler", "empty-input-rejected");

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
        let dir = make_repo_with_replacement("compiler", "empty-input-rejected");

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
