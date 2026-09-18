use hydragrow_shared::{
    IncomingSensorPayload, MqttCommandIn, MqttCommandOut, SensorData, recipe::CropRecipe,
};
use serde_json::Value;
use std::{collections::BTreeMap, fs, path::PathBuf};

fn registry() -> Value {
    read_json("schema/contracts/registry.json")
}

fn read_json(path: &str) -> Value {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let path = root.parent().unwrap().join(path);
    let text =
        fs::read_to_string(&path).unwrap_or_else(|err| panic!("read {}: {err}", path.display()));
    serde_json::from_str(&text).unwrap_or_else(|err| panic!("parse {}: {err}", path.display()))
}

fn fixture(contract: &str) -> Value {
    let registry = registry();
    let relative = registry["contracts"][contract]["fixture"]
        .as_str()
        .unwrap_or_else(|| panic!("missing fixture for {contract}"));
    read_json(&format!("schema/contracts/{relative}"))
}

fn object_keys(value: &Value) -> BTreeMap<String, Value> {
    value
        .as_object()
        .unwrap()
        .iter()
        .map(|(key, value)| (key.clone(), value.clone()))
        .collect()
}

#[test]
fn registry_is_versioned_and_every_contract_has_a_fixture() {
    let registry = registry();
    assert_eq!(registry["registry_version"], "1.0.0");
    assert_eq!(
        registry["schema_dialect"],
        "https://json-schema.org/draft/2020-12/schema"
    );

    let contracts = registry["contracts"].as_object().unwrap();
    assert!(contracts.len() >= 8);
    for (name, contract) in contracts {
        assert!(contract["version"].is_number(), "{name} missing version");
        assert_eq!(
            contract["layer"], "canonical_wire",
            "{name} has wrong owner layer"
        );
        let fixture_path = contract["fixture"].as_str().unwrap();
        assert!(fixture_path.starts_with("fixtures/") && fixture_path.ends_with(".json"));
        assert!(
            fixture(name).is_object(),
            "{name} fixture must be an object"
        );
    }
}

#[test]
fn registry_required_fields_exist_and_are_non_nullable() {
    let contracts = registry()["contracts"].as_object().unwrap().clone();
    for (name, contract) in contracts {
        let value = fixture(&name);
        let object = value.as_object().unwrap();
        for field in contract["required"].as_array().unwrap() {
            let field = field.as_str().unwrap();
            assert!(
                object.contains_key(field),
                "{name}: missing required {field}"
            );
            let nullable = contract["fields"][field]["nullable"]
                .as_bool()
                .unwrap_or(false);
            if !nullable {
                assert!(!object[field].is_null(), "{name}: required {field} is null");
            }
        }
    }
}

#[test]
fn canonical_fixtures_reject_non_finite_values_by_construction() {
    fn walk(value: &Value, path: &str) {
        match value {
            Value::Number(number) => assert!(number.as_f64().is_some(), "invalid number at {path}"),
            Value::Array(items) => items
                .iter()
                .enumerate()
                .for_each(|(i, item)| walk(item, &format!("{path}[{i}]"))),
            Value::Object(items) => items
                .iter()
                .for_each(|(key, item)| walk(item, &format!("{path}.{key}"))),
            _ => {}
        }
    }

    let contracts = registry()["contracts"].as_object().unwrap().clone();
    for name in contracts.keys() {
        walk(&fixture(name), name);
    }
}

#[test]
fn typed_shared_payloads_round_trip_against_canonical_fixtures() {
    let sensor: SensorData = serde_json::from_value(fixture("sensor_data")).unwrap();
    assert_eq!(
        serde_json::to_value(&sensor).unwrap(),
        fixture("sensor_data")
    );

    let command: MqttCommandOut = serde_json::from_value(fixture("mqtt_command")).unwrap();
    assert_eq!(
        serde_json::to_value(&command).unwrap(),
        fixture("mqtt_command")
    );

    let recipe: CropRecipe = serde_json::from_value(fixture("crop_recipe")).unwrap();
    assert_eq!(recipe.recipe_id, "lettuce-romaine-v1");
    assert_eq!(recipe.start_time_sec, 1_777_000_000);
    assert_eq!(recipe.stages[0].duration_sec, 604_800);
    assert_eq!(recipe.stages[1].name, "vegetative");
    let emitted = serde_json::to_value(&recipe).unwrap();
    assert!(
        emitted["stages"][0]
            .get("water_change_interval_days")
            .is_none()
    );
    assert!(emitted["stages"][0].get("auto_dilute_ec_trigger").is_none());
}

#[test]
fn compatibility_aliases_are_never_emitted_by_sensor_fixture() {
    let sensor = object_keys(&fixture("sensor_data"));
    assert!(sensor.contains_key("ec"));
    assert!(!sensor.contains_key("tds"));
    assert!(sensor.contains_key("err_ec"));
    assert!(!sensor.contains_key("err_tds"));
}

#[test]
fn canonical_command_fixture_has_no_legacy_top_level_fields() {
    let command = fixture("mqtt_command");
    let object = command.as_object().unwrap();
    for legacy in ["pump", "pump_id", "duration_sec", "pwm"] {
        assert!(
            !object.contains_key(legacy),
            "legacy command field emitted: {legacy}"
        );
    }
    assert_eq!(command["params"]["pump_id"], "pump_a");
}

#[test]
fn firmware_mqtt_parsers_accept_canonical_sensor_and_command_fixtures() {
    // These are the exact serde payload types consumed by
    // ESP32-C3-CONTROLLER-NODE/src/hw/mqtt_client.rs for MQTT sensor and
    // command packets. The command fixture's HMAC is intentionally not
    // verified here; firmware verifies it before deserializing MqttCommandIn.
    let sensor: IncomingSensorPayload =
        serde_json::from_value(fixture("sensor_data")).expect("firmware sensor parser");
    assert_eq!(sensor.ec, Some(1.5));
    assert_eq!(sensor.ph, Some(6.0));
    assert_eq!(sensor.water_level, Some(20.0));
    assert!(sensor.is_valid());

    let mut command = fixture("mqtt_command");
    let object = command
        .as_object_mut()
        .expect("command fixture must be an object");
    object.remove("ts");
    object.remove("nonce");
    object.remove("signature");
    let command: MqttCommandIn = serde_json::from_value(command).expect("firmware command parser");
    assert_eq!(command.action, "start");
    assert_eq!(command.target.as_deref(), Some("device-001"));
    let params = command.params.expect("canonical nested command params");
    assert_eq!(params.pump_id.as_deref(), Some("pump_a"));
    assert_eq!(params.duration_sec, Some(8));
    assert_eq!(params.pwm, Some(70));
    assert_eq!(params.state, Some(true));
}
