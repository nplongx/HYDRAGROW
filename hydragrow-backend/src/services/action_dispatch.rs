//! Dispatch action_command script output ra MQTT — LUÔN đi qua gate an toàn của
//! `hydragrow_shared::safety` trước. Đây là nơi DUY NHẤT được phép gọi
//! `services::command::send_command` cho lệnh phát sinh từ Blockly action_command
//! script — không handler nào khác được publish trực tiếp cho luồng này (mirror
//! đúng nguyên tắc "safety_guard có quyền phủ quyết mọi actor" của controller-core).

use crate::AppState;
use crate::models::script::ActionCommandOutput;
use crate::services::command::CommandPayload;
use hydragrow_shared::safety::{DoseSafetyLimits, DoseSafetyViolation, check_dose};

#[derive(Debug, PartialEq)]
pub enum ActionSafetyDecision {
    Allow,
    Block(DoseSafetyViolation),
}

/// Thuần — test được không cần DB/MQTT. `hourly_history_ml`/`last_dose_at_sec` do
/// caller cung cấp (Bước sau: đọc từ `safety_config` + lịch sử system_events).
pub fn evaluate_action_safety(
    output: &ActionCommandOutput,
    limits: &DoseSafetyLimits,
    hourly_history_ml: &[(u64, f32)],
    now_sec: u64,
    last_dose_at_sec: Option<u64>,
) -> ActionSafetyDecision {
    if output.action != "dose" {
        return ActionSafetyDecision::Allow;
    }
    let Some(dose_ml) = output.dose_ml else {
        // action="dose" mà thiếu dose_ml là script lỗi hợp đồng — chặn, không đoán giá trị.
        return ActionSafetyDecision::Block(DoseSafetyViolation::ExceedsPerCycleLimit {
            requested_ml: f32::INFINITY,
            max_ml: limits.max_dose_per_cycle_ml,
        });
    };
    match check_dose(limits, hourly_history_ml, now_sec, last_dose_at_sec, dose_ml) {
        Ok(()) => ActionSafetyDecision::Allow,
        Err(violation) => ActionSafetyDecision::Block(violation),
    }
}

/// Convert `ActionCommandOutput` đã pass safety gate thành `CommandPayload` để publish
/// qua `services::command::send_command` (đã tồn tại — không viết lại logic publish/sign).
pub fn to_command_payload(output: &ActionCommandOutput) -> CommandPayload {
    CommandPayload {
        action: output.action.clone(),
        pump: output.pump.clone(),
        duration_sec: output.duration_sec,
    }
}

/// Entry point gọi từ MQTT handler (Task này chỉ định nghĩa hàm; wiring vào
/// `sensors.rs` ở Step 3 phần dưới). Trả `Err` với lý do bị chặn để caller log.
pub async fn dispatch_action_command(
    app_state: &AppState,
    device_id: &str,
    output: ActionCommandOutput,
    limits: &DoseSafetyLimits,
    hourly_history_ml: &[(u64, f32)],
    now_sec: u64,
    last_dose_at_sec: Option<u64>,
) -> Result<(), DoseSafetyViolation> {
    match evaluate_action_safety(&output, limits, hourly_history_ml, now_sec, last_dose_at_sec) {
        ActionSafetyDecision::Block(violation) => Err(violation),
        ActionSafetyDecision::Allow => {
            let payload = to_command_payload(&output);
            if let Err(e) = crate::services::command::send_command(app_state, device_id, &payload).await {
                tracing::error!(error = ?e, device_id, action = %output.action, "Lỗi publish action_command MQTT");
            }
            Ok(())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use hydragrow_shared::safety::DoseSafetyLimits;

    fn limits() -> DoseSafetyLimits {
        DoseSafetyLimits {
            max_dose_per_cycle_ml: 10.0,
            max_dose_per_hour_ml: 30.0,
            cooldown_sec: 60,
        }
    }

    #[test]
    fn non_dose_action_bypasses_safety_check() {
        let output = ActionCommandOutput {
            action: "water_on".to_string(),
            pump: None,
            dose_ml: None,
            duration_sec: Some(10),
        };
        let decision = evaluate_action_safety(&output, &limits(), &[], 1_000, None);
        assert!(matches!(decision, ActionSafetyDecision::Allow));
    }

    #[test]
    fn dose_action_within_limits_is_allowed() {
        let output = ActionCommandOutput {
            action: "dose".to_string(),
            pump: Some("ph_down".to_string()),
            dose_ml: Some(3.0),
            duration_sec: None,
        };
        let decision = evaluate_action_safety(&output, &limits(), &[], 1_000, None);
        assert!(matches!(decision, ActionSafetyDecision::Allow));
    }

    #[test]
    fn dose_action_exceeding_limit_is_blocked() {
        let output = ActionCommandOutput {
            action: "dose".to_string(),
            pump: Some("ph_down".to_string()),
            dose_ml: Some(50.0),
            duration_sec: None,
        };
        let decision = evaluate_action_safety(&output, &limits(), &[], 1_000, None);
        assert!(matches!(decision, ActionSafetyDecision::Block(_)));
    }

    #[test]
    fn dose_action_missing_dose_ml_is_blocked_defensively() {
        let output = ActionCommandOutput {
            action: "dose".to_string(),
            pump: Some("ph_down".to_string()),
            dose_ml: None,
            duration_sec: None,
        };
        let decision = evaluate_action_safety(&output, &limits(), &[], 1_000, None);
        assert!(matches!(decision, ActionSafetyDecision::Block(_)));
    }
}
