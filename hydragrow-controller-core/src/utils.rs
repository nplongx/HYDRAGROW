use hydragrow_shared::{
    log::{emit_system_log_event, LogCategory, LogLevel, SystemLogEvent, UnifiedSystemLog},
    ControllerConfig,
};
use std::{
    sync::{
        atomic::{AtomicU32, Ordering},
        mpsc::Sender,
        RwLock, RwLockWriteGuard,
    },
    time::{Duration, SystemTime, UNIX_EPOCH},
};

static LOG_DROP_COUNT: AtomicU32 = AtomicU32::new(0);

/// Clone a shared value even if a previous writer panicked while holding its
/// lock. Control loops must continue operating safely rather than resetting
/// halfway through an actuator operation.
pub fn read_or_recover<T: Clone>(lock: &RwLock<T>) -> T {
    match lock.read() {
        Ok(guard) => guard.clone(),
        Err(poisoned) => {
            tracing::error!("shared RwLock poisoned; recovering its last value");
            poisoned.into_inner().clone()
        }
    }
}

/// Obtain a write guard after recovering from poisoning. The caller still
/// performs its normal update, but a poisoned lock cannot crash the control
/// loop.
pub fn write_or_recover<T>(lock: &RwLock<T>) -> RwLockWriteGuard<'_, T> {
    match lock.write() {
        Ok(guard) => guard,
        Err(poisoned) => {
            tracing::error!("shared RwLock poisoned; recovering write access");
            poisoned.into_inner()
        }
    }
}

#[cfg(test)]
mod recover_tests {
    use super::*;
    use std::sync::Arc;

    #[test]
    fn read_or_recover_returns_value_after_poison() {
        let lock = Arc::new(RwLock::new(42u32));
        let lock_clone = lock.clone();

        let result = std::panic::catch_unwind(move || {
            let _guard = lock_clone.write().unwrap();
            panic!("simulated panic while holding write lock");
        });
        assert!(result.is_err());
        assert!(lock.is_poisoned());

        assert_eq!(read_or_recover(&lock), 42);
    }
}

#[derive(Debug, Clone, Copy)]
pub enum DosePumpKind {
    PumpA,
    PumpB,
    PhUp,
    PhDown,
}

#[derive(Debug, Clone, serde::Deserialize, serde::Serialize)]
pub struct CropRecipe {
    #[serde(default)]
    pub device_id: String,
    #[serde(default)]
    pub schema_version: u16,
    #[serde(default)]
    pub revision: u32,
    #[serde(default)]
    pub stages: Vec<CropRecipeStage>,
}

#[derive(Debug, Clone, serde::Deserialize, serde::Serialize)]
pub struct CropRecipeStage {
    #[serde(default)]
    pub name: String,
    #[serde(default)]
    pub duration_sec: u32,
    #[serde(default)]
    pub ec_target: f32,
    #[serde(default)]
    pub ph_target: f32,
    #[serde(default)]
    pub misting_on_duration_ms: i32,
    #[serde(default)]
    pub misting_off_duration_ms: i32,
}

pub const CURRENT_RECIPE_SCHEMA_VERSION: u16 = 1;
pub const MAX_RECIPE_STAGES: usize = 16;
pub const MAX_RECIPE_TOTAL_DURATION_SEC: u64 = 180 * 24 * 60 * 60;

pub fn validate_recipe(
    recipe: &CropRecipe,
    config: &ControllerConfig,
    device_id: &str,
    current_revision: Option<u32>,
) -> anyhow::Result<()> {
    if recipe.device_id != device_id {
        anyhow::bail!(
            "device_id_mismatch: recipe={}, expected={}",
            recipe.device_id,
            device_id
        );
    }

    if recipe.schema_version != CURRENT_RECIPE_SCHEMA_VERSION {
        anyhow::bail!(
            "unsupported_schema_version: recipe={}, expected={}",
            recipe.schema_version,
            CURRENT_RECIPE_SCHEMA_VERSION
        );
    }

    if recipe.stages.is_empty() || recipe.stages.len() > MAX_RECIPE_STAGES {
        anyhow::bail!(
            "invalid_stage_count: count={}, allowed=1..={}",
            recipe.stages.len(),
            MAX_RECIPE_STAGES
        );
    }

    let total_duration_sec = recipe
        .stages
        .iter()
        .try_fold(0_u64, |total, stage| {
            total.checked_add(stage.duration_sec as u64)
        })
        .ok_or_else(|| anyhow::anyhow!("total_duration_overflow"))?;
    if total_duration_sec == 0 || total_duration_sec > MAX_RECIPE_TOTAL_DURATION_SEC {
        anyhow::bail!(
            "invalid_total_duration: total_sec={}, allowed=1..={}",
            total_duration_sec,
            MAX_RECIPE_TOTAL_DURATION_SEC
        );
    }

    for (idx, stage) in recipe.stages.iter().enumerate() {
        if stage.duration_sec == 0 {
            anyhow::bail!("invalid_stage_duration: stage={}, duration_sec=0", idx);
        }
        if stage.ec_target < config.min_ec_limit || stage.ec_target > config.max_ec_limit {
            anyhow::bail!(
                "ec_out_of_range: stage={}, ec_target={}, allowed={}..={}",
                idx,
                stage.ec_target,
                config.min_ec_limit,
                config.max_ec_limit
            );
        }
        if stage.ph_target < config.min_ph_limit || stage.ph_target > config.max_ph_limit {
            anyhow::bail!(
                "ph_out_of_range: stage={}, ph_target={}, allowed={}..={}",
                idx,
                stage.ph_target,
                config.min_ph_limit,
                config.max_ph_limit
            );
        }
        if stage.misting_on_duration_ms < 0
            || stage.misting_on_duration_ms as i64 > config.high_temp_misting_on_duration_ms
            || stage.misting_off_duration_ms < 0
            || stage.misting_off_duration_ms < config.misting_off_duration_ms
        {
            anyhow::bail!(
                "misting_out_of_range: stage={}, on_ms={}, off_ms={}, max_on_ms={}, min_off_ms={}",
                idx,
                stage.misting_on_duration_ms,
                stage.misting_off_duration_ms,
                config.high_temp_misting_on_duration_ms,
                config.misting_off_duration_ms
            );
        }
    }

    if let Some(current_revision) = current_revision {
        if recipe.revision < current_revision {
            anyhow::bail!(
                "stale_revision: recipe={}, current={}",
                recipe.revision,
                current_revision
            );
        }
    }

    Ok(())
}

pub fn build_recipe_event(
    device_id: &str,
    status: &str,
    revision: u32,
    reason: Option<&str>,
) -> String {
    serde_json::json!({
        "type": "recipe_event",
        "device_id": device_id,
        "status": status,
        "revision": revision,
        "reason": reason,
        "ts": get_current_time_ms()
    })
    .to_string()
}

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

    let log = UnifiedSystemLog {
        device_id: device_id.to_string(),
        level: level.clone(),
        category: category.clone(),
        title: title.to_string(),
        event: event.clone(),
        timestamp_ms: ts,
    };

    if let Ok(json) = serde_json::to_string(&log) {
        let _ = tx.send(json);
    }

    emit_system_log_event(device_id, level, category, title, event, ts);
}

pub fn get_log_drop_count() -> u32 {
    LOG_DROP_COUNT.load(Ordering::Relaxed)
}

pub fn log_drop_counter() -> &'static AtomicU32 {
    &LOG_DROP_COUNT
}
