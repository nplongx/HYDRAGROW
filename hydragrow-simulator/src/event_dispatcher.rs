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
}
