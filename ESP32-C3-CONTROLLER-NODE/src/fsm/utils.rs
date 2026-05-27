use hydragrow_shared::{
    log::{LogCategory, LogLevel, SystemLogEvent, UnifiedSystemLog},
    ControllerConfig,
};
use std::{
    sync::{
        atomic::{AtomicU32, Ordering},
        mpsc::Sender,
    },
    time::{Duration, SystemTime, UNIX_EPOCH},
};

static LOG_DROP_COUNT: AtomicU32 = AtomicU32::new(0);

// ---------------------------------------------------------------------------
// DosePumpKind – dùng nội bộ để tra flow capacity theo loại bơm
// ---------------------------------------------------------------------------
#[derive(Debug, Clone, Copy)]
pub enum DosePumpKind {
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
    device_id: &str,
    level: LogLevel,
    category: LogCategory,
    title: &str,
    event: SystemLogEvent,
) {
    let ts = get_current_time_ms();
    let log = match level {
        LogLevel::Info => UnifiedSystemLog::info(device_id, category, title, event, ts),
        LogLevel::Warning => UnifiedSystemLog::warning(device_id, category, title, event, ts),
        LogLevel::Critical => UnifiedSystemLog::critical(device_id, category, title, event, ts),
        LogLevel::Success => UnifiedSystemLog::success(device_id, category, title, event, ts),
    };

    if let Ok(json_str) = serde_json::to_string(&log) {
        if tx.send(json_str).is_err() {
            let prev = LOG_DROP_COUNT.fetch_add(1, Ordering::Relaxed);
            // Log cảnh báo mỗi 10 drops để tránh vòng lặp vô hạn
            if prev % 10 == 0 {
                log::warn!(
                    "⚠️ MQTT log channel đầy! Đã drop {} log kể từ boot.",
                    prev + 1
                );
            }
        }
    } else {
        log::error!("Lỗi Serialize SystemLogEvent!");
    }
}

pub fn get_log_drop_count() -> u32 {
    LOG_DROP_COUNT.load(Ordering::Relaxed)
}
