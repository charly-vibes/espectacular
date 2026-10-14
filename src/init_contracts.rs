// Init contract stubbing (empty TOML contracts for deployed scenarios
// without contracts). Split out of src/init.rs (espectacular-r2p
// follow-through): the pretender gate caps app-role files at 400 lines.
use crate::fsutil::write_text;
use crate::init::InitResult;
use crate::openspec;
use anyhow::Context;
use std::fs;
use std::path::Path;

fn stub_contract_if_missing(
    repo_root: &Path,
    scenario: &openspec::Scenario,
    result: &mut InitResult,
) -> anyhow::Result<()> {
    // spec_path is the spec name, e.g. "compiler"
    let spec_name = &scenario.spec_path;
    if spec_name.is_empty() {
        return Ok(());
    }

    let contract_dir = repo_root.join(".espectacular").join(spec_name);
    let contract_path = contract_dir.join(format!("{}.toml", scenario.id));

    if contract_path.exists() {
        return Ok(());
    }

    fs::create_dir_all(&contract_dir)
        .with_context(|| format!("cannot create {}", contract_dir.display()))?;

    let stub = format!(
        "id = \"{id}\"\ndescription = \"\"\narchetype = \"PF\"\nstatus = \"active\"\nsuperseded_by = \"\"\nauthored_with = \"{version}\"\n",
        id = scenario.id,
        version = env!("CARGO_PKG_VERSION")
    );
    write_text(&contract_path, stub)?;

    result
        .stubbed_contracts
        .push(format!("{}/{}.toml", spec_name, scenario.id));
    Ok(())
}

/// Stub contracts for deployed scenarios without existing contracts.
pub(crate) fn stub_missing_contracts(
    repo_root: &Path,
    result: &mut InitResult,
) -> anyhow::Result<()> {
    let specs_dir = repo_root.join("openspec/specs");
    if !specs_dir.exists() {
        return Ok(());
    }
    let specs_str = specs_dir.to_string_lossy().to_string();
    if let Ok(scenarios) = openspec::discover_scenarios(&specs_str) {
        for scenario in &scenarios {
            stub_contract_if_missing(repo_root, scenario, result)?;
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::init::run_init;
    use tempfile::TempDir;

    // 4.3 RED: stub empty TOML contracts for existing deployed scenarios

    fn make_repo_with_scenarios(scenarios: &[(&str, &str)]) -> TempDir {
        let dir = TempDir::new().unwrap();
        fs::create_dir_all(dir.path().join("openspec")).unwrap();
        fs::create_dir_all(dir.path().join("openspec/specs")).unwrap();
        for (spec, scenario_heading) in scenarios {
            let spec_dir = dir.path().join("openspec/specs").join(spec);
            fs::create_dir_all(&spec_dir).unwrap();
            let content = format!(
                "# Capability: {spec}\n\n## DEPLOYED Requirements\n\n### Requirement: Test\n\n#### Scenario: {scenario_heading}\n- **GIVEN** something\n- **WHEN** action\n- **THEN** result\n"
            );
            fs::write(spec_dir.join("spec.md"), content).unwrap();
        }
        dir
    }

    #[test]

    fn init_stubs_contracts_for_scenarios_without_existing_contracts() {
        let repo = make_repo_with_scenarios(&[("compiler", "Empty input rejected")]);
        let result = run_init(repo.path()).unwrap();
        let stub_path = repo
            .path()
            .join(".espectacular/compiler/empty-input-rejected.toml");
        assert!(
            stub_path.exists(),
            "stub contract must be created at .espectacular/compiler/empty-input-rejected.toml"
        );
        assert!(
            result
                .stubbed_contracts
                .iter()
                .any(|s| s.contains("empty-input-rejected")),
            "stubbed_contracts must include the scenario slug"
        );
    }

    #[test]
    fn init_stub_declares_no_tests() {
        let repo = make_repo_with_scenarios(&[("compiler", "Empty input rejected")]);
        run_init(repo.path()).unwrap();
        let stub_path = repo
            .path()
            .join(".espectacular/compiler/empty-input-rejected.toml");
        let content = fs::read_to_string(&stub_path).unwrap();
        // Stub must have required fields but no [[tests.*]] table
        assert!(content.contains("id ="), "stub must have id field");
        assert!(content.contains("status ="), "stub must have status field");
        assert!(
            !content.contains("[[tests"),
            "stub must NOT declare any tests"
        );
    }

    #[test]
    fn init_does_not_overwrite_existing_contracts() {
        let repo = make_repo_with_scenarios(&[("compiler", "Empty input rejected")]);
        // Pre-create the contract
        fs::create_dir_all(repo.path().join(".espectacular/compiler")).unwrap();
        let stub_path = repo
            .path()
            .join(".espectacular/compiler/empty-input-rejected.toml");
        fs::write(&stub_path, "id = \"empty-input-rejected\"\ncustom = true\n").unwrap();
        let result = run_init(repo.path()).unwrap();
        let content = fs::read_to_string(&stub_path).unwrap();
        assert!(
            content.contains("custom = true"),
            "must not overwrite existing contract"
        );
        assert!(
            !result
                .stubbed_contracts
                .iter()
                .any(|s| s.contains("empty-input-rejected")),
            "must not report existing contract as stubbed"
        );
    }

    #[test]

    fn init_stubs_contracts_for_multiple_specs() {
        let repo = make_repo_with_scenarios(&[
            ("compiler", "Empty input rejected"),
            ("runtime", "Handle timeout"),
        ]);
        let result = run_init(repo.path()).unwrap();
        assert_eq!(
            result.stubbed_contracts.len(),
            2,
            "must stub one contract per scenario"
        );
        assert!(repo
            .path()
            .join(".espectacular/compiler/empty-input-rejected.toml")
            .exists());
        assert!(repo
            .path()
            .join(".espectacular/runtime/handle-timeout.toml")
            .exists());
    }
}
