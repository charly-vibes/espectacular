use serde_json::Value;
use std::fs;

use crate::check::ah_check_warning_only_findings_exit_zero;

pub fn assert_schema_valid(instance: &Value) {
    let raw: Value =
        serde_json::from_str(&fs::read_to_string("schemas/check-output.schema.json").unwrap())
            .unwrap();
    let compiled = jsonschema::Validator::new(&raw).unwrap();
    // If the instance is a genesis envelope, unwrap the data payload
    let data = if instance.get("data").is_some() {
        &instance["data"]
    } else {
        instance
    };
    if let Err(error) = compiled.validate(data) {
        panic!("schema validation failed: {error}");
    }
}

fn custom_runner_schema_validation(instance: &Value) -> Result<(), String> {
    let raw: Value =
        serde_json::from_str(&fs::read_to_string("schemas/custom-runner.schema.json").unwrap())
            .unwrap();
    // The custom-runner schema $refs the check-output schema by absolute URL.
    // Register it locally so validation never touches the network.
    let check_output: Value =
        serde_json::from_str(&fs::read_to_string("schemas/check-output.schema.json").unwrap())
            .unwrap();
    let registry = jsonschema::Registry::new()
        .add(
            "https://charly.dev/espectacular/check-output.schema.json",
            jsonschema::Resource::from_contents(check_output),
        )
        .and_then(|builder| builder.prepare())
        .unwrap();
    let compiled = jsonschema::Validator::options()
        .with_registry(&registry)
        .build(&raw)
        .unwrap();
    // If the instance is a genesis envelope, unwrap the data payload
    let data = if instance.get("data").is_some() {
        &instance["data"]
    } else {
        instance
    };
    compiled
        .validate(data)
        .map_err(|error| format!("custom runner schema validation failed: {error}"))
}

#[test]
fn custom_runner_schema_accepts_empty_findings_pass_case() {
    let instance = serde_json::json!({
        "exit_code": 0,
        "passed": true,
        "findings": []
    });
    let outcome = custom_runner_schema_validation(&instance);
    assert!(
        outcome.is_ok(),
        "custom-runner schema must accept the pass case: {:?}",
        outcome.err()
    );
}

#[test]
fn p_schema() {
    ah_check_warning_only_findings_exit_zero();
    assert!(
        std::path::Path::new("schemas/check-output.schema.json").is_file(),
        "aggregated check CLI tests validate against the check-output schema"
    );
}
