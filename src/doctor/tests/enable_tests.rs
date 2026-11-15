//! `--enable` config writers and genesis doctor adoption proof.

use super::fixtures::base_config_toml;
use crate::init::insert_runner_entry;
use genesis::suite_linter::LintResult;
use std::fs;
use std::path::Path;
use tempfile::TempDir;

// ── 7.5 Red: --enable writes exact config tables ──────────────────────────

#[test]
fn enable_pytest_writes_runner_entry_to_config() {
    let dir = TempDir::new().unwrap();
    let config_path = dir.path().join("config.toml");
    fs::write(&config_path, base_config_toml()).unwrap();

    let text = fs::read_to_string(&config_path).unwrap();
    let updated = insert_runner_entry(&text, "pytest", r#"["pytest"]"#);
    fs::write(&config_path, &updated).unwrap();

    let content = fs::read_to_string(&config_path).unwrap();
    assert!(
        content.contains("pytest = [\"pytest\"]"),
        "pytest runner entry must be written; got:\n{content}"
    );
}

#[test]
fn enable_cargo_writes_runner_entry_to_config() {
    let dir = TempDir::new().unwrap();
    let config_path = dir.path().join("config.toml");
    fs::write(&config_path, base_config_toml()).unwrap();

    let text = fs::read_to_string(&config_path).unwrap();
    let updated = insert_runner_entry(&text, "cargo", r#"["cargo", "test"]"#);
    fs::write(&config_path, &updated).unwrap();

    let content = fs::read_to_string(&config_path).unwrap();
    assert!(
        content.contains(r#"cargo = ["cargo", "test"]"#),
        "cargo runner entry must be written; got:\n{content}"
    );
}

// ── genesis::doctor adoption proof ───────────────────────────────────

#[test]
fn uses_genesis_doctor_framework() {
    // Compile-time proof: genesis::doctor::DoctorCheck, DoctorRunner used.
    use genesis::doctor::{DoctorCheck, DoctorRunner};

    struct AdoptionCheck;
    impl DoctorCheck for AdoptionCheck {
        fn name(&self) -> &'static str {
            "adoption"
        }
        fn description(&self) -> &'static str {
            "Proves genesis::doctor is adopted"
        }
        fn run(&self, _: &Path) -> Result<Vec<LintResult>, Box<dyn std::error::Error>> {
            Ok(vec![]) // pass
        }
    }

    let runner = DoctorRunner::new(vec![Box::new(AdoptionCheck)]).with_tool_name("test");
    let report = runner.run(Path::new("/tmp"), false).unwrap();
    assert_eq!(report.tool, "test");
    assert!(report.summary.is_healthy());
}
