//! Dispatch action_command script output ra MQTT — LUÔN đi qua gate an toàn của
//! `hydragrow_shared::safety` trước. Đây là nơi DUY NHẤT được phép gọi
//! `services::command::send_command` cho lệnh phát sinh từ Blockly action_command
//! script — không handler nào khác được publish trực tiếp cho luồng này (mirror
//! đúng nguyên tắc "safety_guard có quyền phủ quyết mọi actor" của controller-core).

use crate::AppState;
use crate::models::config::DosingCalibration;
use crate::models::script::ActionCommandOutput;
use hydragrow_shared::dosing::{
    capacity_ml_per_sec_for_pump, ml_to_duration_sec, normalize_dosing_pump_name,
};
use hydragrow_shared::safety::{DoseSafetyLimits, DoseSafetyViolation, check_dose};
use hydragrow_shared::{MqttCommandOut, MqttCommandParams};

#[derive(Debug, PartialEq)]
pub enum ActionSafetyDecision {
    Allow { duration_sec: Option<u64> },
    Block(DoseSafetyViolation),
}

#[derive(Debug)]
pub enum ActionDispatchError {
    Safety(DoseSafetyViolation),
    UnknownPump(String),
    UnknownAction(String),
    Mqtt(anyhow::Error),
}

/// Thuần — test được không cần DB/MQTT. Thứ tự bắt buộc: check_dose (ml, không
/// phụ thuộc calibration) TRƯỚC, tra calibration để tính duration_sec SAU — để
/// một liều vượt ngưỡng bị chặn ngay cả khi calibration bị thiếu/lỗi.
pub fn evaluate_action_safety(
    output: &ActionCommandOutput,
    limits: &DoseSafetyLimits,
    hourly_history_ml: &[(u64, f32)],
    now_sec: u64,
    last_dose_at_sec: Option<u64>,
    calibration: Option<&DosingCalibration>,
) -> Result<ActionSafetyDecision, ActionDispatchError> {
    if output.action == "water_on" || output.action == "water_off" {
        const VALID_WATER_PUMPS: [&str; 4] =
            ["WATER_PUMP_IN", "WATER_PUMP_OUT", "MIST_VALVE", "OSAKA_PUMP"];
        let pump = output
            .pump
            .as_deref()
            .ok_or_else(|| ActionDispatchError::UnknownPump("pump là bắt buộc cho water_on/water_off".to_string()))?;
        if !VALID_WATER_PUMPS.contains(&pump) {
            return Err(ActionDispatchError::UnknownPump(pump.to_string()));
        }
        return Ok(ActionSafetyDecision::Allow { duration_sec: output.duration_sec });
    }

    if output.action != "dose" {
        return Ok(ActionSafetyDecision::Allow {
            duration_sec: output.duration_sec,
        });
    }

    let (Some(dose_ml), Some(pwm), Some(pump)) =
        (output.dose_ml, output.pwm, output.pump.as_deref())
    else {
        return Ok(ActionSafetyDecision::Block(
            DoseSafetyViolation::ExceedsPerCycleLimit {
                requested_ml: f32::INFINITY,
                max_ml: limits.max_dose_per_cycle_ml,
            },
        ));
    };

    if let Err(violation) = check_dose(
        limits,
        hourly_history_ml,
        now_sec,
        last_dose_at_sec,
        dose_ml,
    ) {
        return Ok(ActionSafetyDecision::Block(violation));
    }

    let Some(normalized_pump) = normalize_dosing_pump_name(pump) else {
        return Err(ActionDispatchError::UnknownPump(pump.to_string()));
    };
    let Some(calibration) = calibration else {
        return Err(ActionDispatchError::UnknownPump(format!(
            "Không có dosing_calibration cho pump {normalized_pump}"
        )));
    };
    let capacity = capacity_ml_per_sec_for_pump(
        calibration.pump_a_capacity_ml_per_sec,
        calibration.pump_b_capacity_ml_per_sec,
        calibration.pump_ph_up_capacity_ml_per_sec,
        calibration.pump_ph_down_capacity_ml_per_sec,
        normalized_pump,
    );
    let Some(duration_sec) = ml_to_duration_sec(capacity, pwm, dose_ml) else {
        return Err(ActionDispatchError::UnknownPump(format!(
            "Không tính được duration_sec cho pump {normalized_pump} (capacity={capacity}, pwm={pwm})"
        )));
    };

    Ok(ActionSafetyDecision::Allow {
        duration_sec: Some(duration_sec),
    })
}

/// Entry point gọi từ MQTT handler.
#[allow(clippy::too_many_arguments)]
pub async fn dispatch_action_command(
    app_state: &AppState,
    device_id: &str,
    output: ActionCommandOutput,
    limits: &DoseSafetyLimits,
    hourly_history_ml: &[(u64, f32)],
    now_sec: u64,
    last_dose_at_sec: Option<u64>,
    calibration: Option<&DosingCalibration>,
) -> Result<(), ActionDispatchError> {
    if output.action == "emergency_stop" {
        return crate::services::command::trigger_emergency_stop(app_state, device_id)
            .await
            .map_err(ActionDispatchError::Mqtt);
    }

    let decision = evaluate_action_safety(
        &output,
        limits,
        hourly_history_ml,
        now_sec,
        last_dose_at_sec,
        calibration,
    )?;
    let duration_sec = match decision {
        ActionSafetyDecision::Block(violation) => {
            return Err(ActionDispatchError::Safety(violation));
        }
        ActionSafetyDecision::Allow { duration_sec } => duration_sec,
    };

    // Đúng 3 giá trị mqtt_action đã xác nhận có thật trong api/control.rs::control_pump
    // (nhánh match req_data.action.as_str()) — không phát minh action string mới.
    let mqtt_action = match output.action.as_str() {
        "dose" => "set_pwm",
        "water_on" => "pump_on",
        "water_off" => "pump_off",
        other => return Err(ActionDispatchError::UnknownAction(other.to_string())),
    };

    let command = MqttCommandOut {
        target: "all".to_string(), // = giá trị mặc định của resolve_control_target(None)
        action: mqtt_action.to_string(),
        params: Some(MqttCommandParams {
            pump_id: output.pump.clone(),
            duration_sec,
            pwm: if output.action == "dose" {
                output.pwm
            } else {
                None
            },
            state: None,
            ota_url: None,
            candidates: None,
        }),
        ts: None,
        nonce: None,
        signature: None,
    };

    crate::api::mqtt_utils::publish_command(app_state, device_id, &command)
        .await
        .map_err(ActionDispatchError::Mqtt)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn limits() -> DoseSafetyLimits {
        DoseSafetyLimits {
            max_dose_per_cycle_ml: 10.0,
            max_dose_per_hour_ml: 30.0,
            cooldown_sec: 60,
        }
    }

    fn calibration() -> crate::models::config::DosingCalibration {
        crate::models::config::DosingCalibration {
            device_id: "d1".to_string(),
            pump_a_capacity_ml_per_sec: 1.0,
            pump_b_capacity_ml_per_sec: 1.0,
            pump_ph_up_capacity_ml_per_sec: 2.0,
            pump_ph_down_capacity_ml_per_sec: 2.0,
            ..Default::default()
        }
    }

    #[test]
    fn non_dose_action_bypasses_safety_and_calibration() {
        let output = ActionCommandOutput {
            action: "water_on".into(),
            pump: Some("WATER_PUMP_IN".into()),
            dose_ml: None,
            pwm: None,
            duration_sec: Some(10),
        };
        let decision = evaluate_action_safety(&output, &limits(), &[], 1_000, None, None).unwrap();
        assert_eq!(
            decision,
            ActionSafetyDecision::Allow {
                duration_sec: Some(10)
            }
        );
    }

    #[test]
    fn dose_within_limits_computes_duration_from_calibration() {
        let output = ActionCommandOutput {
            action: "dose".into(),
            pump: Some("PH_DOWN".into()),
            dose_ml: Some(4.0),
            pwm: Some(100),
            duration_sec: None,
        };
        let decision =
            evaluate_action_safety(&output, &limits(), &[], 1_000, None, Some(&calibration()))
                .unwrap();
        // capacity=2.0ml/s tại pwm=100% → 4.0ml / 2.0ml/s = 2s
        assert_eq!(
            decision,
            ActionSafetyDecision::Allow {
                duration_sec: Some(2)
            }
        );
    }

    #[test]
    fn dose_exceeding_per_cycle_limit_is_blocked_before_calibration_lookup() {
        let output = ActionCommandOutput {
            action: "dose".into(),
            pump: Some("PH_DOWN".into()),
            dose_ml: Some(50.0),
            pwm: Some(100),
            duration_sec: None,
        };
        // calibration=None: nếu code lỡ tính calibration TRƯỚC safety check, test này sẽ panic
        // thay vì trả Block — thứ tự đúng là safety-check ml trước, calibration sau.
        let decision = evaluate_action_safety(&output, &limits(), &[], 1_000, None, None).unwrap();
        assert!(matches!(decision, ActionSafetyDecision::Block(_)));
    }

    #[test]
    fn dose_missing_pwm_is_blocked_defensively() {
        let output = ActionCommandOutput {
            action: "dose".into(),
            pump: Some("PH_DOWN".into()),
            dose_ml: Some(4.0),
            pwm: None,
            duration_sec: None,
        };
        let decision =
            evaluate_action_safety(&output, &limits(), &[], 1_000, None, Some(&calibration()))
                .unwrap();
        assert!(matches!(decision, ActionSafetyDecision::Block(_)));
    }

    #[test]
    fn dose_with_unrecognized_pump_name_errors_instead_of_silently_using_zero_capacity() {
        let output = ActionCommandOutput {
            action: "dose".into(),
            pump: Some("NOT_A_PUMP".into()),
            dose_ml: Some(4.0),
            pwm: Some(100),
            duration_sec: None,
        };
        let result =
            evaluate_action_safety(&output, &limits(), &[], 1_000, None, Some(&calibration()));
        assert!(matches!(result, Err(ActionDispatchError::UnknownPump(_))));
    }

    #[test]
    fn dose_without_calibration_row_errors_instead_of_dosing_blind() {
        let output = ActionCommandOutput {
            action: "dose".into(),
            pump: Some("PH_DOWN".into()),
            dose_ml: Some(4.0),
            pwm: Some(100),
            duration_sec: None,
        };
        let result = evaluate_action_safety(&output, &limits(), &[], 1_000, None, None);
        assert!(matches!(result, Err(ActionDispatchError::UnknownPump(_))));
    }

    #[test]
    fn water_action_with_unrecognized_pump_is_blocked() {
        let output = ActionCommandOutput {
            action: "water_on".into(),
            pump: Some("NOT_A_REAL_PUMP".into()),
            dose_ml: None,
            pwm: None,
            duration_sec: Some(10),
        };
        let result = evaluate_action_safety(&output, &limits(), &[], 1_000, None, None);
        assert!(matches!(result, Err(ActionDispatchError::UnknownPump(_))));
    }

    #[test]
    fn water_action_without_pump_is_blocked() {
        let output = ActionCommandOutput {
            action: "water_off".into(),
            pump: None,
            dose_ml: None,
            pwm: None,
            duration_sec: None,
        };
        let result = evaluate_action_safety(&output, &limits(), &[], 1_000, None, None);
        assert!(matches!(result, Err(ActionDispatchError::UnknownPump(_))));
    }

    #[test]
    fn water_action_with_a_recognized_pump_still_allowed() {
        // Khoá lại hành vi hợp lệ cũ (test Phase 1 `non_dose_action_bypasses_safety_and_calibration`
        // dùng đúng pump này) — không được để việc thêm validate làm hỏng ca hợp lệ.
        let output = ActionCommandOutput {
            action: "water_on".into(),
            pump: Some("WATER_PUMP_IN".into()),
            dose_ml: None,
            pwm: None,
            duration_sec: Some(10),
        };
        let decision = evaluate_action_safety(&output, &limits(), &[], 1_000, None, None).unwrap();
        assert_eq!(decision, ActionSafetyDecision::Allow { duration_sec: Some(10) });
    }
}
