use crate::fsutil::write_text;
use anyhow::Context;
use std::path::Path;

const TOOL_VERSION: &str = env!("CARGO_PKG_VERSION");

pub struct UpgradeReport {
    pub drift: bool,
    pub config_version: String,
    pub binary_version: &'static str,
}

pub fn run_upgrade(repo_root: &Path) -> anyhow::Result<UpgradeReport> {
    let config_path = repo_root.join(".espectacular/config.toml");
    let config_text = std::fs::read_to_string(&config_path)
        .with_context(|| format!("cannot read {}", config_path.display()))?;

    let config_version = parse_tool_version(&config_text)?;

    if config_version == TOOL_VERSION {
        return Ok(UpgradeReport {
            drift: false,
            config_version,
            binary_version: TOOL_VERSION,
        });
    }

    let updated = config_text.replacen(
        &format!("tool_version = \"{config_version}\""),
        &format!("tool_version = \"{TOOL_VERSION}\""),
        1,
    );
    write_text(&config_path, updated)?;

    Ok(UpgradeReport {
        drift: true,
        config_version,
        binary_version: TOOL_VERSION,
    })
}

fn parse_tool_version(config_text: &str) -> anyhow::Result<String> {
    for line in config_text.lines() {
        if let Some(rest) = line.strip_prefix("tool_version = \"") {
            if let Some(version) = rest.strip_suffix('"') {
                return Ok(version.to_string());
            }
        }
    }
    anyhow::bail!("tool_version not found in config")
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    fn make_config(dir: &TempDir, tool_version: &str) {
        fs::create_dir_all(dir.path().join(".espectacular")).unwrap();
        let content = format!(
            "tool_version = \"{tool_version}\"\n\n[paths]\nspecs = \"openspec/specs\"\nchanges = \"openspec/changes\"\n\n[runners]\ncargo = [\"cargo\", \"test\"]\n"
        );
        fs::write(dir.path().join(".espectacular/config.toml"), content).unwrap();
    }

    #[test]
    fn no_drift_when_version_matches() {
        let dir = TempDir::new().unwrap();
        make_config(&dir, TOOL_VERSION);

        let report = run_upgrade(dir.path()).unwrap();
        assert!(!report.drift);
        assert_eq!(report.config_version, TOOL_VERSION);
    }

    #[test]
    fn drift_detected_when_version_differs() {
        let dir = TempDir::new().unwrap();
        make_config(&dir, "0.0.1");

        let report = run_upgrade(dir.path()).unwrap();
        assert!(report.drift);
        assert_eq!(report.config_version, "0.0.1");
        assert_eq!(report.binary_version, TOOL_VERSION);
    }

    #[test]
    fn upgrade_updates_config_tool_version() {
        let dir = TempDir::new().unwrap();
        make_config(&dir, "0.0.1");

        run_upgrade(dir.path()).unwrap();

        let text = fs::read_to_string(dir.path().join(".espectacular/config.toml")).unwrap();
        assert!(
            text.contains(&format!("tool_version = \"{TOOL_VERSION}\"")),
            "config not updated: {text}"
        );
    }

    #[test]
    fn upgrade_does_not_touch_contracts() {
        let dir = TempDir::new().unwrap();
        make_config(&dir, "0.0.1");
        fs::create_dir_all(dir.path().join(".espectacular/spec-a")).unwrap();
        let contract = "id = \"s\"\nauthored_with = \"0.0.1\"\nstatus = \"active\"\n";
        let contract_path = dir.path().join(".espectacular/spec-a/s.toml");
        fs::write(&contract_path, contract).unwrap();

        run_upgrade(dir.path()).unwrap();

        let after = fs::read_to_string(&contract_path).unwrap();
        assert_eq!(after, contract, "contract must not be modified");
    }
}
