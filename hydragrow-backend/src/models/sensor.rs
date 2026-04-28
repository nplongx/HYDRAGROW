use chrono::{DateTime, FixedOffset};
use influxdb2::FromDataPoint;
use serde::{Deserialize, Serialize};
use validator::Validate;

pub use hydragrow_shared::{DeviceState, PumpStatus, SensorData};

#[derive(Debug, Clone, Serialize, Deserialize, FromDataPoint, Default)]
pub struct SensorDataRow {
    pub device_id: String,
    pub ec: f64,
    pub ph: f64,
    pub temp: f64,
    pub water_level: f64,
    pub pump_status: String,

    pub time: DateTime<FixedOffset>,
}

impl From<SensorDataRow> for SensorData {
    fn from(row: SensorDataRow) -> Self {
        let pump_status = serde_json::from_str(&row.pump_status).unwrap_or_default();
        Self {
            device_id: row.device_id,
            ec: row.ec,
            ph: row.ph,
            temp: row.temp,
            water_level: row.water_level,
            pump_status,

            time: row.time.to_rfc3339(),
            rssi: None,
            free_heap: None,
            is_continuous: None,
            uptime: None,
            err_water: None,
            err_temp: None,
            err_ph: None,
            err_ec: None,
            ph_voltage_mv: None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Validate)]
pub struct PumpCommandReq {
    pub target: Option<String>,

    pub action: String,

    #[serde(default)]
    pub params: Option<PumpCommandParams>,

    #[serde(default, alias = "pump")]
    pub pump_id: Option<String>,
    #[serde(default)]
    pub duration_sec: Option<u64>,
    #[serde(default)]
    pub pwm: Option<u32>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Validate)]
pub struct PumpCommandParams {
    #[serde(default)]
    pub pump_id: Option<String>,
    #[serde(default)]
    pub duration_sec: Option<u64>,
    #[serde(default)]
    pub pwm: Option<u32>,
    #[serde(default)]
    pub state: Option<bool>,
}
