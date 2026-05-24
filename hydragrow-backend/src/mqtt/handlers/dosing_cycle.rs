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

#[cfg(test)]
mod tests {
    use hydragrow_shared::telemetry::cycle::{
        CycleOutcome, DosingCycleEvent, DosingDoseRecord, DosingPhaseSnapshot,
    };

    fn sample_cycle_event() -> DosingCycleEvent {
        DosingCycleEvent {
            cycle_id: "test-cycle-001".into(),
            device_id: "device_001".into(),
            trigger: "auto_mimo".into(),
            pre: DosingPhaseSnapshot {
                ec: 1.2,
                ph: 6.0,
                water_level: 18.0,
                temp: None,
            },
            post_mixing: DosingPhaseSnapshot {
                ec: 1.5,
                ph: 6.1,
                water_level: 18.0,
                temp: None,
            },
            post_stable: DosingPhaseSnapshot {
                ec: 1.55,
                ph: 6.05,
                water_level: 18.0,
                temp: None,
            },
            target_ec: 1.6,
            target_ph: 6.0,
            dose: DosingDoseRecord {
                pump_a_ml: 5.0,
                pump_b_ml: 4.5,
                ph_up_ml: 0.0,
                ph_down_ml: 1.2,
                water_in_sec: 0.0,
                water_out_sec: 0.0,
            },
            outcome: CycleOutcome::PartialSuccess {
                ec_reached: false,
                ph_reached: true,
            },
            duration_ms: 45_000,
            mixing_duration_ms: 20_000,
            stabilize_duration_ms: 15_000,
            timestamp_ms: 1_748_000_000_000,
            kalman: None,
            season_id: None,
        }
    }

    #[test]
    fn dosing_cycle_event_deserializes_from_json() {
        let event = sample_cycle_event();
        let json = serde_json::to_string(&event).unwrap();
        let decoded: DosingCycleEvent = serde_json::from_str(&json).unwrap();
        assert_eq!(decoded.cycle_id, "test-cycle-001");
        assert_eq!(decoded.dose.pump_a_ml, 5.0);
    }

    #[test]
    fn delta_ec_computed_correctly() {
        let event = sample_cycle_event();
        // post_stable.ec - pre.ec = 1.55 - 1.2 = 0.35
        assert!((event.delta_ec() - 0.35).abs() < 1e-4);
    }

    #[test]
    fn total_ph_ml_computed_correctly() {
        let event = sample_cycle_event();
        // ph_up_ml + ph_down_ml = 0.0 + 1.2 = 1.2
        assert!((event.total_ph_ml() - 1.2).abs() < 1e-4);
    }

    #[test]
    fn total_nutrient_ml_computed_correctly() {
        let event = sample_cycle_event();
        // pump_a + pump_b = 5.0 + 4.5 = 9.5
        assert!((event.total_nutrient_ml() - 9.5).abs() < 1e-4);
    }

    #[test]
    fn malformed_payload_returns_early_without_panic() {
        // handle_dosing_cycle đọc &[u8] và return nếu parse fail
        // Test logic parse:
        let bad_payload = b"{ invalid json }";
        let result: Result<DosingCycleEvent, _> = serde_json::from_slice(bad_payload);
        assert!(result.is_err(), "Malformed JSON should return Err");
    }
}
