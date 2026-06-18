use anyhow::Context;
use serde::Deserialize;
use std::collections::HashMap;
use std::fs;

#[allow(dead_code)]
#[derive(Debug, Clone, Deserialize)]
pub struct Contract {
    pub id: String,
    pub description: String,
    pub archetype: String,
    pub status: String,
    pub superseded_by: String,
    pub authored_with: String,
    pub tests: HashMap<String, Vec<TestEntry>>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct TestEntry {
    pub flags: Option<String>,
    pub command: Option<String>,
    pub timeout_seconds: Option<u64>,
}

pub fn load_contract(toml_path: &str) -> anyhow::Result<Contract> {
    let text = fs::read_to_string(toml_path).with_context(|| format!("cannot read {toml_path}"))?;
    let contract: Contract =
        toml::from_str(&text).with_context(|| format!("invalid TOML in {toml_path}"))?;
    validate_contract(&contract)?;
    Ok(contract)
}

fn validate_contract(contract: &Contract) -> anyhow::Result<()> {
    match contract.status.as_str() {
        "active" | "superseded" => {}
        other => anyhow::bail!("unknown status: {other}"),
    }
    if contract.status == "superseded" {
        anyhow::ensure!(
            !contract.superseded_by.is_empty(),
            "superseded contract must set superseded_by"
        );
    }

    anyhow::ensure!(
        !contract.authored_with.is_empty(),
        "authored_with must be non-empty"
    );
    anyhow::ensure!(!contract.id.is_empty(), "id must be non-empty");

    for (test_type, entries) in &contract.tests {
        for entry in entries {
            anyhow::ensure!(
                entry.timeout_seconds.unwrap_or(1) > 0,
                "test entry timeout_seconds must be positive"
            );
            if test_type == "shell" {
                anyhow::ensure!(
                    entry
                        .command
                        .as_deref()
                        .is_some_and(|value| !value.is_empty()),
                    "shell test entry must declare command"
                );
                anyhow::ensure!(
                    entry.flags.is_none(),
                    "shell test entry may not declare flags"
                );
            } else {
                anyhow::ensure!(
                    entry
                        .flags
                        .as_deref()
                        .is_some_and(|value| !value.is_empty()),
                    "non-shell test entry must declare flags"
                );
                anyhow::ensure!(
                    entry.command.is_none(),
                    "non-shell test entry may not declare command"
                );
            }
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    const VALID: &str = "tests/fixtures/simple/.espectacular/compiler/empty-input-rejected.toml";
    const SUPERSEDED: &str =
        "tests/fixtures/simple/.espectacular/compiler/superseded-scenario.toml";

    #[test]
    fn loads_valid_contract() {
        let c = load_contract(VALID).unwrap();
        assert_eq!(c.id, "empty-input-rejected");
    }

    #[test]
    fn contract_status_active() {
        let c = load_contract(VALID).unwrap();
        assert_eq!(c.status, "active");
    }

    #[test]
    fn contract_tests_non_empty() {
        let c = load_contract(VALID).unwrap();
        assert!(!c.tests.is_empty());
    }

    #[test]
    fn superseded_contract_has_superseded_by() {
        let c = load_contract(SUPERSEDED).unwrap();
        assert_eq!(c.status, "superseded");
        assert!(!c.superseded_by.is_empty());
    }

    #[test]
    fn unknown_status_fails() {
        let toml = r#"
id = "foo"
description = ""
archetype = ""
status = "invalid-status"
superseded_by = ""
authored_with = "0.1.0"
[tests]
unit = [{flags = "test::foo"}]
"#;
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("foo.toml");
        std::fs::write(&path, toml).unwrap();
        let result = load_contract(path.to_str().unwrap());
        assert!(result.is_err(), "unknown status should fail validation");
    }

    #[test]
    fn superseded_without_superseded_by_fails() {
        let toml = r#"
id = "foo"
description = ""
archetype = ""
status = "superseded"
superseded_by = ""
authored_with = "0.1.0"
[tests]
unit = [{flags = "test::foo"}]
"#;
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("foo.toml");
        std::fs::write(&path, toml).unwrap();
        let result = load_contract(path.to_str().unwrap());
        assert!(
            result.is_err(),
            "superseded with empty superseded_by should fail"
        );
    }

    #[test]
    fn missing_required_field_fails() {
        let toml = r#"
description = ""
archetype = ""
status = "active"
superseded_by = ""
authored_with = "0.1.0"
[tests]
unit = [{flags = "test::foo"}]
"#;
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("foo.toml");
        std::fs::write(&path, toml).unwrap();
        assert!(load_contract(path.to_str().unwrap()).is_err());
    }

    #[test]
    fn shell_entry_requires_command_only() {
        let toml = r#"
id = "foo"
description = ""
archetype = ""
status = "active"
superseded_by = ""
authored_with = "0.1.0"
[tests]
shell = [{flags = "oops"}]
"#;
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("foo.toml");
        std::fs::write(&path, toml).unwrap();
        assert!(load_contract(path.to_str().unwrap()).is_err());
    }

    #[test]
    fn non_shell_entry_requires_flags_only() {
        let toml = r#"
id = "foo"
description = ""
archetype = ""
status = "active"
superseded_by = ""
authored_with = "0.1.0"
[tests]
unit = [{command = "echo nope"}]
"#;
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("foo.toml");
        std::fs::write(&path, toml).unwrap();
        assert!(load_contract(path.to_str().unwrap()).is_err());
    }
}
