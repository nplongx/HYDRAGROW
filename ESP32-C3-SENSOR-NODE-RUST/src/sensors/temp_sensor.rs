/// Config nhiệt độ từ AppConfig.
#[derive(Debug, Clone)]
pub struct TempSensorConfig {
    pub offset: f32,
}

impl Default for TempSensorConfig {
    fn default() -> Self {
        Self { offset: 0.0 }
    }
}

/// Wrapper đọc DS18B20 qua esp-idf-hal + one-wire-bus.
pub struct TempSensor {
    config: TempSensorConfig,
    last_temp: f32,
}

impl TempSensor {
    pub fn new(config: TempSensorConfig) -> Self {
        Self {
            config,
            last_temp: f32::NAN,
        }
    }

    pub fn set_config(&mut self, config: TempSensorConfig) {
        self.config = config;
    }

    /// Đọc nhiệt độ. `raw_celsius` được cung cấp từ sensor_manager.
    pub fn process(&mut self, raw_celsius: f32) -> f32 {
        if raw_celsius.is_nan() {
            self.last_temp = f32::NAN;
            return f32::NAN;
        }
        let temp = raw_celsius + self.config.offset;
        self.last_temp = temp;
        temp
    }

    pub fn last(&self) -> f32 {
        self.last_temp
    }
}
