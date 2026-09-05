use hydragrow_shared::sensors::IncomingSensorPayload;
use hydragrow_shared::SensorData;

fn base_sensor_data() -> SensorData {
    SensorData {
        device_id: "test_node".to_string(),
        ec: 1.0,
        ph: 6.0,
        temp: 25.0,
        water_level: 20.0,
        pump_status: Default::default(),
        time: "2026-09-05T00:00:00Z".to_string(),
        controller_received_ms: Some(1000),
        rssi: Some(-60),
        free_heap: Some(150_000),
        uptime: Some(100),
        err_water: None,
        err_temp: None,
        err_ph: None,
        err_ec: None,
        is_continuous: None,
        ph_voltage_mv: None,
    }
}

#[test]
fn empty_packet_does_not_advance_freshness_timestamp() {
    let mut sensors = base_sensor_data();
    let raw_json = "{}";
    let payload: IncomingSensorPayload = serde_json::from_str(raw_json).unwrap();

    let accepted = sensors.merge_incoming_payload(&payload, 2000);
    assert!(!accepted, "Empty packet must not be accepted as a valid measurement");
    assert_eq!(
        sensors.controller_received_ms,
        Some(1000),
        "controller_received_ms must not advance on empty packet"
    );
}

#[test]
fn partial_packet_preserves_previous_fields_and_error_flags() {
    let mut sensors = base_sensor_data();

    // First: receive partial packet with EC and err_ec=true
    let raw_1 = r#"{"ec":1.5,"err_ec":true}"#;
    let payload_1: IncomingSensorPayload = serde_json::from_str(raw_1).unwrap();
    let accepted_1 = sensors.merge_incoming_payload(&payload_1, 2000);

    assert!(accepted_1, "Valid partial measurement must be accepted");
    assert_eq!(sensors.ec, 1.5);
    assert_eq!(sensors.err_ec, Some(true));
    assert_eq!(sensors.controller_received_ms, Some(2000));

    // Next: receive empty packet {}
    let raw_2 = "{}";
    let payload_2: IncomingSensorPayload = serde_json::from_str(raw_2).unwrap();
    let accepted_2 = sensors.merge_incoming_payload(&payload_2, 3000);

    assert!(!accepted_2);
    assert_eq!(sensors.ec, 1.5, "EC must be preserved across empty packets");
    assert_eq!(
        sensors.err_ec,
        Some(true),
        "err_ec flag must remain latched across empty packets"
    );
    assert_eq!(
        sensors.controller_received_ms,
        Some(2000),
        "controller_received_ms must not advance"
    );
}

#[test]
fn malformed_and_non_finite_packets_are_rejected() {
    let mut sensors = base_sensor_data();

    // Case 1: NaN EC in raw payload
    let payload_nan = IncomingSensorPayload {
        ec: Some(f32::NAN),
        ..Default::default()
    };
    let accepted_nan = sensors.merge_incoming_payload(&payload_nan, 2000);
    assert!(!accepted_nan, "NaN measurement must be rejected");
    assert_eq!(sensors.controller_received_ms, Some(1000));
    assert_eq!(sensors.ec, 1.0);

    // Case 2: Infinity pH
    let payload_inf = IncomingSensorPayload {
        ph: Some(f32::INFINITY),
        ..Default::default()
    };
    let accepted_inf = sensors.merge_incoming_payload(&payload_inf, 2000);
    assert!(!accepted_inf, "Infinity measurement must be rejected");
    assert_eq!(sensors.controller_received_ms, Some(1000));
    assert_eq!(sensors.ph, 6.0);

    // Case 3: Physically impossible values (e.g. negative EC or negative water level)
    let payload_negative_ec = IncomingSensorPayload {
        ec: Some(-0.5),
        ..Default::default()
    };
    let accepted_neg = sensors.merge_incoming_payload(&payload_negative_ec, 2000);
    assert!(!accepted_neg, "Negative EC must be rejected");
    assert_eq!(sensors.controller_received_ms, Some(1000));
}
