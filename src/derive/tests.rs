use super::*;

fn row(id: &str, kind: Option<&str>, cells: &[(&str, &str)]) -> Row {
    Row {
        id: id.to_string(),
        kind: kind.map(str::to_string),
        cells: cells
            .iter()
            .map(|(k, v)| (k.to_string(), v.to_string()))
            .collect(),
    }
}

// ---- 2.1: spk parse --json IR deserialization -----------------------

const ENVELOPE_FIXTURE: &str = r#"{
  "ok": true,
  "envelope_version": "0.1",
  "cli_version": "0.3.0",
  "envelope_kind": "ok",
  "data": {
    "path": "openspec/specs/lint/spec.md",
    "intent": "THE lint SHALL ...",
    "constraints": [
      {"id": "C-vague", "kind": "invariant", "cells": {"expr": "unbound qualitative terms emit a finding", "id": "C-vague", "kind": "invariant"}}
    ],
    "states": [
      {"id": "scanning", "kind": null, "cells": {"id": "scanning"}}
    ],
    "transitions": [
      {"id": "t-rules", "from": "scanning", "to": "reported", "guard": "[[spec.C-vague]]"}
    ],
    "properties": [
      {"id": "P-vague", "kind": "unit", "cells": {
        "id": "P-vague", "kind": "unit",
        "derives_from": "[[spec.C-vague]]",
        "generator": "bodies with unbound qualifiers",
        "predicate": "findings exactly in the unbound cases"
      }}
    ]
  },
  "warnings": [],
  "hints": [],
  "meta": {"duration_ms": 1, "tx": null, "request_id": null}
}"#;

#[test]
fn deserializes_full_parse_envelope() {
    let ir = parse_ir(ENVELOPE_FIXTURE).expect("envelope parses");
    assert_eq!(ir.properties.len(), 1);
    let p = &ir.properties[0];
    assert_eq!(p.id, "P-vague");
    assert_eq!(p.kind.as_deref(), Some("unit"));
    assert_eq!(
        p.cells.get("derives_from").map(String::as_str),
        Some("[[spec.C-vague]]")
    );
    assert_eq!(ir.constraints.len(), 1);
    assert_eq!(ir.transitions[0].guard.as_deref(), Some("[[spec.C-vague]]"));
    assert_eq!(ir.states.len(), 1);
}

#[test]
fn tolerates_unknown_envelope_and_data_fields() {
    let with_extras: serde_json::Value = {
        let mut v = serde_json::from_str::<serde_json::Value>(ENVELOPE_FIXTURE).unwrap();
        v["future_field"] = serde_json::json!({"anything": true});
        v["data"]["future_ir_field"] = serde_json::json!(42);
        v["data"]["properties"][0]["cells"]["future_cell"] = serde_json::json!("ignored");
        v["data"]["properties"][0]["future_row_field"] = serde_json::json!(null);
        v
    };
    let ir = parse_ir(&with_extras.to_string()).expect("unknown fields tolerated");
    assert_eq!(ir.properties[0].id, "P-vague");
}

#[test]
fn data_null_degrades_to_empty_ir() {
    let ir = parse_ir(r#"{"ok": true, "data": null}"#).expect("data null tolerated");
    assert_eq!(ir, ParseData::default());
}

#[test]
fn error_envelope_is_an_err() {
    let out = parse_ir(r#"{"ok": false, "warnings": [{"message": "boom"}]}"#);
    assert!(out.is_err(), "ok:false must not yield an IR");
}

// ---- 2.2: canonical-JSON row hash stability -------------------------

fn property_fixture() -> Row {
    row(
        "P-vague",
        Some("unit"),
        &[
            ("id", "P-vague"),
            ("kind", "unit"),
            ("derives_from", "[[spec.C-vague]]"),
            ("generator", "bodies with unbound qualifiers"),
            ("predicate", "findings exactly in the unbound cases"),
        ],
    )
}

#[test]
fn hash_is_stable_across_cell_insertion_order() {
    let a = row(
        "P-vague",
        Some("unit"),
        &[
            ("id", "P-vague"),
            ("kind", "unit"),
            ("derives_from", "[[spec.C-vague]]"),
            ("generator", "g"),
            ("predicate", "p"),
        ],
    );
    // Same five cells, inserted in reverse order.
    let b = row(
        "P-vague",
        Some("unit"),
        &[
            ("predicate", "p"),
            ("generator", "g"),
            ("derives_from", "[[spec.C-vague]]"),
            ("kind", "unit"),
            ("id", "P-vague"),
        ],
    );
    assert_eq!(canonical_hash(&a), canonical_hash(&b));
}

#[test]
fn hash_changes_when_a_cell_is_edited() {
    let base = property_fixture();
    let mut edited = base.clone();
    edited
        .cells
        .insert("predicate".to_string(), "different predicate".to_string());
    assert_ne!(canonical_hash(&base), canonical_hash(&edited));
}

#[test]
fn hash_format_is_id_at_12_hex() {
    let h = canonical_hash(&property_fixture());
    let (id, digest) = h.split_once('@').expect("format <id>@<digest>");
    assert_eq!(id, "P-vague");
    assert_eq!(digest.len(), 12);
    assert!(digest.chars().all(|c| c.is_ascii_hexdigit()));
}

// ---- 2.3: D5 archetype / falsifiability inference -------------------

fn ir_with(
    properties: Vec<Row>,
    constraints: Vec<Row>,
    states: Vec<Row>,
    transitions: Vec<Transition>,
) -> ParseData {
    ParseData {
        properties,
        constraints,
        states,
        transitions,
    }
}

#[test]
fn effect_property_infers_ce_safety() {
    let ir = ir_with(
        vec![row(
            "P-emit",
            Some("effect"),
            &[("derives_from", "[[spec.C-x]]")],
        )],
        vec![row("C-x", Some("invariant"), &[])],
        vec![],
        vec![],
    );
    let got = infer(&ir, &ir.properties[0]);
    assert_eq!(got.archetype, "CE");
    assert_eq!(got.falsifiability_class, "safety");
}

#[test]
fn property_cited_by_state_emits_infers_ce_safety() {
    let ir = ir_with(
        vec![row(
            "P-linked",
            Some("unit"),
            &[("derives_from", "[[spec.C-x]]")],
        )],
        vec![row("C-x", Some("invariant"), &[])],
        vec![row("s1", None, &[("emits", "[[spec.C-x]] on acceptance")])],
        vec![],
    );
    let got = infer(&ir, &ir.properties[0]);
    assert_eq!(got.archetype, "CE");
    assert_eq!(got.falsifiability_class, "safety");
}

#[test]
fn property_cited_by_transition_guard_infers_sa_safety() {
    let ir = ir_with(
        vec![row(
            "P-guarded",
            Some("unit"),
            &[("derives_from", "[[spec.C-y]]")],
        )],
        vec![row("C-y", Some("invariant"), &[])],
        vec![],
        vec![Transition {
            id: "t1".into(),
            from: Some("a".into()),
            to: Some("b".into()),
            guard: Some("[[spec.C-y]], [[spec.C-z]]".into()),
        }],
    );
    let got = infer(&ir, &ir.properties[0]);
    assert_eq!(got.archetype, "SA");
    assert_eq!(got.falsifiability_class, "safety");
}

#[test]
fn law_property_infers_pf_safety() {
    let ir = ir_with(
        vec![row(
            "P-law",
            Some("law"),
            &[("derives_from", "[[spec.C-z]]")],
        )],
        vec![row("C-z", Some("invariant"), &[])],
        vec![],
        vec![],
    );
    let got = infer(&ir, &ir.properties[0]);
    assert_eq!(got.archetype, "PF");
    assert_eq!(got.falsifiability_class, "safety");
}

#[test]
fn uncited_property_falls_back_to_pf_safety() {
    let ir = ir_with(
        vec![row(
            "P-orphan",
            Some("unit"),
            &[("derives_from", "[[spec.C-nowhere]]")],
        )],
        vec![row("C-unrelated", Some("invariant"), &[])],
        vec![],
        vec![],
    );
    let got = infer(&ir, &ir.properties[0]);
    assert_eq!(got.archetype, "PF");
    assert_eq!(got.falsifiability_class, "safety");
}

#[test]
fn effect_beats_guard_citation() {
    // First matching D5 row wins: kind effect wins over a guard citation.
    let ir = ir_with(
        vec![row(
            "P-both",
            Some("effect"),
            &[("derives_from", "[[spec.C-y]]")],
        )],
        vec![row("C-y", Some("invariant"), &[])],
        vec![],
        vec![Transition {
            id: "t1".into(),
            from: None,
            to: None,
            guard: Some("[[spec.C-y]]".into()),
        }],
    );
    let got = infer(&ir, &ir.properties[0]);
    assert_eq!(got.archetype, "CE");
}

#[test]
fn cited_constraint_ids_extracts_spec_links() {
    let got = cited_constraint_ids("[[spec.C-a]], [[spec.C-b]] or [[spec.C-c]]");
    assert_eq!(got, vec!["C-a", "C-b", "C-c"]);
    assert!(cited_constraint_ids("no links here").is_empty());
}
