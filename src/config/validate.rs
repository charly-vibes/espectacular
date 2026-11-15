//! Per-section validation helpers for `Config::validate`.
//!
//! Each helper owns one config section's checks so `validate` itself stays a
//! flat sequence of calls. Message text, field names, and emission order match
//! the pre-split single-function validator exactly.

use genesis::config::ConfigValidation;

/// Push an error when a `[paths]` field is empty or escapes the repo root.
pub(super) fn path_field(field: &str, value: &str, results: &mut Vec<ConfigValidation>) {
    if value.is_empty() {
        results.push(ConfigValidation::error(
            field,
            format!("{field} must be non-empty"),
        ));
    } else if value.starts_with('/') || value.contains("..") {
        results.push(ConfigValidation::error(
            field,
            format!("{field} must not be absolute or contain '..': {value}"),
        ));
    }
}

/// Push an error when a runner has no argv entry or an empty one.
pub(super) fn runner(name: &str, argv: &[String], results: &mut Vec<ConfigValidation>) {
    if argv.is_empty() {
        results.push(ConfigValidation::error(
            format!("runners.{name}"),
            format!("runner {name} must have at least one argv entry"),
        ));
        return;
    }
    if argv.iter().any(|arg| arg.is_empty()) {
        results.push(ConfigValidation::error(
            format!("runners.{name}"),
            format!("runner {name} has an empty argv entry"),
        ));
    }
}
