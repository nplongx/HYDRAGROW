use serde_json::Value;
use sqlx::PgPool;

use crate::db::config_sync;

/// Application boundary for durable desired configuration.
/// Transport handlers provide already-authorized device context and prepared
/// domain payloads; this service owns revision allocation and durable commit.
pub async fn persist_desired_revision(
    pool: &PgPool,
    device_id: &str,
    mut controller: Value,
    mut sensor: Value,
) -> Result<i64, String> {
    let mut tx = pool
        .begin()
        .await
        .map_err(|e| format!("Lỗi bắt đầu transaction config sync: {e:?}"))?;
    let version = config_sync::next_version(&mut tx, device_id)
        .await
        .map_err(|e| format!("Lỗi tạo config version: {e:?}"))?;
    add_revision(&mut controller, version);
    add_revision(&mut sensor, version);
    config_sync::upsert_desired(&mut *tx, device_id, version, &controller, &sensor)
        .await
        .map_err(|e| format!("Lỗi lưu desired config: {e:?}"))?;
    tx.commit()
        .await
        .map_err(|e| format!("Lỗi commit config sync: {e:?}"))?;
    Ok(version)
}

fn add_revision(value: &mut Value, version: i64) {
    value["config_version"] = serde_json::json!(version);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn service_adds_revision_to_both_wire_payloads() {
        let mut controller = serde_json::json!({"mode":"auto"});
        let mut sensor = serde_json::json!({"enable_ph_sensor":true});
        add_revision(&mut controller, 42);
        add_revision(&mut sensor, 42);
        assert_eq!(controller["config_version"], 42);
        assert_eq!(sensor["config_version"], 42);
    }
}
