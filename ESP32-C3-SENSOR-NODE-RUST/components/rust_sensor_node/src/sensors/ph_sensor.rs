use crate::ffi;

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum CalibrationMode {
    TwoPoint,
    ThreePoint,
}

#[derive(Debug, Clone)]
pub struct PhSensorConfig {
    pub v686: f32, // Điện áp tại pH 6.86 (mV)
    pub v4: f32,   // Điện áp tại pH 4.00 (mV)
    pub v918: f32, // Điện áp tại pH 9.18 (mV)
    pub calibration_mode: CalibrationMode,
    pub enable_temp_comp: bool,
    pub nominal_vcc_mv: f32, // VCC chuẩn (mV)
}

impl Default for PhSensorConfig {
    fn default() -> Self {
        Self {
            v686: 2650.0,
            v4: 3555.0,
            v918: 1750.0,
            calibration_mode: CalibrationMode::TwoPoint,
            enable_temp_comp: true,
            nominal_vcc_mv: 5000.0,
        }
    }
}

pub struct PhSensor {
    addr: u8,
    config: PhSensorConfig,
    connected: bool,
    last_voltage_mv: f32,
    last_ph: f32,
}

impl PhSensor {
    pub fn new(addr: u8, config: PhSensorConfig) -> Self {
        Self {
            addr,
            config,
            connected: false,
            last_voltage_mv: f32::NAN,
            last_ph: f32::NAN,
        }
    }

    /// Khởi tạo ADS1115. Gọi sau khi I2C driver đã init.
    pub fn begin(&mut self) -> bool {
        let ok = unsafe { ffi::ads1115_init(self.addr, ffi::GAIN_TWOTHIRDS) };
        self.connected = ok != 0;
        self.connected
    }

    pub fn is_connected(&self) -> bool {
        self.connected
    }

    pub fn set_config(&mut self, config: PhSensorConfig) {
        self.config = config;
    }

    pub fn last_voltage_mv(&self) -> f32 {
        self.last_voltage_mv
    }

    pub fn last_ph(&self) -> f32 {
        self.last_ph
    }

    /// Đọc pH. temperature dùng cho bù nhiệt.
    pub fn read(&mut self, temperature: f32) -> f32 {
        if !self.connected {
            self.last_voltage_mv = f32::NAN;
            self.last_ph = f32::NAN;
            return f32::NAN;
        }

        let diff_mv = unsafe { ffi::ads1115_read_differential_mv(self.addr, 10) };
        let vcc_mv = unsafe { ffi::ads1115_read_single_mv(self.addr, 3, 3) };

        if diff_mv.is_nan() || vcc_mv <= 1000.0 {
            self.last_voltage_mv = f32::NAN;
            self.last_ph = f32::NAN;
            return f32::NAN;
        }

        // Bù VCC
        let compensated_mv = diff_mv * (self.config.nominal_vcc_mv / vcc_mv);
        self.last_voltage_mv = compensated_mv;

        let ph = self.calculate_ph(compensated_mv, temperature);
        self.last_ph = ph;
        ph
    }

    fn calculate_ph(&self, voltage_mv: f32, temperature: f32) -> f32 {
        let (mut slope, base_ph, base_v) = match self.config.calibration_mode {
            CalibrationMode::ThreePoint => {
                if voltage_mv > self.config.v686 {
                    let diff = self.config.v4 - self.config.v686;
                    let s = if diff.abs() < 0.1 {
                        -0.006
                    } else {
                        (4.0 - 6.86) / diff
                    };
                    (s, 6.86_f32, self.config.v686)
                } else {
                    let diff = self.config.v686 - self.config.v918;
                    let s = if diff.abs() < 0.1 {
                        -0.006
                    } else {
                        (6.86 - 9.18) / diff
                    };
                    (s, 9.18_f32, self.config.v918)
                }
            }
            CalibrationMode::TwoPoint => {
                let diff = self.config.v4 - self.config.v686;
                let s = if diff.abs() < 0.1 {
                    -0.006
                } else {
                    (4.0 - 6.86) / diff
                };
                (s, 6.86_f32, self.config.v686)
            }
        };

        if self.config.enable_temp_comp {
            let temp_ratio = (temperature + 273.15) / (25.0 + 273.15);
            slope /= temp_ratio;
        }

        let result = base_ph + slope * (voltage_mv - base_v);
        result.clamp(0.0, 14.0)
    }
}
