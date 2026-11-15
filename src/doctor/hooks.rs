//! Hook-framework wiring checks and mutation-ownership policy check.

use crate::config;
use genesis::doctor::DoctorCheck;
use genesis::git_hooks;
use genesis::suite_linter::{LintResult, Severity};
use std::path::Path;

pub(super) struct MutationOwnershipCheck {
    pub(super) config: config::Config,
}
impl DoctorCheck for MutationOwnershipCheck {
    fn name(&self) -> &'static str {
        "mutation-ownership"
    }
    fn description(&self) -> &'static str {
        "Check mutation scoring ownership (espectacular-djw)"
    }
    fn run(&self, _repo_root: &Path) -> Result<Vec<LintResult>, Box<dyn std::error::Error>> {
        let Some(mutation) = &self.config.quality.mutation else {
            return Ok(vec![]);
        };
        Ok(mutation_ownership_results(mutation))
    }
}

/// Decision A (espectacular-djw): pretender owns mutation execution and
/// scoring; espectacular consumes its report. Warn only on raw-mutator
/// commands — the double-execution/dual-threshold risk. The `{}`
/// generated-runner default (espectacular-mu5) stays warning-free for
/// repos without pretender; it is deprecated in the README instead.
fn mutation_ownership_results(mutation: &config::MutationConfig) -> Vec<LintResult> {
    if mutation.command.is_empty() {
        return vec![];
    }
    let first = mutation.command.first().map(String::as_str).unwrap_or("");
    if first == "pretender" || !is_raw_mutator(first) {
        return vec![];
    }
    vec![LintResult::new(
        "pretender owns mutation execution and scoring \
         (suite decision espectacular-djw, option A). Wire the report consumer: \
         command = [\"pretender\", \"mutation\", \"--format\", \"json\"]; \
         running a raw mutator here risks a duplicate run and a competing threshold",
        Severity::Warning,
    )]
}

fn is_raw_mutator(command: &str) -> bool {
    const RAW_MUTATORS: [&str; 4] = ["cargo-mutants", "mutmut", "stryker", "pit"];
    RAW_MUTATORS.contains(&command)
}

pub(super) struct HookCheck;
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
pub(super) struct HookWiredCheck;
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
