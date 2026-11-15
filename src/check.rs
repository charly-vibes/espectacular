//! Spec↔test correspondence check (`ah check`).
//!
//! Purpose: decide whether every in-scope scenario is falsified by a passing
//! contract test, and report the exact structural or execution defect when it
//! is not.
//!
//! Responsibilities:
//! - resolve the checked scope: deployed specs plus selected change overlays
//! - run the structural correspondence rules (contract presence, id match,
//!   falsifiability class, test declarations, supersede, liveness timeout)
//! - execute the contract bindings of the selected capabilities
//! - emit the shared finding schema, summary counts, and selection report
//!
//! Rationale: the gate's trustworthiness depends on never reporting a green
//! it did not earn — selection and degradation paths are therefore
//! conservative and reported, never silent.
//!
//! Module map: [`scope`] resolves what is in scope, [`verifies`] handles
//! VERIFIES coverage, [`structural`] the correspondence rules, [`stale`]
//! derived-contract drift, [`selection`] changed-file narrowing, [`execute`]
//! contract execution, [`works`] the orchestration, [`report`] the output
//! schema, [`ordering`] stable finding order, and [`contract_files`] on-disk
//! discovery. This root owns the public surface and the shared test fixtures.

mod batch;
mod contract_files;
mod execute;
mod ordering;
mod report;
mod scope;
mod selection;
mod stale;
mod structural;
mod verifies;
mod works;

pub use contract_files::collect_base_contract_files;
pub use report::{CheckOutput, ReportFinding};
pub(crate) use verifies::{contract_path, verifies_property_ids};
#[cfg(test)]
pub(crate) use works::run_check;
pub use works::run_check_with_selection;

/// Fixtures shared by the module tests: throwaway repos with the config,
/// specs, contracts, and runner scripts a check needs.
#[cfg(test)]
pub(crate) mod test_support {
    use std::fs;
    use std::os::unix::fs::PermissionsExt;
    use std::path::Path;

    pub(crate) const SPECS: &str = "tests/fixtures/four-findings/openspec/specs";
    pub(crate) const CONTRACTS: &str = "tests/fixtures/four-findings/.espectacular";

    pub(crate) fn write_executable(path: &Path, body: &str) {
        fs::write(path, format!("#!/bin/sh\nset -eu\n{body}\n")).unwrap();
        let mut perms = fs::metadata(path).unwrap().permissions();
        perms.set_mode(0o755);
        fs::set_permissions(path, perms).unwrap();
    }

    /// One capability, one scenario, one unit binding whose runner echoes its
    /// flags — the smallest repo where a check can legitimately pass.
    pub(crate) fn success_repo() -> tempfile::TempDir {
        let dir = tempfile::tempdir().unwrap();
        let repo = dir.path();
        fs::create_dir_all(repo.join("openspec/specs/compiler")).unwrap();
        fs::create_dir_all(repo.join(".espectacular/compiler")).unwrap();
        fs::write(
            repo.join("openspec/specs/compiler/spec.md"),
            "# Capability: compiler\n\n#### Scenario: Green path\n- **WHEN** it runs\n- **THEN** it passes\n",
        )
        .unwrap();
        fs::write(
            repo.join(".espectacular/config.toml"),
            "tool_version = \"0.1.0\"\n\n[paths]\nspecs = \"openspec/specs\"\nchanges = \"openspec/changes\"\n\n[runners]\nunit = [\"/bin/sh\", \"runner.sh\"]\n",
        )
        .unwrap();
        fs::write(
            repo.join(".espectacular/compiler/green-path.toml"),
            "id = \"green-path\"\ndescription = \"\"\narchetype = \"PF\"\nstatus = \"active\"\nsuperseded_by = \"\"\nauthored_with = \"0.1.0\"\n\n[[tests.unit]]\nflags = \"ok\"\n",
        )
        .unwrap();
        write_executable(&repo.join("runner.sh"), "printf '%s' \"$1\"");
        dir
    }
}

/// Contract-property meta tests (the `p_*` group): each drives the
/// behavioural tests bound to the same property so one property sees every
/// falsifying case in a single run.
#[cfg(test)]
mod tests;
