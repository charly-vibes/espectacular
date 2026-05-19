use anyhow::Context;
use serde::Deserialize;
use std::collections::HashMap;
use std::fs;

#[derive(Debug, Deserialize)]
pub struct Contract {
    pub id: String,
    pub description: String,
    pub archetype: String,
    pub status: String,
    pub superseded_by: String,
    pub authored_with: String,
    pub tests: HashMap<String, Vec<TestEntry>>,
}

#[derive(Debug, Deserialize)]
pub struct TestEntry {
    pub flags: Option<String>,
    pub command: Option<String>,
    pub timeout_seconds: Option<u64>,
}

pub fn load_contract(toml_path: &str) -> anyhow::Result<Contract> {
    let text = fs::read_to_string(toml_path)
        .with_context(|| format!("cannot read {toml_path}"))?;
    let contract: Contract = toml::from_str(&text)
        .with_context(|| format!("invalid TOML in {toml_path}"))?;
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
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    const VALID: &str = "tests/fixtures/simple/.espectacular/compiler/empty-input-rejected.toml";
    const SUPERSEDED: &str = "tests/fixtures/simple/.espectacular/compiler/superseded-scenario.toml";

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
        assert!(result.is_err(), "superseded with empty superseded_by should fail");
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
}
