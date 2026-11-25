//! Dual-format lint bridge (espectacular-zlw).
//!
//! Purpose: relay specodelic (`spk`) findings for dual-format spec files into
//! the shared lint finding schema by invoking `spk lint` — never
//! reimplementing specodelic rules.
//!
//! Responsibilities:
//! - detect dual-format spec files (`id: spec` YAML frontmatter)
//! - invoke `spk lint <file> --json` and map `data.issues` to findings with
//!   `kind = "spk.<rule_id>"` at warning severity
//! - degrade advisory-only: missing binary → one `spk-unavailable` finding;
//!   broken invocation/parse → one `spk-bridge-failure` finding; plain
//!   openspec files are never touched
//!
//! Rationale: delta spec "Requirement: Dual-Format Lint Bridge" in
//! `openspec/changes/add-spec-quality-checks/specs/lint/spec.md`.
//!
//! Layout: `relay` owns the per-spec relay and its advisory degradations,
//! `invoke` owns the `spk lint` process seam, and `graph` owns the additive
//! `spk graph` relay (task 5.2).

#[cfg(test)]
mod dir_mode_tests;
mod graph;
mod invoke;
mod relay;
#[cfg(test)]
mod tests;

pub(crate) use invoke::{invoke, invoke_dir, issue_matches_path, Failure, SpkIssue};
pub use relay::relay_dual_format;

/// A dual-format spec file carries YAML frontmatter with an `id:` line.
/// Since specodelic Revision 18 the id VALUE follows the parent-dir naming
/// law (`linter.id_matches_file`): any id value routes the file to spk —
/// whether the value is correct is spk's call, never espectacular's. The
/// old rev-17 `id: spec` convention is just one accepted spelling now.
pub(crate) fn is_dual_format(raw: &str) -> bool {
    crate::openspec::frontmatter_id(raw).is_some()
}
