use hydragrow_shared::{ControllerConfig, SensorData};
use log::info;

use super::events::OrchestratorEvent;
use super::system_context::PeripheralState;

pub struct PeripheralController;

impl PeripheralController {
    pub fn tick_osaka(
        peripherals: &mut PeripheralState,
        is_dosing_active: bool,
        config: &ControllerConfig,
    ) -> Vec<OrchestratorEvent> {
        let mut events = Vec::new();
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
                info!(
                    "🌀 [OSAKA] Bật bơm Osaka {}% (dosing={}, misting={}, mixing={})",
                    target_pwm,
                    is_dosing_active,
                    peripherals.is_misting_active,
                    peripherals.is_scheduled_mixing_active
                );
                events.push(OrchestratorEvent::StartOsakaSoft {
                    target_pwm_percent: target_pwm,
                });
                peripherals.pump_status.osaka_pump = true;
                peripherals.pump_status.osaka_pwm = Some(target_pwm);
                peripherals.osaka_pwm = target_pwm;
                peripherals.osaka_active = true;
            } else if peripherals.osaka_pwm != target_pwm {
                events.push(OrchestratorEvent::SetOsakaPump {
                    pwm_percent: target_pwm,
                });
                peripherals.pump_status.osaka_pwm = Some(target_pwm);
                peripherals.osaka_pwm = target_pwm;
                peripherals.osaka_active = true;
            }
        } else if peripherals.pump_status.osaka_pump {
            info!("⏹️ [OSAKA] Tắt bơm Osaka — không còn nhu cầu nào.");
            events.push(OrchestratorEvent::SetOsakaPump { pwm_percent: 0 });
            peripherals.pump_status.osaka_pump = false;
            peripherals.pump_status.osaka_pwm = Some(0);
            peripherals.osaka_pwm = 0;
            peripherals.osaka_active = false;
        }
        events
    }

    pub fn tick_misting(
        peripherals: &mut PeripheralState,
        sensors: &SensorData,
        now_ms: u64,
        config: &ControllerConfig,
    ) -> Vec<OrchestratorEvent> {
        let mut events = Vec::new();
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
                peripherals.is_misting_active = false;
                peripherals.last_mist_toggle_time = now_ms;
                peripherals.pump_status.mist_valve = false;
            }
        } else if now_ms >= peripherals.last_mist_toggle_time + off_duration {
            events.push(OrchestratorEvent::SetMistValve { on: true });
            peripherals.is_misting_active = true;
            peripherals.last_mist_toggle_time = now_ms;
            peripherals.pump_status.mist_valve = true;
        }
        events
    }

    pub fn tick_scheduled_mixing(
        peripherals: &mut PeripheralState,
        now_sec: u64,
        config: &ControllerConfig,
    ) {
        if config.scheduled_mixing_interval_sec > 0 && config.scheduled_mixing_duration_sec > 0 {
            if peripherals.is_scheduled_mixing_active {
                let end_time =
                    peripherals.last_mixing_start_sec + config.scheduled_mixing_duration_sec as u64;
                if now_sec >= end_time {
                    info!(
                    "⏹️ [MIXING] Kết thúc chu kỳ sục trộn định kỳ. Đã chạy {}s. Next trong {}s.",
                    config.scheduled_mixing_duration_sec,
                    config.scheduled_mixing_interval_sec
                );
                    peripherals.is_scheduled_mixing_active = false;
                    peripherals.last_mixing_start_sec = now_sec;
                }
            } else {
                let next_trigger =
                    peripherals.last_mixing_start_sec + config.scheduled_mixing_interval_sec as u64;
                if now_sec >= next_trigger {
                    info!(
                        "▶️ [MIXING] Bắt đầu chu kỳ sục trộn định kỳ. Sẽ chạy trong {}s.",
                        config.scheduled_mixing_duration_sec
                    );
                    peripherals.is_scheduled_mixing_active = true;
                    peripherals.last_mixing_start_sec = now_sec;
                }
            }
        } else {
            peripherals.is_scheduled_mixing_active = false;
        }
    }
}

