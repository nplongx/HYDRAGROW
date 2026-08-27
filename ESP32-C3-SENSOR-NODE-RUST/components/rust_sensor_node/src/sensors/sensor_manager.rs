use crate::config::AppConfig;
use crate::ffi;
use crate::filters::hybrid_filter::HybridFilter;
use crate::sensors::{ph_sensor::*, tds_sensor::*, temp_sensor::*, water_level_sensor::*};
use log::warn;
use std::time::{Duration, Instant};

/// Dữ liệu cảm biến tổng hợp — mirror SensorData của C++.
#[derive(Debug, Clone)]
pub struct SensorData {
    pub temperature: f32,
    pub water_level: f32,
    pub raw_water_level: f32,
    pub ph: f32,
    pub raw_ph: f32,
    pub ph_voltage_mv: f32,
    pub tds: f32, // EC (mS/cm)
    pub err_temperature: bool,
    pub err_water_level: bool,
    pub err_ph: bool,
    pub err_tds: bool,
}

impl Default for SensorData {
    fn default() -> Self {
        Self {
            temperature: 25.0,
            water_level: 20.0,
            raw_water_level: 20.0,
            ph: 6.86,
            raw_ph: 6.86,
            ph_voltage_mv: f32::NAN,
            tds: 0.0,
            err_temperature: false,
            err_water_level: false,
            err_ph: false,
            err_tds: false,
        }
    }
}

const PIN_DS18B20: i32 = 2;
const PIN_TRIG: i32 = 3;
const PIN_ECHO: i32 = 5;

const ADS_PH_ADDR: u8 = ffi::ADS_PH_ADDR;
const ADS_TDS_ADDR: u8 = ffi::ADS_TDS_ADDR;

const SAMPLE_INTERVAL: Duration = Duration::from_millis(200);

pub struct SensorManager {
    temp: TempSensor,
    water: WaterLevelSensor,
    ph: PhSensor,
    tds: TdsSensor,
    temp_filter: HybridFilter,
    water_filter: HybridFilter,
    ph_filter: HybridFilter,
    tds_filter: HybridFilter,
    data: SensorData,
    last_sample: Option<Instant>,
    enable_temp: bool,
    enable_water: bool,
    enable_ph: bool,
    enable_tds: bool,
}

impl SensorManager {
    pub fn new() -> Self {
        Self {
            temp: TempSensor::new(TempSensorConfig::default()),
            water: WaterLevelSensor::new(100.0),
            ph: PhSensor::new(ADS_PH_ADDR, PhSensorConfig::default()),
            tds: TdsSensor::new(ADS_TDS_ADDR, TdsSensorConfig::default()),
            temp_filter: HybridFilter::new(5.0, 0.125),
            water_filter: HybridFilter::new(20.0, 0.125),
            ph_filter: HybridFilter::new(1.5, 0.125),
            tds_filter: HybridFilter::new(0.5, 0.125),
            data: SensorData::default(),
            last_sample: None,
            enable_temp: true,
            enable_water: true,
            enable_ph: true,
            enable_tds: true,
        }
    }

    /// Khởi tạo hardware. Gọi sau khi I2C driver và GPIO đã được cấu hình.
    pub fn begin(&mut self) {
        // HC-SR04
        unsafe {
            ffi::hcsr04_init(PIN_TRIG, PIN_ECHO);
        }

        // ADS1115 pH
        if !self.ph.begin() {
            warn!("[SensorManager] Không tìm thấy ADS1115 pH (0x48)!");
        }

        // ADS1115 TDS
        if !self.tds.begin() {
            warn!("[SensorManager] Không tìm thấy ADS1115 TDS (0x49)!");
        }

        // DS18B20 init stub (Rust đọc trực tiếp qua ds18b20 crate)
        unsafe {
            ffi::ds18b20_init(PIN_DS18B20);
        }
    }

    pub fn update(&mut self, raw_temp_celsius: Option<f32>) {
        let now = Instant::now();
        if let Some(last) = self.last_sample {
            if now.duration_since(last) < SAMPLE_INTERVAL {
                return;
            }
        }
        self.last_sample = Some(now);

        if self.enable_temp {
            let raw = raw_temp_celsius.unwrap_or(f32::NAN);
            let processed = self.temp.process(raw);
            if processed.is_nan() {
                self.data.err_temperature = true;
            } else {
                self.data.err_temperature = false;
                self.data.temperature = self.temp_filter.update(processed);
            }
        }

        if self.enable_water {
            let raw = self.water.read();
            self.data.raw_water_level = raw;
            if raw.is_nan() {
                self.data.err_water_level = true;
            } else {
                self.data.err_water_level = false;
                self.data.water_level = self.water_filter.update(raw);
            }
        }

        if self.enable_ph {
            let raw = self.ph.read(self.data.temperature);
            self.data.raw_ph = raw;
            self.data.ph_voltage_mv = self.ph.last_voltage_mv();
            if raw.is_nan() {
                self.data.err_ph = true;
            } else {
                self.data.err_ph = false;
                self.data.ph = self.ph_filter.update(raw);
            }
        }

        if self.enable_tds {
            let raw = self.tds.read(self.data.temperature);
            if raw.is_nan() {
                self.data.err_tds = true;
            } else {
                self.data.err_tds = false;
                self.data.tds = self.tds_filter.update(raw);
            }
        }
    }

    pub fn data(&self) -> &SensorData {
        &self.data
    }

    pub fn apply_config(&mut self, cfg: &AppConfig) {
        self.enable_temp = cfg.sensor.enable_temp;
        self.enable_water = cfg.sensor.enable_water;
        self.enable_ph = cfg.sensor.enable_ph;
        self.enable_tds = cfg.sensor.enable_tds;

        self.water.set_tank_height(cfg.sensor.tank_height);
        self.temp
            .set_config(TempSensorConfig {
                offset: cfg.sensor.temp_offset,
            });
        self.ph.set_config(PhSensorConfig {
            v686: cfg.sensor.ph_v686,
            v4: cfg.sensor.ph_v4,
            v918: cfg.sensor.ph_v918,
            ..Default::default()
        });
        self.tds.set_config(TdsSensorConfig {
            tds_factor: cfg.sensor.tds_factor,
            ec_offset: cfg.sensor.ec_offset,
            ..Default::default()
        });
    }
}
