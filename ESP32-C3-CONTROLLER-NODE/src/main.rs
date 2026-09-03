// src/main.rs
use esp_idf_hal::gpio::PinDriver;
use esp_idf_hal::i2c::{I2cConfig, I2cDriver};
use esp_idf_hal::ledc::config::TimerConfig;
use esp_idf_hal::ledc::{LedcDriver, LedcTimerDriver};
use esp_idf_hal::peripherals::Peripherals;
use esp_idf_svc::eventloop::EspSystemEventLoop;
use esp_idf_svc::log::EspLogger;
use esp_idf_svc::nvs::EspDefaultNvsPartition;
use hydragrow_controller_core::utils::get_current_time_sec;
use log::{info, warn};
use std::sync::{mpsc, Arc};

mod config;
mod hw;
mod runtime;

use config::create_shared_config;
use hw::{connect_wifi, create_shared_sensor_data, sync_sntp_time, NvsStore, PumpController};
use runtime::fsm_loop::start_fsm_control_loop;
use runtime::health::run_main_health_loop;

use crate::hw::pcf857x::I2cExpander;

const WIFI_SSID: &str = env!(
    "HYDRAGROW_WIFI_SSID",
    "Lỗi build: Thiếu biến HYDRAGROW_WIFI_SSID"
);
const WIFI_PASS: &str = env!(
    "HYDRAGROW_WIFI_PASSWORD",
    "Lỗi build: Thiếu biến HYDRAGROW_WIFI_PASSWORD"
);
const MQTT_URL: &str = env!(
    "HYDRAGROW_MQTT_URL",
    "Lỗi build: Thiếu biến HYDRAGROW_MQTT_URL"
);
const MQTT_COMMAND_SECRET: &str = env!(
    "HYDRAGROW_MQTT_COMMAND_SECRET",
    "Lỗi build: Thiếu biến HYDRAGROW_MQTT_COMMAND_SECRET"
);
const DEVICE_ID: &str = env!(
    "HYDRAGROW_DEVICE_ID",
    "Lỗi build: Thiếu biến HYDRAGROW_DEVICE_ID"
);

fn main() -> anyhow::Result<()> {
    esp_idf_svc::sys::link_patches();
    EspLogger::initialize_default();

    info!("🚀 Khởi động hệ thống FSM Thủy canh Agitech (ESP32-C3)...");

    let peripherals = Peripherals::take().unwrap();
    let sysloop = EspSystemEventLoop::take()?;
    let nvs_partition = EspDefaultNvsPartition::take()?;

    let default_mqtt_user = option_env!("HYDRAGROW_MQTT_USER").unwrap_or("");
    let default_mqtt_pass = option_env!("HYDRAGROW_MQTT_PASSWORD").unwrap_or("");

    let shared_config = create_shared_config();
    let mut nvs_store = NvsStore::new(nvs_partition.clone());
    let device_id = nvs_store.load_or_init_device_id(DEVICE_ID);
    let (mqtt_user, mqtt_password) =
        nvs_store.load_or_init_mqtt_credentials(default_mqtt_user, default_mqtt_pass);

    if let Ok(mut config) = shared_config.write() {
        config.base_config.device_id = device_id.clone();
    }

    match nvs_store.load_active_recipe() {
        Ok(Some(_recipe)) => info!("Đã khôi phục active recipe từ NVS"),
        Ok(None) => info!("Không có active recipe trong NVS"),
        Err(error) => warn!(
            "recipe_rejected: không thể đọc active recipe từ NVS khi boot: {:?}",
            error
        ),
    }

    let shared_sensors = create_shared_sensor_data(&device_id);

    let (conn_tx, conn_rx) = mpsc::channel();
    let (cmd_tx, cmd_rx) = mpsc::channel();
    let (fsm_tx, fsm_rx) = mpsc::channel();
    let health_fsm_tx = fsm_tx.clone();
    let (dosing_report_tx, dosing_report_rx) = mpsc::channel();
    let (sensor_cmd_tx, sensor_cmd_rx) = mpsc::channel();
    let (int_tx, int_rx) = mpsc::channel::<()>();

    // 1. Hardware Drivers
    let timer_driver = Arc::new(LedcTimerDriver::new(
        peripherals.ledc.timer0,
        &TimerConfig::new().frequency(esp_idf_hal::units::Hertz(20000)),
    )?);

    let i2c_driver = I2cDriver::new(
        peripherals.i2c0,
        peripherals.pins.gpio20,
        peripherals.pins.gpio21,
        &I2cConfig::default(),
    )?;
    let mut valve = I2cExpander::new(i2c_driver);
    let mut pcf_ok = false;
    for attempt in 1..=3 {
        match valve.init() {
            Ok(()) => {
                pcf_ok = true;
                break;
            }
            Err(e) => {
                warn!("PCF8574 init attempt {} failed: {:?}", attempt, e);
                std::thread::sleep(std::time::Duration::from_millis(200));
            }
        }
    }
    if !pcf_ok {
        warn!("⚠️ PCF8574 không khởi tạo được sau 3 lần thử — tiếp tục boot, valve/pump sẽ không hoạt động.");
    }

    // Water pump IN/OUT no longer consume GPIO5/GPIO1.
    // They are driven by PCF8574 P6/P7 through PumpController.
    let pump_controller = PumpController::new(
        LedcDriver::new(
            peripherals.ledc.channel1,
            timer_driver.clone(),
            peripherals.pins.gpio6,
        )?,
        LedcDriver::new(
            peripherals.ledc.channel2,
            timer_driver.clone(),
            peripherals.pins.gpio7,
        )?,
        LedcDriver::new(
            peripherals.ledc.channel3,
            timer_driver.clone(),
            peripherals.pins.gpio0,
        )?,
        LedcDriver::new(
            peripherals.ledc.channel4,
            timer_driver.clone(),
            peripherals.pins.gpio4,
        )?,
        valve,
        PinDriver::output(peripherals.pins.gpio2)?,
        LedcDriver::new(
            peripherals.ledc.channel0,
            timer_driver.clone(),
            peripherals.pins.gpio3,
        )?,
    )?;

    let mut int_pin = PinDriver::input(peripherals.pins.gpio10, esp_idf_hal::gpio::Pull::Up)?;
    int_pin.set_interrupt_type(esp_idf_hal::gpio::InterruptType::NegEdge)?;
    unsafe {
        int_pin.subscribe(move || {
            let _ = int_tx.send(());
        })?;
    }
    int_pin.enable_interrupt()?;

    // 2. Network & Time Sync
    let mut wifi_candidates = hw::load_wifi_list(nvs_partition.clone()).sorted_valid();
    if wifi_candidates.is_empty() {
        info!("📶 [WIFI] No provisioned WiFi list; using compile-time fallback.");
        wifi_candidates.push(hydragrow_shared::WifiCandidate {
            ssid: WIFI_SSID.to_string(),
            password: WIFI_PASS.to_string(),
            priority: 0,
        });
    }
    connect_wifi(
        peripherals.modem,
        sysloop.clone(),
        nvs_partition.clone(),
        wifi_candidates.clone(),
        conn_tx.clone(),
    )?;

    use std::time::Duration as StdDuration;
    let _wifi_up = match conn_rx.recv_timeout(StdDuration::from_secs(120)) {
        Ok(crate::hw::mqtt_client::ConnectionState::WifiConnected) => {
            info!("✅ WiFi connected normally.");
            let _ = conn_tx.send(crate::hw::mqtt_client::ConnectionState::WifiConnected);
            true
        }
        _ => {
            warn!("⚠️ WiFi không kết nối được trong 2 phút. Mở Captive Portal...");
            match hw::run_captive_portal(nvs_partition.clone(), None) {
                Ok(true) => {
                    info!("✅ [PORTAL] Credentials saved, rebooting...");
                    std::thread::sleep(StdDuration::from_millis(500));
                    unsafe {
                        esp_idf_svc::sys::esp_restart();
                    }
                }
                Ok(false) | Err(_) => {
                    warn!("⚠️ [PORTAL] Không có credentials. Tiếp tục không có WiFi.");
                    false
                }
            }
        }
    };

    let _sntp = sync_sntp_time()?;

    // 3. Spawn FSM Thread
    let fsm_cfg = shared_config.clone();
    let fsm_sns = shared_sensors.clone();
    let fsm_nvs_part = nvs_partition.clone();

    std::thread::Builder::new()
        .stack_size(60000)
        .name("fsm_thread".to_string())
        .spawn(move || {
            start_fsm_control_loop(
                fsm_cfg,
                fsm_sns,
                pump_controller,
                fsm_nvs_part,
                cmd_rx,
                fsm_tx,
                dosing_report_tx,
                sensor_cmd_tx,
                int_rx,
                get_current_time_sec(),
            );
        })?;

    // 4. Main Event & Health Loop
    run_main_health_loop(
        MQTT_URL,
        &mqtt_user,
        &mqtt_password,
        MQTT_COMMAND_SECRET,
        shared_config,
        shared_sensors,
        conn_rx,
        conn_tx,
        cmd_tx,
        health_fsm_tx,
        fsm_rx,
        dosing_report_rx,
        sensor_cmd_rx,
        nvs_partition.clone(),
    )
}
