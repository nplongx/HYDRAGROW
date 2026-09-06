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

        let result =
            dispatch_events_transactional(&mut hw, &events, Some(DosingPumpTarget::NutrientA));
        assert!(result.is_err(), "Dispatcher must report hardware failure");
        assert!(!hw.pump_a.on);
        assert!(
            !hw.pump_b.on,
            "Pump B must not be attempted after Pump A fault"
        );
        assert!(
            !hw.water_pump_in.on,
            "Water pump must not be attempted after Pump A fault"
        );
    }

    #[test]
    fn terminal_reboot_continues_dispatch_even_after_actuator_fault() {
        let mut hw = VirtualHardwareState::default();
        hw.water_pump_in.on = true;

        let events = vec![
            OrchestratorEvent::SetDosingPump {
                pump: DosingPumpTarget::NutrientA,
                on: true,
                pwm_percent: 50,
            },
            OrchestratorEvent::SetWaterPump {
                direction: WaterDirection::Stop,
            },
            OrchestratorEvent::RebootDevice,
        ];

        let result =
            dispatch_events_transactional(&mut hw, &events, Some(DosingPumpTarget::NutrientA));
        assert!(
            result.is_err(),
            "Dispatcher must still report hardware failure"
        );
        assert!(
            !hw.water_pump_in.on,
            "Water pump stop must execute despite pump A fault when terminal reboot is present"
        );
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

        assert_eq!(
            primary_fault, "PumpA hardware fault",
            "Primary fault must remain latched"
        );
        assert!(secondary.is_some(), "Secondary failure must be observable");
        assert_eq!(secondary.unwrap(), "Failed to turn off pump NutrientB");
        assert!(!hw.pump_a.on, "Pump A OFF must still have been attempted");
    }

    #[test]
    fn emergency_stop_water_in_off_fails_still_attempts_water_out_off() {
        let mut hw = VirtualHardwareState::default();
        hw.water_pump_in.on = true;
        hw.water_pump_out.on = true;

        let mut driver = VirtualWaterPumpDriver::new();
        driver.cached_direction = Some(WaterDirection::In);

        let result = driver.set_water_pump(WaterDirection::Stop, &mut hw, true);

        assert!(result.is_err(), "Must report failure of Water IN OFF");
        assert!(
            hw.water_pump_in.on,
            "Water IN remains on due to simulated write failure"
        );
        assert!(
            !hw.water_pump_out.on,
            "Water OUT OFF must still be attempted and shut off!"
        );
        assert_eq!(
            driver.cached_direction, None,
            "Direction cache must be invalidated on error"
        );
    }

    #[test]
    fn cached_direction_recovery_reasserts_hardware_write() {
        let mut hw = VirtualHardwareState::default();
        let mut driver = VirtualWaterPumpDriver::new();

        let res1 = driver.set_water_pump(WaterDirection::In, &mut hw, false);
        assert!(res1.is_ok());
        assert_eq!(driver.physical_writes, 1);
        assert!(hw.water_pump_in.on);

        // Simulate external shutdown / reset
        hw.water_pump_in.on = false;

        // Invalidate cache on fault/recovery
        driver.invalidate_cache();
        assert_eq!(driver.cached_direction, None);

        // Next IN request must reassert real write
        let res2 = driver.set_water_pump(WaterDirection::In, &mut hw, false);
        assert!(res2.is_ok());
        assert_eq!(
            driver.physical_writes, 2,
            "Must issue real physical write after cache invalidation"
        );
        assert!(hw.water_pump_in.on, "Hardware must be reasserted to ON");
        assert_eq!(driver.cached_direction, Some(WaterDirection::In));
    }
}

#[derive(Debug, Default)]
pub struct VirtualWaterPumpDriver {
    pub cached_direction: Option<WaterDirection>,
    pub physical_writes: usize,
}

impl VirtualWaterPumpDriver {
    pub fn new() -> Self {
        Self {
            cached_direction: Some(WaterDirection::Stop),
            physical_writes: 0,
        }
    }

    pub fn set_water_pump(
        &mut self,
        direction: WaterDirection,
        hw: &mut VirtualHardwareState,
        fail_in_off: bool,
    ) -> Result<(), String> {
        if self.cached_direction == Some(direction) {
            return Ok(());
        }

        match direction {
            WaterDirection::In => {
                self.physical_writes += 1;
                hw.water_pump_in.on = true;
                hw.water_pump_out.on = false;
                self.cached_direction = Some(WaterDirection::In);
                Ok(())
            }
            WaterDirection::Out => {
                self.physical_writes += 1;
                hw.water_pump_in.on = false;
                hw.water_pump_out.on = true;
                self.cached_direction = Some(WaterDirection::Out);
                Ok(())
            }
            WaterDirection::Stop => {
                self.physical_writes += 1;
                let mut err_in = None;
                if fail_in_off {
                    err_in = Some("PCF8574 WaterPumpIn OFF failed");
                } else {
                    hw.water_pump_in.on = false;
                }

                // Water OUT OFF MUST still be attempted even if IN OFF failed!
                hw.water_pump_out.on = false;

                if let Some(e) = err_in {
                    self.cached_direction = None; // Invalidate cache on error
                    Err(e.to_string())
                } else {
                    self.cached_direction = Some(WaterDirection::Stop);
                    Ok(())
                }
            }
        }
    }

    pub fn invalidate_cache(&mut self) {
        self.cached_direction = None;
    }
}

pub fn dispatch_events_transactional(
    hw: &mut VirtualHardwareState,
    events: &[OrchestratorEvent],
    failing_pump: Option<DosingPumpTarget>,
) -> Result<(), String> {
    let has_terminal = events.iter().any(|e| {
        matches!(
            e,
            OrchestratorEvent::RebootDevice | OrchestratorEvent::FactoryReset
        )
    });
    let mut first_fault = None;

    for event in events {
        if let OrchestratorEvent::SetDosingPump { pump, .. } = event
            && Some(*pump) == failing_pump
        {
            if first_fault.is_none() {
                first_fault = Some(format!("Hardware fault on pump {:?}", pump));
            }
            if !has_terminal {
                return Err(format!("Hardware fault on pump {:?}", pump));
            }
            continue;
        }
        apply_event(hw, event);
    }

    if let Some(err) = first_fault {
        Err(err)
    } else {
        Ok(())
    }
}

pub fn dispatch_best_effort_all_off(
    hw: &mut VirtualHardwareState,
    events: &[OrchestratorEvent],
    failing_off_pump: Option<DosingPumpTarget>,
) -> Option<String> {
    let mut secondary_fault = None;
    for event in events {
        if let OrchestratorEvent::SetDosingPump {
            pump, on: false, ..
        } = event
            && Some(*pump) == failing_off_pump
        {
            if secondary_fault.is_none() {
                secondary_fault = Some(format!("Failed to turn off pump {:?}", pump));
            }
            continue;
        }
        apply_event(hw, event);
    }
    secondary_fault
}
