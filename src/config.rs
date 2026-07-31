//! Tool config, thinned over `genesis::config`.
//!
//! The struct + `ConfigFile` impl live here; reading and parsing delegate
//! to genesis (`load()` goes through a `ConfigStore` backed by a
//! `ConfigRegistry`). Config *writes* remain tool-owned — `doctor --enable`
//! and `upgrade` do surgical text edits rather than full re-serialization,
//! so they don't go through `Config::write` — but the trait impl makes the
//! write path available for future use.

use anyhow::anyhow;
use genesis::config::{
    ConfigError, ConfigFile, ConfigRegistry, ConfigStore, ConfigValidation, ValidationSeverity,
};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::{Path, PathBuf};

/// Marker path for the espectacular config, relative to the repo root.
pub const CONFIG_MARKER: &str = ".espectacular/config.toml";

/// Tool name under which the config is registered with `ConfigRegistry`.
pub const TOOL_NAME: &str = "espectacular";

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Config {
    pub tool_version: String,
    pub paths: Paths,
    pub runners: HashMap<String, Vec<String>>,
    #[serde(default)]
    pub quality: QualityConfig,
    #[serde(default)]
    pub capabilities: CapabilitiesConfig,
}

#[derive(Debug, Default, Clone, Deserialize, Serialize)]
pub struct QualityConfig {
    pub mutation: Option<MutationConfig>,
}

#[derive(Debug, Default, Clone, Deserialize, Serialize)]
pub struct CapabilitiesConfig {
    pub mutation: Option<CapabilityFlag>,
    pub property: Option<CapabilityFlag>,
    pub snapshot: Option<CapabilityFlag>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct CapabilityFlag {
    pub enabled: bool,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct MutationConfig {
    pub enabled: bool,
    #[serde(default = "default_threshold")]
    pub threshold: f64,
    pub command: Vec<String>,
}

fn default_threshold() -> f64 {
    0.80
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Paths {
    pub specs: String,
    pub changes: String,
}

impl ConfigFile for Config {
    fn path(repo_root: &Path) -> PathBuf {
        repo_root.join(CONFIG_MARKER)
    }

    fn validate(&self) -> Result<Vec<ConfigValidation>, ConfigError> {
        let mut results = Vec::new();
        if self.tool_version.is_empty() {
            results.push(ConfigValidation::error(
                "tool_version",
                "tool_version must be non-empty",
            ));
        }
        if self.paths.specs.is_empty() {
            results.push(ConfigValidation::error(
                "paths.specs",
                "paths.specs must be non-empty",
            ));
        }
        if self.paths.changes.is_empty() {
            results.push(ConfigValidation::error(
                "paths.changes",
                "paths.changes must be non-empty",
            ));
        }
        for (name, argv) in &self.runners {
            if argv.is_empty() {
                results.push(ConfigValidation::error(
                    format!("runners.{name}"),
                    format!("runner {name} must have at least one argv entry"),
                ));
                continue;
            }
            for arg in argv {
                if arg.is_empty() {
                    results.push(ConfigValidation::error(
                        format!("runners.{name}"),
                        format!("runner {name} has an empty argv entry"),
                    ));
                    break;
                }
            }
        }
        Ok(results)
    }
}

/// Build a `ConfigRegistry` with the espectacular config registered.
///
/// Registration happens here (and is exercised on every `load`); the
/// `ConfigStore` wraps it for discovery, validation, and typed access.
pub fn registry() -> ConfigRegistry {
    let mut reg = ConfigRegistry::new();
    reg.register::<Config>(TOOL_NAME, CONFIG_MARKER);
    reg
}

/// Load and validate the config from `repo_root/.espectacular/config.toml`.
///
/// All file I/O and parsing delegates to `genesis::config` via a
/// `ConfigStore` backed by [`registry`]. Validation errors (severity
/// `Error`) are surfaced as a single `anyhow` error so callers can treat
/// a bad config as a hard failure.
pub fn load(repo_root: &Path) -> anyhow::Result<Config> {
    let store = ConfigStore::new(registry());
    let cfg: Config = store
        .get(TOOL_NAME, repo_root)
        .map_err(|e| anyhow!("{}", e))?;
    let validations = cfg.validate().map_err(|e| anyhow!("{}", e))?;
    let errors: Vec<&ConfigValidation> = validations
        .iter()
        .filter(|v| v.severity == ValidationSeverity::Error)
        .collect();
    if errors.is_empty() {
        Ok(cfg)
    } else {
        let detail: Vec<&str> = errors.iter().map(|v| v.message.as_str()).collect();
        Err(anyhow!(
            "invalid config at {}: {}",
            Config::path(repo_root).display(),
            detail.join("; ")
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const VALID_REPO: &str = "tests/fixtures/simple";
    const BAD_REPO: &str = "tests/fixtures/bad-config";

    #[test]
    fn config_implements_config_file() {
        // Compile-time proof of adoption: Config: genesis::config::ConfigFile.
        fn _accept<T: ConfigFile>(_: &T) {}
        _accept(&Config {
            tool_version: String::new(),
            paths: Paths {
                specs: String::new(),
                changes: String::new(),
            },
            runners: HashMap::new(),
            quality: QualityConfig::default(),
            capabilities: CapabilitiesConfig::default(),
        });
    }

    #[test]
    fn config_path_uses_espectacular_marker() {
        let path = Config::path(Path::new("repo"));
        assert_eq!(path, Path::new("repo/.espectacular/config.toml"));
    }

    #[test]
    fn registry_registers_espectacular_config() {
        let reg = registry();
        assert!(reg.is_registered(TOOL_NAME));
        assert_eq!(reg.marker(TOOL_NAME), Some(CONFIG_MARKER));
    }

    #[test]
    fn loads_valid_config() {
        let config = load(Path::new(VALID_REPO)).unwrap();
        assert_eq!(config.tool_version, "0.4.0");
    }

    #[test]
    fn config_paths_populated() {
        let config = load(Path::new(VALID_REPO)).unwrap();
        assert_eq!(config.paths.specs, "openspec/specs");
        assert_eq!(config.paths.changes, "openspec/changes");
    }

    #[test]
    fn config_runners_populated() {
        let config = load(Path::new(VALID_REPO)).unwrap();
        assert!(config.runners.contains_key("pytest"));
        assert_eq!(config.runners["cargo"], vec!["cargo", "test"]);
    }

    #[test]
    fn missing_paths_fails() {
        let result = load(Path::new(BAD_REPO));
        assert!(result.is_err());
    }

    #[test]
    fn missing_config_file_fails() {
        let dir = tempfile::tempdir().unwrap();
        let result = load(dir.path());
        assert!(result.is_err());
    }

    #[test]
    fn runner_argv_must_be_non_empty_strings() {
        let dir = tempfile::tempdir().unwrap();
        let config_path = dir.path().join(".espectacular/config.toml");
        std::fs::create_dir_all(config_path.parent().unwrap()).unwrap();
        std::fs::write(
            &config_path,
            r#"
tool_version = "0.4.0"
[paths]
specs = "openspec/specs"
changes = "openspec/changes"
[runners]
bad = [""]
"#,
        )
        .unwrap();
        let result = load(dir.path());
        assert!(result.is_err(), "empty runner argv should fail validation");
    }

    #[test]
    fn validate_flags_empty_tool_version() {
        let config = Config {
            tool_version: String::new(),
            paths: Paths {
                specs: "s".into(),
                changes: "c".into(),
            },
            runners: HashMap::new(),
            quality: QualityConfig::default(),
            capabilities: CapabilitiesConfig::default(),
        };
        let results = config.validate().unwrap();
        assert!(results
            .iter()
            .any(|v| v.field == "tool_version" && v.severity == ValidationSeverity::Error));
    }
}
