use hydragrow_shared::ControllerConfig;
use reqwest::Client;
use rumqttc::{AsyncClient, Event, MqttOptions, Packet, QoS};
use serde_json::Value;
use sqlx::PgPool;
use std::process::{Child, Command};
use std::time::Duration;
use uuid::Uuid;

const DEVICE_ID: &str = "e2e-backend-mqtt-device";
const API_KEY: &str = "e2e-api-key";
const COMMAND_SECRET: &str = "e2e-command-secret";
const SERVER_PORT: u16 = 18080;

fn database_url() -> String {
    std::env::var("E2E_DATABASE_URL").unwrap_or_else(|_| {
        "postgres://hydragrow:dev_only_password@127.0.0.1:55432/hydragrow".to_string()
    })
}

struct BackendProcess(Child);

impl Drop for BackendProcess {
    fn drop(&mut self) {
        let _ = self.0.kill();
        let _ = self.0.wait();
    }
}

struct TwinProcess(Child);

impl Drop for TwinProcess {
    fn drop(&mut self) {
        let _ = self.0.kill();
        let _ = self.0.wait();
    }
}

async fn migrate(pool: &PgPool) {
    sqlx::migrate!("./migrations").run(pool).await.unwrap();
}

fn backend() -> BackendProcess {
    let database_url = database_url();
    let child = Command::new(env!("CARGO_BIN_EXE_hydragrow-backend"))
        .env("DATABASE_URL", database_url)
        .env("INFLUX_URL", "http://127.0.0.1:8086")
        .env("INFLUX_TOKEN", "dev_only_token")
        .env("INFLUX_ORG", "hydragrow")
        .env("INFLUX_BUCKET", "sensors")
        .env("MQTT_HOST", "127.0.0.1")
        .env("MQTT_PORT", "1883")
        .env("MQTT_TLS", "false")
        .env("MQTT_CLIENT_ID", "e2e-backend")
        .env("MQTT_USER", "")
        .env("MQTT_PASSWORD", "")
        .env("MQTT_COMMAND_SECRET", COMMAND_SECRET)
        .env("API_KEY", API_KEY)
        .env("FIREBASE_PROJECT_ID", "e2e-project")
        .env("PRIVILEGED_CONTROL_SECRET", "e2e-privileged-secret")
        .env("SERVER_HOST", "127.0.0.1")
        .env("SERVER_PORT", SERVER_PORT.to_string())
        .env("ALLOWED_ORIGINS", "")
        .env("LOKI_URL", "http://127.0.0.1:3100")
        .spawn()
        .unwrap();
    BackendProcess(child)
}

fn twin() -> TwinProcess {
    let simulator_bin = std::env::var("HYDRAGROW_SIMULATOR_BIN").unwrap_or_else(|_| {
        format!(
            "{}/../hydragrow-simulator/target/debug/hydragrow-simulator",
            env!("CARGO_MANIFEST_DIR")
        )
    });
    let child = Command::new(simulator_bin)
        .args([
            "run",
            "--ticks",
            "10000",
            "--tick-ms",
            "250",
            "--realtime",
            "--device-id",
            DEVICE_ID,
            "--mqtt",
            "mqtt://127.0.0.1:1883",
        ])
        .env("MQTT_COMMAND_SECRET", COMMAND_SECRET)
        .env("RUST_LOG", "debug")
        .spawn()
        .unwrap();
    TwinProcess(child)
}

async fn wait_for_backend(client: &Client) {
    let url = format!("http://127.0.0.1:{SERVER_PORT}/api/devices/{DEVICE_ID}/control/commands");
    for _ in 0..100 {
        if let Ok(response) = client.get(&url).header("X-API-Key", API_KEY).send().await
            && response.status().is_success()
        {
            return;
        }
        tokio::time::sleep(Duration::from_millis(100)).await;
    }
    panic!("backend did not become ready on {url}");
}

#[tokio::test]
#[ignore = "requires local postgres + mosquitto + influxdb"]
async fn backend_api_command_reaches_real_mqtt_broker_postgres_and_twin() {
    let pool = PgPool::connect(&database_url()).await.unwrap();
    migrate(&pool).await;
    sqlx::query(
        r#"INSERT INTO device_config
           (device_id, ec_target, ec_tolerance, ph_target, ph_tolerance,
            control_mode, is_enabled, delay_between_a_and_b_sec, last_updated)
           VALUES ($1, 1.2, 0.05, 6.0, 0.1, 'auto', true, 10, CURRENT_TIMESTAMP)
           ON CONFLICT (device_id) DO NOTHING"#,
    )
    .bind(DEVICE_ID)
    .execute(&pool)
    .await
    .unwrap();

    let topic = format!("AGITECH/{DEVICE_ID}/controller/command");
    let lifecycle_topic = format!("AGITECH/{DEVICE_ID}/controller/command-status");
    let controller_status_topic = format!("AGITECH/{DEVICE_ID}/controller/status");
    let fsm_topic = format!("AGITECH/{DEVICE_ID}/fsm/state");
    let sensor_topic = format!("AGITECH/{DEVICE_ID}/sensors");
    let mut mqtt_options = MqttOptions::new("e2e-verifier", "127.0.0.1", 1883);
    mqtt_options.set_keep_alive(Duration::from_secs(2));
    let (mqtt, mut eventloop) = AsyncClient::new(mqtt_options, 16);
    mqtt.subscribe(&topic, QoS::AtLeastOnce).await.unwrap();
    mqtt.subscribe(&lifecycle_topic, QoS::AtLeastOnce)
        .await
        .unwrap();
    mqtt.subscribe(&controller_status_topic, QoS::AtLeastOnce)
        .await
        .unwrap();
    mqtt.subscribe(&fsm_topic, QoS::AtLeastOnce).await.unwrap();
    mqtt.subscribe(&sensor_topic, QoS::AtLeastOnce)
        .await
        .unwrap();

    let _backend = backend();
    let client = Client::new();
    wait_for_backend(&client).await;

    // Device is intentionally offline here. ConfigurationSync must retain the
    // desired revision and deliver it when the Twin reconnects.
    let mut desired_config = serde_json::to_value(ControllerConfig::default()).unwrap();
    desired_config["device_id"] = serde_json::json!(DEVICE_ID);
    desired_config["config_version"] = serde_json::json!(42);
    sqlx::query(
        r#"INSERT INTO configuration_sync
           (device_id, config_version, desired_controller_config, desired_sensor_config,
            controller_state, sensor_state, controller_attempts, sensor_attempts, updated_at)
           VALUES ($1, 42, $2, $3, 'pending', 'pending', 0, 0, CURRENT_TIMESTAMP)
           ON CONFLICT (device_id) DO UPDATE SET
             config_version = 42,
             desired_controller_config = $2,
             desired_sensor_config = $3,
             controller_state = 'pending', sensor_state = 'pending',
             controller_attempts = 0, sensor_attempts = 0,
             controller_applied_at = NULL, sensor_applied_at = NULL,
             updated_at = CURRENT_TIMESTAMP"#,
    )
    .bind(DEVICE_ID)
    .bind(&desired_config)
    .bind(serde_json::json!({"config_version": 42}))
    .execute(&pool)
    .await
    .unwrap();

    let mut twin_process = twin();
    let twin_sensor = tokio::time::timeout(Duration::from_secs(5), async {
        loop {
            match eventloop.poll().await.unwrap() {
                Event::Incoming(Packet::Publish(publish)) if publish.topic == sensor_topic => {
                    let value: Value = serde_json::from_slice(&publish.payload).unwrap();
                    if value["device_id"] == DEVICE_ID && value["ec"].is_number() {
                        break;
                    }
                }
                _ => {}
            }
        }
    });
    twin_sensor.await.unwrap();

    // Reconcile the durable desired revision after the offline interval.
    tokio::time::timeout(Duration::from_secs(15), async {
        loop {
            match eventloop.poll().await.unwrap() {
                Event::Incoming(Packet::Publish(publish))
                    if publish.topic == controller_status_topic =>
                {
                    let value: Value = serde_json::from_slice(&publish.payload).unwrap();
                    if value["device_id"] == DEVICE_ID && value["config_version"] == 42 {
                        break;
                    }
                }
                _ => {}
            }
        }
    })
    .await
    .unwrap();
    tokio::time::timeout(Duration::from_secs(5), async {
        loop {
            let sync_state: String = sqlx::query_scalar(
                "SELECT controller_state FROM configuration_sync WHERE device_id = $1 AND config_version = 42",
            )
            .bind(DEVICE_ID)
            .fetch_one(&pool)
            .await
            .unwrap();
            if sync_state == "applied" {
                break;
            }
            tokio::time::sleep(Duration::from_millis(100)).await;
        }
    })
    .await
    .unwrap();

    // Restart the Twin. Its runtime-applied revision is reset; retained
    // canonical config must restore it without another API write.
    twin_process.0.kill().unwrap();
    twin_process.0.wait().unwrap();
    let _twin_process = twin();
    tokio::time::timeout(Duration::from_secs(5), async {
        loop {
            match eventloop.poll().await.unwrap() {
                Event::Incoming(Packet::Publish(publish))
                    if publish.topic == controller_status_topic =>
                {
                    let value: Value = serde_json::from_slice(&publish.payload).unwrap();
                    if value["device_id"] == DEVICE_ID && value["config_version"] == 0 {
                        break;
                    }
                }
                _ => {}
            }
        }
    })
    .await
    .unwrap();
    tokio::time::timeout(Duration::from_secs(5), async {
        loop {
            let state: String = sqlx::query_scalar(
                "SELECT controller_state FROM configuration_sync WHERE device_id = $1 AND config_version = 42",
            )
            .bind(DEVICE_ID)
            .fetch_one(&pool)
            .await
            .unwrap();
            if state == "pending" || state == "published" {
                break;
            }
            tokio::time::sleep(Duration::from_millis(100)).await;
        }
    })
    .await
    .unwrap();
    tokio::time::timeout(Duration::from_secs(15), async {
        loop {
            match eventloop.poll().await.unwrap() {
                Event::Incoming(Packet::Publish(publish))
                    if publish.topic == controller_status_topic =>
                {
                    let value: Value = serde_json::from_slice(&publish.payload).unwrap();
                    if value["device_id"] == DEVICE_ID && value["config_version"] == 42 {
                        break;
                    }
                }
                _ => {}
            }
        }
    })
    .await
    .unwrap();
    tokio::time::timeout(Duration::from_secs(5), async {
        loop {
            let sync_state: String = sqlx::query_scalar(
                "SELECT controller_state FROM configuration_sync WHERE device_id = $1 AND config_version = 42",
            )
            .bind(DEVICE_ID)
            .fetch_one(&pool)
            .await
            .unwrap();
            if sync_state == "applied" {
                break;
            }
            tokio::time::sleep(Duration::from_millis(100)).await;
        }
    })
    .await
    .unwrap();

    let response = client
        .post(format!(
            "http://127.0.0.1:{SERVER_PORT}/api/devices/{DEVICE_ID}/control/privileged-token"
        ))
        .header("X-API-Key", API_KEY)
        .header("X-User-Confirmed", "true")
        .json(&serde_json::json!({
            "action_class": "dangerous_control"
        }))
        .send()
        .await
        .unwrap();
    assert!(
        response.status().is_success(),
        "privileged-token response: {}",
        response.status()
    );
    let token_json: Value = response.json().await.unwrap();
    let privileged_token = token_json["token"].as_str().unwrap();

    let response = client
        .post(format!(
            "http://127.0.0.1:{SERVER_PORT}/api/devices/{DEVICE_ID}/control"
        ))
        .header("X-API-Key", API_KEY)
        .header("X-User-Confirmed", "true")
        .header("X-Privileged-Token", privileged_token)
        .json(&serde_json::json!({
            "action": "on",
            "pump": "PUMP_A",
            // Keep reruns isolated: the E2E database is intentionally persistent
            // across local runs, so a fixed idempotency key could return an older
            // TIMEOUT command without publishing anything to the broker.
            "idempotency_key": format!("e2e-command-{}", Uuid::new_v4())
        }))
        .send()
        .await
        .unwrap();
    assert!(
        response.status().is_success(),
        "API response: {}",
        response.status()
    );
    let response_json: Value = response.json().await.unwrap();
    let command_id = response_json["command_id"].as_str().unwrap();

    let mut published: Option<Value> = None;
    let mut saw_ack = false;
    let mut saw_fsm = false;
    let mut saw_controller_status = false;
    let mut saw_sensor = true;
    tokio::time::timeout(Duration::from_secs(15), async {
        while !(published.is_some() && saw_ack && saw_fsm && saw_controller_status && saw_sensor) {
            match eventloop.poll().await.unwrap() {
                Event::Incoming(Packet::Publish(publish)) if publish.topic == topic => {
                    published = Some(serde_json::from_slice::<Value>(&publish.payload).unwrap());
                }
                Event::Incoming(Packet::Publish(publish)) if publish.topic == lifecycle_topic => {
                    let value: Value = serde_json::from_slice(&publish.payload).unwrap();
                    if value["command_id"] == command_id && value["lifecycle"] == "ACKNOWLEDGED" {
                        saw_ack = true;
                    }
                }
                Event::Incoming(Packet::Publish(publish)) if publish.topic == fsm_topic => {
                    let value: Value = serde_json::from_slice(&publish.payload).unwrap();
                    saw_fsm = value["pump_status"]["pump_a"] == true;
                }
                Event::Incoming(Packet::Publish(publish))
                    if publish.topic == controller_status_topic =>
                {
                    let value: Value = serde_json::from_slice(&publish.payload).unwrap();
                    saw_controller_status = value["pump_status"]["pump_a"] == true;
                }
                Event::Incoming(Packet::Publish(publish)) if publish.topic == sensor_topic => {
                    let value: Value = serde_json::from_slice(&publish.payload).unwrap();
                    saw_sensor = value["device_id"] == DEVICE_ID && value["ec"].is_number();
                }
                _ => {}
            }
        }
    })
    .await
    .unwrap();

    let published =
        published.expect("backend command was not observed on the canonical MQTT topic");
    assert_eq!(published["action"], "pump_on");
    assert_eq!(published["params"]["pump_id"], "PUMP_A");
    assert_eq!(published["metadata"]["command_id"], command_id);
    assert!(
        published["signature"]
            .as_str()
            .is_some_and(|s| !s.is_empty())
    );
    assert!(published["ts"].as_i64().is_some());
    assert!(published["nonce"].as_str().is_some_and(|s| !s.is_empty()));

    let lifecycle = tokio::time::timeout(Duration::from_secs(5), async {
        loop {
            let lifecycle: (String,) =
                sqlx::query_as("SELECT lifecycle FROM commands WHERE command_id = $1")
                    .bind(command_id)
                    .fetch_one(&pool)
                    .await
                    .unwrap();
            if lifecycle.0 == "CONFIRMED" {
                break lifecycle.0;
            }
            tokio::time::sleep(Duration::from_millis(50)).await;
        }
    })
    .await
    .expect("backend did not persist CONFIRMED lifecycle after authoritative controller status");
    assert_eq!(lifecycle, "CONFIRMED");
}
