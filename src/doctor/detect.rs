//! Framework/capability detection: turns adapters' findings into
//! detections and enable suggestions.

use super::types::{CapabilitySuggestion, FrameworkDetection};
use crate::adapters::{self, DetectionSource};
use crate::config;
use std::fs;
use std::path::Path;

pub(super) fn framework_result(
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

pub(super) fn property_result(
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
