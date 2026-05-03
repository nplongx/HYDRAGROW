use log::{debug, info, warn};
use std::sync::mpsc::Sender;

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
pub fn start_pending_calibration_sample(
    ctx: &mut ControlContext,
    start_ec: f32,
    start_ph: f32,
    pump_a_ml: f32,
    pump_b_ml: f32,
    ph_up_ml: f32,
    ph_down_ml: f32,
    current_time_ms: u64,
    config: &ControllerConfig,
) {
    debug!(
        "🧪 [CALIB START] Bắt đầu lấy mẫu EMA: EC đầu={:.2}, pH đầu={:.2} | Đã châm: A={:.2}ml, B={:.2}ml, UP={:.2}ml, DOWN={:.2}ml",
        start_ec, start_ph, pump_a_ml, pump_b_ml, ph_up_ml, ph_down_ml
    );

    ctx.pending_calibration_sample = Some(PendingCalibrationSample {
        start_ec,
        start_ph,
        pump_a_ml,
        pump_b_ml,
        ph_up_ml,
        ph_down_ml,
        active_mixing_start_ms: current_time_ms,
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
        .saturating_sub(sample.active_mixing_start_ms);
    let stabilizing_elapsed_ms = stabilizing_finish_ms.saturating_sub(stabilizing_start_ms);

    let mixing_ok = active_mixing_elapsed_ms >= MIN_ACTIVE_MIXING_SEC_FOR_CALIB * 1000;
    let stabilizing_ok = stabilizing_elapsed_ms >= MIN_STABILIZING_SEC_FOR_CALIB * 1000;

    if sample.invalid_by_noise
        || sample.invalid_by_water_change
        || !mixing_ok
        || !stabilizing_ok
        || sensors.err_ec
        || sensors.err_ph
    {
        warn!(
            "⏭️ [EMA SKIP] Bỏ qua mẫu EMA (noise={}, water_change={}, mixing_ok={}, stabilizing_ok={}, err_ec={}, err_ph={})",
            sample.invalid_by_noise,
            sample.invalid_by_water_change,
            mixing_ok,
            stabilizing_ok,
            sensors.err_ec,
            sensors.err_ph
        );
        return;
    }

    let ec_after = sensors.ec;
    let ph_after = sensors.ph;
    let total_ec_ml = sample.pump_a_ml + sample.pump_b_ml;

    debug!(
        "📊 [EMA OBSERVED] Kết quả sau ổn định: EC={:.2} -> {:.2} | pH={:.2} -> {:.2}. (Tổng EC_ml={:.2}, pH_Up={:.2}, pH_Down={:.2})",
        sample.start_ec, ec_after, sample.start_ph, ph_after, total_ec_ml, sample.ph_up_ml, sample.ph_down_ml
    );

    let observed_ec_gain_per_ml = if total_ec_ml > MIN_TOTAL_EC_DOSE_ML {
        Some((ec_after - sample.start_ec) / total_ec_ml)
    } else {
        None
    };
    let observed_ph_up_per_ml = if sample.ph_up_ml > MIN_PH_DOSE_ML {
        Some((ph_after - sample.start_ph) / sample.ph_up_ml)
    } else {
        None
    };
    let observed_ph_down_per_ml = if sample.ph_down_ml > MIN_PH_DOSE_ML {
        Some((sample.start_ph - ph_after) / sample.ph_down_ml)
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
                info!("📈 [EMA UPDATE EC] Gain/ml: Cũ={:.4} | Đo đạc={:.4} | EMA Mới={:.4} (Alpha={})", old_val, observed, cfg.ec_gain_per_ml, EMA_ALPHA);
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
                info!("📈 [EMA UPDATE PH UP] Shift/ml: Cũ={:.4} | Đo đạc={:.4} | EMA Mới={:.4} (Alpha={})", old_val, observed, cfg.ph_shift_up_per_ml, EMA_ALPHA);
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
                info!("📈 [EMA UPDATE PH DOWN] Shift/ml: Cũ={:.4} | Đo đạc={:.4} | EMA Mới={:.4} (Alpha={})", old_val, observed, cfg.ph_shift_down_per_ml, EMA_ALPHA);
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
            "pump_a_ml": sample.pump_a_ml,
            "pump_b_ml": sample.pump_b_ml,
            "ph_up_ml": sample.ph_up_ml,
            "ph_down_ml": sample.ph_down_ml,
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

