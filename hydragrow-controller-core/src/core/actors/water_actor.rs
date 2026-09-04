use hydragrow_shared::{
    ControllerConfig, SensorData,
    log::{LogCategory, LogLevel, SystemLogEvent, UnifiedSystemLog, WaterMetadata},
};

use crate::{WaterDirection, core::fsm::OrchestratorEvent};

#[derive(Debug, Clone)]
pub enum WaterSubState {
    Idle,
    Starting,
    Filling { job: WaterJob },
    Draining { job: WaterJob },
}

#[derive(Debug, Clone)]
pub struct WaterJob {
    pub trigger: String,
    pub target_level: f32,
    pub start_level: f32,
    pub start_ms: u64,
}

#[must_use]
pub enum WaterEvent {
    Pending,
    Done { success: bool, duration_sec: u64 },
}

pub struct WaterActor {
    pub sub_state: WaterSubState,
    pub retry_refill: u32,
    pub device_id: String,
}

impl WaterActor {
    pub fn new(device_id: impl Into<String>) -> Self {
        Self {
            sub_state: WaterSubState::Idle,
            retry_refill: 0,
            device_id: device_id.into(),
        }
    }

    /// Reset hoàn toàn trạng thái WaterActor về Idle và xoá số lần retry.
    pub fn reset(&mut self) {
        self.sub_state = WaterSubState::Idle;
        self.retry_refill = 0;
    }

    /// Bắt đầu cấp nước, trả về log Info (nếu muốn gửi ngay).
    pub fn start_fill(
        &mut self,
        now_ms: u64,
        target: f32,
        sensors: &SensorData,
        trigger: &str,
    ) -> Option<UnifiedSystemLog> {
        self.sub_state = WaterSubState::Filling {
            job: WaterJob {
                trigger: trigger.into(),
                target_level: target,
                start_level: sensors.water_level,
                start_ms: now_ms,
            },
        };
        self.retry_refill = 0;

        // Log bắt đầu cấp nước
        let event = SystemLogEvent::WaterEvent(WaterMetadata {
            source: "water_pump".into(),
            trigger: trigger.into(),
            level_before: sensors.water_level,
            level_after: sensors.water_level, // lúc bắt đầu chưa thay đổi
            target_level: target,
            duration_sec: 0, // sẽ cập nhật khi kết thúc
            success: false,  // đang tiến hành
            cycle_id: None,
            retry_count: Some(self.retry_refill),
        });

        Some(UnifiedSystemLog::info(
            self.device_id.clone(),
            LogCategory::Water,
            "Bắt đầu cấp nước",
            event,
            now_ms,
        ))
    }

    /// Bắt đầu xả nước, trả về log Info.
    pub fn start_drain(
        &mut self,
        now_ms: u64,
        target: f32,
        sensors: &SensorData,
        trigger: &str,
    ) -> Option<UnifiedSystemLog> {
        self.sub_state = WaterSubState::Draining {
            job: WaterJob {
                trigger: trigger.into(),
                target_level: target,
                start_level: sensors.water_level,
                start_ms: now_ms,
            },
        };

        let event = SystemLogEvent::WaterEvent(WaterMetadata {
            source: "water_pump".into(),
            trigger: trigger.into(),
            level_before: sensors.water_level,
            level_after: sensors.water_level,
            target_level: target,
            duration_sec: 0,
            success: false,
            cycle_id: None,
            retry_count: None, // xả không có retry
        });

        Some(UnifiedSystemLog::info(
            self.device_id.clone(),
            LogCategory::Water,
            "Bắt đầu xả nước",
            event,
            now_ms,
        ))
    }

    /// Hàm tick chính, nay trả về thêm Vec<UnifiedSystemLog> cho các sự kiện kết thúc.
    pub fn tick(
        &mut self,
        now_ms: u64,
        sensors: &SensorData,
        config: &ControllerConfig,
    ) -> (WaterEvent, Vec<OrchestratorEvent>, Vec<UnifiedSystemLog>) {
        match self.sub_state.clone() {
            WaterSubState::Starting => (
                WaterEvent::Pending,
                vec![OrchestratorEvent::SetWaterPump {
                    direction: WaterDirection::Stop,
                }],
                vec![],
            ),
            WaterSubState::Filling { job } => {
                let elapsed_ms = now_ms.saturating_sub(job.start_ms);
                let timeout_ms = (config.max_refill_duration_sec as u64).saturating_mul(1000);
                let reached = sensors.water_level >= job.target_level;
                let timeout = elapsed_ms >= timeout_ms;

                if reached || timeout {
                    let elapsed = elapsed_ms / 1000;
                    let level_after = sensors.water_level;
                    let event = SystemLogEvent::WaterEvent(WaterMetadata {
                        source: "water_pump".into(),
                        level_before: job.start_level,
                        trigger: job.trigger.clone(),
                        level_after,
                        target_level: job.target_level,
                        duration_sec: elapsed,
                        success: reached,
                        cycle_id: None,
                        retry_count: Some(self.retry_refill),
                    });

                    let level = if reached {
                        LogLevel::Success
                    } else {
                        LogLevel::Warning
                    };
                    let title = if reached {
                        "Cấp nước hoàn tất"
                    } else {
                        "Cấp nước timeout"
                    };

                    let log = UnifiedSystemLog {
                        device_id: self.device_id.clone(),
                        level,
                        category: LogCategory::Water,
                        title: title.into(),
                        event,
                        timestamp_ms: now_ms,
                    };

                    self.sub_state = WaterSubState::Idle;
                    return (
                        WaterEvent::Done {
                            success: reached,
                            duration_sec: elapsed,
                        },
                        vec![OrchestratorEvent::SetWaterPump {
                            direction: WaterDirection::Stop,
                        }],
                        vec![log],
                    );
                }

                (
                    WaterEvent::Pending,
                    vec![OrchestratorEvent::SetWaterPump {
                        direction: WaterDirection::In,
                    }],
                    vec![],
                )
            }
            WaterSubState::Draining { job } => {
                let elapsed_ms = now_ms.saturating_sub(job.start_ms);
                let timeout_ms = (config.max_drain_duration_sec as u64).saturating_mul(1000);
                let reached = sensors.water_level <= job.target_level;
                let timeout = elapsed_ms >= timeout_ms;

                if reached || timeout {
                    let elapsed = elapsed_ms / 1000;
                    let level_after = sensors.water_level;
                    let event = SystemLogEvent::WaterEvent(WaterMetadata {
                        source: "water_pump".into(),
                        trigger: job.trigger.clone(),
                        level_before: job.start_level,
                        level_after,
                        target_level: job.target_level,
                        duration_sec: elapsed,
                        success: reached,
                        cycle_id: None,
                        retry_count: None,
                    });

                    let level = if reached {
                        LogLevel::Success
                    } else {
                        LogLevel::Warning
                    };
                    let title = if reached {
                        "Xả nước hoàn tất"
                    } else {
                        "Xả nước timeout"
                    };

                    let log = UnifiedSystemLog {
                        device_id: self.device_id.clone(),
                        level,
                        category: LogCategory::Water,
                        title: title.into(),
                        event,
                        timestamp_ms: now_ms,
                    };

                    self.sub_state = WaterSubState::Idle;
                    return (
                        WaterEvent::Done {
                            success: reached,
                            duration_sec: elapsed,
                        },
                        vec![OrchestratorEvent::SetWaterPump {
                            direction: WaterDirection::Stop,
                        }],
                        vec![log],
                    );
                }

                (
                    WaterEvent::Pending,
                    vec![OrchestratorEvent::SetWaterPump {
                        direction: WaterDirection::Out,
                    }],
                    vec![],
                )
            }
            WaterSubState::Idle => (
                WaterEvent::Pending,
                vec![OrchestratorEvent::SetWaterPump {
                    direction: WaterDirection::Stop,
                }],
                vec![],
            ),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn dummy_sensors(level: f32) -> SensorData {
        SensorData {
            device_id: "test_device".to_string(),
            ec: 1.5,
            ph: 6.0,
            temp: 25.0,
            water_level: level,
            pump_status: Default::default(),
            time: "2026-09-04T00:00:00Z".to_string(),
            controller_received_ms: Some(1_000_000),
            rssi: Some(-60),
            free_heap: Some(100_000),
            uptime: Some(1000),
            err_water: None,
            err_temp: None,
            err_ec: None,
            err_ph: None,
            is_continuous: None,
            ph_voltage_mv: None,
        }
    }

    #[test]
    fn water_actor_timeout_at_exact_boundary() {
        let mut actor = WaterActor::new("test_actor");
        let sensors = dummy_sensors(10.0);
        let mut config = ControllerConfig::default();
        config.max_refill_duration_sec = 2; // 2000ms

        actor.start_fill(1_000, 50.0, &sensors, "test");

        // At 2999ms (elapsed 1999ms) -> not timed out
        let (event_2999, _, _) = actor.tick(2_999, &sensors, &config);
        assert!(matches!(event_2999, WaterEvent::Pending));

        // At 3000ms (elapsed 2000ms >= timeout 2000ms) -> timed out!
        let (event_3000, _, _) = actor.tick(3_000, &sensors, &config);
        match event_3000 {
            WaterEvent::Done {
                success,
                duration_sec,
            } => {
                assert!(!success, "Should fail on timeout");
                assert_eq!(duration_sec, 2);
            }
            _ => panic!("Expected WaterEvent::Done on exact timeout boundary"),
        }
    }
}
