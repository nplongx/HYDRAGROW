use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SensorConfig {
    pub ph_v686: f32, // ph_v7
    pub ph_v4: f32,
    pub ph_v918: f32, // ph_v10
    pub tds_factor: f32, // ec_factor
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
            tds_factor: 0.88,
            ec_offset: 0.0,
            temp_offset: 0.0,
            tank_height: 100.0,
            enable_ph: true,
            enable_tds: true,
            enable_temp: false,
            enable_water: false,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppConfig {
    pub sensor: SensorConfig,
    pub publish_interval_ms: u64,
    pub debug_log: bool,
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            sensor: SensorConfig::default(),
            publish_interval_ms: 5000,
            debug_log: true,
        }
    }
}

impl AppConfig {
    /// Áp config từ JSON document gửi từ backend.
    /// Chỉ update field có trong JSON (merge partial).
    pub fn apply_from_json(&mut self, doc: &Value) {
        if let Some(v) = doc["ph_v7"].as_f64() {
            self.sensor.ph_v686 = v as f32;
        }
        if let Some(v) = doc["ph_v4"].as_f64() {
            self.sensor.ph_v4 = v as f32;
        }
        if let Some(v) = doc["ph_v10"].as_f64() {
            self.sensor.ph_v918 = v as f32;
        }
        if let Some(v) = doc["ec_factor"].as_f64() {
            self.sensor.tds_factor = v as f32;
        }
        if let Some(v) = doc["ec_offset"].as_f64() {
            self.sensor.ec_offset = v as f32;
        }
        if let Some(v) = doc["temp_offset"].as_f64() {
            self.sensor.temp_offset = v as f32;
        }
        if let Some(v) = doc["tank_height"].as_f64() {
            self.sensor.tank_height = v as f32;
        }
        if let Some(v) = doc["enable_ph_sensor"].as_bool() {
            self.sensor.enable_ph = v;
        }
        if let Some(v) = doc["enable_ec_sensor"].as_bool() {
            self.sensor.enable_tds = v;
        }
        if let Some(v) = doc["enable_temp_sensor"].as_bool() {
            self.sensor.enable_temp = v;
        }
        if let Some(v) = doc["enable_water_level_sensor"].as_bool() {
            self.sensor.enable_water = v;
        }
        if let Some(v) = doc["publish_interval"].as_u64() {
            self.publish_interval_ms = v;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_apply_partial_json() {
        let mut cfg = AppConfig::default();
        cfg.apply_from_json(&json!({ "ph_v7": 2700.0, "publish_interval": 10000 }));
        assert_eq!(cfg.sensor.ph_v686, 2700.0);
        assert_eq!(cfg.publish_interval_ms, 10000);
        assert_eq!(cfg.sensor.ph_v4, 3555.0); // unchanged
    }

    #[test]
    fn test_enable_flags() {
        let mut cfg = AppConfig::default();
        cfg.apply_from_json(&json!({ "enable_ph_sensor": false, "enable_ec_sensor": false }));
        assert!(!cfg.sensor.enable_ph);
        assert!(!cfg.sensor.enable_tds);
        assert!(!cfg.sensor.enable_temp); // unchanged (default false)
    }
}
