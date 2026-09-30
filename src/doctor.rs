use crate::adapters::{self, DetectionSource};
use crate::archetypes;
use crate::init::ah_block_injector;
use crate::openspec;
use crate::{config, contracts};
use genesis::doctor::DoctorCheck;
use genesis::git_hooks;
use genesis::status::StatusSection;
use genesis::suite_linter::{LintResult, Severity};
use std::collections::HashSet;
use std::fs;
use std::path::Path;

// ── Domain types (not in genesis) ─────────────────────────────────────

#[derive(Debug, PartialEq, Eq, Clone)]
pub struct FrameworkDetection {
    pub name: String,
    pub detection_source: DetectionSource,
}

/// A suggestion to enable a detected-but-unconfigured capability.
#[derive(Debug, PartialEq, Eq, Clone)]
pub struct CapabilitySuggestion {
    pub capability: String,
    pub detail: String,
    pub apply_command: String,
}

/// A suggested next step for the current session (advisory-only, never a
/// gate blocker) — e.g. run `ah lint` before it has been run once.
#[derive(Debug, PartialEq, Eq, Clone)]
pub struct SessionSuggestion {
    pub detail: String,
    pub apply_command: String,
}

/// Outcome of a doctor run: genesis report + domain-specific detections and suggestions.
#[derive(Debug)]
pub struct DoctorOutcome {
    /// The genesis doctor report with all check results.
    pub genesis_report: genesis::doctor::DoctorReport,
    /// Framework detections (domain-specific, not in genesis).
    pub detections: Vec<FrameworkDetection>,
    /// Capability suggestions (domain-specific, not in genesis).
    pub suggestions: Vec<CapabilitySuggestion>,
    /// Session step suggestions (advisory-only; never affect health).
    pub session_suggestions: Vec<SessionSuggestion>,
}

#[derive(Debug)]
pub enum DoctorEnableResult {
    Written { path: String, table_name: String },
    AlreadyEnabled,
}

const TOOL_VERSION: &str = env!("CARGO_PKG_VERSION");
const KNOWN_CAPABILITIES: &[&str] = &[
    "pytest", "cargo", "vitest", "mutation", "property", "snapshot",
];

/// Map a `genesis::doctor::CheckEntry` to a `StatusSection` item.
fn check_entry_to_status_item(entry: genesis::doctor::CheckEntry) -> genesis::status::StatusItem {
    let level = match entry.status {
        genesis::doctor::CheckStatus::Fail => genesis::status::StatusLevel::Error,
        genesis::doctor::CheckStatus::Warn => genesis::status::StatusLevel::Warning,
        genesis::doctor::CheckStatus::Pass => genesis::status::StatusLevel::Healthy,
    };
    genesis::status::StatusItem {
        label: entry.message,
        value: String::new(),
        level,
    }
}

// ── DoctorCheck implementations ───────────────────────────────────────

struct ConfigCheck;
impl DoctorCheck for ConfigCheck {
    fn name(&self) -> &'static str {
        "config"
    }
    fn description(&self) -> &'static str {
        "Validate .espectacular/config.toml"
    }
    fn run(&self, repo_root: &Path) -> Result<Vec<LintResult>, Box<dyn std::error::Error>> {
        match config::load(repo_root) {
            Ok(_) => Ok(vec![]),
            Err(e) => Ok(vec![LintResult::new(
                format!("bad config: {e:#}"),
                Severity::Error,
            )]),
        }
    }
}

struct VersionDriftCheck {
    config: config::Config,
}
impl DoctorCheck for VersionDriftCheck {
    fn name(&self) -> &'static str {
        "version-drift"
    }
    fn description(&self) -> &'static str {
        "Check that config tool_version matches binary version"
    }
    fn run(&self, _repo_root: &Path) -> Result<Vec<LintResult>, Box<dyn std::error::Error>> {
        if self.config.tool_version == TOOL_VERSION {
            Ok(vec![])
        } else {
            Ok(vec![LintResult::new(
                format!(
                    "config tool_version {} does not match binary version {TOOL_VERSION}",
                    self.config.tool_version
                ),
                Severity::Error,
            )])
        }
    }
}

struct SpecsDirCheck {
    specs_dir: std::path::PathBuf,
}
impl DoctorCheck for SpecsDirCheck {
    fn name(&self) -> &'static str {
        "missing-specs-dir"
    }
    fn description(&self) -> &'static str {
        "Check that the specs directory exists"
    }
    fn run(&self, _repo_root: &Path) -> Result<Vec<LintResult>, Box<dyn std::error::Error>> {
        if self.specs_dir.exists() {
            Ok(vec![])
        } else {
            Ok(vec![LintResult::new(
                format!("specs directory not found: {}", self.specs_dir.display()),
                Severity::Error,
            )])
        }
    }
}

struct ChangesDirCheck {
    changes_dir: std::path::PathBuf,
}
impl DoctorCheck for ChangesDirCheck {
    fn name(&self) -> &'static str {
        "missing-changes-dir"
    }
    fn description(&self) -> &'static str {
        "Check that the changes directory exists"
    }
    fn run(&self, _repo_root: &Path) -> Result<Vec<LintResult>, Box<dyn std::error::Error>> {
        if self.changes_dir.exists() {
            Ok(vec![])
        } else {
            Ok(vec![LintResult::new(
                format!(
                    "changes directory not found: {}",
                    self.changes_dir.display()
                ),
                Severity::Error,
            )])
        }
    }
}

struct CollisionCheck {
    #[allow(dead_code)]
    repo_root: std::path::PathBuf,
    specs_dir: std::path::PathBuf,
}
impl DoctorCheck for CollisionCheck {
    fn name(&self) -> &'static str {
        "scenario-collisions"
    }
    fn description(&self) -> &'static str {
        "Check for duplicate scenario slugs"
    }
    fn run(&self, _repo_root: &Path) -> Result<Vec<LintResult>, Box<dyn std::error::Error>> {
        if !self.specs_dir.exists() {
            return Ok(vec![]);
        }
        let specs_str = self.specs_dir.to_string_lossy().to_string();
        let scenarios = match openspec::discover_scenarios(&specs_str) {
            Ok(s) => s,
            Err(_) => return Ok(vec![]),
        };
        let collisions = openspec::detect_slug_collisions(&scenarios);
        Ok(collisions
            .into_iter()
            .map(|(spec, slug, _heading)| {
                LintResult::new(
                    format!("duplicate scenario slug '{slug}' in spec '{spec}'"),
                    Severity::Error,
                )
            })
            .collect())
    }
}

struct OrphanContractCheck {
    repo_root: std::path::PathBuf,
    specs_dir: std::path::PathBuf,
}
impl DoctorCheck for OrphanContractCheck {
    fn name(&self) -> &'static str {
        "orphan-contracts"
    }
    fn description(&self) -> &'static str {
        "Check for contracts with no matching scenario"
    }
    fn run(&self, _repo_root: &Path) -> Result<Vec<LintResult>, Box<dyn std::error::Error>> {
        if !self.specs_dir.exists() {
            return Ok(vec![]);
        }
        let specs_str = self.specs_dir.to_string_lossy().to_string();
        let scenarios = match openspec::discover_scenarios(&specs_str) {
            Ok(s) => s,
            Err(_) => return Ok(vec![]),
        };
        let known_spec_slugs: HashSet<(String, String)> = scenarios
            .iter()
            .map(|s| (s.spec_path.clone(), s.id.clone()))
            .collect();

        let espectacular_dir = self.repo_root.join(".espectacular");
        let mut results = Vec::new();
        if let Ok(entries) = fs::read_dir(&espectacular_dir) {
            for entry in entries.flatten() {
                let entry_path = entry.path();
                if !entry_path.is_dir() {
                    continue;
                }
                let spec_name = entry_path
                    .file_name()
                    .unwrap()
                    .to_string_lossy()
                    .to_string();
                if spec_name == "changes" {
                    continue;
                }
                if let Ok(contract_entries) = fs::read_dir(&entry_path) {
                    for ce in contract_entries.flatten() {
                        let cp = ce.path();
                        if cp.extension().and_then(|e| e.to_str()) != Some("toml") {
                            continue;
                        }
                        let slug = cp.file_stem().unwrap().to_string_lossy().to_string();
                        if !known_spec_slugs.contains(&(spec_name.clone(), slug.clone())) {
                            results.push(LintResult::new(
                                format!(
                                    "contract {}/{}.toml has no matching scenario",
                                    spec_name, slug
                                ),
                                Severity::Error,
                            ));
                        }
                    }
                }
            }
        }
        Ok(results)
    }
}

struct UnknownArchetypeCheck {
    repo_root: std::path::PathBuf,
    specs_dir: std::path::PathBuf,
}
impl DoctorCheck for UnknownArchetypeCheck {
    fn name(&self) -> &'static str {
        "unknown-archetypes"
    }
    fn description(&self) -> &'static str {
        "Check contracts reference known archetypes"
    }
    fn run(&self, _repo_root: &Path) -> Result<Vec<LintResult>, Box<dyn std::error::Error>> {
        if !self.specs_dir.exists() {
            return Ok(vec![]);
        }
        let mut results = Vec::new();
        let espectacular_dir = self.repo_root.join(".espectacular");
        if let Ok(entries) = fs::read_dir(&espectacular_dir) {
            for entry in entries.flatten() {
                let entry_path = entry.path();
                if !entry_path.is_dir() {
                    continue;
                }
                let spec_name = entry_path
                    .file_name()
                    .unwrap()
                    .to_string_lossy()
                    .to_string();
                if spec_name == "changes" {
                    continue;
                }
                if let Ok(contract_entries) = fs::read_dir(&entry_path) {
                    for ce in contract_entries.flatten() {
                        let cp = ce.path();
                        if cp.extension().and_then(|e| e.to_str()) != Some("toml") {
                            continue;
                        }
                        let slug = cp.file_stem().unwrap().to_string_lossy().to_string();
                        if let Ok(contract) = contracts::load_contract(cp.to_str().unwrap()) {
                            if !contract.archetype.is_empty()
                                && !archetypes::is_known(&contract.archetype)
                            {
                                results.push(LintResult::new(
                                    format!(
                                        "{}/{}.toml has unknown archetype: {}",
                                        spec_name, slug, contract.archetype
                                    ),
                                    Severity::Error,
                                ));
                            }
                        }
                    }
                }
            }
        }
        Ok(results)
    }
}

struct ManagedBlockCheck {
    filename: &'static str,
}
impl DoctorCheck for ManagedBlockCheck {
    fn name(&self) -> &'static str {
        "managed-block"
    }
    fn description(&self) -> &'static str {
        "Check that agent files have the ah managed block"
    }
    fn run(&self, repo_root: &Path) -> Result<Vec<LintResult>, Box<dyn std::error::Error>> {
        let path = repo_root.join(self.filename);
        if path.exists() && !ah_block_injector().has_block(&path, "ah:managed") {
            Ok(vec![LintResult::new(
                format!(
                    "{filename} is missing the ah managed block",
                    filename = self.filename
                ),
                Severity::Error,
            )])
        } else {
            Ok(vec![])
        }
    }
}

struct HookCheck;
impl DoctorCheck for HookCheck {
    fn name(&self) -> &'static str {
        "hook-framework"
    }
    fn description(&self) -> &'static str {
        "Check that a supported pre-commit hook framework is installed"
    }
    fn run(&self, repo_root: &Path) -> Result<Vec<LintResult>, Box<dyn std::error::Error>> {
        // genesis::git_hooks is the canonical detector (espectacular-6ye);
        // Husky counts as detected (present), wiring remains lefthook-only.
        match git_hooks::framework(repo_root) {
            git_hooks::Framework::None => Ok(vec![LintResult::new(
                "no supported pre-commit hook framework detected (lefthook or prek)",
                Severity::Error,
            )]),
            _ => Ok(vec![]),
        }
    }
}

/// Stage-scoped wiring check (espectacular-6ye R4): a lefthook repo must have
/// `ah check` in the pre-commit stage, and should have it in pre-push too —
/// a framework that is detected but not wired is decorative.
struct HookWiredCheck;
impl DoctorCheck for HookWiredCheck {
    fn name(&self) -> &'static str {
        "hook-wired"
    }
    fn description(&self) -> &'static str {
        "Check that ah check is wired into the lefthook pre-commit and pre-push stages"
    }
    fn run(&self, repo_root: &Path) -> Result<Vec<LintResult>, Box<dyn std::error::Error>> {
        use genesis::git_hooks::lefthook::{is_wired, Stage};
        if git_hooks::framework(repo_root) != git_hooks::Framework::Lefthook {
            return Ok(vec![]);
        }
        let _results: Vec<LintResult> = Vec::new();
        if !is_wired(repo_root, git_hooks::lefthook::Stage::PreCommit, "ah check") {
            return Ok(vec![LintResult::new(
                "lefthook pre-commit stage does not run `ah check` — run `ah init` to wire it",
                Severity::Error,
            )]);
        }
        if !is_wired(repo_root, Stage::PrePush, "ah check") {
            return Ok(vec![LintResult::new(
                "lefthook pre-push stage does not run `ah check` — run `ah init` to wire it",
                Severity::Error,
            )]);
        }
        Ok(vec![])
    }
}

// ── Framework detection checks (produce detections & recommendations) ──

fn framework_result(
    framework: &str,
    repo_root: &Path,
    cfg: &config::Config,
) -> (Option<FrameworkDetection>, Option<CapabilitySuggestion>) {
    match adapters::detect(repo_root, cfg, framework) {
        Some(DetectionSource::Configured) => (
            Some(FrameworkDetection {
                name: framework.to_string(),
                detection_source: DetectionSource::Configured,
            }),
            None,
        ),
        Some(source) => (
            None,
            Some(CapabilitySuggestion {
                capability: framework.to_string(),
                detail: format!("{framework} detected via {}", source_label(source)),
                apply_command: format!("ah doctor --enable {framework}"),
            }),
        ),
        None => (None, None),
    }
}

fn property_result(
    repo_root: &Path,
    cfg: &config::Config,
) -> (Option<FrameworkDetection>, Option<CapabilitySuggestion>) {
    match detect_property(repo_root, cfg) {
        Some(DetectionSource::Configured) => (
            Some(FrameworkDetection {
                name: "property".to_string(),
                detection_source: DetectionSource::Configured,
            }),
            None,
        ),
        Some(source) => (
            None,
            Some(CapabilitySuggestion {
                capability: "property".to_string(),
                detail: format!(
                    "property-based testing framework detected via {}",
                    source_label(source)
                ),
                apply_command: "ah doctor --enable property".to_string(),
            }),
        ),
        None => (None, None),
    }
}

fn source_label(source: DetectionSource) -> &'static str {
    match source {
        DetectionSource::Configured => "configured",
        DetectionSource::Manifest => "manifest",
        DetectionSource::Environment => "environment",
        DetectionSource::SourceImport => "source_import",
    }
}

fn detect_property(repo_root: &Path, cfg: &config::Config) -> Option<DetectionSource> {
    if cfg
        .capabilities
        .property
        .as_ref()
        .map(|c| c.enabled)
        .unwrap_or(false)
    {
        return Some(DetectionSource::Configured);
    }
    if let Ok(text) = fs::read_to_string(repo_root.join("pyproject.toml")) {
        if text.contains("hypothesis") {
            return Some(DetectionSource::Manifest);
        }
    }
    if let Ok(text) = fs::read_to_string(repo_root.join("Cargo.toml")) {
        if text.contains("proptest") {
            return Some(DetectionSource::Manifest);
        }
    }
    None
}

// ── Build the runner ──────────────────────────────────────────────────

fn build_checks(repo_root: &Path) -> Vec<Box<dyn DoctorCheck>> {
    let mut checks: Vec<Box<dyn DoctorCheck>> = Vec::new();

    // Config-independent checks
    checks.push(Box::new(ConfigCheck));
    checks.push(Box::new(HookCheck));
    checks.push(Box::new(HookWiredCheck));
    for &filename in &["AGENTS.md", "CLAUDE.md"] {
        checks.push(Box::new(ManagedBlockCheck { filename }));
    }

    // Config-dependent checks
    if let Ok(cfg) = config::load(repo_root) {
        let specs_dir = repo_root.join(&cfg.paths.specs);

        checks.push(Box::new(VersionDriftCheck {
            config: cfg.clone(),
        }));
        checks.push(Box::new(SpecsDirCheck {
            specs_dir: specs_dir.clone(),
        }));
        checks.push(Box::new(ChangesDirCheck {
            changes_dir: repo_root.join(&cfg.paths.changes),
        }));
        checks.push(Box::new(CollisionCheck {
            repo_root: repo_root.to_path_buf(),
            specs_dir: specs_dir.clone(),
        }));
        checks.push(Box::new(OrphanContractCheck {
            repo_root: repo_root.to_path_buf(),
            specs_dir: specs_dir.clone(),
        }));
        checks.push(Box::new(UnknownArchetypeCheck {
            repo_root: repo_root.to_path_buf(),
            specs_dir,
        }));
    }

    checks
}

// ── Public API ────────────────────────────────────────────────────────

pub fn run_doctor(repo_root: &Path) -> anyhow::Result<DoctorOutcome> {
    let checks = build_checks(repo_root);
    let mut detections: Vec<FrameworkDetection> = Vec::new();
    let mut suggestions: Vec<CapabilitySuggestion> = Vec::new();
    let mut session_suggestions: Vec<SessionSuggestion> = Vec::new();

    // Run genesis doctor framework
    let runner = genesis::doctor::DoctorRunner::new(checks).with_tool_name("ah");
    let genesis_report = runner
        .run(repo_root, false)
        .map_err(|e| anyhow::anyhow!("{}", e))?;

    // Framework detection (domain-specific, not in genesis)
    if let Ok(cfg) = config::load(repo_root) {
        for &fw in &["pytest", "cargo", "vitest"] {
            let (det, rec) = framework_result(fw, repo_root, &cfg);
            if let Some(d) = det {
                detections.push(d);
            }
            if let Some(r) = rec {
                suggestions.push(r);
            }
        }
        let (det, rec) = property_result(repo_root, &cfg);
        if let Some(d) = det {
            detections.push(d);
        }
        if let Some(r) = rec {
            suggestions.push(r);
        }
    }

    if let Some(s) = lint_session_suggestion(repo_root) {
        session_suggestions.push(s);
    }

    Ok(DoctorOutcome {
        genesis_report,
        detections,
        suggestions,
        session_suggestions,
    })
}

/// Marker recording a completed lint run. Its absence is the doctor's signal
/// that no lint run has completed in the session — a suggested step, never a
/// gate blocker.
pub fn lint_marker_path(repo_root: &Path) -> std::path::PathBuf {
    repo_root.join(".espectacular/state/lint-last-run")
}

/// Record a completed lint run. Best-effort: only configured repos get the
/// marker (fixture and adopting repos stay untouched), and write failures
/// never fail the lint command.
pub fn record_lint_run(repo_root: &Path) {
    if !repo_root.join(crate::config::CONFIG_MARKER).exists() {
        return;
    }
    let marker = lint_marker_path(repo_root);
    if let Some(parent) = marker.parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    let _ = std::fs::write(&marker, chrono_now_iso());
}

/// Session timestamp (no chrono dependency in this tool — UTC from unix time).
fn chrono_now_iso() -> String {
    let secs = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    format!("lint-run unix_ts={secs}")
}

/// Suggest `ah lint` when the repo is configured but no lint run has
/// completed in the session. Advisory-only by construction: it lands in
/// `session_suggestions`, which never influences the health verdict.
fn lint_session_suggestion(repo_root: &Path) -> Option<SessionSuggestion> {
    if !repo_root.join(crate::config::CONFIG_MARKER).exists() {
        return None;
    }
    if lint_marker_path(repo_root).exists() {
        return None;
    }
    Some(SessionSuggestion {
        detail:
            "no lint run has completed in this session — surface authoring-quality findings early"
                .to_string(),
        apply_command: "ah lint".to_string(),
    })
}

/// Serialize a doctor outcome to a JSON envelope.
///
/// The envelope data contains both the genesis doctor report (checks, summary, tool)
/// and the domain-specific suggestions as a `suggestions` array.
pub fn doctor_to_envelope(outcome: &DoctorOutcome) -> serde_json::Value {
    let suggestions_json: Vec<serde_json::Value> = outcome
        .suggestions
        .iter()
        .map(|s| {
            serde_json::json!({
                "kind": "recommendation",
                "suggested_action": "enable_capability",
                "playbook_command": "ah explain enable_capability",
                "apply_command": s.apply_command,
                "detail": s.detail,
                "capability": s.capability,
            })
        })
        .collect();

    let session_suggestions_json: Vec<serde_json::Value> = outcome
        .session_suggestions
        .iter()
        .map(|s| {
            serde_json::json!({
                "kind": "suggestion",
                "suggested_action": "run_ah_lint",
                "playbook_command": "ah explain lint",
                "apply_command": s.apply_command,
                "detail": s.detail,
            })
        })
        .collect();

    let data = serde_json::json!({
        "tool": outcome.genesis_report.tool,
        "checks": outcome.genesis_report.checks,
        "summary": outcome.genesis_report.summary,
        "suggestions": suggestions_json,
        "session_suggestions": session_suggestions_json,
    });

    let envelope = genesis::envelope::Envelope::success(
        env!("CARGO_PKG_VERSION"),
        genesis::envelope::EnvelopeKind::Doctor,
        &data,
        vec![],
        vec![],
    );
    serde_json::to_value(&envelope).expect("envelope serialization")
}

// ── Status section ────────────────────────────────────────────────────

#[allow(dead_code)]
pub fn status_section(repo_root: &Path) -> Result<StatusSection, String> {
    let outcome = run_doctor(repo_root).map_err(|e| e.to_string())?;
    let genesis_report = &outcome.genesis_report;

    let summary = if genesis_report.is_healthy() {
        format!(
            "all checks passed ({} detections)",
            outcome.detections.len()
        )
    } else {
        let issues: Vec<_> = genesis_report
            .checks
            .iter()
            .filter(|c| c.status.is_issue())
            .collect();
        format!("{} issue(s) found", issues.len())
    };

    let items: Vec<genesis::status::StatusItem> = genesis_report
        .checks
        .iter()
        .filter(|c| !c.status.is_pass())
        .cloned()
        .map(check_entry_to_status_item)
        .collect();

    Ok(StatusSection::with_items("espectacular", summary, items))
}

// ── Enable capability ─────────────────────────────────────────────────

use crate::init::{append_capability_block, insert_runner_entry};

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
        "pytest" | "cargo" | "vitest" => {
            if cfg.runners.contains_key(capability) {
                return Ok(DoctorEnableResult::AlreadyEnabled);
            }
            let value_toml = match capability {
                "pytest" => r#"["pytest"]"#,
                "cargo" => r#"["cargo", "test"]"#,
                "vitest" => r#"["vitest", "run"]"#,
                _ => unreachable!(),
            };
            let text = fs::read_to_string(&config_path)?;
            let updated = insert_runner_entry(&text, capability, value_toml);
            fs::write(&config_path, &updated)?;
            Ok(DoctorEnableResult::Written {
                path: config_path.to_string_lossy().into_owned(),
                table_name: format!("runners.{capability}"),
            })
        }
        "mutation" => {
            if cfg.quality.mutation.is_some() {
                return Ok(DoctorEnableResult::AlreadyEnabled);
            }
            let text = fs::read_to_string(&config_path)?;
            let trimmed = text.trim_end();
            let updated = format!(
                "{trimmed}\n\n[quality.mutation]\nenabled = true\nthreshold = 0.80\ncommand = [\"{{}}\"]\n"
            );
            fs::write(&config_path, &updated)?;
            Ok(DoctorEnableResult::Written {
                path: config_path.to_string_lossy().into_owned(),
                table_name: "quality.mutation".to_string(),
            })
        }
        "property" => {
            if cfg.capabilities.property.is_some() {
                return Ok(DoctorEnableResult::AlreadyEnabled);
            }
            let text = fs::read_to_string(&config_path)?;
            let updated = append_capability_block(&text, "property");
            fs::write(&config_path, &updated)?;
            Ok(DoctorEnableResult::Written {
                path: config_path.to_string_lossy().into_owned(),
                table_name: "capabilities.property".to_string(),
            })
        }
        "snapshot" => {
            if cfg.capabilities.snapshot.is_some() {
                return Ok(DoctorEnableResult::AlreadyEnabled);
            }
            let text = fs::read_to_string(&config_path)?;
            let updated = append_capability_block(&text, "snapshot");
            fs::write(&config_path, &updated)?;
            Ok(DoctorEnableResult::Written {
                path: config_path.to_string_lossy().into_owned(),
                table_name: "capabilities.snapshot".to_string(),
            })
        }
        _ => unreachable!(),
    }
}

// ── Tests ─────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::init::insert_runner_entry;
    use std::fs;
    use tempfile::TempDir;

    pub(super) fn make_healthy_repo() -> TempDir {
        let dir = TempDir::new().unwrap();
        let root = dir.path();

        fs::create_dir_all(root.join("openspec/specs")).unwrap();
        fs::create_dir_all(root.join("openspec/changes")).unwrap();
        fs::create_dir_all(root.join(".espectacular")).unwrap();
        fs::write(
            root.join(".espectacular/config.toml"),
            format!(
                r#"tool_version = "{}"
[paths]
specs = "openspec/specs"
changes = "openspec/changes"
[runners]
"#,
                TOOL_VERSION
            ),
        )
        .unwrap();

        fs::write(
            root.join("AGENTS.md"),
            format!(
                "# Project\n\n{}\n",
                crate::init::AH_BLOCK_CONTENT_WITH_MARKERS
            ),
        )
        .unwrap();
        fs::write(
            root.join("CLAUDE.md"),
            format!(
                "# Project\n\n{}\n",
                crate::init::AH_BLOCK_CONTENT_WITH_MARKERS
            ),
        )
        .unwrap();
        fs::write(
            root.join("lefthook.yml"),
            "pre-commit:\n  commands:\n    ah-check:\n      run: ah check\npre-push:\n  commands:\n    ah-check:\n      run: ah check\n",
        )
        .unwrap();

        dir
    }

    fn base_config_toml() -> String {
        format!(
            "tool_version = \"{}\"\n[paths]\nspecs = \"openspec/specs\"\nchanges = \"openspec/changes\"\n[runners]\n",
            TOOL_VERSION
        )
    }

    /// Checks that did not pass, in the genesis report.
    fn issues(outcome: &DoctorOutcome) -> Vec<&genesis::doctor::CheckEntry> {
        outcome
            .genesis_report
            .checks
            .iter()
            .filter(|c| !c.status.is_pass())
            .collect()
    }

    fn has_issue(outcome: &DoctorOutcome, kind: &str) -> bool {
        issues(outcome).iter().any(|c| c.name == kind)
    }

    #[test]
    fn healthy_repo_exits_zero_with_no_diagnostics() {
        let repo = make_healthy_repo();
        let report = run_doctor(repo.path()).unwrap();
        assert!(
            report.genesis_report.is_healthy(),
            "healthy repo must report healthy=true; diagnostics: {:?}",
            issues(&report)
        );
        assert!(
            issues(&report).is_empty(),
            "healthy repo must have no diagnostics; got: {:?}",
            issues(&report)
        );
    }

    #[test]
    fn bad_config_emits_config_schema_diagnostic() {
        let repo = make_healthy_repo();
        fs::write(
            repo.path().join(".espectacular/config.toml"),
            "tool_version = \"\"\n[paths]\nspecs = \"\"\nchanges = \"\"\n[runners]\n",
        )
        .unwrap();
        let report = run_doctor(repo.path()).unwrap();
        assert!(!report.genesis_report.is_healthy());
        assert!(
            has_issue(&report, "config"),
            "bad config must emit config diagnostic; got: {:?}",
            issues(&report)
        );
    }

    #[test]
    fn missing_specs_path_emits_missing_paths_diagnostic() {
        let repo = make_healthy_repo();
        fs::remove_dir_all(repo.path().join("openspec/specs")).unwrap();
        let report = run_doctor(repo.path()).unwrap();
        assert!(!report.genesis_report.is_healthy());
        assert!(
            has_issue(&report, "missing-specs-dir"),
            "missing specs dir must emit missing-specs-dir diagnostic; got: {:?}",
            issues(&report)
        );
    }

    #[test]
    fn missing_changes_path_emits_missing_paths_diagnostic() {
        let repo = make_healthy_repo();
        fs::remove_dir_all(repo.path().join("openspec/changes")).unwrap();
        let report = run_doctor(repo.path()).unwrap();
        assert!(!report.genesis_report.is_healthy());
        assert!(
            has_issue(&report, "missing-changes-dir"),
            "missing changes dir must emit missing-changes-dir diagnostic; got: {:?}",
            issues(&report)
        );
    }

    #[test]
    fn version_drift_emits_version_drift_diagnostic() {
        let repo = make_healthy_repo();
        fs::write(
            repo.path().join(".espectacular/config.toml"),
            r#"tool_version = "0.0.1"
[paths]
specs = "openspec/specs"
changes = "openspec/changes"
[runners]
"#,
        )
        .unwrap();
        let report = run_doctor(repo.path()).unwrap();
        assert!(!report.genesis_report.is_healthy());
        assert!(
            has_issue(&report, "version-drift"),
            "version mismatch must emit version-drift diagnostic; got: {:?}",
            issues(&report)
        );
    }

    #[test]
    fn missing_managed_block_in_agents_md_emits_diagnostic() {
        let repo = make_healthy_repo();
        fs::write(
            repo.path().join("AGENTS.md"),
            "# Project\n\nNo block here.\n",
        )
        .unwrap();
        let report = run_doctor(repo.path()).unwrap();
        assert!(!report.genesis_report.is_healthy());
        assert!(
            has_issue(&report, "managed-block"),
            "missing managed block must emit managed-block diagnostic; got: {:?}",
            issues(&report)
        );
    }

    #[test]
    fn missing_managed_block_in_claude_md_emits_diagnostic() {
        let repo = make_healthy_repo();
        fs::write(
            repo.path().join("CLAUDE.md"),
            "# Project\n\nNo block here.\n",
        )
        .unwrap();
        let report = run_doctor(repo.path()).unwrap();
        assert!(!report.genesis_report.is_healthy());
        assert!(
            has_issue(&report, "managed-block"),
            "missing managed block in CLAUDE.md must emit managed-block diagnostic; got: {:?}",
            issues(&report)
        );
    }

    #[test]
    fn hook_absent_emits_hook_absent_diagnostic() {
        let repo = make_healthy_repo();
        fs::remove_file(repo.path().join("lefthook.yml")).unwrap();
        let report = run_doctor(repo.path()).unwrap();
        assert!(!report.genesis_report.is_healthy());
        assert!(
            has_issue(&report, "hook-framework"),
            "no hook framework must emit hook-framework diagnostic; got: {:?}",
            issues(&report)
        );
    }

    #[test]
    fn hook_prek_toml_is_supported_framework() {
        // RED (espectacular-6ye R1): genesis::git_hooks detects `prek.toml`;
        // the legacy detector only knew .prek/prek.yml, so a prek.toml repo
        // wrongly got a hook-framework Error diagnostic.
        let repo = make_healthy_repo();
        fs::remove_file(repo.path().join("lefthook.yml")).unwrap();
        fs::write(repo.path().join("prek.toml"), "[hooks]\n").unwrap();
        let report = run_doctor(repo.path()).unwrap();
        assert!(
            !has_issue(&report, "hook-framework"),
            "prek.toml repo must not emit hook-framework diagnostic; got: {:?}",
            issues(&report)
        );
    }

    #[test]
    fn hook_prek_dotfile_is_no_longer_a_detection_signal() {
        // BEHAVIORAL DRIFT accepted (6ye): genesis detects prek via prek.toml
        // only; the legacy .prek/prek.yml signals are dropped with it.
        let repo = make_healthy_repo();
        fs::remove_file(repo.path().join("lefthook.yml")).unwrap();
        fs::write(repo.path().join(".prek"), "").unwrap();
        let report = run_doctor(repo.path()).unwrap();
        assert!(
            has_issue(&report, "hook-framework"),
            ".prek alone must not count as a supported framework; got: {:?}",
            issues(&report)
        );
    }

    #[test]
    fn hook_wired_missing_in_prepush_emits_hook_wired_diagnostic() {
        // RED (espectacular-6ye R4): lefthook present but ah check not wired
        // into the pre-push stage → hook-wired diagnostic.
        let repo = make_healthy_repo();
        fs::write(
            repo.path().join("lefthook.yml"),
            "pre-commit:\n  commands:\n    ah-check:\n      run: ah check\n",
        )
        .unwrap();
        let report = run_doctor(repo.path()).unwrap();
        assert!(
            has_issue(&report, "hook-wired"),
            "lefthook without pre-push ah check must emit hook-wired; got: {:?}",
            issues(&report)
        );
    }

    #[test]
    fn hook_wired_both_stages_passes() {
        let repo = make_healthy_repo();
        fs::write(
            repo.path().join("lefthook.yml"),
            "pre-commit:\n  commands:\n    ah-check:\n      run: ah check\npre-push:\n  commands:\n    ah-check:\n      run: ah check\n",
        )
        .unwrap();
        let report = run_doctor(repo.path()).unwrap();
        assert!(
            !has_issue(&report, "hook-wired"),
            "both stages wired must pass; got: {:?}",
            issues(&report)
        );
    }

    #[test]
    fn scenario_collision_emits_collision_diagnostic() {
        let repo = make_healthy_repo();
        let spec_dir = repo.path().join("openspec/specs/compiler");
        fs::create_dir_all(&spec_dir).unwrap();
        // adopt-dual-format-specs: collisions now mean same id AND different
        // bodies; identical bodies are section-sync mirrors and dedupe.
        let content = "# Capability: compiler\n\n## DEPLOYED Requirements\n\n### Requirement: R\n\n#### Scenario: Empty input rejected\n- **GIVEN** x\n- **WHEN** y\n- **THEN** z\n\n#### Scenario: Empty input rejected\n- **GIVEN** null bytes\n- **WHEN** y\n- **THEN** z\n";
        fs::write(spec_dir.join("spec.md"), content).unwrap();
        let report = run_doctor(repo.path()).unwrap();
        assert!(!report.genesis_report.is_healthy());
        assert!(
            has_issue(&report, "scenario-collisions"),
            "duplicate scenario headings must emit scenario-collisions diagnostic; got: {:?}",
            issues(&report)
        );
    }

    #[test]
    fn orphan_contract_emits_orphan_diagnostic() {
        let repo = make_healthy_repo();
        let contract_dir = repo.path().join(".espectacular/compiler");
        fs::create_dir_all(&contract_dir).unwrap();
        fs::write(
            contract_dir.join("ghost-scenario.toml"),
            "id = \"ghost-scenario\"\ndescription = \"\"\narchetype = \"PF\"\nstatus = \"active\"\nsuperseded_by = \"\"\nauthored_with = \"0.1.0\"\n[tests]\n",
        )
        .unwrap();
        let report = run_doctor(repo.path()).unwrap();
        assert!(!report.genesis_report.is_healthy());
        assert!(
            has_issue(&report, "orphan-contracts"),
            "orphan contract must emit orphan-contracts diagnostic; got: {:?}",
            issues(&report)
        );
    }

    #[test]
    fn unknown_archetype_emits_diagnostic() {
        let repo = make_healthy_repo();
        let spec_dir = repo.path().join("openspec/specs/compiler");
        fs::create_dir_all(&spec_dir).unwrap();
        let spec_content = "# Capability: compiler\n\n## DEPLOYED Requirements\n\n### Requirement: R\n\n#### Scenario: Empty input rejected\n- **GIVEN** x\n- **WHEN** y\n- **THEN** z\n";
        fs::write(spec_dir.join("spec.md"), spec_content).unwrap();
        let contract_dir = repo.path().join(".espectacular/compiler");
        fs::create_dir_all(&contract_dir).unwrap();
        fs::write(
            contract_dir.join("empty-input-rejected.toml"),
            "id = \"empty-input-rejected\"\ndescription = \"\"\narchetype = \"UNKNOWN_TYPE\"\nstatus = \"active\"\nsuperseded_by = \"\"\nauthored_with = \"0.1.0\"\n[tests]\n",
        )
        .unwrap();
        let report = run_doctor(repo.path()).unwrap();
        assert!(!report.genesis_report.is_healthy());
        assert!(
            has_issue(&report, "unknown-archetypes"),
            "unknown archetype must emit unknown-archetypes diagnostic; got: {:?}",
            issues(&report)
        );
    }

    #[test]
    fn each_problem_emits_exactly_one_diagnostic_kind() {
        let repo = make_healthy_repo();
        fs::write(
            repo.path().join(".espectacular/config.toml"),
            "tool_version = \"\"\n[paths]\nspecs = \"\"\nchanges = \"\"\n[runners]\n",
        )
        .unwrap();
        let report = run_doctor(repo.path()).unwrap();
        let config_count = issues(&report)
            .iter()
            .filter(|c| c.name == "config")
            .count();
        assert_eq!(config_count, 1, "should emit exactly one config diagnostic");
    }

    #[test]
    fn configured_pytest_runner_appears_in_detections() {
        let repo = make_healthy_repo();
        fs::write(
            repo.path().join(".espectacular/config.toml"),
            format!(
                "tool_version = \"{TOOL_VERSION}\"\n[paths]\nspecs = \"openspec/specs\"\nchanges = \"openspec/changes\"\n[runners]\npytest = [\"pytest\"]\n"
            ),
        )
        .unwrap();
        let report = run_doctor(repo.path()).unwrap();
        assert!(
            report
                .detections
                .iter()
                .any(|d| d.name == "pytest" && d.detection_source == DetectionSource::Configured),
            "configured pytest must appear in detections; got: {:?}",
            report.detections
        );
    }

    #[test]
    fn configured_cargo_runner_appears_in_detections() {
        let repo = make_healthy_repo();
        fs::write(
            repo.path().join(".espectacular/config.toml"),
            format!(
                "tool_version = \"{TOOL_VERSION}\"\n[paths]\nspecs = \"openspec/specs\"\nchanges = \"openspec/changes\"\n[runners]\ncargo = [\"cargo\", \"test\"]\n"
            ),
        )
        .unwrap();
        let report = run_doctor(repo.path()).unwrap();
        assert!(
            report
                .detections
                .iter()
                .any(|d| d.name == "cargo" && d.detection_source == DetectionSource::Configured),
            "configured cargo must appear in detections; got: {:?}",
            report.detections
        );
    }

    #[test]
    fn configured_vitest_runner_appears_in_detections() {
        let repo = make_healthy_repo();
        fs::write(
            repo.path().join(".espectacular/config.toml"),
            format!(
                "tool_version = \"{TOOL_VERSION}\"\n[paths]\nspecs = \"openspec/specs\"\nchanges = \"openspec/changes\"\n[runners]\nvitest = [\"vitest\", \"run\"]\n"
            ),
        )
        .unwrap();
        let report = run_doctor(repo.path()).unwrap();
        assert!(
            report
                .detections
                .iter()
                .any(|d| d.name == "vitest" && d.detection_source == DetectionSource::Configured),
            "configured vitest must appear in detections; got: {:?}",
            report.detections
        );
    }

    #[test]
    fn configured_property_capability_appears_in_detections() {
        let repo = make_healthy_repo();
        fs::write(
            repo.path().join(".espectacular/config.toml"),
            format!(
                "tool_version = \"{TOOL_VERSION}\"\n[paths]\nspecs = \"openspec/specs\"\nchanges = \"openspec/changes\"\n[runners]\n[capabilities.property]\nenabled = true\n"
            ),
        )
        .unwrap();
        let report = run_doctor(repo.path()).unwrap();
        assert!(
            report
                .detections
                .iter()
                .any(|d| d.name == "property" && d.detection_source == DetectionSource::Configured),
            "configured property capability must appear in detections; got: {:?}",
            report.detections
        );
    }

    #[test]
    fn framework_detection_does_not_affect_healthy_flag() {
        let repo = make_healthy_repo();
        fs::write(repo.path().join("pytest.ini"), "[pytest]\n").unwrap();
        let report = run_doctor(repo.path()).unwrap();
        assert!(
            report.genesis_report.is_healthy(),
            "recommendations must not make healthy=false; diagnostics: {:?}",
            issues(&report)
        );
    }

    #[test]
    fn pytest_manifest_detected_but_not_configured_emits_recommendation() {
        let repo = make_healthy_repo();
        fs::write(repo.path().join("pytest.ini"), "[pytest]\n").unwrap();
        let report = run_doctor(repo.path()).unwrap();
        let rec = report.suggestions.iter().find(|r| r.capability == "pytest");
        assert!(
            rec.is_some(),
            "pytest detected via manifest must emit recommendation; got: {:?}",
            report.suggestions
        );
        let rec = rec.unwrap();
        assert_eq!(
            rec.apply_command, "ah doctor --enable pytest",
            "recommendation apply_command must be --enable invocation"
        );
    }

    #[test]
    fn cargo_manifest_detected_but_not_configured_emits_recommendation() {
        let repo = make_healthy_repo();
        fs::write(repo.path().join("Cargo.toml"), "[package]\nname = \"x\"\n").unwrap();
        let report = run_doctor(repo.path()).unwrap();
        let rec = report.suggestions.iter().find(|r| r.capability == "cargo");
        assert!(
            rec.is_some(),
            "cargo detected via manifest must emit recommendation; got: {:?}",
            report.suggestions
        );
        assert_eq!(rec.unwrap().apply_command, "ah doctor --enable cargo");
    }

    #[test]
    fn vitest_manifest_detected_but_not_configured_emits_recommendation() {
        let repo = make_healthy_repo();
        fs::write(
            repo.path().join("package.json"),
            r#"{"devDependencies":{"vitest":"^1.0"}}"#,
        )
        .unwrap();
        let report = run_doctor(repo.path()).unwrap();
        let rec = report.suggestions.iter().find(|r| r.capability == "vitest");
        assert!(
            rec.is_some(),
            "vitest detected via manifest must emit recommendation; got: {:?}",
            report.suggestions
        );
        assert_eq!(rec.unwrap().apply_command, "ah doctor --enable vitest");
    }

    #[test]
    fn property_hypothesis_detected_emits_recommendation() {
        let repo = make_healthy_repo();
        fs::write(
            repo.path().join("pyproject.toml"),
            "[tool.pytest.ini_options]\n\n[project]\ndependencies = [\"hypothesis\"]\n",
        )
        .unwrap();
        let report = run_doctor(repo.path()).unwrap();
        let rec = report
            .suggestions
            .iter()
            .find(|r| r.capability == "property");
        assert!(
            rec.is_some(),
            "hypothesis in pyproject must emit property recommendation; got: {:?}",
            report.suggestions
        );
        assert_eq!(rec.unwrap().apply_command, "ah doctor --enable property");
    }

    #[test]
    fn configured_framework_emits_detection_not_recommendation() {
        let repo = make_healthy_repo();
        fs::write(repo.path().join("pytest.ini"), "[pytest]\n").unwrap();
        fs::write(
            repo.path().join(".espectacular/config.toml"),
            format!(
                "tool_version = \"{TOOL_VERSION}\"\n[paths]\nspecs = \"openspec/specs\"\nchanges = \"openspec/changes\"\n[runners]\npytest = [\"pytest\"]\n"
            ),
        )
        .unwrap();
        let report = run_doctor(repo.path()).unwrap();
        assert!(
            report
                .detections
                .iter()
                .any(|d| d.name == "pytest" && d.detection_source == DetectionSource::Configured),
            "configured pytest should be in detections"
        );
        assert!(
            !report.suggestions.iter().any(|r| r.capability == "pytest"),
            "configured pytest must NOT be in recommendations; got: {:?}",
            report.suggestions
        );
    }

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
}

#[cfg(test)]
mod lint_session_tests {
    use super::*;
    use tempfile::TempDir;

    use super::tests::make_healthy_repo;

    // espectacular-eia (task 5.1): doctor suggests `ah lint` when no lint
    // run has completed in the session. Advisory-only — never a gate blocker.

    #[test]
    fn doctor_suggests_lint_when_no_lint_run_completed() {
        let repo = make_healthy_repo();
        let outcome = run_doctor(repo.path()).unwrap();
        let sugg = outcome
            .session_suggestions
            .iter()
            .find(|s| s.apply_command == "ah lint");
        assert!(sugg.is_some(), "expected an ah lint suggestion");
        assert!(sugg.unwrap().detail.contains("lint"));
    }

    #[test]
    fn doctor_stays_silent_after_lint_run_completed() {
        let repo = make_healthy_repo();
        record_lint_run(repo.path());
        let outcome = run_doctor(repo.path()).unwrap();
        assert!(
            !outcome
                .session_suggestions
                .iter()
                .any(|s| s.apply_command == "ah lint"),
            "lint suggestion must disappear once a lint run completed"
        );
    }

    #[test]
    fn lint_suggestion_never_fails_the_doctor_run() {
        // Anti-goal guard: a pending lint suggestion must not flip the
        // genesis health verdict.
        let repo = make_healthy_repo();
        let outcome = run_doctor(repo.path()).unwrap();
        assert!(outcome.genesis_report.is_healthy());
        assert!(!outcome.session_suggestions.is_empty());
    }

    #[test]
    fn record_lint_run_writes_marker_best_effort() {
        let repo = TempDir::new().unwrap();
        // No .espectacular dir → no marker, no error (adoption-friendly).
        record_lint_run(repo.path());
        assert!(!lint_marker_path(repo.path()).exists());
        // Configured repo → marker written.
        fs::create_dir_all(repo.path().join(".espectacular")).unwrap();
        fs::write(
            repo.path().join(".espectacular/config.toml"),
            concat!("tool_version = \"", env!("CARGO_PKG_VERSION"), "\"\n[paths]\nspecs = \"openspec/specs\"\nchanges = \"openspec/changes\"\n[runners]\n"),
        )
        .unwrap();
        record_lint_run(repo.path());
        assert!(lint_marker_path(repo.path()).exists());
    }
}
