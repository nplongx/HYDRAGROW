use actix_web::web;
use hydragrow_shared::DosingReportPayload;
use serde_json::json;
use tracing::{error, info, instrument, warn};

use crate::AppState;
use hydragrow_shared::events::AppEvent;
use crate::db::postgres::{NewSystemEventRecord, insert_system_event};
use crate::models::alert::AlertMessage;
use crate::models::config::DosingCalibration;

#[instrument(skip(app_state, payload), fields(device_id = %device_id))]
pub async fn handle_report(device_id: String, payload: &[u8], app_state: web::Data<AppState>) {
    let report: DosingReportPayload = match serde_json::from_slice(payload) {
        Ok(data) => data,
        Err(e) => {
            let payload_preview = String::from_utf8_lossy(payload);
            error!(error = ?e, payload = %payload_preview, "Lỗi parse DosingReport (Cấu trúc không khớp)");
            return;
        }
    };

    info!(
        "🌿 Báo cáo châm phân: A: {:.2}ml, B: {:.2}ml. Đang lưu vào DB...",
        report.dose.pump_a_ml, report.dose.pump_b_ml
    );

    update_dosing_dynamic_learning(&device_id, &report, &app_state).await;

    let season_id_opt =
        match crate::db::postgres::get_active_crop_season(&app_state.pg_pool, &device_id).await {
            Ok(Some(season)) => Some(season.id.to_string()),
            _ => None,
        };

    let report_payload = json!({
        "device_id": device_id,
        "season_id": season_id_opt,
        "timestamp": chrono::Utc::now().to_rfc3339(),
        "dosing_data": report
    });

    if let Err(db_err) = crate::db::postgres::insert_dosing_report(
        &app_state.pg_pool,
        &device_id,
        season_id_opt.as_deref(),
        report.dose.pump_a_ml,
        report.dose.pump_b_ml,
        report.dose.ph_up_ml,
        report.dose.ph_down_ml,
        &report_payload,
    )
    .await
    {
        error!("❌ Lỗi lưu báo cáo châm phân vào DB: {:?}", db_err);
        return;
    }

    let alert_msg_text = format!(
        "Đã lưu báo cáo châm phân: A: {:.1}ml | B: {:.1}ml | pH Up: {:.1}ml | pH Down: {:.1}ml",
        report.dose.pump_a_ml, report.dose.pump_b_ml, report.dose.ph_up_ml, report.dose.ph_down_ml
    );

    let mut metadata = json!({
        "event_type": "dosing_cycle",
        "pre": report.pre,
        "post_mixing": report.post_mixing,
        "post_stable": report.post_stable,
        "dose": report.dose,
        "target": { "ec": report.target_ec, "ph": report.target_ph },
        "error": { "ec": report.error_ec, "ph": report.error_ph },
        "delta": { "ec": report.delta_ec, "ph": report.delta_ph },
        "duration_ms": report.duration_ms,
        "ema_ec_gain_used": report.ema_ec_gain_used,
        "ema_ph_shift_used": report.ema_ph_shift_used,
        "step_ratio_ec": report.step_ratio_ec,
        "step_ratio_ph": report.step_ratio_ph,
        "cycle_id": report.cycle_id,
        "trigger": report.trigger,
        "correction_progress": {
            "ec_remaining": report.target_ec - report.post_stable.ec,
            "ph_remaining": report.target_ph - report.post_stable.ph
        }
    });

    let has_ec_dose = report.dose.pump_a_ml > 0.0 || report.dose.pump_b_ml > 0.0;
    let has_ph_dose = report.dose.ph_up_ml > 0.0 || report.dose.ph_down_ml > 0.0;

    if has_ec_dose {
        metadata["ema_ec_gain_used"] = json!(report.ema_ec_gain_used);
        metadata["step_ratio_ec"] = json!(report.step_ratio_ec);
    }

    // CHỈ chèn thông số pH nếu chu kỳ này thực sự có châm pH
    if has_ph_dose {
        metadata["ema_ph_shift_used"] = json!(report.ema_ph_shift_used);
        metadata["step_ratio_ph"] = json!(report.step_ratio_ph);
    }

    let alert_title = match (has_ec_dose, has_ph_dose) {
        (true, true) => "Lưu Báo Cáo Châm EC/pH Thành Công",
        (true, false) => "Lưu Báo Cáo Châm EC Thành Công",
        (false, true) => "Lưu Báo Cáo Châm pH Thành Công",
        (false, false) => "Lưu Báo Cáo Châm Dinh Dưỡng Thành Công",
    };

    let alert = AlertMessage {
        level: "success".to_string(),
        category: "dosing".to_string(),
        title: alert_title.to_string(),
        message: alert_msg_text,
        device_id: device_id.clone(),
        timestamp: chrono::Utc::now().timestamp_millis() as u64,
        reason: None,
        metadata: Some(metadata),
    };
    let _ = app_state.event_bus.send(AppEvent::SystemAlert(alert));
}

async fn update_dosing_dynamic_learning(
    device_id: &str,
    report: &DosingReportPayload,
    app_state: &web::Data<AppState>,
) {
    const MAX_SAMPLES: usize = 50;
    const SIGNIFICANT_COEF_DELTA_RATIO: f32 = 0.1;

    let dosing_cfg_res = sqlx::query_as::<_, DosingCalibration>(
        "SELECT * FROM dosing_calibration WHERE device_id = $1",
    )
    .bind(device_id)
    .fetch_optional(&app_state.pg_pool)
    .await;

    let dosing_cfg = match dosing_cfg_res {
        Ok(Some(cfg)) => cfg,
        Ok(None) => return,
        Err(e) => {
            warn!(
                "Không thể đọc dosing_calibration để học hệ số động {}: {:?}",
                device_id, e
            );
            return;
        }
    };

    let total_dosed_ml = report.dose.pump_a_ml + report.dose.pump_b_ml;
    if total_dosed_ml <= 0.0 || dosing_cfg.ec_gain_per_ml <= 0.0 {
        return;
    }

    let before_ec = report.pre.ec;
    let after_ec = report.post_mixing.ec;
    let stabilized_ec_value = report.post_stable.ec;

    let before_ph = report.pre.ph;
    let after_ph = report.post_mixing.ph;
    let stabilized_ph = report.post_stable.ph;

    let observed_gain = (stabilized_ec_value - before_ec) / total_dosed_ml;
    if !observed_gain.is_finite() || observed_gain <= 0.0 {
        return;
    }

    let target_gain = (report.target_ec - before_ec) / total_dosed_ml;
    let quality = if target_gain.is_finite() && target_gain.abs() > f32::EPSILON {
        (1.0 - ((observed_gain - target_gain).abs() / target_gain.abs())).clamp(0.0, 1.0)
    } else {
        0.5
    };

    let sample = crate::DosingLearningSample {
        before_ec: Some(before_ec),
        after_ec: Some(after_ec),
        stabilized_ec: Some(stabilized_ec_value),
        before_ph: Some(before_ph),
        after_ph: Some(after_ph),
        stabilized_ph: Some(stabilized_ph),
        stabilized_window_sec: report.stabilized_window_sec,
        reported_at: chrono::Utc::now(),
    };

    let mut states = app_state.dosing_dynamic_states.write().await;
    let state = states
        .entry(device_id.to_string())
        .or_insert_with(|| crate::DosingDynamicState {
            base_ec_gain_per_ml: dosing_cfg.ec_gain_per_ml,
            dynamic_ec_gain_per_ml: dosing_cfg.ec_gain_per_ml,
            confidence: 0.0,
            sample_count: 0,
            last_updated: chrono::Utc::now(),
            samples: std::collections::VecDeque::new(),
        });

    state.base_ec_gain_per_ml = dosing_cfg.ec_gain_per_ml;
    state.samples.push_back(sample);
    while state.samples.len() > MAX_SAMPLES {
        state.samples.pop_front();
    }

    let previous_dynamic = state.dynamic_ec_gain_per_ml;
    let observed_dynamic = observed_gain.clamp(
        dosing_cfg.ec_gain_per_ml * 0.5,
        dosing_cfg.ec_gain_per_ml * 1.5,
    );

    let alpha = 0.18;
    state.dynamic_ec_gain_per_ml =
        ((1.0 - alpha) * state.dynamic_ec_gain_per_ml + alpha * observed_dynamic).max(0.0001);
    state.sample_count = state.samples.len() as u32;
    let sample_confidence = (state.sample_count as f32 / 20.0).clamp(0.0, 1.0);
    state.confidence = ((state.confidence * 0.8) + (quality * 0.2)).max(sample_confidence * 0.6);
    state.last_updated = chrono::Utc::now();

    let delta_ratio = if previous_dynamic.abs() > f32::EPSILON {
        ((state.dynamic_ec_gain_per_ml - previous_dynamic).abs() / previous_dynamic.abs()).abs()
    } else {
        0.0
    };

    if delta_ratio >= SIGNIFICANT_COEF_DELTA_RATIO {
        let _ = insert_system_event(
            &app_state.pg_pool,
            &NewSystemEventRecord {
                device_id: device_id.to_string(),
                level: "info".to_string(),
                category: "calibration".to_string(),
                title: "Cập nhật hệ số châm phân động".to_string(),
                message: format!(
                    "Hệ số EC động thay đổi từ {:.5} lên {:.5} (Δ {:.1}%)",
                    previous_dynamic,
                    state.dynamic_ec_gain_per_ml,
                    delta_ratio * 100.0
                ),
                reason: None,
                metadata: Some(json!({
                    "event_type": "ema_update",
                    "base_ec_gain_per_ml": state.base_ec_gain_per_ml,
                    "dynamic_ec_gain_per_ml": state.dynamic_ec_gain_per_ml,
                    "confidence": state.confidence,
                    "sample_count": state.sample_count,
                    "latest_sample": state.samples.back(),
                    "stabilized_window_sec": report.stabilized_window_sec
                })),
                timestamp: chrono::Utc::now().timestamp_millis(),
            },
        )
        .await;
    }
}
