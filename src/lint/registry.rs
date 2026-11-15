//! Check registry for `ah lint`: the single place checks are appended.

use super::checks::{flow, shape, trace};
use super::LintCheck;
use std::path::Path;

/// The check registry. Append additively (one entry per check), do not
/// reorder — finding order follows spec-then-check iteration.
pub(crate) fn checks(cfg: &crate::config::LintConfig) -> Vec<Box<dyn LintCheck>> {
    vec![
        Box::new(flow::VagueQualifierCheck),
        Box::new(flow::ImperativeStepCheck),
        Box::new(flow::ConjunctiveBloatCheck {
            max_and_steps: cfg.max_and_steps,
        }),
        Box::new(flow::MissingNegativeScenarioCheck),
        Box::new(shape::MissingNonGoalsCheck),
        Box::new(shape::UnresolvedAmbiguityCheck),
        Box::new(shape::EntangledSpecCheck),
        Box::new(trace::VerifiesDanglingCheck),
        Box::new(trace::ScenarioUnlinkedCheck),
        Box::new(trace::PropertyUntracedCheck),
    ]
}

/// Resolve the `[lint]` config for a specs directory by walking up to the
/// nearest repo root carrying an espectacular config. Falls back to defaults
/// when no config exists (adoption-friendly, mirroring `specs_dir_for`).
pub(crate) fn lint_config_for(specs_dir: &Path) -> anyhow::Result<crate::config::LintConfig> {
    for dir in specs_dir.ancestors() {
        if dir.join(crate::config::CONFIG_MARKER).exists() {
            return Ok(crate::config::load(dir)?.lint);
        }
    }
    Ok(crate::config::LintConfig::default())
}

/// Registered checks, optionally filtered to a single kind (`--check`).
/// An unknown kind is a hard error listing the valid kinds.
pub(crate) fn select_checks(
    cfg: &crate::config::LintConfig,
    check: Option<&str>,
) -> anyhow::Result<Vec<Box<dyn LintCheck>>> {
    let all = checks(cfg);
    let Some(kind) = check else {
        return Ok(all);
    };
    if !all.iter().any(|c| c.kind() == kind) {
        let valid = all.iter().map(|c| c.kind()).collect::<Vec<_>>().join(", ");
        anyhow::bail!("unknown check kind '{kind}' — valid kinds: {valid}");
    }
    Ok(all.into_iter().filter(|c| c.kind() == kind).collect())
}
