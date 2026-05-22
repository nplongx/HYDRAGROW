use hydragrow_shared::{ControllerConfig, SensorData};
use log::info;

use super::events::OrchestratorEvent;
use super::system_context::PeripheralState;

pub struct PeripheralController;

// src/fsm/peripheral.rs
use crate::fsm::tick_result::PeripheralDelta;

impl PeripheralController {
    pub fn tick_osaka(
        peripherals: &PeripheralState, // <-- không còn mut
        is_dosing_active: bool,
        config: &ControllerConfig,
    ) -> (PeripheralDelta, Vec<OrchestratorEvent>) {
        let mut events = Vec::new();
        let mut delta = PeripheralDelta::default();

        let needs_osaka = is_dosing_active
            || peripherals.is_misting_active
            || peripherals.is_scheduled_mixing_active;

        if needs_osaka {
            let target_pwm = if peripherals.is_misting_active {
                config.osaka_misting_pwm_percent as u32
            } else {
                config.osaka_mixing_pwm_percent as u32
            };

            if !peripherals.pump_status.osaka_pump {
                info!("🌀 [OSAKA] Bật bơm Osaka {}%", target_pwm);
                events.push(OrchestratorEvent::StartOsakaSoft {
                    target_pwm_percent: target_pwm,
                });
                delta.osaka_pump = Some(true);
                delta.osaka_pwm = Some(target_pwm);
            } else if peripherals.osaka_pwm != target_pwm {
                events.push(OrchestratorEvent::SetOsakaPump {
                    pwm_percent: target_pwm,
                });
                delta.osaka_pwm = Some(target_pwm);
            }
        } else if peripherals.pump_status.osaka_pump {
            info!("⏹️ [OSAKA] Tắt bơm Osaka.");
            events.push(OrchestratorEvent::SetOsakaPump { pwm_percent: 0 });
            delta.osaka_pump = Some(false);
            delta.osaka_pwm = Some(0);
        }

        (delta, events)
    }

    pub fn tick_misting(
        peripherals: &PeripheralState, // <-- không còn mut
        sensors: &SensorData,
        now_ms: u64,
        config: &ControllerConfig,
    ) -> (PeripheralDelta, Vec<OrchestratorEvent>) {
        let mut events = Vec::new();
        let mut delta = PeripheralDelta::default();

        let is_hot = config.enable_temp_sensor && sensors.temp >= config.misting_temp_threshold;
        let on_duration = if is_hot {
            config.high_temp_misting_on_duration_ms as u64
        } else {
            config.misting_on_duration_ms as u64
        };
        let off_duration = if is_hot {
            config.high_temp_misting_off_duration_ms as u64
        } else {
            config.misting_off_duration_ms as u64
        };

        if peripherals.is_misting_active {
            if now_ms >= peripherals.last_mist_toggle_time + on_duration {
                events.push(OrchestratorEvent::SetMistValve { on: false });
                delta.is_misting_active = Some(false);
                delta.last_mist_toggle_time = Some(now_ms);
                delta.mist_valve = Some(false);
            }
        } else if now_ms >= peripherals.last_mist_toggle_time + off_duration {
            events.push(OrchestratorEvent::SetMistValve { on: true });
            delta.is_misting_active = Some(true);
            delta.last_mist_toggle_time = Some(now_ms);
            delta.mist_valve = Some(true);
        }

        (delta, events)
    }

    pub fn tick_scheduled_mixing(
        peripherals: &PeripheralState, // <-- không còn mut
        now_sec: u64,
        config: &ControllerConfig,
    ) -> PeripheralDelta {
        let mut delta = PeripheralDelta::default();

        if config.scheduled_mixing_interval_sec > 0 && config.scheduled_mixing_duration_sec > 0 {
            if peripherals.is_scheduled_mixing_active {
                let end_time =
                    peripherals.last_mixing_start_sec + config.scheduled_mixing_duration_sec as u64;
                if now_sec >= end_time {
                    info!("⏹️ [MIXING] Kết thúc chu kỳ sục trộn định kỳ.");
                    delta.is_scheduled_mixing_active = Some(false);
                    delta.last_mixing_start_sec = Some(now_sec);
                }
            } else {
                let next_trigger =
                    peripherals.last_mixing_start_sec + config.scheduled_mixing_interval_sec as u64;
                if now_sec >= next_trigger {
                    info!("▶️ [MIXING] Bắt đầu chu kỳ sục trộn định kỳ.");
                    delta.is_scheduled_mixing_active = Some(true);
                    delta.last_mixing_start_sec = Some(now_sec);
                }
            }
        } else {
            delta.is_scheduled_mixing_active = Some(false);
        }

        delta
    }
}
