//! Contract-file discovery on disk.
//!
//! Purpose: enumerate `<contracts_root>/<spec>/<id>.toml` bindings for the
//! structural pass, including the staged `.espectacular/changes/...` overlay.
//!
//! Responsibilities: base (deployed) contract files, the recursive staged
//! walk, and the `changes` pseudo-spec exclusion.
//!
//! Rationale: discovery order is filesystem-dependent, so callers sort or
//! dedupe on the returned tuples; keeping the walk here keeps every caller
//! on the same shape `(spec, id, path)`.

use std::fs;
use std::path::{Path, PathBuf};

pub fn collect_base_contract_files(contracts_root: &Path) -> Vec<(String, String, PathBuf)> {
    collect_contract_files(contracts_root)
        .into_iter()
        .filter(|(spec, _, _)| spec != "changes")
        .collect()
}

pub(crate) fn collect_contract_files(root: &Path) -> Vec<(String, String, PathBuf)> {
    let mut files = Vec::new();
    let Ok(spec_dirs) = fs::read_dir(root) else {
        return files;
    };
    for spec_dir in spec_dirs.flatten() {
        if !is_dir(&spec_dir) {
            continue;
        }
        let spec = spec_dir.file_name().to_string_lossy().into_owned();
        files.extend(spec_contract_files(&spec, &spec_dir.path()));
    }
    files
}

/// Contract files inside one spec directory: `*.toml` files only, ids taken
/// from the file stem.
fn spec_contract_files(spec: &str, dir: &Path) -> Vec<(String, String, PathBuf)> {
    let Ok(entries) = fs::read_dir(dir) else {
        return Vec::new();
    };
    entries
        .flatten()
        .filter(|entry| is_file(entry) && is_toml(&entry.path()))
        .map(|entry| {
            let path = entry.path();
            let id = path
                .file_stem()
                .and_then(|value| value.to_str())
                .unwrap_or_default()
                .to_string();
            (spec.to_string(), id, path)
        })
        .collect()
}

fn is_dir(entry: &fs::DirEntry) -> bool {
    is_kind(entry, |kind| kind.is_dir())
}

fn is_file(entry: &fs::DirEntry) -> bool {
    is_kind(entry, |kind| kind.is_file())
}

fn is_kind(entry: &fs::DirEntry, predicate: impl Fn(fs::FileType) -> bool) -> bool {
    entry.file_type().map(predicate).unwrap_or(false)
}

fn is_toml(path: &Path) -> bool {
    path.extension().and_then(|value| value.to_str()) == Some("toml")
}
