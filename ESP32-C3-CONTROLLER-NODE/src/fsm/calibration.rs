use log::{debug, info, warn};
use std::sync::mpsc::Sender;
use uuid::Uuid;

use super::context::ControlContext;
use super::types::PendingCalibrationSample;
use super::utils::{
    CALIBRATION_PERSIST_BATCH_SIZE, EMA_ALPHA, MIN_ACTIVE_MIXING_SEC_FOR_CALIB, MIN_PH_DOSE_ML,
    MIN_STABILIZING_SEC_FOR_CALIB, MIN_TOTAL_EC_DOSE_ML,
};
use crate::config::SharedConfig;
use crate::mqtt::SensorData;
use hydragrow_shared::ControllerConfig;

// ---------------------------------------------------------------------------
// start_pending_calibration_sample
// Khởi tạo một mẫu calibration mới ngay trước khi vào ActiveMixing.
// ---------------------------------------------------------------------------
#[allow(clippy::too_many_arguments)]
pub fn start_pending_calibration_sample(
    ctx: &mut ControlContext,
    start_ec: f32,
    start_ph: f32,
    dose_a_ml: f32,
    dose_b_ml: f32,
    dose_ph_up_ml: f32,
    dose_ph_down_ml: f32,
    current_time_ms: u64,
    config: &ControllerConfig,
) {
    // Xác định trigger dựa trên liều lượng bơm
    let trigger = if dose_a_ml > 0.0 && dose_b_ml > 0.0 {
        "auto_ec".to_string()
    } else if dose_ph_up_ml > 0.0 || dose_ph_down_ml > 0.0 {
        "auto_ph".to_string()
    } else {
        "manual".to_string()
    };

    // Lấy water_level lúc bắt đầu (giả sử có lưu lại trong ctx hoặc dùng config.water_level_target tạm)
    // Tốt nhất là thêm last_water_before_dosing vào ControlContext, ở đây dùng tạm target nếu ko có
    let start_water_level = ctx
        .last_water_before_refill
        .unwrap_or(config.water_level_target);

    debug!(
        "🧪 [CALIB START] Bắt đầu lấy mẫu EMA ({}): EC đầu={:.2}, pH đầu={:.2} | Đã châm: A={:.2}ml, B={:.2}ml, UP={:.2}ml, DOWN={:.2}ml",
        trigger, start_ec, start_ph, dose_a_ml, dose_b_ml, dose_ph_up_ml, dose_ph_down_ml
    );

    ctx.pending_calibration_sample = Some(PendingCalibrationSample {
        cycle_id: uuid::Uuid::new_v4().to_string(),
        trigger,
        start_ec,
        start_ph,
        start_water_level,
        target_ec: config.ec_target,
        target_ph: config.ph_target,
        dose_a_ml,
        dose_b_ml,
        dose_ph_up_ml,
        dose_ph_down_ml,
        post_mixing_ec: 0.0, // Sẽ được cập nhật ở ActiveMixing -> Stabilizing
        post_mixing_ph: 0.0,
        start_ms: current_time_ms,
        active_mixing_finish_ms: current_time_ms + (config.active_mixing_sec as u64 * 1000),
        stabilizing_start_ms: None,
        stabilizing_finish_ms: None,
        invalid_by_noise: false,
        invalid_by_water_change: false,
    });
}

// ---------------------------------------------------------------------------
// apply_runtime_calibration_ema
// Áp dụng EMA để cập nhật hệ số ec_gain_per_ml, ph_shift_up/down_per_ml
// dựa trên phản hồi thực tế sau mỗi chu kỳ bơm hoàn chỉnh.
// ---------------------------------------------------------------------------
pub fn apply_runtime_calibration_ema(
    sensors: &SensorData,
    shared_config: &SharedConfig,
    ctx: &mut ControlContext,
    fsm_mqtt_tx: &Sender<String>,
) {
    let sample = match ctx.pending_calibration_sample.take() {
        Some(s) => s,
        None => return,
    };
    let stabilizing_start_ms = match sample.stabilizing_start_ms {
        Some(v) => v,
        None => {
            warn!("⚠️ [EMA] Thiếu stabilizing_start_ms. Hủy tính toán EMA.");
            return;
        }
    };
    let stabilizing_finish_ms = match sample.stabilizing_finish_ms {
        Some(v) => v,
        None => {
            warn!("⚠️ [EMA] Thiếu stabilizing_finish_ms. Hủy tính toán EMA.");
            return;
        }
    };

    let active_mixing_elapsed_ms = sample
        .active_mixing_finish_ms
        .saturating_sub(sample.start_ms); // Tính từ lúc bắt đầu cycle (trước là active_mixing_start_ms)
    let stabilizing_elapsed_ms = stabilizing_finish_ms.saturating_sub(stabilizing_start_ms);

    let mixing_ok = active_mixing_elapsed_ms >= MIN_ACTIVE_MIXING_SEC_FOR_CALIB * 1000;
    let stabilizing_ok = stabilizing_elapsed_ms >= MIN_STABILIZING_SEC_FOR_CALIB * 1000;

    // --- XÁC ĐỊNH LÝ DO SKIP (Cho Log P2-D) ---
    let mut skip_reason: Option<&str> = None;
    if sample.invalid_by_noise {
        skip_reason = Some("noise");
    } else if sample.invalid_by_water_change {
        skip_reason = Some("water_change");
    } else if !mixing_ok {
        skip_reason = Some("short_mixing");
    } else if !stabilizing_ok {
        skip_reason = Some("short_stabilizing");
    } else if sensors.err_ec || sensors.err_ph {
        skip_reason = Some("sensor_error");
    }

    if let Some(reason) = skip_reason {
        warn!(
            r#"[EMA UPDATE] {{ "parameter": "skipped", "skip_reason": "{}" }}"#,
            reason
        );
        return;
    }

    let ec_after = sensors.ec;
    let ph_after = sensors.ph;
    let total_ec_ml = sample.dose_a_ml + sample.dose_b_ml;

    debug!(
        "📊 [EMA OBSERVED] Kết quả sau ổn định: EC={:.2} -> {:.2} | pH={:.2} -> {:.2}. (Tổng EC_ml={:.2}, pH_Up={:.2}, pH_Down={:.2})",
        sample.start_ec, ec_after, sample.start_ph, ph_after, total_ec_ml, sample.dose_ph_up_ml, sample.dose_ph_down_ml
    );

    let observed_ec_gain_per_ml = if total_ec_ml > MIN_TOTAL_EC_DOSE_ML {
        Some((ec_after - sample.start_ec) / total_ec_ml)
    } else {
        None
    };
    let observed_ph_up_per_ml = if sample.dose_ph_up_ml > MIN_PH_DOSE_ML {
        Some((ph_after - sample.start_ph) / sample.dose_ph_up_ml)
    } else {
        None
    };
    let observed_ph_down_per_ml = if sample.dose_ph_down_ml > MIN_PH_DOSE_ML {
        Some((sample.start_ph - ph_after) / sample.dose_ph_down_ml)
    } else {
        None
    };

    let mut updated = false;
    let mut applied_ec_gain = None;
    let mut applied_ph_up = None;
    let mut applied_ph_down = None;

    if let Ok(mut cfg) = shared_config.write() {
        // --- EC EMA ---
        if let Some(observed) = observed_ec_gain_per_ml {
            if observed.is_finite() && observed > 0.0 {
                let old_val = cfg.ec_gain_per_ml;
                cfg.ec_gain_per_ml = old_val * (1.0 - EMA_ALPHA) + observed * EMA_ALPHA;
                applied_ec_gain = Some(cfg.ec_gain_per_ml);
                updated = true;

                // --- BẮT ĐẦU: GHI LOG EMA EC (P2 - D) ---
                info!(
                    r#"[EMA UPDATE] {{ "parameter": "ec_gain_per_ml", "old_value": {:.4}, "observed": {:.4}, "new_ema": {:.4}, "alpha": {:.2}, "sample_count": {}, "skip_reason": null }}"#,
                    old_val,
                    observed,
                    cfg.ec_gain_per_ml,
                    EMA_ALPHA,
                    ctx.calibration_sample_count_ec + 1
                );
                ctx.calibration_sample_count_ec += 1;
                // --- KẾT THÚC LOG EMA EC ---
            } else {
                warn!(
                    "⚠️ [EMA UPDATE EC] Bỏ qua quan trắc EC bất thường: {:.4}",
                    observed
                );
            }
        }

        // --- PH UP EMA ---
        if let Some(observed) = observed_ph_up_per_ml {
            if observed.is_finite() && observed > 0.0 {
                let old_val = cfg.ph_shift_up_per_ml;
                cfg.ph_shift_up_per_ml = old_val * (1.0 - EMA_ALPHA) + observed * EMA_ALPHA;
                applied_ph_up = Some(cfg.ph_shift_up_per_ml);
                updated = true;

                // --- BẮT ĐẦU: GHI LOG EMA PH UP (P2 - D) ---
                info!(
                    r#"[EMA UPDATE] {{ "parameter": "ph_shift_up_per_ml", "old_value": {:.4}, "observed": {:.4}, "new_ema": {:.4}, "alpha": {:.2}, "sample_count": {}, "skip_reason": null }}"#,
                    old_val,
                    observed,
                    cfg.ph_shift_up_per_ml,
                    EMA_ALPHA,
                    ctx.calibration_sample_count_ph_up + 1
                );
                ctx.calibration_sample_count_ph_up += 1;
                // --- KẾT THÚC LOG EMA PH UP ---
            } else {
                warn!(
                    "⚠️ [EMA UPDATE PH UP] Bỏ qua quan trắc pH UP bất thường: {:.4}",
                    observed
                );
            }
        }

        // --- PH DOWN EMA ---
        if let Some(observed) = observed_ph_down_per_ml {
            if observed.is_finite() && observed > 0.0 {
                let old_val = cfg.ph_shift_down_per_ml;
                cfg.ph_shift_down_per_ml = old_val * (1.0 - EMA_ALPHA) + observed * EMA_ALPHA;
                applied_ph_down = Some(cfg.ph_shift_down_per_ml);
                updated = true;

                // --- BẮT ĐẦU: GHI LOG EMA PH DOWN (P2 - D) ---
                info!(
                    r#"[EMA UPDATE] {{ "parameter": "ph_shift_down_per_ml", "old_value": {:.4}, "observed": {:.4}, "new_ema": {:.4}, "alpha": {:.2}, "sample_count": {}, "skip_reason": null }}"#,
                    old_val,
                    observed,
                    cfg.ph_shift_down_per_ml,
                    EMA_ALPHA,
                    ctx.calibration_sample_count_ph_down + 1
                );
                ctx.calibration_sample_count_ph_down += 1;
                // --- KẾT THÚC LOG EMA PH DOWN ---
            } else {
                warn!(
                    "⚠️ [EMA UPDATE PH DOWN] Bỏ qua quan trắc pH DOWN bất thường: {:.4}",
                    observed
                );
            }
        }
    }

    if !updated {
        debug!("ℹ️ [EMA] Không có thông số nào được cập nhật trong chu kỳ này.");
        return;
    }

    ctx.calibration_pending_publish_count += 1;
    if ctx.calibration_pending_publish_count >= CALIBRATION_PERSIST_BATCH_SIZE {
        info!(
            "📤 [EMA PUBLISH] Đạt ngưỡng Batch ({}). Tiến hành gửi MQTT cập nhật Backend...",
            CALIBRATION_PERSIST_BATCH_SIZE
        );
        ctx.calibration_pending_publish_count = 0;
        let payload = serde_json::json!({
            "type": "runtime_calibration_update",
            "alpha": EMA_ALPHA,
            "persist": true,
            "persist_target": "backend_api",
            "start_ec": sample.start_ec,
            "start_ph": sample.start_ph,
            "ec_after": ec_after,
            "ph_after": ph_after,
            "pump_a_ml": sample.dose_a_ml,
            "pump_b_ml": sample.dose_b_ml,
            "ph_up_ml": sample.dose_ph_up_ml,
            "ph_down_ml": sample.dose_ph_down_ml,
            "observed_ec_gain_per_ml": observed_ec_gain_per_ml,
            "observed_ph_up_per_ml": observed_ph_up_per_ml,
            "observed_ph_down_per_ml": observed_ph_down_per_ml,
            "runtime_coefficients": {
                "ec_gain_per_ml": applied_ec_gain,
                "ph_shift_up_per_ml": applied_ph_up,
                "ph_shift_down_per_ml": applied_ph_down
            }
        });
        let _ = fsm_mqtt_tx.send(payload.to_string());
    } else {
        debug!(
            "📥 [EMA BATCH] Đã lưu vào bộ đệm gửi MQTT: {}/{}",
            ctx.calibration_pending_publish_count, CALIBRATION_PERSIST_BATCH_SIZE
        );
    }
}
