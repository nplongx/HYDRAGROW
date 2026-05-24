// hydragrow-backend/src/mqtt/handlers/dosing_cycle.rs
use actix_web::web;
use hydragrow_shared::events::AppEvent;
use hydragrow_shared::telemetry::cycle::DosingCycleEvent;
use tracing::{error, info, instrument};

use crate::AppState;
use crate::db::postgres::{NewSystemEventRecord, insert_dosing_report, insert_system_event};
use crate::models::alert::AlertMessage;

#[instrument(skip(app_state, payload), fields(device_id = %device_id))]
pub async fn handle_dosing_cycle(
    device_id: String,
    payload: &[u8],
    app_state: web::Data<AppState>,
) {
    let event: DosingCycleEvent = match serde_json::from_slice(payload) {
        Ok(e) => e,
        Err(err) => {
            let preview = std::str::from_utf8(payload)
                .map(|s| &s[..s.len().min(200)])
                .unwrap_or("<invalid>");
            error!(error = ?err, payload = %preview, "Lỗi parse DosingCycleEvent");
            return;
        }
    };

    info!(
        cycle_id = %event.cycle_id,
        trigger = %event.trigger,
        pump_a = event.dose.pump_a_ml,
        pump_b = event.dose.pump_b_ml,
        "📊 DosingCycleEvent nhận được"
    );

    // 1. Lấy season_id từ DB
    let season_id =
        match crate::db::postgres::get_active_crop_season(&app_state.pg_pool, &device_id).await {
            Ok(Some(season)) => Some(season.id),
            _ => event.season_id.clone(),
        };

    // 2. Lưu vào dosing_reports
    let report_payload = serde_json::to_value(&event).unwrap_or_default();
    if let Err(e) = insert_dosing_report(
        &app_state.pg_pool,
        &device_id,
        season_id.as_deref(),
        event.dose.pump_a_ml,
        event.dose.pump_b_ml,
        event.dose.ph_up_ml,
        event.dose.ph_down_ml,
        &report_payload,
    )
    .await
    {
        error!("Lỗi lưu DosingCycleEvent vào DB: {:?}", e);
    }

    // 3. Fan-out lên event bus
    let alert = AlertMessage {
        level: "success".to_string(),
        category: "dosing".to_string(),
        title: format!("Chu kỳ {} hoàn tất", event.trigger),
        message: format!(
            "A: {:.1}ml | B: {:.1}ml | pH: {:.1}ml | ΔEC: {:.3} | ΔpH: {:.3}",
            event.dose.pump_a_ml,
            event.dose.pump_b_ml,
            event.total_ph_ml(),
            event.delta_ec(),
            event.delta_ph(),
        ),
        device_id: device_id.clone(),
        timestamp: event.timestamp_ms,
        reason: None,
        metadata: Some(report_payload),
    };

    let _ = app_state.event_bus.send(AppEvent::SystemAlert(alert));
    let _ = app_state.event_bus.send(AppEvent::DosingCycle(event));
}
