use hydragrow_shared::SensorData;
use hydragrow_simulator::telemetry::mqtt_bridge::MqttBridge;
use rumqttc::{Client, Event, MqttOptions, Packet, QoS};
use std::time::Duration;

#[test]
fn test_mqtt_publish_and_receive() {
    // 1. Subscribe to broker
    let mut mqttoptions = MqttOptions::new("test-subscriber", "localhost", 1883);
    mqttoptions.set_keep_alive(Duration::from_secs(5)); // Short keep-alive so connection.iter() unblocks quickly if idle
    let (mut client, mut connection) = Client::new(mqttoptions, 10);
    client
        .subscribe("AGITECH/sim-test/sensors", QoS::AtMostOnce)
        .unwrap();

    // Loop until we receive the SubAck
    let suback_timeout = std::time::Instant::now() + Duration::from_secs(5);
    for notification in connection.iter() {
        match notification {
            Ok(Event::Incoming(Packet::SubAck(_))) => break,
            Err(e) => {
                println!("MQTT Connection Error while waiting for SubAck: {:?}", e);
                break;
            }
            _ => {}
        }
        if std::time::Instant::now() > suback_timeout {
            panic!("Timeout waiting for SubAck");
        }
    }

    // 2. Publish using bridge
    let mut bridge = MqttBridge::new("sim-test", "mqtt://localhost:1883");

    // Let the background thread of the publisher connect
    std::thread::sleep(Duration::from_millis(100));
    let data = SensorData {
        device_id: "sim-test".to_string(),
        ec: 0.0,
        ph: 0.0,
        temp: 0.0,
        water_level: 0.0,
        pump_status: hydragrow_shared::PumpStatus::default(),
        time: "2024-01-01T00:00:00Z".to_string(),
        controller_received_ms: None,
        rssi: None,
        free_heap: None,
        uptime: None,
        err_water: None,
        err_ph: None,
        err_ec: None,
        err_temp: None,
        is_continuous: None,
        ph_voltage_mv: None,
    };
    bridge.publish_sensors(&data);

    // 3. Wait for message and verify
    let mut received = false;

    // Add a timeout to prevent infinite loops if the broker connection fails or messages aren't received
    let timeout = std::time::Instant::now() + Duration::from_secs(5);

    for notification in connection.iter() {
        match notification {
            Ok(Event::Incoming(Packet::Publish(p))) => {
                assert_eq!(p.topic, "AGITECH/sim-test/sensors");
                received = true;
                break;
            }
            Ok(_) => {} // Ignore other events like PingReq
            Err(e) => {
                println!("MQTT Connection Error: {:?}", e);
                break; // Exit on error rather than hanging
            }
        }

        if std::time::Instant::now() > timeout {
            break;
        }
    }
    assert!(
        received,
        "Did not receive the expected MQTT message within the timeout period."
    );
}
