// src/main.rs
use esp_idf_hal::gpio::PinDriver;
use esp_idf_hal::i2c::{I2cConfig, I2cDriver};
use esp_idf_hal::ledc::config::TimerConfig;
use esp_idf_hal::ledc::{LedcDriver, LedcTimerDriver};
use esp_idf_hal::peripherals::Peripherals;
use esp_idf_svc::eventloop::EspSystemEventLoop;
use esp_idf_svc::log::EspLogger;
use esp_idf_svc::nvs::EspDefaultNvsPartition;
use hydragrow_controller_core::utils::{get_current_time_sec, write_or_recover};
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

/// Compile-time provisioning inputs are OPTIONAL: release firmware must
/// not depend on per-device WiFi secrets or identity. Provisioned NVS values
/// (or the captive portal / claim flow) supply them at runtime instead.
const WIFI_SSID: &str = match option_env!("HYDRAGROW_WIFI_SSID") {
    Some(value) => value,
    None => "",
};
const WIFI_PASS: &str = match option_env!("HYDRAGROW_WIFI_PASSWORD") {
    Some(value) => value,
    None => "",
};
const MQTT_URL: &str = env!(
    "HYDRAGROW_MQTT_URL",
    "Lỗi build: Thiếu biến HYDRAGROW_MQTT_URL"
);
const MQTT_COMMAND_SECRET: &str = env!(
    "HYDRAGROW_MQTT_COMMAND_SECRET",
    "Lỗi build: Thiếu biến HYDRAGROW_MQTT_COMMAND_SECRET"
);
const DEVICE_ID: &str = match option_env!("HYDRAGROW_DEVICE_ID") {
    Some(value) => value,
    // Empty default: NvsStore derives a stable factory identity from the
    // chip MAC and persists it, so the release binary stays fleet-generic.
    None => "",
};

/// Bounded retry policy for provisioning boot: never infinite-loop on a bad password.
const PENDING_WIFI_MAX_ATTEMPTS: usize = 3;
const ACTIVE_WIFI_MAX_ATTEMPTS: usize = 3;
const BOOT_WIFI_CONNECT_TIMEOUT_SECS: u64 = 120;

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

    {
        let mut state = write_or_recover(&shared_config);
        state.base_config.device_id = device_id.clone();
    }

    match nvs_store.load_active_recipe() {
        Ok(Some(recipe)) => {
            info!("Đã khôi phục active recipe từ NVS: {}", recipe.recipe_id);
            let mut state = write_or_recover(&shared_config);
            state.base_config.active_recipe = Some(recipe);
            state.recompute_effective_config();
        }
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

    // Water pump: GPIO2 = IN, GPIO1 = OUT.
    // GPIO5 = Osaka pump enable.
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
        PinDriver::output(peripherals.pins.gpio1)?,
        PinDriver::output(peripherals.pins.gpio5)?,
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

    // Transactional boot WiFi resolution:
    // Prepared → try pending first (pending candidates first in the ordered
    // list), commit on connect success, rollback + reboot on failure.
    // Committed/none → active list only.
    let tx_state = hw::load_transaction_state(nvs_partition.clone());
    let boot_decision = hydragrow_shared::wifi_tx::BootWifiDecision::from_state(&tx_state);
    let mut wifi_candidates = hw::load_wifi_list(nvs_partition.clone()).sorted_valid();
    let mut boot_used_pending = false;
    if boot_decision == hydragrow_shared::wifi_tx::BootWifiDecision::TryPendingThenActive {
        if let Some(pending) = hw::load_pending_wifi_list(nvs_partition.clone()) {
            let mut pending_candidates = pending.sorted_valid();
            if !pending_candidates.is_empty() {
                if pending_candidates.len() > PENDING_WIFI_MAX_ATTEMPTS {
                    pending_candidates.truncate(PENDING_WIFI_MAX_ATTEMPTS);
                }
                info!(
                    "📶 [WIFI] Pending transaction found (state={}); trying {} pending SSID(s) first (metadata only, no secrets logged).",
                    tx_state,
                    pending_candidates.len()
                );
                // Pending first, then active as fallback within one connect session.
                pending_candidates.extend(wifi_candidates);
                wifi_candidates = pending_candidates;
                boot_used_pending = true;
            }
        }
    }
    if wifi_candidates.len() > PENDING_WIFI_MAX_ATTEMPTS + ACTIVE_WIFI_MAX_ATTEMPTS {
        wifi_candidates.truncate(PENDING_WIFI_MAX_ATTEMPTS + ACTIVE_WIFI_MAX_ATTEMPTS);
    }
    if wifi_candidates.is_empty() {
        // No provisioned list: compile-time fallback only when explicitly
        // baked in (dev builds). Release builds go straight to the portal.
        if !WIFI_SSID.is_empty() {
            info!("📶 [WIFI] No provisioned WiFi list; using compile-time fallback.");
            wifi_candidates.push(hydragrow_shared::WifiCandidate {
                ssid: WIFI_SSID.to_string(),
                password: WIFI_PASS.to_string(),
                priority: 0,
            });
        } else {
            info!("📶 [WIFI] No provisioned WiFi and no fallback; opening captive portal.");
        }
    }
    use std::time::Duration as StdDuration;
    // Captive-portal-first boot when nothing is provisioned: connect_wifi
    // requires a non-empty candidate list, so skip it and let the portal
    // collect credentials (it reboots on success).
    let _wifi_up = if wifi_candidates.is_empty() {
        info!("⚠️ Chưa có WiFi. Mở Captive Portal...");
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
    } else {
        connect_wifi(
            peripherals.modem,
            sysloop.clone(),
            nvs_partition.clone(),
            wifi_candidates.clone(),
            conn_tx.clone(),
        )?;
        match conn_rx.recv_timeout(StdDuration::from_secs(BOOT_WIFI_CONNECT_TIMEOUT_SECS)) {
            Ok(crate::hw::mqtt_client::ConnectionState::WifiConnected) => {
                info!("✅ WiFi connected normally.");
                let _ = conn_tx.send(crate::hw::mqtt_client::ConnectionState::WifiConnected);
                if boot_used_pending {
                    // Pending credentials proved good: promote them to active.
                    match esp_idf_svc::nvs::EspNvs::new(nvs_partition.clone(), "agitech", true) {
                        Ok(mut nvs) => match hw::commit_pending_wifi(&mut nvs) {
                            Ok(()) => info!("📶 [WIFI] Pending config applied and committed."),
                            Err(e) => warn!("📶 [WIFI] Failed to commit pending config: {:?}", e),
                        },
                        Err(e) => warn!("📶 [WIFI] Cannot open NVS to commit pending: {:?}", e),
                    }
                }
                true
            }
            _ => {
                if boot_used_pending {
                    // Pending credentials failed: roll back and reboot into
                    // a clean active-only boot so bad passwords can't brick us.
                    match esp_idf_svc::nvs::EspNvs::new(nvs_partition.clone(), "agitech", true) {
                        Ok(mut nvs) => {
                            if let Err(e) = hw::rollback_pending_wifi(&mut nvs) {
                                warn!("📶 [WIFI] Failed to roll back pending config: {:?}", e);
                            } else {
                                info!(
                                    "📶 [WIFI] Pending config rolled back; rebooting to active WiFi."
                                );
                            }
                        }
                        Err(e) => warn!("📶 [WIFI] Cannot open NVS to roll back: {:?}", e),
                    }
                    std::thread::sleep(StdDuration::from_millis(500));
                    unsafe {
                        esp_idf_svc::sys::esp_restart();
                    }
                }
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
        }
    };

    let _sntp = sync_sntp_time()?;

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
