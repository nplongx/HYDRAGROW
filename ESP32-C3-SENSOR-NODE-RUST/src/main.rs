//! ESP32-C3 Sensor Node — Rust + C FFI integration
//! DS18B20: Rust ds18b20 crate | ADS1115 + HC-SR04: C FFI wrapper

use anyhow::{anyhow, Result};
use ds18b20::{Ds18b20, Resolution};
use esp_idf_hal::delay::FreeRtos;
use esp_idf_hal::gpio::PinDriver;
use esp_idf_hal::i2c::{I2cConfig, I2cDriver};
use esp_idf_hal::peripherals::Peripherals;
use esp_idf_hal::units::Hertz;
use esp_idf_svc::eventloop::EspSystemEventLoop;
use esp_idf_svc::nvs::EspDefaultNvsPartition;
use esp_idf_svc::sntp::{EspSntp, SntpConf};
use esp_idf_svc::wifi::{BlockingWifi, EspWifi};
use log::{error, info, warn};
use one_wire_bus::{Address, OneWire};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

mod config;
mod ffi;
mod filters;
mod mqtt;
mod sensors;
mod utils;
mod wifi;

use config::AppConfig;
use mqtt::manager::{MqttConfig, MqttManager};
use sensors::sensor_manager::{SensorData, SensorManager};
use wifi::provisioner::{connect_wifi, WifiProvisioner};

// Secrets nhúng từ environment variables (giống CONTROLLER-NODE)
const WIFI_FALLBACK_SSID: &str = env!(
    "HYDRAGROW_WIFI_SSID",
    "Lỗi build: Thiếu HYDRAGROW_WIFI_SSID"
);
const WIFI_FALLBACK_PASS: &str = env!(
    "HYDRAGROW_WIFI_PASSWORD",
    "Lỗi build: Thiếu HYDRAGROW_WIFI_PASSWORD"
);
const MQTT_BROKER_URL: &str = env!("HYDRAGROW_MQTT_URL", "Lỗi build: Thiếu HYDRAGROW_MQTT_URL");
const MQTT_CLIENT_ID: &str = env!(
    "HYDRAGROW_DEVICE_ID",
    "Lỗi build: Thiếu HYDRAGROW_DEVICE_ID"
);
const MQTT_USERNAME: &str = env!(
    "HYDRAGROW_MQTT_USER",
    "Lỗi build: Thiếu HYDRAGROW_MQTT_USER"
);
const MQTT_PASSWORD: &str = env!(
    "HYDRAGROW_MQTT_PASSWORD",
    "Lỗi build: Thiếu HYDRAGROW_MQTT_PASSWORD"
);
const MQTT_COMMAND_SECRET: &str = env!(
    "HYDRAGROW_MQTT_COMMAND_SECRET",
    "Lỗi build: Thiếu HYDRAGROW_MQTT_COMMAND_SECRET"
);

// GPIO pins
const PIN_DS18B20: i32 = 2;
const PIN_SDA: u8 = 6;
const PIN_SCL: u8 = 7;

fn main() -> Result<()> {
    // Bắt buộc — link ESP-IDF patches cho Rust std
    esp_idf_svc::sys::link_patches();
    utils::logger::init(true);

    info!("🌱 HYDRAGROW Sensor Node (Rust + C FFI) khởi động...");

    let peripherals = Peripherals::take().unwrap();
    let sysloop = EspSystemEventLoop::take()?;
    let nvs_partition = EspDefaultNvsPartition::take()?;

    // ── Shared state ──
    let shared_config = Arc::new(Mutex::new(AppConfig::default()));
    let shared_data = Arc::new(Mutex::new(SensorData::default()));

    // ── I2C init (I2C0, SDA=6, SCL=7, 100kHz) ──
    let i2c = I2cDriver::new(
        peripherals.i2c0,
        peripherals.pins.gpio6, // SDA
        peripherals.pins.gpio7, // SCL
        &I2cConfig::new().baudrate(Hertz(100_000)),
    )?;
    // I2C được esp-idf-sys install globally; ADS1115 C wrapper dùng I2C_NUM_0

    // ── Sensor Manager init ──
    let mut sensor_manager = SensorManager::new();
    sensor_manager.begin();
    {
        let cfg = shared_config.lock().unwrap();
        sensor_manager.apply_config(&cfg);
    }

    // ── WiFi ──
    let mut provisioner = WifiProvisioner::new(
        nvs_partition.clone(),
        WIFI_FALLBACK_SSID,
        WIFI_FALLBACK_PASS,
    )?;
    let candidates = provisioner.load();
    let mut wifi = BlockingWifi::wrap(
        EspWifi::new(
            peripherals.modem,
            sysloop.clone(),
            Some(nvs_partition.clone()),
        )?,
        sysloop.clone(),
    )?;
    if let Err(e) = connect_wifi(&mut wifi, &candidates) {
        error!("WiFi thất bại: {:?}. Tiếp tục không có mạng...", e);
    }

    // ── SNTP time sync ──
    let _sntp = EspSntp::new_with_callback(
        &SntpConf {
            servers: ["pool.ntp.org"],
            ..Default::default()
        },
        |_| {},
    )?;
    // Đợi sync (tối đa 10s)
    let sntp_deadline = std::time::Instant::now() + Duration::from_secs(10);
    while SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
        < 100_000
        && std::time::Instant::now() < sntp_deadline
    {
        thread::sleep(Duration::from_millis(200));
    }

    // ── MQTT Manager (background thread) ──
    let mqtt_manager = MqttManager::new(
        MqttConfig {
            broker_url: MQTT_BROKER_URL,
            client_id: MQTT_CLIENT_ID,
            username: MQTT_USERNAME,
            password: MQTT_PASSWORD,
            command_secret: MQTT_COMMAND_SECRET,
        },
        Arc::clone(&shared_data),
        Arc::clone(&shared_config),
    );
    mqtt_manager.run()?;

    // ── DS18B20 One-Wire init ──
    let one_wire_pin =
        PinDriver::input_output_od(peripherals.pins.gpio2, esp_idf_hal::gpio::Pull::Up)?;
    let mut one_wire_bus = OneWire::new(one_wire_pin).map_err(|e| anyhow!("{:?}", e))?;
    let mut delay = esp_idf_hal::delay::FreeRtos;

    // Search DS18B20 device
    let ds18b20_device = Ds18b20::new::<anyhow::Error>(Address(0)).ok(); // sẽ search khi đọc

    // ── Main sensor loop ──
    let mut last_publish = std::time::Instant::now();
    loop {
        let raw_temp: Option<f32> = {
            // Start conversion
            ds18b20::start_simultaneous_temp_measurement(&mut one_wire_bus, &mut delay).ok();
            FreeRtos::delay_ms(Resolution::Bits12.max_measurement_time_millis() as u32);
            // Read
            if let Ok(devices) = one_wire_bus
                .devices(false, &mut delay)
                .collect::<Result<Vec<_>, _>>()
            {
                devices
                    .first()
                    .and_then(|addr| Ds18b20::new::<anyhow::Error>(*addr).ok())
                    .and_then(|sensor| sensor.read_data(&mut one_wire_bus, &mut delay).ok())
                    .map(|data| data.temperature)
            } else {
                None
            }
        };

        sensor_manager.update(raw_temp);

        // Update shared data
        {
            let mut data = shared_data.lock().unwrap();
            *data = sensor_manager.data().clone();
        }

        // Apply dynamic config (có thể thay đổi từ MQTT)
        {
            let cfg = shared_config.lock().unwrap();
            sensor_manager.apply_config(&cfg);
        }

        // Publish theo interval
        let interval = {
            let cfg = shared_config.lock().unwrap();
            Duration::from_millis(cfg.publish_interval_ms)
        };
        if last_publish.elapsed() >= interval {
            last_publish = std::time::Instant::now();
            info!(
                "[Sensor] pH={:.2}, EC={:.3} mS/cm, T={:.1}°C, Level={:.1}cm",
                sensor_manager.data().ph,
                sensor_manager.data().tds,
                sensor_manager.data().temperature,
                sensor_manager.data().water_level,
            );
        }
        thread::sleep(Duration::from_millis(200));
    }
}
