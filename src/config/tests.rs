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
        lint: LintConfig::default(),
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

/// Copy the simple fixture into a tempdir with `tool_version` rewritten
/// to the current crate version — keeps the fixture version-agnostic
/// (espectacular-n8o) and parallel-safe (no shared-fixture mutation).
fn valid_repo_with_current_version() -> tempfile::TempDir {
    fn copy_recursive(src: &Path, dst: &Path) {
        std::fs::create_dir_all(dst).unwrap();
        for entry in std::fs::read_dir(src).unwrap() {
            let entry = entry.unwrap();
            let to = dst.join(entry.file_name());
            if entry.file_type().unwrap().is_dir() {
                copy_recursive(&entry.path(), &to);
            } else {
                std::fs::copy(entry.path(), &to).unwrap();
            }
        }
    }
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path().join("simple");
    copy_recursive(Path::new(VALID_REPO), &root);
    let cfg = root.join(".espectacular/config.toml");
    let text = std::fs::read_to_string(&cfg).unwrap();
    let updated: String = text
        .lines()
        .map(|line| {
            if line.starts_with("tool_version = ") {
                format!("tool_version = \"{}\"", env!("CARGO_PKG_VERSION"))
            } else {
                line.to_string()
            }
        })
        .collect::<Vec<_>>()
        .join("\n");
    std::fs::write(&cfg, updated).unwrap();
    dir
}

#[test]
fn loads_valid_config() {
    let dir = valid_repo_with_current_version();
    let config = load(&dir.path().join("simple")).unwrap();
    assert_eq!(config.tool_version, env!("CARGO_PKG_VERSION"));
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
fn lint_config_defaults_when_section_missing() {
    let config = load(Path::new(VALID_REPO)).unwrap();
    assert_eq!(config.lint.max_and_steps, 5);
}

#[test]
fn lint_config_parses_max_and_steps_override() {
    let dir = tempfile::tempdir().unwrap();
    let config_path = dir.path().join(".espectacular/config.toml");
    std::fs::create_dir_all(config_path.parent().unwrap()).unwrap();
    std::fs::write(
        &config_path,
        concat!("tool_version = \"", env!("CARGO_PKG_VERSION"), "\"\n[paths]\nspecs = \"openspec/specs\"\nchanges = \"openspec/changes\"\n[runners]\n\n[lint]\nmax_and_steps = 3\n"),
    )
    .unwrap();
    let config = load(dir.path()).unwrap();
    assert_eq!(config.lint.max_and_steps, 3);
}

#[test]
fn lint_config_rejects_zero_max_and_steps() {
    let dir = tempfile::tempdir().unwrap();
    let config_path = dir.path().join(".espectacular/config.toml");
    std::fs::create_dir_all(config_path.parent().unwrap()).unwrap();
    std::fs::write(
        &config_path,
        concat!("tool_version = \"", env!("CARGO_PKG_VERSION"), "\"\n[paths]\nspecs = \"openspec/specs\"\nchanges = \"openspec/changes\"\n[runners]\n\n[lint]\nmax_and_steps = 0\n"),
    )
    .unwrap();
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
        lint: LintConfig::default(),
    };
    let results = config.validate().unwrap();
    assert!(results
        .iter()
        .any(|v| v.field == "tool_version" && v.severity == ValidationSeverity::Error));
}

#[test]
fn validate_flags_zero_max_and_steps() {
    let config = Config {
        tool_version: env!("CARGO_PKG_VERSION").into(),
        paths: Paths {
            specs: "s".into(),
            changes: "c".into(),
        },
        runners: HashMap::new(),
        quality: QualityConfig::default(),
        capabilities: CapabilitiesConfig::default(),
        lint: LintConfig { max_and_steps: 0 },
    };
    let results = config.validate().unwrap();
    assert!(results
        .iter()
        .any(|v| v.field == "lint.max_and_steps" && v.severity == ValidationSeverity::Error));
}

// ---- property-derived contract bindings (DDL migration, design D6) ----

#[test]
fn p_configfile() {
    config_implements_config_file();
}

#[test]
fn p_configstore() {
    config_implements_config_file();
    config_path_uses_espectacular_marker();
}

#[test]
fn p_registration() {
    registry_registers_espectacular_config();
}

#[test]
fn p_prune() {
    config_implements_config_file();
    registry_registers_espectacular_config();
    config_path_uses_espectacular_marker();
}
