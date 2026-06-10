use serde::{Deserialize, Serialize};
use std::path::Path;

#[derive(Debug, Deserialize)]
struct RejectionEvent {
    event_kind: String,
    rule_name: String,
    timestamp: String,
    tool: String,
    violations: Vec<RejectionViolation>,
}

#[derive(Debug, Deserialize)]
struct RejectionViolation {
    entity_id: String,
    detail: String,
}

#[derive(Debug, Serialize, PartialEq)]
pub struct ViolationSummary {
    pub entity_id: String,
    pub detail: String,
}

#[derive(Debug, Serialize, PartialEq)]
pub struct DriftSignal {
    pub schema_version: String,
    pub signal_kind: String,
    pub source_tool: String,
    pub rule_name: String,
    pub timestamp: String,
    pub violation_count: usize,
    pub violations: Vec<ViolationSummary>,
    pub openspec_hint: String,
}

/// Read `.dont/events/*.json` files under `project_root` and return drift signals.
///
/// Returns an empty vec when no events directory exists or no valid event files
/// are found. Malformed files are silently skipped.
pub fn collect_drift_signals(project_root: &Path) -> Vec<DriftSignal> {
    let events_dir = project_root.join(".dont").join("events");
    if !events_dir.is_dir() {
        return vec![];
    }
    let entries = match std::fs::read_dir(&events_dir) {
        Ok(e) => e,
        Err(_) => return vec![],
    };
    let mut signals: Vec<DriftSignal> = entries
        .flatten()
        .filter(|e| {
            e.path()
                .extension()
                .and_then(|x| x.to_str())
                .map(|x| x == "json")
                .unwrap_or(false)
        })
        .filter_map(|e| parse_event_file(&e.path()))
        .collect();
    // Sort by timestamp for deterministic output.
    signals.sort_by(|a, b| a.timestamp.cmp(&b.timestamp));
    signals
}

fn parse_event_file(path: &Path) -> Option<DriftSignal> {
    let content = std::fs::read_to_string(path).ok()?;
    let event: RejectionEvent = serde_json::from_str(&content).ok()?;
    if event.event_kind != "rule_rejection" {
        return None;
    }
    let hint = openspec_hint(&event.rule_name);
    Some(DriftSignal {
        schema_version: "1.0".to_string(),
        signal_kind: "dont-rejection".to_string(),
        source_tool: event.tool,
        rule_name: event.rule_name,
        timestamp: event.timestamp,
        violation_count: event.violations.len(),
        violations: event
            .violations
            .iter()
            .map(|v| ViolationSummary {
                entity_id: v.entity_id.clone(),
                detail: v.detail.clone(),
            })
            .collect(),
        openspec_hint: hint,
    })
}

fn openspec_hint(rule_name: &str) -> String {
    match rule_name {
        "ungrounded" => {
            "Claims with unresolved CURIE dependencies cannot be verified; spec drift detected — \
             ground the claim or define the missing term"
                .to_string()
        }
        other => {
            format!("dont rule '{other}' violations detected — spec assertions may be drifting")
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    fn write_event(dir: &TempDir, filename: &str, content: &str) {
        let events_dir = dir.path().join(".dont").join("events");
        fs::create_dir_all(&events_dir).unwrap();
        fs::write(events_dir.join(filename), content).unwrap();
    }

    fn ungrounded_event(violations: &[(&str, &str)]) -> String {
        let vs: serde_json::Value = violations
            .iter()
            .map(|(id, detail)| serde_json::json!({"entity_id": id, "detail": detail}))
            .collect::<Vec<_>>()
            .into();
        serde_json::to_string(&serde_json::json!({
            "schema_version": "1.0",
            "tool": "dont",
            "event_kind": "rule_rejection",
            "rule_name": "ungrounded",
            "timestamp": "2026-06-10T12:00:00Z",
            "violations": vs
        }))
        .unwrap()
    }

    #[test]
    fn returns_empty_when_no_events_dir() {
        let dir = TempDir::new().unwrap();
        let signals = collect_drift_signals(dir.path());
        assert!(signals.is_empty());
    }

    #[test]
    fn returns_empty_when_events_dir_empty() {
        let dir = TempDir::new().unwrap();
        fs::create_dir_all(dir.path().join(".dont").join("events")).unwrap();
        let signals = collect_drift_signals(dir.path());
        assert!(signals.is_empty());
    }

    #[test]
    fn reads_ungrounded_event_file() {
        let dir = TempDir::new().unwrap();
        write_event(
            &dir,
            "rejection-ungrounded-2026.json",
            &ungrounded_event(&[("claim:abc", "depends on unresolved CURIE 'foo:bar'")]),
        );
        let signals = collect_drift_signals(dir.path());
        assert_eq!(signals.len(), 1);
        let s = &signals[0];
        assert_eq!(s.rule_name, "ungrounded");
        assert_eq!(s.signal_kind, "dont-rejection");
        assert_eq!(s.source_tool, "dont");
        assert_eq!(s.violation_count, 1);
        assert_eq!(s.violations[0].entity_id, "claim:abc");
        assert!(s.openspec_hint.contains("CURIE"));
    }

    #[test]
    fn skips_non_rejection_event_files() {
        let dir = TempDir::new().unwrap();
        write_event(
            &dir,
            "other.json",
            r#"{"event_kind":"something_else","rule_name":"x","timestamp":"t","tool":"dont","violations":[]}"#,
        );
        let signals = collect_drift_signals(dir.path());
        assert!(signals.is_empty());
    }

    #[test]
    fn skips_malformed_json() {
        let dir = TempDir::new().unwrap();
        write_event(&dir, "bad.json", "not json at all");
        let signals = collect_drift_signals(dir.path());
        assert!(signals.is_empty());
    }

    #[test]
    fn multiple_events_sorted_by_timestamp() {
        let dir = TempDir::new().unwrap();
        let e1 = serde_json::to_string(&serde_json::json!({
            "schema_version": "1.0", "tool": "dont",
            "event_kind": "rule_rejection", "rule_name": "ungrounded",
            "timestamp": "2026-06-10T11:00:00Z", "violations": []
        }))
        .unwrap();
        let e2 = serde_json::to_string(&serde_json::json!({
            "schema_version": "1.0", "tool": "dont",
            "event_kind": "rule_rejection", "rule_name": "ungrounded",
            "timestamp": "2026-06-10T13:00:00Z", "violations": []
        }))
        .unwrap();
        write_event(&dir, "b.json", &e2);
        write_event(&dir, "a.json", &e1);
        let signals = collect_drift_signals(dir.path());
        assert_eq!(signals.len(), 2);
        assert_eq!(signals[0].timestamp, "2026-06-10T11:00:00Z");
        assert_eq!(signals[1].timestamp, "2026-06-10T13:00:00Z");
    }
}
