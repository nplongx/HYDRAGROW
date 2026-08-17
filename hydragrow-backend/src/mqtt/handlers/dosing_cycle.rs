// hydragrow-backend/src/mqtt/handlers/dosing_cycle.rs

use actix_web::web;
use hydragrow_shared::events::AppEvent;
use hydragrow_shared::telemetry::cycle::{CycleOutcome, DosingCycleEvent};
use tracing::{error, info, instrument};

use crate::AppState;
use crate::db::postgres::{NewSystemEventRecord, insert_dosing_report, insert_system_event};
use crate::metrics::DOSING_CYCLES_TOTAL;
use crate::models::alert::AlertMessage;

#[instrument(
    skip(app_state, payload),
    fields(
        device_id = %device_id
    )
)]
pub async fn handle_dosing_cycle(
    device_id: String,
    payload: &[u8],
    app_state: web::Data<AppState>,
) {
    // ============================================================
    // 1. Parse DosingCycleEvent
    // ============================================================

    let event: DosingCycleEvent = match serde_json::from_slice(payload) {
        Ok(e) => e,

        Err(err) => {
            let preview = std::str::from_utf8(payload)
                .map(|s| &s[..s.len().min(200)])
                .unwrap_or("<invalid>");

            error!(
                error = ?err,
                payload = %preview,
                "Lỗi parse DosingCycleEvent"
            );

            return;
        }
    };

    // ============================================================
    // 2. Xác định outcome để ghi metric
    // ============================================================

    let outcome_str = match &event.outcome {
        CycleOutcome::Success => "success",

        CycleOutcome::PartialSuccess { .. } => "partial_success",

        CycleOutcome::Timeout => "timeout",

        CycleOutcome::HardwareFault { .. } => "hardware_fault",
    };

    // ============================================================
    // 3. Prometheus metric
    //
    // dosing_cycles_total{
    //     trigger="auto_mimo",
    //     outcome="success"
    // }
    // ============================================================

    DOSING_CYCLES_TOTAL
        .with_label_values(&[event.trigger.as_str(), outcome_str])
        .inc();

    // ============================================================
    // 4. Logging
    // ============================================================

    info!(
        cycle_id = %event.cycle_id,
        trigger = %event.trigger,
        outcome = %outcome_str,
        duration_ms = event.duration_ms,
        mixing_duration_ms = event.mixing_duration_ms,
        stabilize_duration_ms = event.stabilize_duration_ms,
        pump_a_ml = event.dose.pump_a_ml,
        pump_b_ml = event.dose.pump_b_ml,
        ph_up_ml = event.dose.ph_up_ml,
        ph_down_ml = event.dose.ph_down_ml,
        "DosingCycleEvent nhận được"
    );

    // ============================================================
    // 5. Lấy season_id từ DB
    // ============================================================

    let season_id =
        match crate::db::postgres::get_active_crop_season(&app_state.pg_pool, &device_id).await {
            Ok(Some(season)) => Some(season.id),

            _ => event.season_id.clone(),
        };

    // ============================================================
    // 6. Serialize report payload
    // ============================================================

    let report_payload = serde_json::to_value(&event).unwrap_or_default();

    // ============================================================
    // 7. Lưu vào dosing_reports
    // ============================================================

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
        error!(
            cycle_id = %event.cycle_id,
            device_id = %device_id,
            error = ?e,
            "Lỗi lưu DosingCycleEvent vào DB"
        );
    }

    // ============================================================
    // 8. Tạo system event
    // ============================================================

    let event_record = NewSystemEventRecord {
        device_id: device_id.clone(),

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

        reason: None,

        metadata: Some(report_payload.clone()),

        timestamp: event.timestamp_ms as i64,
    };

    if let Err(e) = insert_system_event(&app_state.pg_pool, &event_record).await {
        error!(
            cycle_id = %event.cycle_id,
            device_id = %device_id,
            error = ?e,
            "Lỗi lưu DosingCycleEvent vào system_events"
        );
    }

    // ============================================================
    // 9. Fan-out lên Event Bus
    // ============================================================

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

    // Gửi SystemAlert cho WebSocket/frontend
    let _ = app_state.event_bus.send(AppEvent::SystemAlert(alert));

    // Gửi DosingCycleEvent cho các subscriber khác
    let _ = app_state.event_bus.send(AppEvent::DosingCycle(event));
}

// ================================================================
// Tests
// ================================================================

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

        // post_stable.ec - pre.ec
        // = 1.55 - 1.2
        // = 0.35

        assert!((event.delta_ec() - 0.35).abs() < 1e-4);
    }

    #[test]
    fn total_ph_ml_computed_correctly() {
        let event = sample_cycle_event();

        // ph_up_ml + ph_down_ml
        // = 0.0 + 1.2
        // = 1.2

        assert!((event.total_ph_ml() - 1.2).abs() < 1e-4);
    }

    #[test]
    fn total_nutrient_ml_computed_correctly() {
        let event = sample_cycle_event();

        // pump_a + pump_b
        // = 5.0 + 4.5
        // = 9.5

        assert!((event.total_nutrient_ml() - 9.5).abs() < 1e-4);
    }

    #[test]
    fn outcome_can_be_classified() {
        let event = sample_cycle_event();

        let outcome_str = match &event.outcome {
            CycleOutcome::Success => "success",

            CycleOutcome::PartialSuccess { .. } => "partial_success",

            CycleOutcome::Timeout => "timeout",

            CycleOutcome::HardwareFault { .. } => "hardware_fault",
        };
        assert_eq!(outcome_str, "partial_success");
    }

    #[test]
    fn malformed_payload_returns_error_without_panic() {
        let bad_payload = b"{ invalid json }";

        let result: Result<DosingCycleEvent, _> = serde_json::from_slice(bad_payload);

        assert!(result.is_err(), "Malformed JSON should return Err");
    }
}
