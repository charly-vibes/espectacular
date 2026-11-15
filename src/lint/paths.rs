//! Repo-path resolution for lint: specs and changes directories.

use std::path::Path;

/// Resolve the specs directory for a repo root.
///
/// Falls back to the plain openspec layout (`openspec/specs`) when no
/// espectacular config exists — lint is adoption-friendly and must work in
/// any openspec repo. An existing-but-invalid config is a hard error.
pub fn specs_dir_for(repo_root: &Path) -> anyhow::Result<std::path::PathBuf> {
    if repo_root.join(crate::config::CONFIG_MARKER).exists() {
        let cfg = crate::config::load(repo_root)?;
        return Ok(repo_root.join(&cfg.paths.specs));
    }
    Ok(repo_root.join("openspec/specs"))
}

/// Resolve the changes directory for a repo root (mirrors `specs_dir_for`).
pub fn changes_dir_for(repo_root: &Path) -> anyhow::Result<std::path::PathBuf> {
    if repo_root.join(crate::config::CONFIG_MARKER).exists() {
        let cfg = crate::config::load(repo_root)?;
        return Ok(repo_root.join(&cfg.paths.changes));
    }
    Ok(repo_root.join("openspec/changes"))
}
