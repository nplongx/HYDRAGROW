use actix_web::web;
use serde::{Deserialize, Serialize};
use tracing::{error, info};

use crate::AppState;
use crate::db::postgres::{NewSystemEventRecord, insert_system_event};

#[derive(Debug, Deserialize, Serialize)]
pub struct WaterCyclePayload {
    pub device_id: String,
    pub trigger: String, // "refill" | "drain" | "manual"
    pub level_before: f32,
    pub level_after: f32,
    pub duration_sec: Option<u32>,
    pub timestamp_ms: u64,
}

pub fn parse_water_cycle_payload(payload: &[u8]) -> Result<WaterCyclePayload, serde_json::Error> {
    serde_json::from_slice(payload)
}

pub async fn handle(device_id: String, payload: &[u8], app_state: web::Data<AppState>) {
    let data = match parse_water_cycle_payload(payload) {
        Ok(d) => d,
        Err(e) => {
            error!(
                device_id = %device_id,
                error = ?e,
                "Lỗi parse water_cycle payload"
            );
            return;
        }
    };

    let message = format!(
        "Mức nước: {:.1}% → {:.1}% (trigger: {}{})",
        data.level_before,
        data.level_after,
        data.trigger,
        data.duration_sec
            .map(|d| format!(", {}s", d))
            .unwrap_or_default()
    );

    let record = NewSystemEventRecord {
        device_id: device_id.clone(),
        level: "info".to_string(),
        category: "water".to_string(),
        title: format!("Chu kỳ nước — {}", data.trigger),
        message: message.clone(),
        reason: Some(data.trigger.clone()),
        metadata: serde_json::to_value(&data).ok(),
        timestamp: data.timestamp_ms as i64,
    };

    if let Err(e) = insert_system_event(&app_state.pg_pool, &record).await {
        error!(device_id = %device_id, error = ?e, "Lỗi lưu water_cycle vào DB");
    }

    info!(device_id = %device_id, trigger = %data.trigger, "Water cycle event recorded");
}

#[cfg(test)]
mod tests {
    use super::parse_water_cycle_payload;

    #[test]
    fn valid_water_cycle_payload_is_parsed() {
        let json = r#"{
"device_id": "dev1",
"trigger": "refill",
"level_before": 20.0,
"level_after": 80.0,
"duration_sec": 45,
"timestamp_ms": 1234567890123
}"#;
        let result = parse_water_cycle_payload(json.as_bytes());
        assert!(result.is_ok());
        let payload = result.expect("Failed to parse valid water cycle payload");
        assert_eq!(payload.trigger, "refill");
        assert_eq!(payload.level_before, 20.0);
    }

    #[test]
    fn invalid_json_returns_error() {
        let result = parse_water_cycle_payload(b"not json");
        assert!(result.is_err());
    }
}
