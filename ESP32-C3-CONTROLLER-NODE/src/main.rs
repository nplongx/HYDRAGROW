use esp_idf_hal::gpio::PinDriver;
use esp_idf_hal::ledc::config::TimerConfig;
use esp_idf_hal::ledc::{LedcDriver, LedcTimerDriver};
use esp_idf_hal::peripherals::Peripherals;
use esp_idf_hal::units::FromValueType;
use esp_idf_svc::eventloop::EspSystemEventLoop;
use esp_idf_svc::mqtt::client::{EspMqttClient, QoS};
use esp_idf_svc::nvs::EspDefaultNvsPartition;
use esp_idf_svc::wifi::{AuthMethod, ClientConfiguration, Configuration, EspWifi};
use log::{error, info, warn};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{mpsc, Arc};
use std::thread;
use std::time::Duration;

// THÊM THƯ VIỆN UART ĐỂ GIAO TIẾP VỚI ESP32-C3 SENSOR NODE
use esp_idf_hal::uart::{config::Config as UartConfig, UartDriver};

mod config;
mod controller;
mod mqtt;
mod pump;
mod sensors;

use crate::controller::{start_fsm_control_loop, SystemState};
use config::create_shared_config;
use mqtt::{create_shared_sensor_data, ConnectionState};
use pump::PumpController;

const WIFI_SSID: &str = "Huynh Hong";
const WIFI_PASS: &str = "123443215";
const MQTT_URL: &str = "mqtt://interchange.proxy.rlwy.net:50133";
const DEVICE_ID: &str = "device_001";

fn main() -> anyhow::Result<()> {
    esp_idf_svc::sys::link_patches();
    esp_idf_svc::log::EspLogger::initialize_default();
    info!("🚀 Khởi động hệ thống FSM Thủy canh Agitech (Phiên bản ESP32-C3)...");

    let peripherals = Peripherals::take().unwrap();
    let sysloop = EspSystemEventLoop::take()?;
    let nvs = EspDefaultNvsPartition::take()?;

    let shared_config = create_shared_config();
    let shared_sensor_data = create_shared_sensor_data();

    // Khởi tạo các Channel giao tiếp giữa các luồng
    let (conn_tx, conn_rx) = mpsc::channel::<ConnectionState>();
    let (cmd_tx, cmd_rx) = mpsc::channel();
    let (fsm_tx, fsm_rx) = mpsc::channel::<String>();
    let (dosing_report_tx, dosing_report_rx) = mpsc::channel::<String>();
    let (sensor_cmd_tx, sensor_cmd_rx) = mpsc::channel::<String>();

    let timer_driver = Arc::new(LedcTimerDriver::new(
        peripherals.ledc.timer0,
        &TimerConfig::new().frequency(20000.Hz()),
    )?);

    // ===============================
    // 1. KHỞI TẠO BƠM VÀ VAN (Map lại chân cho ESP32-C3)
    // ===============================
    let valve_mist = PinDriver::output(peripherals.pins.gpio10)?;
    let osaka_en = PinDriver::output(peripherals.pins.gpio0)?;

    let water_pump_in = PinDriver::output(peripherals.pins.gpio1)?;
    let water_pump_out = PinDriver::output(peripherals.pins.gpio2)?;

    let osaka_rpwm = LedcDriver::new(
        peripherals.ledc.channel0,
        timer_driver.clone(),
        peripherals.pins.gpio3,
    )?;
    let pump_a = LedcDriver::new(
        peripherals.ledc.channel1,
        timer_driver.clone(),
        peripherals.pins.gpio6,
    )?;
    let pump_b = LedcDriver::new(
        peripherals.ledc.channel2,
        timer_driver.clone(),
        peripherals.pins.gpio7,
    )?;
    let pump_ph_up = LedcDriver::new(
        peripherals.ledc.channel3,
        timer_driver.clone(),
        peripherals.pins.gpio8,
    )?;
    let pump_ph_down = LedcDriver::new(
        peripherals.ledc.channel4,
        timer_driver.clone(),
        peripherals.pins.gpio21,
    )?;

    let pump_controller = PumpController::new(
        pump_a,
        pump_b,
        pump_ph_up,
        pump_ph_down,
        valve_mist,
        water_pump_in,
        water_pump_out,
        osaka_en,
        osaka_rpwm,
    )?;

    // ===============================
    // 3. KHỞI CHẠY BỘ ĐIỀU KHIỂN FSM
    // ===============================
    let fsm_config = shared_config.clone();
    let fsm_sensor_data = shared_sensor_data.clone();
    let fsm_nvs = nvs.clone();

    std::thread::Builder::new()
        .stack_size(10240)
        .name("fsm_thread".to_string())
        .spawn(move || {
            start_fsm_control_loop(
                fsm_config,
                fsm_sensor_data,
                pump_controller,
                fsm_nvs,
                cmd_rx,
                fsm_tx,
                dosing_report_tx,
                sensor_cmd_tx,
            );
        })?;

    // ===============================
    // 4. KẾT NỐI WIFI
    // ===============================
    let mut wifi = EspWifi::new(peripherals.modem, sysloop.clone(), Some(nvs.clone()))?;
    wifi.set_configuration(&Configuration::Client(ClientConfiguration {
        ssid: WIFI_SSID.try_into().unwrap(),
        password: WIFI_PASS.try_into().unwrap(),
        auth_method: AuthMethod::WPA2Personal,
        ..Default::default()
    }))?;

    wifi.start()?;
    wifi.connect()?;

    let conn_tx_wifi = conn_tx.clone();
    thread::spawn(move || {
        let mut was_connected = false;
        loop {
            let is_l2_connected = wifi.is_connected().unwrap_or(false);
            let has_ip = wifi
                .sta_netif()
                .get_ip_info()
                .map(|info| !info.ip.is_unspecified())
                .unwrap_or(false);
            let is_fully_connected = is_l2_connected && has_ip;

            if is_fully_connected && !was_connected {
                let _ = conn_tx_wifi.send(ConnectionState::WifiConnected);
                was_connected = true;
            } else if !is_fully_connected && was_connected {
                let _ = conn_tx_wifi.send(ConnectionState::WifiDisconnected);
                was_connected = false;
                if !is_l2_connected {
                    let _ = wifi.connect();
                }
            }
            thread::sleep(Duration::from_secs(2));
        }
    });

    // ===============================
    // 5. MAIN EVENT LOOP (MQTT & STATUS)
    // ===============================
    let mut mqtt_client: Option<EspMqttClient> = None;
    let mut is_mqtt_connected = false;

    info!("🔄 Đang chạy Main Event Loop...");

    let mut force_publish_next = false;
    let mut last_config_hash = String::new();

    // 🟢 MỚI: Biến lưu thời điểm cuối cùng gửi Health Status
    let mut last_health_publish = std::time::Instant::now();

    loop {
        // --- XỬ LÝ TRẠNG THÁI KẾT NỐI ---
        if let Ok(state) = conn_rx.try_recv() {
            match state {
                ConnectionState::WifiConnected => {
                    info!("🛜 Đã kết nối WiFi. Tiến hành khởi tạo MQTT...");
                    if mqtt_client.is_none() {
                        match mqtt::init_mqtt_client(
                            MQTT_URL,
                            shared_config.clone(),
                            shared_sensor_data.clone(),
                            cmd_tx.clone(),
                            conn_tx.clone(),
                        ) {
                            Ok(client) => mqtt_client = Some(client),
                            Err(e) => error!("❌ Lỗi khởi tạo MQTT: {:?}", e),
                        }
                    }
                }
                ConnectionState::WifiDisconnected => {
                    warn!("⚠️ Rớt mạng WiFi!");
                    is_mqtt_connected = false;
                    mqtt_client = None;
                }
                ConnectionState::MqttConnected => {
                    info!("📡 MQTT Client: ĐÃ KẾT NỐI THÀNH CÔNG");
                    is_mqtt_connected = true;

                    if let Some(client) = mqtt_client.as_mut() {
                        let topic_config = format!("AGITECH/{}/controller/config", DEVICE_ID);
                        let topic_command = format!("AGITECH/{}/controller/command", DEVICE_ID);
                        let topic_status = format!("AGITECH/{}/status", DEVICE_ID);
                        let topic_sensors = format!("AGITECH/{}/sensors", DEVICE_ID);

                        let _ = client.publish(
                            &topic_status,
                            QoS::AtLeastOnce,
                            true, // Retain = true để giữ trạng thái
                            r#"{"online": true}"#.as_bytes(),
                        );
                        let _ = client.subscribe(&topic_config, QoS::AtLeastOnce);
                        let _ = client.subscribe(&topic_command, QoS::AtLeastOnce);
                        let _ = client.subscribe(&topic_sensors, QoS::AtLeastOnce);
                    }
                }
                ConnectionState::MqttDisconnected => {
                    warn!("📡 MQTT Client: MẤT KẾT NỐI");
                    is_mqtt_connected = false;
                }
            }
        }

        // --- XỬ LÝ PAYLOAD TỪ FSM THREAD ---
        if let Ok(payload) = fsm_rx.try_recv() {
            if is_mqtt_connected {
                if let Some(client) = mqtt_client.as_mut() {
                    let topic = format!("AGITECH/{}/fsm", DEVICE_ID);
                    let _ = client.publish(&topic, QoS::AtLeastOnce, false, payload.as_bytes());
                }
            }
        }

        if let Ok(report_json) = dosing_report_rx.try_recv() {
            if is_mqtt_connected {
                if let Some(client) = mqtt_client.as_mut() {
                    let topic = format!("AGITECH/{}/dosing_report", DEVICE_ID);
                    let _ = client.publish(&topic, QoS::AtLeastOnce, false, report_json.as_bytes());
                }
            }
        }

        if let Ok(sensor_cmd_json) = sensor_cmd_rx.try_recv() {
            if sensor_cmd_json.contains("\"command\":\"force_publish\"") {
                force_publish_next = true;
            } else if is_mqtt_connected {
                if let Some(client) = mqtt_client.as_mut() {
                    let topic_sensor_cmd = format!("AGITECH/{}/sensor/command", DEVICE_ID);
                    let _ = client.publish(
                        &topic_sensor_cmd,
                        QoS::AtLeastOnce,
                        false,
                        sensor_cmd_json.as_bytes(),
                    );
                }
            }
        }

        // ==========================================
        // 🟢 MỚI: GỬI SỨC KHỎE THIẾT BỊ MỖI 10 GIÂY
        // ==========================================
        if is_mqtt_connected && last_health_publish.elapsed().as_secs() >= 10 {
            last_health_publish = std::time::Instant::now();

            if let Some(client) = mqtt_client.as_mut() {
                // Đọc trạng thái bơm mới nhất từ shared struct
                let current_pump_status = shared_sensor_data.read().unwrap().pump_status.clone();

                // Tạo payload sức khỏe thiết bị
                let health_payload = crate::mqtt::ControllerHealthPayload {
                    free_heap: crate::mqtt::get_free_heap(),
                    uptime_sec: crate::mqtt::get_uptime_sec(),
                    rssi: crate::mqtt::get_wifi_rssi(),
                    pump_status: current_pump_status,
                };

                if let Ok(json_string) = serde_json::to_string(&health_payload) {
                    let topic_health = format!("AGITECH/{}/controller/status", DEVICE_ID);
                    let _ = client.publish(
                        &topic_health,
                        QoS::AtMostOnce, // QoS 0 cho bản tin telemetry tần suất cao
                        false,
                        json_string.as_bytes(),
                    );
                }
            }
        }

        // Nghỉ 50ms để nhường CPU cho các tác vụ khác (RTOS Task)
        thread::sleep(Duration::from_millis(50));
    }
}

