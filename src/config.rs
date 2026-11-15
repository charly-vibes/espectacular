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

mod validate;

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
    #[serde(default)]
    pub lint: LintConfig,
}

/// Spec-lint settings (`[lint]` table). All keys optional; defaults keep
/// existing configs valid — lint is additive and advisory.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct LintConfig {
    #[serde(default = "default_max_and_steps")]
    pub max_and_steps: usize,
}

impl Default for LintConfig {
    fn default() -> Self {
        LintConfig {
            max_and_steps: default_max_and_steps(),
        }
    }
}

fn default_max_and_steps() -> usize {
    5
}

#[derive(Debug, Default, Clone, Deserialize, Serialize)]
pub struct QualityConfig {
    pub mutation: Option<MutationConfig>,
    pub composability: Option<QualityToolConfig>,
    pub cost: Option<QualityToolConfig>,
    pub boundary_coverage: Option<QualityToolConfig>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct QualityToolConfig {
    pub enabled: bool,
    pub command: Vec<String>,
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
        validate::path_field("paths.specs", &self.paths.specs, &mut results);
        validate::path_field("paths.changes", &self.paths.changes, &mut results);
        for (name, argv) in &self.runners {
            validate::runner(name, argv, &mut results);
        }
        if self.lint.max_and_steps == 0 {
            results.push(ConfigValidation::error(
                "lint.max_and_steps",
                "lint.max_and_steps must be at least 1",
            ));
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
mod tests;
