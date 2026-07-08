use anyhow::Context;
use serde::Deserialize;
use std::collections::HashMap;
use std::fs;

#[derive(Debug, Deserialize)]
pub struct Config {
    pub tool_version: String,
    pub paths: Paths,
    pub runners: HashMap<String, Vec<String>>,
    #[serde(default)]
    pub quality: QualityConfig,
    #[serde(default)]
    pub capabilities: CapabilitiesConfig,
}

#[derive(Debug, Default, Deserialize)]
pub struct QualityConfig {
    pub mutation: Option<MutationConfig>,
}

#[derive(Debug, Default, Deserialize)]
pub struct CapabilitiesConfig {
    pub mutation: Option<CapabilityFlag>,
    pub property: Option<CapabilityFlag>,
    pub snapshot: Option<CapabilityFlag>,
}

#[derive(Debug, Deserialize)]
pub struct CapabilityFlag {
    pub enabled: bool,
}

#[derive(Debug, Deserialize)]
pub struct MutationConfig {
    pub enabled: bool,
    #[serde(default = "default_threshold")]
    pub threshold: f64,
    pub command: Vec<String>,
}

fn default_threshold() -> f64 {
    0.80
}

#[derive(Debug, Deserialize)]
pub struct Paths {
    pub specs: String,
    pub changes: String,
}

pub fn load_config(config_path: &str) -> anyhow::Result<Config> {
    let text =
        fs::read_to_string(config_path).with_context(|| format!("cannot read {config_path}"))?;
    let config: Config =
        toml::from_str(&text).with_context(|| format!("invalid TOML in {config_path}"))?;
    validate_config(&config)?;
    Ok(config)
}

fn validate_config(config: &Config) -> anyhow::Result<()> {
    anyhow::ensure!(
        !config.tool_version.is_empty(),
        "tool_version must be non-empty"
    );
    anyhow::ensure!(
        !config.paths.specs.is_empty(),
        "paths.specs must be non-empty"
    );
    anyhow::ensure!(
        !config.paths.changes.is_empty(),
        "paths.changes must be non-empty"
    );
    for (name, argv) in &config.runners {
        anyhow::ensure!(
            !argv.is_empty(),
            "runner {name} must have at least one argv entry"
        );
        for arg in argv {
            anyhow::ensure!(!arg.is_empty(), "runner {name} has an empty argv entry");
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    const VALID_CONFIG: &str = "tests/fixtures/simple/.espectacular/config.toml";
    const BAD_CONFIG: &str = "tests/fixtures/bad-config/.espectacular/config.toml";

    #[test]
    fn loads_valid_config() {
        let config = load_config(VALID_CONFIG).unwrap();
        assert_eq!(config.tool_version, "0.2.0");
    }

    #[test]
    fn config_paths_populated() {
        let config = load_config(VALID_CONFIG).unwrap();
        assert_eq!(config.paths.specs, "openspec/specs");
        assert_eq!(config.paths.changes, "openspec/changes");
    }

    #[test]
    fn config_runners_populated() {
        let config = load_config(VALID_CONFIG).unwrap();
        assert!(config.runners.contains_key("pytest"));
        assert_eq!(config.runners["cargo"], vec!["cargo", "test"]);
    }

    #[test]
    fn missing_paths_fails() {
        let result = load_config(BAD_CONFIG);
        assert!(result.is_err());
    }

    #[test]
    fn runner_argv_must_be_non_empty_strings() {
        // runners with empty argv entries should fail
        let toml = r#"
tool_version = "0.2.0"
[paths]
specs = "openspec/specs"
changes = "openspec/changes"
[runners]
bad = [""]
"#;
        // toml parsing succeeds for empty strings; validation must be explicit
        // load_config enforces this — use a temp file approach here
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("config.toml");
        std::fs::write(&path, toml).unwrap();
        let result = load_config(path.to_str().unwrap());
        assert!(result.is_err(), "empty runner argv should fail validation");
    }
}
