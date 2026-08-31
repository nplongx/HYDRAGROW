use hydragrow_shared::SensorData;
use hydragrow_simulator::telemetry::mqtt_bridge::MqttBridge;
use rumqttc::{Client, Event, MqttOptions, Packet, QoS};
use std::time::Duration;
use std::net::TcpListener;
use std::io::{Read, Write};

fn spawn_mock_broker() -> u16 {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let port = listener.local_addr().unwrap().port();
    std::thread::spawn(move || {
        let mut buf = [0u8; 1024];

        // 1. Accept subscriber connection
        if let Ok((mut sub_conn, _)) = listener.accept() {
            let _ = sub_conn.read(&mut buf); // CONNECT
            let _ = sub_conn.write_all(&[0x20, 0x02, 0x00, 0x00]); // CONNACK

            let _ = sub_conn.read(&mut buf); // SUBSCRIBE
            // A basic QoS 0/1 subscribe has packet ID in 2nd and 3rd byte after remaining length
            let pkt_id_msb = buf[2];
            let pkt_id_lsb = buf[3];
            let _ = sub_conn.write_all(&[0x90, 0x03, pkt_id_msb, pkt_id_lsb, 0x00]); // SUBACK

            // 2. Accept publisher connection
            if let Ok((mut pub_conn, _)) = listener.accept() {
                let _ = pub_conn.read(&mut buf); // CONNECT
                let _ = pub_conn.write_all(&[0x20, 0x02, 0x00, 0x00]); // CONNACK

                // 3. Read PUBLISH and forward
                if let Ok(n) = pub_conn.read(&mut buf) {
                    if n > 0 {
                        let _ = sub_conn.write_all(&buf[..n]);
                        let _ = sub_conn.flush();
                    }
                }
            }
        }
    });
    port
}

#[test]
fn test_mqtt_publish_and_receive() {
    let port = spawn_mock_broker();

    // 1. Subscribe to broker
    let mut mqttoptions = MqttOptions::new("test-subscriber", "127.0.0.1", port);
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
    let mut bridge = MqttBridge::new("sim-test", &format!("mqtt://127.0.0.1:{}", port));

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
