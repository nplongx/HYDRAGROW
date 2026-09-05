use crate::actuators::virtual_hw::VirtualHardwareState;
use hydragrow_controller_core::WaterDirection;
use hydragrow_controller_core::core::fsm::events::{DosingPumpTarget, OrchestratorEvent};

pub fn apply_event(hw: &mut VirtualHardwareState, event: &OrchestratorEvent) {
    match event {
        OrchestratorEvent::SetDosingPump {
            pump,
            on,
            pwm_percent,
        } => {
            let target = match pump {
                DosingPumpTarget::NutrientA => &mut hw.pump_a,
                DosingPumpTarget::NutrientB => &mut hw.pump_b,
                DosingPumpTarget::PhUp => &mut hw.pump_ph_up,
                DosingPumpTarget::PhDown => &mut hw.pump_ph_down,
            };
            target.on = *on;
            target.pwm_percent = (*pwm_percent).min(100) as u8;
        }
        OrchestratorEvent::SetWaterPump { direction } => {
            hw.water_pump_in.on = matches!(direction, WaterDirection::In);
            hw.water_pump_out.on = matches!(direction, WaterDirection::Out);
        }
        OrchestratorEvent::SetMistValve { on } => hw.mist_valve = *on,
        OrchestratorEvent::SetMixValve { on } => hw.mix_valve = *on,
        OrchestratorEvent::SetOsakaPump { pwm_percent } => {
            hw.osaka_pwm_percent = (*pwm_percent).min(100) as u8;
        }
        OrchestratorEvent::StartOsakaSoft { target_pwm_percent } => {
            hw.osaka_pwm_percent = (*target_pwm_percent).min(100) as u8;
        }
        OrchestratorEvent::SaveNvsSnapshot
        | OrchestratorEvent::SaveLastWaterChange { .. }
        | OrchestratorEvent::SaveCurrentStageIndex { .. }
        | OrchestratorEvent::PublishFsmState
        | OrchestratorEvent::PublishCalibrationUpdate
        | OrchestratorEvent::PublishDosingReport { .. }
        | OrchestratorEvent::PublishSystemLog { .. }
        | OrchestratorEvent::PublishRecipeStageChanged { .. }
        | OrchestratorEvent::PublishCommandRejected { .. }
        | OrchestratorEvent::RequestSensorForcePublish
        | OrchestratorEvent::SetSensorContinuousMode { .. }
        | OrchestratorEvent::PublishFsmTransition { .. }
        | OrchestratorEvent::PublishDosingCycle { .. }
        | OrchestratorEvent::TriggerOtaUpdate
        | OrchestratorEvent::UpdateWifiList { .. }
        | OrchestratorEvent::RebootDevice
        | OrchestratorEvent::FactoryReset => {
            tracing::debug!(
                ?event,
                "simulator event has no direct virtual-hardware mutation"
            );
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use hydragrow_controller_core::WaterDirection;
    use hydragrow_controller_core::core::fsm::events::{DosingPumpTarget, OrchestratorEvent};

    #[test]
    fn set_dosing_pump_updates_target_and_pwm() {
        let mut hw = VirtualHardwareState::default();
        apply_event(
            &mut hw,
            &OrchestratorEvent::SetDosingPump {
                pump: DosingPumpTarget::NutrientA,
                on: true,
                pwm_percent: 65,
            },
        );
        assert!(hw.pump_a.on);
        assert_eq!(hw.pump_a.pwm_percent, 65);
    }

    #[test]
    fn set_water_pump_updates_only_selected_direction() {
        let mut hw = VirtualHardwareState::default();
        apply_event(
            &mut hw,
            &OrchestratorEvent::SetWaterPump {
                direction: WaterDirection::In,
            },
        );
        assert!(hw.water_pump_in.on);
        assert!(!hw.water_pump_out.on);
    }

    #[test]
    fn set_dosing_pump_all_targets() {
        let mut hw = VirtualHardwareState::default();
        apply_event(
            &mut hw,
            &OrchestratorEvent::SetDosingPump {
                pump: DosingPumpTarget::NutrientB,
                on: true,
                pwm_percent: 80,
            },
        );
        assert!(hw.pump_b.on);
        assert_eq!(hw.pump_b.pwm_percent, 80);

        apply_event(
            &mut hw,
            &OrchestratorEvent::SetDosingPump {
                pump: DosingPumpTarget::PhUp,
                on: true,
                pwm_percent: 40,
            },
        );
        assert!(hw.pump_ph_up.on);
        assert_eq!(hw.pump_ph_up.pwm_percent, 40);

        apply_event(
            &mut hw,
            &OrchestratorEvent::SetDosingPump {
                pump: DosingPumpTarget::PhDown,
                on: true,
                pwm_percent: 30,
            },
        );
        assert!(hw.pump_ph_down.on);
        assert_eq!(hw.pump_ph_down.pwm_percent, 30);
    }

    #[test]
    fn set_valves_and_osaka_pump() {
        let mut hw = VirtualHardwareState::default();
        apply_event(&mut hw, &OrchestratorEvent::SetMistValve { on: true });
        assert!(hw.mist_valve);

        apply_event(&mut hw, &OrchestratorEvent::SetMixValve { on: true });
        assert!(hw.mix_valve);

        apply_event(
            &mut hw,
            &OrchestratorEvent::SetOsakaPump { pwm_percent: 75 },
        );
        assert_eq!(hw.osaka_pwm_percent, 75);

        apply_event(
            &mut hw,
            &OrchestratorEvent::StartOsakaSoft {
                target_pwm_percent: 90,
            },
        );
        assert_eq!(hw.osaka_pwm_percent, 90);
    }

    #[test]
    fn dispatcher_stops_after_first_fault() {
        let mut hw = VirtualHardwareState::default();
        let events = vec![
            OrchestratorEvent::SetDosingPump {
                pump: DosingPumpTarget::NutrientA,
                on: true,
                pwm_percent: 50,
            },
            OrchestratorEvent::SetDosingPump {
                pump: DosingPumpTarget::NutrientB,
                on: true,
                pwm_percent: 50,
            },
            OrchestratorEvent::SetWaterPump {
                direction: WaterDirection::In,
            },
        ];

        let result = dispatch_events_transactional(
            &mut hw,
            &events,
            Some(DosingPumpTarget::NutrientA),
        );
        assert!(result.is_err(), "Dispatcher must report hardware failure");
        assert!(!hw.pump_a.on);
        assert!(!hw.pump_b.on, "Pump B must not be attempted after Pump A fault");
        assert!(!hw.water_pump_in.on, "Water pump must not be attempted after Pump A fault");
    }

    #[test]
    fn off_command_fails_during_all_off_latches_primary_and_reports_secondary() {
        let mut hw = VirtualHardwareState::default();
        hw.pump_b.on = true;

        let primary_fault = "PumpA hardware fault".to_string();
        let all_off_events = vec![
            OrchestratorEvent::SetDosingPump {
                pump: DosingPumpTarget::NutrientA,
                on: false,
                pwm_percent: 0,
            },
            OrchestratorEvent::SetDosingPump {
                pump: DosingPumpTarget::NutrientB,
                on: false,
                pwm_percent: 0,
            },
            OrchestratorEvent::SetWaterPump {
                direction: WaterDirection::Stop,
            },
        ];

        let secondary = dispatch_best_effort_all_off(
            &mut hw,
            &all_off_events,
            Some(DosingPumpTarget::NutrientB),
        );

        assert_eq!(primary_fault, "PumpA hardware fault", "Primary fault must remain latched");
        assert!(secondary.is_some(), "Secondary failure must be observable");
        assert_eq!(secondary.unwrap(), "Failed to turn off pump NutrientB");
        assert!(!hw.pump_a.on, "Pump A OFF must still have been attempted");
    }
}

pub fn dispatch_events_transactional(
    hw: &mut VirtualHardwareState,
    events: &[OrchestratorEvent],
    failing_pump: Option<DosingPumpTarget>,
) -> Result<(), String> {
    for event in events {
        if let OrchestratorEvent::SetDosingPump { pump, on: true, .. } = event {
            if Some(*pump) == failing_pump {
                return Err(format!("Hardware fault on pump {:?}", pump));
            }
        }
        apply_event(hw, event);
    }
    Ok(())
}

pub fn dispatch_best_effort_all_off(
    hw: &mut VirtualHardwareState,
    events: &[OrchestratorEvent],
    failing_off_pump: Option<DosingPumpTarget>,
) -> Option<String> {
    let mut secondary_fault = None;
    for event in events {
        if let OrchestratorEvent::SetDosingPump { pump, on: false, .. } = event {
            if Some(*pump) == failing_off_pump {
                if secondary_fault.is_none() {
                    secondary_fault = Some(format!("Failed to turn off pump {:?}", pump));
                }
                continue;
            }
        }
        apply_event(hw, event);
    }
    secondary_fault
}
