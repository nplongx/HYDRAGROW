#[derive(Debug, Clone)]
pub struct SensorConfig {
    pub ph_v686: f32,
    pub ph_v4: f32,
    pub ph_v918: f32,
    pub tds_factor: f32,
    pub ec_offset: f32,
    pub temp_offset: f32,
    pub tank_height: f32,
    pub enable_ph: bool,
    pub enable_tds: bool,
    pub enable_temp: bool,
    pub enable_water: bool,
}

impl Default for SensorConfig {
    fn default() -> Self {
        Self {
            ph_v686: 2650.0,
            ph_v4: 3555.0,
            ph_v918: 1750.0,
            tds_factor: 500.0,
            ec_offset: 0.0,
            temp_offset: 0.0,
            tank_height: 100.0,
            enable_ph: true,
            enable_tds: true,
            enable_temp: true,
            enable_water: true,
        }
    }
}

#[derive(Debug, Clone, Default)]
pub struct AppConfig {
    pub sensor: SensorConfig,
    pub publish_interval: u64,
    pub debug_log: bool,
    pub continuous_level: bool,
}
