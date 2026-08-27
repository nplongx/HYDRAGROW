use crate::ffi;

#[derive(Debug, Clone)]
pub struct TdsSensorConfig {
    pub k_value: f32,          // Hệ số hiệu chuẩn
    pub ec_offset: f32,        // mS/cm offset
    pub tds_factor: f32,       // ppm scale (thường 500)
    pub temp_compensation: bool,
    pub temp_coefficient: f32, // 0.02 = 2%/°C
}

impl Default for TdsSensorConfig {
    fn default() -> Self {
        Self {
            k_value: 1.0,
            ec_offset: 0.0,
            tds_factor: 500.0,
            temp_compensation: true,
            temp_coefficient: 0.02,
        }
    }
}

pub struct TdsSensor {
    addr: u8,
    config: TdsSensorConfig,
    connected: bool,
    last_voltage_mv: f32,
    last_ec: f32,  // mS/cm
    last_tds: f32, // ppm
}

impl TdsSensor {
    pub fn new(addr: u8, config: TdsSensorConfig) -> Self {
        Self {
            addr,
            config,
            connected: false,
            last_voltage_mv: f32::NAN,
            last_ec: f32::NAN,
            last_tds: f32::NAN,
        }
    }

    pub fn begin(&mut self) -> bool {
        let ok = unsafe { ffi::ads1115_init(self.addr, ffi::GAIN_ONE) };
        self.connected = ok != 0;
        self.connected
    }

    pub fn is_connected(&self) -> bool {
        self.connected
    }
    pub fn last_voltage_mv(&self) -> f32 {
        self.last_voltage_mv
    }
    pub fn last_ec(&self) -> f32 {
        self.last_ec
    }
    pub fn last_tds(&self) -> f32 {
        self.last_tds
    }
    pub fn set_config(&mut self, c: TdsSensorConfig) {
        self.config = c;
    }

    /// Trả về EC (mS/cm).
    pub fn read(&mut self, temperature: f32) -> f32 {
        if !self.connected {
            self.last_voltage_mv = f32::NAN;
            self.last_ec = f32::NAN;
            self.last_tds = f32::NAN;
            return f32::NAN;
        }

        let mv = unsafe { ffi::ads1115_read_single_mv(self.addr, 0, 10) };
        if mv.is_nan() {
            self.last_voltage_mv = f32::NAN;
            self.last_ec = f32::NAN;
            self.last_tds = f32::NAN;
            return f32::NAN;
        }
        self.last_voltage_mv = mv;

        let v = mv / 1000.0; // mV -> V
        // Bù nhiệt độ về 25°C
        let comp_coeff = if self.config.temp_compensation
            && !temperature.is_nan()
            && temperature > 0.0
        {
            let c = 1.0 + self.config.temp_coefficient * (temperature - 25.0);
            if c <= 0.0 {
                1.0
            } else {
                c
            }
        } else {
            1.0
        };

        let comp_v = v / comp_coeff;
        // Đa thức DFRobot -> EC (µS/cm)
        let ec_us = (133.42 * comp_v.powi(3) - 255.86 * comp_v.powi(2) + 857.39 * comp_v)
            * self.config.k_value;
        let ec_us = ec_us.max(0.0);
        let ec_ms = (ec_us / 1000.0 + self.config.ec_offset).max(0.0);
        let tds_ppm = ec_ms * self.config.tds_factor;

        self.last_ec = ec_ms;
        self.last_tds = tds_ppm;
        ec_ms
    }
}
