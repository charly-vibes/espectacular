//! `ah doctor --enable <capability>` config writers.

use super::types::{DoctorEnableResult, KNOWN_CAPABILITIES};
use crate::config;
use crate::init::{append_capability_block, insert_runner_entry};
use std::fs;
use std::path::Path;

pub fn run_doctor_enable(repo_root: &Path, capability: &str) -> anyhow::Result<DoctorEnableResult> {
    if !KNOWN_CAPABILITIES.contains(&capability) {
        anyhow::bail!(
            "unknown capability: {capability}; known: {}",
            KNOWN_CAPABILITIES.join(", ")
        );
    }

    let config_path = repo_root.join(".espectacular/config.toml");
    let cfg = config::load(repo_root)?;

    match capability {
        "pytest" | "cargo" | "vitest" => enable_runner(&config_path, &cfg, capability),
        "mutation" => enable_mutation(&config_path, &cfg),
        "property" | "snapshot" => enable_capability(&config_path, &cfg, capability),
        _ => unreachable!(),
    }
}

fn enable_runner(
    config_path: &Path,
    cfg: &config::Config,
    capability: &str,
) -> anyhow::Result<DoctorEnableResult> {
    if cfg.runners.contains_key(capability) {
        return Ok(DoctorEnableResult::AlreadyEnabled);
    }
    let value_toml = match capability {
        "pytest" => r#"["pytest"]"#,
        "cargo" => r#"["cargo", "test"]"#,
        "vitest" => r#"["vitest", "run"]"#,
        _ => unreachable!(),
    };
    let text = fs::read_to_string(config_path)?;
    let updated = insert_runner_entry(&text, capability, value_toml);
    fs::write(config_path, &updated)?;
    Ok(written(config_path, format!("runners.{capability}")))
}

fn enable_mutation(config_path: &Path, cfg: &config::Config) -> anyhow::Result<DoctorEnableResult> {
    if cfg.quality.mutation.is_some() {
        return Ok(DoctorEnableResult::AlreadyEnabled);
    }
    let text = fs::read_to_string(config_path)?;
    let trimmed = text.trim_end();
    let updated = format!(
        "{trimmed}\n\n[quality.mutation]\nenabled = true\nthreshold = 0.80\ncommand = [\"{{}}\"]\n"
    );
    fs::write(config_path, &updated)?;
    Ok(written(config_path, "quality.mutation".to_string()))
}

fn enable_capability(
    config_path: &Path,
    cfg: &config::Config,
    capability: &str,
) -> anyhow::Result<DoctorEnableResult> {
    let already_enabled = if capability == "property" {
        cfg.capabilities.property.is_some()
    } else {
        cfg.capabilities.snapshot.is_some()
    };
    if already_enabled {
        return Ok(DoctorEnableResult::AlreadyEnabled);
    }
    let text = fs::read_to_string(config_path)?;
    let updated = append_capability_block(&text, capability);
    fs::write(config_path, &updated)?;
    Ok(written(config_path, format!("capabilities.{capability}")))
}

fn written(config_path: &Path, table_name: String) -> DoctorEnableResult {
    DoctorEnableResult::Written {
        path: config_path.to_string_lossy().into_owned(),
        table_name,
    }
}
