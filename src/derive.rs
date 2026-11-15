//! Derivation core (derive-contracts-from-specodelic, task group 2).
//!
//! Purpose: deserialize the `spk parse --json` Spec IR, derive the canonical
//! drift hash for property rows (design D4), and infer archetype /
//! falsifiability class per design D5 — `ah` never re-parses specodelic
//! markdown tables (design D2).

use serde::Deserialize;
use sha2::Digest;
use std::collections::BTreeMap;

/// `spk parse --json` envelope. Unknown fields are tolerated (serde default
/// behavior) so pre-1.0 envelope evolution cannot crash the consumer (D2).
#[derive(Deserialize, Debug, Clone)]
pub(crate) struct ParseEnvelope {
    pub ok: bool,
    #[serde(default)]
    pub data: Option<ParseData>,
}

/// The structured Spec IR slice the derivation needs. Every field defaults so
/// `data: null` and future additive fields degrade to an empty IR.
#[derive(Deserialize, Debug, Clone, Default, PartialEq)]
#[serde(default)]
pub(crate) struct ParseData {
    pub properties: Vec<Row>,
    pub constraints: Vec<Row>,
    pub states: Vec<Row>,
    pub transitions: Vec<Transition>,
}

/// A specodelic table row: top-level `id`/`kind` plus the raw cell map.
#[derive(Deserialize, Debug, Clone, PartialEq)]
pub(crate) struct Row {
    pub id: String,
    #[serde(default)]
    pub kind: Option<String>,
    #[serde(default)]
    pub cells: BTreeMap<String, String>,
}

/// A Model-section transition row (`from`/`guard`/`to` cells flattened).
#[derive(Deserialize, Debug, Clone, PartialEq)]
pub(crate) struct Transition {
    pub id: String,
    #[serde(default)]
    pub from: Option<String>,
    #[serde(default)]
    pub to: Option<String>,
    #[serde(default)]
    pub guard: Option<String>,
}

/// Deserialize `spk parse --json` stdout into the IR slice.
pub(crate) fn parse_ir(stdout: &str) -> Result<ParseData, String> {
    let envelope: ParseEnvelope = serde_json::from_str(stdout.trim())
        .map_err(|e| format!("spk parse produced unparseable output: {e}"))?;
    if !envelope.ok {
        return Err("spk parse reported ok:false".to_string());
    }
    Ok(envelope.data.unwrap_or_default())
}

/// Invoke `spk parse --json` on a spec markdown file and deserialize the IR
/// (task 4.2). Callers degrade on Err: a missing or broken spk binary means
/// the consumer skips spk-dependent verification (design D2).
pub(crate) fn parse_spec_ir(spk: &str, path: &std::path::Path) -> anyhow::Result<ParseData> {
    let output = crate::runner::spawn_with_text_busy_retry(
        spk,
        &["parse", path.to_str().unwrap_or(""), "--json"],
    )
    .map_err(|e| anyhow::anyhow!("failed to invoke spk parse: {e}"))?
    .stdout;
    parse_ir(&String::from_utf8_lossy(&output))
        .map_err(|e| anyhow::anyhow!("spk parse failed for {}: {e}", path.display()))
}

/// Cell keys participating in the derivation hash (D4).
const HASH_CELLS: [&str; 5] = ["id", "kind", "derives_from", "generator", "predicate"];

/// Canonical-JSON sha256 prefix (12 hex chars) of the property row's five
/// derivation cells (`id`, `kind`, `derives_from`, `generator`, `predicate`)
/// in BTreeMap key order — D4. Reordering input keys must not change the
/// hash; editing any cell must.
pub(crate) fn canonical_hash(row: &Row) -> String {
    let mut cells: BTreeMap<&str, serde_json::Value> = BTreeMap::new();
    for key in HASH_CELLS {
        let value = match key {
            "id" => Some(row.id.as_str()),
            "kind" => row.kind.as_deref(),
            other => row.cells.get(other).map(String::as_str),
        };
        cells.insert(key, value.map(serde_json::Value::from).unwrap_or_default());
    }
    let canonical = serde_json::to_string(&cells).unwrap_or_default();
    let digest = sha2::Sha256::digest(canonical.as_bytes());
    let hex: String = digest.iter().map(|b| format!("{b:02x}")).collect();
    format!("{}@{}", row.id, &hex[..12])
}

/// Extract the constraint ids cited by a cell value's `[[spec.<id>]]` links.
pub(crate) fn cited_constraint_ids(cell: &str) -> Vec<String> {
    let mut ids = Vec::new();
    let mut rest = cell;
    while let Some(start) = rest.find("[[spec.") {
        let after = &rest[start + "[[spec.".len()..];
        if let Some(end) = after.find("]]") {
            ids.push(after[..end].to_string());
            rest = &after[end + 2..];
        } else {
            break;
        }
    }
    ids
}

/// Inferred contract archetype + falsifiability class (D5 table).
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct Inferred {
    pub archetype: &'static str,
    pub falsifiability_class: &'static str,
}

/// D5 inference. First matching row wins; `safety` is always inferred
/// (liveness is not expressible in the current grammar).
pub(crate) fn infer(ir: &ParseData, property: &Row) -> Inferred {
    let cites = property
        .cells
        .get("derives_from")
        .map(|c| cited_constraint_ids(c))
        .unwrap_or_default();
    let cited_by = |cell_text: &str| {
        cited_constraint_ids(cell_text)
            .iter()
            .any(|id| cites.contains(id))
    };

    let archetype = if property.kind.as_deref() == Some("effect")
        || ir
            .states
            .iter()
            .any(|s| s.cells.get("emits").is_some_and(|e| cited_by(e)))
    {
        "CE"
    } else if ir
        .transitions
        .iter()
        .any(|t| t.guard.as_deref().is_some_and(cited_by))
    {
        "SA"
    } else {
        // law kind and the fallback row share PF.
        "PF"
    };
    Inferred {
        archetype,
        falsifiability_class: "safety",
    }
}

#[cfg(test)]
mod tests;
