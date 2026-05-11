use hydragrow_shared::{ControllerConfig, LogCategory, LogLevel, SystemLogEvent, UnifiedSystemLog};
use std::{
    sync::mpsc::Sender,
    time::{Duration, SystemTime, UNIX_EPOCH},
};

// ---------------------------------------------------------------------------
// Constants
// ---------------------------------------------------------------------------
pub const EMA_ALPHA: f32 = 0.1;
pub const MIN_TOTAL_EC_DOSE_ML: f32 = 0.05;
pub const MIN_PH_DOSE_ML: f32 = 0.05;
pub const MIN_ACTIVE_MIXING_SEC_FOR_CALIB: u64 = 3;
pub const MIN_STABILIZING_SEC_FOR_CALIB: u64 = 3;
pub const CALIBRATION_PERSIST_BATCH_SIZE: u32 = 3;

// ---------------------------------------------------------------------------
// DosePumpKind – dùng nội bộ để tra flow capacity theo loại bơm
// ---------------------------------------------------------------------------
#[derive(Debug, Clone, Copy)]
pub enum DosePumpKind {
    PumpA,
    PumpB,
    PhUp,
    PhDown,
}

// ---------------------------------------------------------------------------
// effective_flow_ml_per_sec
// Trả về None nếu cấu hình không hợp lệ hoặc PWM dưới ngưỡng tối thiểu.
// ---------------------------------------------------------------------------
pub fn effective_flow_ml_per_sec(
    pump: DosePumpKind,
    pwm_percent: u32,
    config: &ControllerConfig,
) -> Option<f32> {
    let (capacity, min_pwm) = match pump {
        DosePumpKind::PumpA => (
            config.pump_a_capacity_ml_per_sec,
            config
                .pump_a_min_pwm_percent
                .unwrap_or(config.dosing_min_pwm_percent),
        ),
        DosePumpKind::PumpB => (
            config.pump_b_capacity_ml_per_sec,
            config
                .pump_b_min_pwm_percent
                .unwrap_or(config.dosing_min_pwm_percent),
        ),
        DosePumpKind::PhUp => (
            config.pump_ph_up_capacity_ml_per_sec,
            config
                .pump_ph_up_min_pwm_percent
                .unwrap_or(config.dosing_min_pwm_percent),
        ),
        DosePumpKind::PhDown => (
            config.pump_ph_down_capacity_ml_per_sec,
            config
                .pump_ph_down_min_pwm_percent
                .unwrap_or(config.dosing_min_pwm_percent),
        ),
    };

    let safe_pwm = pwm_percent.clamp(1, 100);
    let safe_min_pwm = min_pwm.clamp(1, 100) as u32;
    if capacity <= 0.0 || safe_pwm < safe_min_pwm {
        return None;
    }

    Some(capacity * (safe_pwm as f32 / 100.0))
}

// ---------------------------------------------------------------------------
// soft_deadband_scale
// Trả về 0.0 khi error <= tolerance, tăng dần lên 1.0 trong vùng mềm.
// ---------------------------------------------------------------------------
pub fn soft_deadband_scale(error: f32, tolerance: f32) -> f32 {
    let soft_zone_end = (tolerance * 3.0).max(tolerance + 0.01);
    if error <= tolerance {
        0.0
    } else if error >= soft_zone_end {
        1.0
    } else {
        0.35 + 0.65 * ((error - tolerance) / (soft_zone_end - tolerance))
    }
}

// ---------------------------------------------------------------------------
// Thời gian hệ thống
// ---------------------------------------------------------------------------
pub fn get_current_time_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or(Duration::from_secs(0))
        .as_millis() as u64
}

pub fn get_current_time_sec() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or(Duration::from_secs(0))
        .as_secs()
}

/// Hàm tiện ích để đóng gói và gửi log hệ thống
pub fn send_system_log(
    tx: &Sender<String>,
    device_id: &str, // Có thể lấy từ config
    level: LogLevel,
    category: LogCategory,
    title: &str,
    event: SystemLogEvent,
) {
    let log = UnifiedSystemLog {
        device_id: device_id.to_string(),
        level,
        category,
        title: title.to_string(),
        event,
        timestamp_ms: get_current_time_ms(),
    };

    // Đóng gói thành JSON và gửi vào channel MQTT
    if let Ok(json_str) = serde_json::to_string(&log) {
        // Lưu ý: Task MQTT pub của bạn cần được cấu hình để
        // publish các message từ channel này vào topic `.../system_log`
        let _ = tx.send(json_str);
    } else {
        log::error!("Lỗi Serialize SystemLogEvent!");
    }
}
