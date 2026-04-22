use esp_idf_svc::nvs::{EspDefaultNvs, EspDefaultNvsPartition, EspNvs};
use log::{error, info, warn};
use std::collections::HashMap;
use std::sync::mpsc::{Receiver, Sender};
use std::sync::{Arc, RwLock};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

// Import thư viện cho Cron
use chrono::{Local, TimeZone};
use cron::Schedule;
use std::str::FromStr;

use crate::config::{ControlMode, DeviceConfig, SharedConfig};
use crate::mqtt::{MqttCommandPayload, PumpStatus, SensorData};
use crate::pump::{PumpController, PumpType, WaterDirection};

pub type SharedSensorData = Arc<RwLock<SensorData>>;

#[derive(Debug, Clone, PartialEq)]
pub enum PendingDose {
    EC {
        dose_ml: f32,
        target_ec: f32,
        pwm_percent: u32,
    },
    PH {
        is_up: bool,
        dose_ml: f32,
        duration_ms: u64,
        target_ph: f32,
        pwm_percent: u32,
    },
    ScheduledDose {
        dose_a_ml: f32,
        dose_b_ml: f32,
        pwm_percent: u32,
    },
}

#[derive(Debug, Clone, PartialEq)]
pub enum SystemState {
    Monitoring,
    EmergencyStop(String),
    SystemFault(String),
    WaterRefilling {
        target_level: f32,
        start_time: u64,
    },
    WaterDraining {
        target_level: f32,
        start_time: u64,
    },
    StartingOsakaPump {
        finish_time: u64,
        pending_action: PendingDose,
    },
    DosingPumpA {
        finish_time: u64,
        dose_a_ml: f32,
        dose_b_ml: f32,
        target_ec: f32,
        start_ec: f32,
        start_ph: f32,
    },
    WaitingBetweenDose {
        finish_time: u64,
        dose_b_ml: f32,
        target_ec: f32,
        start_ec: f32,
        start_ph: f32,
        dose_a_ml_reported: f32,
    },
    DosingPumpB {
        finish_time: u64,
        dose_b_ml: f32,
        target_ec: f32,
        start_ec: f32,
        start_ph: f32,
        dose_a_ml_reported: f32,
    },
    DosingPH {
        finish_time: u64,
        is_up: bool,
        dose_ml: f32,
        target_ph: f32,
        start_ec: f32,
        start_ph: f32,
    },
    ActiveMixing {
        finish_time: u64,
    },
    Stabilizing {
        finish_time: u64,
    },
}

impl SystemState {
    pub fn to_payload_string(&self) -> String {
        match self {
            SystemState::Monitoring => "Monitoring".to_string(),
            SystemState::EmergencyStop(reason) => format!("EmergencyStop:{}", reason),
            SystemState::SystemFault(reason) => format!("SystemFault:{}", reason),
            SystemState::WaterRefilling { .. } => "WaterRefilling".to_string(),
            SystemState::WaterDraining { .. } => "WaterDraining".to_string(),
            SystemState::DosingPumpA { .. } => "DosingPumpA".to_string(),
            SystemState::WaitingBetweenDose { .. } => "WaitingBetweenDose".to_string(),
            SystemState::DosingPumpB { .. } => "DosingPumpB".to_string(),
            SystemState::DosingPH { .. } => "DosingPH".to_string(),
            SystemState::StartingOsakaPump { .. } => "StartingOsakaPump".to_string(),
            SystemState::ActiveMixing { .. } => "ActiveMixing".to_string(),
            SystemState::Stabilizing { .. } => "Stabilizing".to_string(),
        }
    }
}

pub struct ControlContext {
    pub current_state: SystemState,
    pub last_water_change_time: u64,
    pub last_scheduled_dose_time_sec: u64,

    pub next_cron_trigger_sec: Option<u64>,
    pub current_cron_expr: String,
    pub next_water_change_trigger_sec: Option<u64>,
    pub current_water_change_cron_expr: String,

    pub ec_retry_count: u8,
    pub ph_retry_count: u8,
    pub water_refill_retry_count: u8,
    pub last_ec_before_dosing: Option<f32>,
    pub last_ph_before_dosing: Option<f32>,
    pub last_ph_dosing_is_up: Option<bool>,
    pub last_water_before_refill: Option<f32>,
    pub last_water_before_drain: Option<f32>,
    pub previous_ec_value: Option<f32>,
    pub previous_ph_value: Option<f32>,
    pub last_continuous_level: bool,
    pub is_misting_active: bool,
    pub last_mist_toggle_time: u64,
    pub is_scheduled_mixing_active: bool,
    pub last_mixing_start_sec: u64,
    pub fsm_osaka_active: bool,
    pub current_osaka_pwm: u32,
    pub pump_status: PumpStatus,
    pub manual_timeouts: HashMap<String, u64>,
    pub safety_override_until: u64,
}

impl Default for ControlContext {
    fn default() -> Self {
        Self {
            current_state: SystemState::Monitoring,
            last_water_change_time: 0,
            last_scheduled_dose_time_sec: 0,

            next_cron_trigger_sec: None,
            current_cron_expr: String::new(),
            next_water_change_trigger_sec: None,
            current_water_change_cron_expr: String::new(),

            ec_retry_count: 0,
            ph_retry_count: 0,
            water_refill_retry_count: 0,
            last_ec_before_dosing: None,
            last_ph_before_dosing: None,
            last_ph_dosing_is_up: None,
            last_water_before_refill: None,
            last_water_before_drain: None,
            previous_ec_value: None,
            previous_ph_value: None,
            last_continuous_level: false,
            is_misting_active: false,
            last_mist_toggle_time: 0,
            is_scheduled_mixing_active: false,
            last_mixing_start_sec: 0,
            fsm_osaka_active: false,
            current_osaka_pwm: 0,
            pump_status: PumpStatus::default(),
            manual_timeouts: HashMap::new(),
            safety_override_until: 0,
        }
    }
}

impl ControlContext {
    fn stop_all_pumps(&mut self, pump_ctrl: &mut PumpController) {
        let _ = pump_ctrl.stop_all();
        self.pump_status = PumpStatus::default();
        self.is_misting_active = false;
        self.is_scheduled_mixing_active = false;
        self.fsm_osaka_active = false;
        self.current_osaka_pwm = 0;
        self.manual_timeouts.clear();
    }

    fn reset_faults(&mut self) {
        self.ec_retry_count = 0;
        self.ph_retry_count = 0;
        self.water_refill_retry_count = 0;
        self.last_ec_before_dosing = None;
        self.last_ph_before_dosing = None;
        self.last_water_before_refill = None;
        self.fsm_osaka_active = false;
        self.current_state = SystemState::Monitoring;
    }

    pub fn turn_off_pump(&mut self, pump_name: &str, pump_ctrl: &mut PumpController) {
        let _ = match pump_name {
            "A" | "PUMP_A" => {
                self.pump_status.pump_a = false;
                pump_ctrl.set_pump_state(PumpType::NutrientA, false)
            }
            "B" | "PUMP_B" => {
                self.pump_status.pump_b = false;
                pump_ctrl.set_pump_state(PumpType::NutrientB, false)
            }
            "PH_UP" | "PUMP_PH_UP" => {
                self.pump_status.ph_up = false;
                pump_ctrl.set_pump_state(PumpType::PhUp, false)
            }
            "PH_DOWN" | "PUMP_PH_DOWN" => {
                self.pump_status.ph_down = false;
                pump_ctrl.set_pump_state(PumpType::PhDown, false)
            }
            "OSAKA_PUMP" | "OSAKA" => {
                self.pump_status.osaka_pump = false;
                pump_ctrl.set_osaka_pump(false)
            }
            "MIST_VALVE" | "MIST" => {
                self.pump_status.mist_valve = false;
                self.is_misting_active = false;
                pump_ctrl.set_mist_valve(false)
            }
            "WATER_PUMP" | "WATER_PUMP_IN" | "PUMP_IN" => {
                self.pump_status.water_pump_in = false;
                pump_ctrl.set_water_pump(WaterDirection::Stop)
            }
            "DRAIN_PUMP" | "WATER_PUMP_OUT" | "PUMP_OUT" => {
                self.pump_status.water_pump_out = false;
                pump_ctrl.set_water_pump(WaterDirection::Stop)
            }
            _ => Ok(()),
        };
    }

    fn check_and_update_noise(&mut self, sensors: &SensorData, config: &DeviceConfig) -> bool {
        let mut is_noisy = false;
        if config.enable_ec_sensor && !sensors.err_ec {
            if let Some(prev_ec) = self.previous_ec_value {
                if (sensors.ec_value - prev_ec).abs() > config.max_ec_delta {
                    warn!("⚠️ Nhiễu EC. Bỏ qua nhịp này!");
                    is_noisy = true;
                }
            }
            self.previous_ec_value = Some(sensors.ec_value);
        }
        if config.enable_ph_sensor && !sensors.err_ph {
            if let Some(prev_ph) = self.previous_ph_value {
                if (sensors.ph_value - prev_ph).abs() > config.max_ph_delta {
                    warn!("⚠️ Nhiễu pH. Bỏ qua nhịp này!");
                    is_noisy = true;
                }
            }
            self.previous_ph_value = Some(sensors.ph_value);
        }
        is_noisy
    }

    fn verify_sensor_ack(&mut self, sensors: &SensorData, config: &DeviceConfig) {
        if config.enable_ec_sensor && !sensors.err_ec {
            if let Some(last_ec) = self.last_ec_before_dosing {
                if (sensors.ec_value - last_ec) >= config.ec_ack_threshold {
                    self.ec_retry_count = 0;
                } else {
                    self.ec_retry_count += 1;
                    warn!("⚠️ EC không tăng! Lần thử: {}/3", self.ec_retry_count);
                }
                self.last_ec_before_dosing = None;
            }
        }
        if config.enable_ph_sensor && !sensors.err_ph {
            if let Some(last_ph) = self.last_ph_before_dosing {
                let is_up = self.last_ph_dosing_is_up.unwrap_or(true);
                let is_ack_ok = if is_up {
                    (sensors.ph_value - last_ph) >= config.ph_ack_threshold
                } else {
                    (last_ph - sensors.ph_value) >= config.ph_ack_threshold
                };
                if is_ack_ok {
                    self.ph_retry_count = 0;
                } else {
                    self.ph_retry_count += 1;
                    warn!("⚠️ pH không đổi hướng! Lần thử: {}/3", self.ph_retry_count);
                }
                self.last_ph_before_dosing = None;
                self.last_ph_dosing_is_up = None;
            }
        }
        if config.enable_water_level_sensor && !sensors.err_water {
            if let Some(w) = self.last_water_before_refill {
                if (sensors.water_level - w) >= config.water_ack_threshold {
                    self.water_refill_retry_count = 0;
                } else {
                    self.water_refill_retry_count += 1;
                }
                self.last_water_before_refill = None;
            }
            if let Some(w) = self.last_water_before_drain {
                if (w - sensors.water_level) >= config.water_ack_threshold {
                    self.water_refill_retry_count = 0;
                } else {
                    self.water_refill_retry_count += 1;
                }
                self.last_water_before_drain = None;
            }
        }
    }
}

fn get_current_time_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or(Duration::from_secs(0))
        .as_millis() as u64
}

fn get_current_time_sec() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or(Duration::from_secs(0))
        .as_secs()
}

pub fn start_fsm_control_loop(
    shared_config: SharedConfig,
    shared_sensors: SharedSensorData,
    mut pump_ctrl: PumpController,
    nvs_partition: EspDefaultNvsPartition,
    cmd_rx: Receiver<MqttCommandPayload>,
    fsm_mqtt_tx: Sender<String>,
    dosing_report_tx: Sender<String>,
    sensor_cmd_tx: Sender<String>,
) {
    std::thread::spawn(move || {
        let mut ctx = ControlContext::default();
        let mut last_reported_state = "".to_string();

        let mut nvs = EspNvs::new(nvs_partition, "agitech", true).ok();
        let current_time_on_boot = get_current_time_sec();

        ctx.last_water_change_time = nvs
            .as_mut()
            .and_then(|flash| flash.get_u64("last_w_change").unwrap_or(None))
            .unwrap_or_else(|| {
                if let Some(flash) = nvs.as_mut() {
                    let _ = flash.set_u64("last_w_change", current_time_on_boot);
                }
                current_time_on_boot
            });

        ctx.last_scheduled_dose_time_sec = nvs
            .as_mut()
            .and_then(|flash| flash.get_u64("last_sched_dose").unwrap_or(None))
            .unwrap_or_else(|| {
                if let Some(flash) = nvs.as_mut() {
                    let _ = flash.set_u64("last_sched_dose", current_time_on_boot);
                }
                current_time_on_boot
            });

        ctx.last_mixing_start_sec = current_time_on_boot;

        info!("🚀 Bắt đầu chạy Máy trạng thái (FSM) Đa luồng Hợp nhất...");

        loop {
            let config = shared_config.read().unwrap().clone();
            let sensors = shared_sensors.read().unwrap().clone();
            let current_time_ms = get_current_time_ms();
            let current_time_sec = current_time_ms / 1000;

            // 🟢 NHẬN LỆNH & XỬ LÝ CƯỠNG CHẾ
            let force_sync =
                process_mqtt_commands(&cmd_rx, &config, &mut pump_ctrl, &mut ctx, current_time_ms);

            let mut expired_pumps = Vec::new();
            for (pump, &finish_time) in &ctx.manual_timeouts {
                if current_time_ms >= finish_time {
                    expired_pumps.push(pump.clone());
                }
            }
            for pump in expired_pumps {
                ctx.manual_timeouts.remove(&pump);
                info!("⏱️ HẾT GIỜ (SAFE TIMEOUT): Tự động tắt bơm {}!", pump);
                ctx.turn_off_pump(&pump, &mut pump_ctrl);
            }

            // 🟢 Thay vì Timeout (Vì giờ có 2 node, timeout có thể do Node Controller tự đếm), FSM kiểm tra qua Error Flags.
            let is_safety_overridden = current_time_ms < ctx.safety_override_until;

            if !is_safety_overridden {
                // Kiểm tra nhiễu (Bỏ qua nhịp nếu nhiễu)
                if ctx.check_and_update_noise(&sensors, &config)
                    && config.control_mode == ControlMode::Auto
                {
                    // Skip
                } else {
                    let is_water_critical = config.enable_water_level_sensor
                        && (sensors.water_level < config.water_level_critical_min);
                    let is_ec_out_of_bounds = config.enable_ec_sensor
                        && (sensors.ec_value < config.min_ec_limit
                            || sensors.ec_value > config.max_ec_limit);
                    let is_ph_out_of_bounds = config.enable_ph_sensor
                        && (sensors.ph_value < config.min_ph_limit
                            || sensors.ph_value > config.max_ph_limit);

                    // 🟢 MỚI: Bóc tách lý do Emergency cụ thể, BAO GỒM CỜ LỖI CẢM BIẾN (Từ Sensor Node gửi sang)
                    let mut emergency_reason = String::new();
                    if config.emergency_shutdown {
                        emergency_reason = "MANUAL_STOP".to_string();
                    } else if config.enable_water_level_sensor && sensors.err_water {
                        emergency_reason = "SENSOR_FAULT_WATER".to_string();
                    } else if config.enable_ec_sensor && sensors.err_ec {
                        emergency_reason = "SENSOR_FAULT_EC".to_string();
                    } else if config.enable_ph_sensor && sensors.err_ph {
                        emergency_reason = "SENSOR_FAULT_PH".to_string();
                    } else if config.enable_temp_sensor && sensors.err_temp {
                        emergency_reason = "SENSOR_FAULT_TEMP".to_string();
                    } else if is_water_critical {
                        emergency_reason = "WATER_CRITICAL".to_string();
                    } else if is_ec_out_of_bounds {
                        emergency_reason = "EC_OUT_OF_BOUNDS".to_string();
                    } else if is_ph_out_of_bounds {
                        emergency_reason = "PH_OUT_OF_BOUNDS".to_string();
                    }

                    let should_emergency_stop = !emergency_reason.is_empty();

                    // 🟢 NẾU NGUY HIỂM VÀ KHÔNG BỊ CƯỠNG CHẾ -> NGẮT
                    if should_emergency_stop {
                        if !matches!(ctx.current_state, SystemState::EmergencyStop(_)) {
                            error!(
                                "⚠️ DỪNG KHẨN CẤP Toàn bộ hệ thống! Lý do: {}",
                                emergency_reason
                            );
                            ctx.stop_all_pumps(&mut pump_ctrl);
                            ctx.current_state = SystemState::EmergencyStop(emergency_reason);
                        }
                    } else if !config.is_enabled {
                        if ctx.current_state != SystemState::Monitoring {
                            ctx.stop_all_pumps(&mut pump_ctrl);
                            ctx.current_state = SystemState::Monitoring;
                        }
                    } else if matches!(ctx.current_state, SystemState::EmergencyStop(_)) {
                        // 🟢 NẾU MÔI TRƯỜNG ĐÃ AN TOÀN LẠI -> HỦY DỪNG KHẨN CẤP
                        if !should_emergency_stop {
                            info!("✅ Hệ thống an toàn trở lại (hoặc đang Cưỡng chế).");
                            ctx.current_state = SystemState::Monitoring;
                        }
                    } else if config.control_mode == ControlMode::Auto {
                        // ==== LOGIC AUTO ====
                        let is_hot = config.enable_temp_sensor
                            && (sensors.temp_value >= config.misting_temp_threshold);
                        let on_duration = if is_hot {
                            config.high_temp_misting_on_duration_ms
                        } else {
                            config.misting_on_duration_ms
                        };
                        let off_duration = if is_hot {
                            config.high_temp_misting_off_duration_ms
                        } else {
                            config.misting_off_duration_ms
                        };

                        if ctx.is_misting_active {
                            if current_time_ms >= ctx.last_mist_toggle_time + on_duration {
                                let _ = pump_ctrl.set_mist_valve(false);
                                ctx.is_misting_active = false;
                                ctx.last_mist_toggle_time = current_time_ms;
                                ctx.pump_status.mist_valve = false;
                            }
                        } else {
                            if current_time_ms >= ctx.last_mist_toggle_time + off_duration {
                                let _ = pump_ctrl.set_mist_valve(true);
                                ctx.is_misting_active = true;
                                ctx.last_mist_toggle_time = current_time_ms;
                                ctx.pump_status.mist_valve = true;
                            }
                        }

                        if config.scheduled_mixing_interval_sec > 0
                            && config.scheduled_mixing_duration_sec > 0
                        {
                            if ctx.is_scheduled_mixing_active {
                                if current_time_sec
                                    >= ctx.last_mixing_start_sec
                                        + config.scheduled_mixing_duration_sec
                                {
                                    ctx.is_scheduled_mixing_active = false;
                                }
                            } else {
                                if current_time_sec
                                    >= ctx.last_mixing_start_sec
                                        + config.scheduled_mixing_interval_sec
                                {
                                    ctx.is_scheduled_mixing_active = true;
                                    ctx.last_mixing_start_sec = current_time_sec;
                                }
                            }
                        } else {
                            ctx.is_scheduled_mixing_active = false;
                        }

                        if !matches!(ctx.current_state, SystemState::SystemFault(_)) {
                            run_auto_fsm(
                                current_time_ms,
                                &config,
                                &sensors,
                                &mut ctx,
                                &mut pump_ctrl,
                                &mut nvs,
                                &dosing_report_tx,
                            );
                        }

                        let needs_osaka = ctx.fsm_osaka_active
                            || ctx.is_misting_active
                            || ctx.is_scheduled_mixing_active;
                        if needs_osaka {
                            let target_pwm = if ctx.is_misting_active {
                                config.osaka_misting_pwm_percent
                            } else {
                                config.osaka_mixing_pwm_percent
                            };
                            if !ctx.pump_status.osaka_pump {
                                let _ = pump_ctrl.start_osaka_pump_soft(target_pwm);
                                ctx.pump_status.osaka_pump = true;
                                ctx.current_osaka_pwm = target_pwm;
                            } else if ctx.current_osaka_pwm != target_pwm {
                                let _ = pump_ctrl.set_osaka_pump_pwm(target_pwm);
                                ctx.current_osaka_pwm = target_pwm;
                            }
                        } else {
                            if ctx.pump_status.osaka_pump {
                                let _ = pump_ctrl.set_osaka_pump_pwm(0);
                                ctx.pump_status.osaka_pump = false;
                                ctx.current_osaka_pwm = 0;
                            }
                        }
                    } else if !matches!(
                        ctx.current_state,
                        SystemState::Monitoring
                            | SystemState::SystemFault(_)
                            | SystemState::EmergencyStop(_)
                    ) {
                        info!("Chuyển sang chế độ MANUAL.");
                        ctx.stop_all_pumps(&mut pump_ctrl);
                        ctx.current_state = SystemState::Monitoring;
                    }
                }
            }

            // 🟢 LỆNH COMMAND XUỐNG SENSOR NODE: Bật đo nước liên tục nếu đang bơm/xả
            let needs_continuous = matches!(
                ctx.current_state,
                SystemState::WaterRefilling { .. } | SystemState::WaterDraining { .. }
            );
            if needs_continuous != ctx.last_continuous_level {
                let payload = format!(
                    r#"{{"command":"continuous_level", "state": {}}}"#,
                    needs_continuous
                );
                let _ = sensor_cmd_tx.send(payload);
                ctx.last_continuous_level = needs_continuous;
            }

            // First update shared sensors with latest pump status
            if let Ok(mut sensors_lock) = shared_sensors.write() {
                sensors_lock.pump_status = ctx.pump_status.clone();
            }

            // Report state changes and sync online status
            let state_changed = report_state_if_changed(&ctx.current_state, &mut last_reported_state, &fsm_mqtt_tx);
            
            if state_changed || force_sync {
                // Always send full status when state changes or forced sync
                let status_msg = serde_json::json!({
                    "online": true,
                    "current_state": ctx.current_state.to_payload_string(),
                    "pump_status": ctx.pump_status
                }).to_string();
                
                let _ = fsm_mqtt_tx.send(status_msg);
                
                if force_sync {
                    last_reported_state = "".to_string();
                    let _ = sensor_cmd_tx.send(r#"{"command":"force_publish"}"#.to_string());
                    info!("⚡ Đã ép luồng chính Publish trạng thái bơm mới nhất lên App!");
                }
            }

            std::thread::sleep(Duration::from_millis(100));
        }
    });
}

fn report_state_if_changed(
    current_state: &SystemState,
    last_reported_state: &mut String,
    fsm_mqtt_tx: &Sender<String>,
) -> bool {
    let current_state_str = current_state.to_payload_string();
    if current_state_str != *last_reported_state {
        let payload = format!(r#"{{"current_state": "{}"}}"#, current_state_str);
        if fsm_mqtt_tx.send(payload).is_ok() {
            info!("📡 Trạng thái FSM: [{}]", current_state_str);
        }
        *last_reported_state = current_state_str;
        true
    } else {
        false
    }
}

fn process_mqtt_commands(
    cmd_rx: &Receiver<MqttCommandPayload>,
    config: &DeviceConfig,
    pump_ctrl: &mut PumpController,
    ctx: &mut ControlContext,
    current_time_ms: u64,
) -> bool {
    let mut force_sync = false;

    let is_emergency_state = matches!(
        ctx.current_state,
        SystemState::EmergencyStop(_) | SystemState::SystemFault(_)
    );

    while let Ok(cmd) = cmd_rx.try_recv() {
        if cmd.action == "SYNC_STATUS" {
            force_sync = true;
            continue;
        }

        if cmd.action == "reset_fault" {
            info!("🔄 Nhận lệnh Reset. Khôi phục hệ thống...");
            ctx.stop_all_pumps(pump_ctrl);
            ctx.reset_faults();
            force_sync = true;
            continue;
        }

        if config.control_mode == ControlMode::Auto {
            warn!("Bỏ qua lệnh thủ công vì đang ở AUTO.");
            continue;
        }

        let pump_name = cmd.pump.to_uppercase();
        let action_lower = cmd.action.to_lowercase();

        let is_force_on = action_lower == "force_on";
        let is_set_pwm = action_lower == "set_pwm";

        let is_on = is_force_on
            || action_lower == "pump_on"
            || action_lower == "on"
            || action_lower == "true"
            || action_lower == "1"
            || (is_set_pwm && cmd.pwm.unwrap_or(0) > 0);

        if is_emergency_state && is_on && !is_force_on {
            warn!("❌ BLOCKED: Không thể điều khiển {} bình thường vì hệ thống đang Lỗi / EmergencyStop. Vui lòng dùng FORCE.", pump_name);
            continue;
        }

        if is_force_on {
            info!("⚠️ NGƯỜI DÙNG CƯỠNG CHẾ BẬT {}!", pump_name);
            let duration = cmd.duration_sec.unwrap_or(120);
            ctx.safety_override_until = current_time_ms + (duration as u64 * 1000);
        }

        if is_on {
            if let Some(duration) = cmd.duration_sec {
                if duration > 0 {
                    let finish_time = current_time_ms + (duration as u64 * 1000);
                    ctx.manual_timeouts.insert(pump_name.clone(), finish_time);
                }
            }
        } else {
            ctx.manual_timeouts.remove(&pump_name);
        }

        let pwm_val = if let Some(p) = cmd.pwm {
            p
        } else {
            if is_on {
                100
            } else {
                0
            }
        };

        let _ = match pump_name.as_str() {
            "A" | "PUMP_A" => {
                ctx.pump_status.pump_a = is_on;
                if cmd.pwm.is_some() || is_set_pwm {
                    pump_ctrl.set_dosing_pump_pwm(PumpType::NutrientA, is_on, pwm_val)
                } else {
                    pump_ctrl.set_pump_state(PumpType::NutrientA, is_on)
                }
            }
            "B" | "PUMP_B" => {
                ctx.pump_status.pump_b = is_on;
                if cmd.pwm.is_some() || is_set_pwm {
                    pump_ctrl.set_dosing_pump_pwm(PumpType::NutrientB, is_on, pwm_val)
                } else {
                    pump_ctrl.set_pump_state(PumpType::NutrientB, is_on)
                }
            }
            "PH_UP" | "PUMP_PH_UP" => {
                ctx.pump_status.ph_up = is_on;
                if cmd.pwm.is_some() || is_set_pwm {
                    pump_ctrl.set_dosing_pump_pwm(PumpType::PhUp, is_on, pwm_val)
                } else {
                    pump_ctrl.set_pump_state(PumpType::PhUp, is_on)
                }
            }
            "PH_DOWN" | "PUMP_PH_DOWN" => {
                ctx.pump_status.ph_down = is_on;
                if cmd.pwm.is_some() || is_set_pwm {
                    pump_ctrl.set_dosing_pump_pwm(PumpType::PhDown, is_on, pwm_val)
                } else {
                    pump_ctrl.set_pump_state(PumpType::PhDown, is_on)
                }
            }
            "OSAKA_PUMP" | "OSAKA" => {
                ctx.pump_status.osaka_pump = is_on;
                if cmd.pwm.is_some() || is_set_pwm {
                    pump_ctrl.set_osaka_pump_pwm(pwm_val)
                } else {
                    pump_ctrl.set_osaka_pump(is_on)
                }
            }
            "MIST_VALVE" | "MIST" => {
                ctx.pump_status.mist_valve = is_on;
                ctx.is_misting_active = is_on;
                if is_on {
                    ctx.last_mist_toggle_time = current_time_ms;
                }
                pump_ctrl.set_mist_valve(is_on)
            }
            "WATER_PUMP" | "WATER_PUMP_IN" | "PUMP_IN" => {
                ctx.pump_status.water_pump_in = is_on;
                if is_on {
                    ctx.pump_status.water_pump_out = false;
                }
                pump_ctrl.set_water_pump(if is_on {
                    WaterDirection::In
                } else {
                    WaterDirection::Stop
                })
            }
            "DRAIN_PUMP" | "WATER_PUMP_OUT" | "PUMP_OUT" => {
                ctx.pump_status.water_pump_out = is_on;
                if is_on {
                    ctx.pump_status.water_pump_in = false;
                }
                pump_ctrl.set_water_pump(if is_on {
                    WaterDirection::Out
                } else {
                    WaterDirection::Stop
                })
            }
            _ => Ok(()),
        };

        force_sync = true;
    }

    force_sync
}

fn run_auto_fsm(
    current_time_ms: u64,
    config: &DeviceConfig,
    sensors: &SensorData,
    ctx: &mut ControlContext,
    pump_ctrl: &mut PumpController,
    nvs: &mut Option<EspDefaultNvs>,
    dosing_report_tx: &Sender<String>,
) {
    let current_time_sec = current_time_ms / 1000;

    match ctx.current_state {
        SystemState::SystemFault(ref reason) => {
            warn!("🚨 BÁO LỖI: [{}]. Chờ reset...", reason);
        }

        SystemState::Monitoring => {
            ctx.verify_sensor_ack(sensors, config);

            // 🟢 THAY NƯỚC ĐỊNH KỲ THEO LỊCH CRON
            if config.enable_water_level_sensor
                && config.scheduled_water_change_enabled
                && !config.water_change_cron.is_empty()
            {
                if ctx.current_water_change_cron_expr != config.water_change_cron {
                    ctx.current_water_change_cron_expr = config.water_change_cron.clone();
                    if let Ok(schedule) = Schedule::from_str(&ctx.current_water_change_cron_expr) {
                        if let Some(next) = schedule.upcoming(Local).next() {
                            ctx.next_water_change_trigger_sec = Some(next.timestamp() as u64);
                            info!("⏰ Cập nhật lịch Thay nước Cron: {}", next);
                        }
                    } else {
                        warn!("⚠️ Lỗi cú pháp Cron Thay nước!");
                        ctx.next_water_change_trigger_sec = None;
                    }
                }

                if let Some(next_trigger) = ctx.next_water_change_trigger_sec {
                    if current_time_sec >= next_trigger {
                        info!("⏰ Đã đến giờ THAY NƯỚC ĐỊNH KỲ theo lịch CRON!");

                        if let Ok(schedule) =
                            Schedule::from_str(&ctx.current_water_change_cron_expr)
                        {
                            let future = Local::now() + chrono::Duration::seconds(1);
                            if let Some(next) = schedule.after(&future).next() {
                                ctx.next_water_change_trigger_sec = Some(next.timestamp() as u64);
                            }
                        }

                        let target = (sensors.water_level - config.scheduled_drain_amount_cm)
                            .max(config.water_level_min);
                        ctx.last_water_change_time = current_time_sec;

                        if let Some(flash) = nvs.as_mut() {
                            let _ = flash.set_u64("last_w_change", current_time_sec);
                        }

                        ctx.current_state = SystemState::WaterDraining {
                            target_level: target,
                            start_time: current_time_ms,
                        };
                        let _ = pump_ctrl.set_water_pump(WaterDirection::Out);
                        ctx.pump_status.water_pump_out = true;
                        ctx.pump_status.water_pump_in = false;
                        ctx.fsm_osaka_active = false;

                        return;
                    }
                }
            }

            if config.enable_water_level_sensor
                && config.auto_refill_enabled
                && sensors.water_level < (config.water_level_target - config.water_level_tolerance)
            {
                if ctx.water_refill_retry_count >= 3 {
                    ctx.stop_all_pumps(pump_ctrl);
                    ctx.current_state = SystemState::SystemFault("WATER_REFILL_FAILED".to_string());
                } else {
                    ctx.last_water_before_refill = Some(sensors.water_level);
                    ctx.current_state = SystemState::WaterRefilling {
                        target_level: config.water_level_target,
                        start_time: current_time_ms,
                    };
                    let _ = pump_ctrl.set_water_pump(WaterDirection::In);
                    ctx.pump_status.water_pump_in = true;
                    ctx.pump_status.water_pump_out = false;
                    ctx.fsm_osaka_active = false;
                }
            } else if config.enable_water_level_sensor
                && config.auto_drain_overflow
                && sensors.water_level > config.water_level_max
            {
                ctx.current_state = SystemState::WaterDraining {
                    target_level: config.water_level_target,
                    start_time: current_time_ms,
                };
                let _ = pump_ctrl.set_water_pump(WaterDirection::Out);
                ctx.pump_status.water_pump_out = true;
                ctx.pump_status.water_pump_in = false;
                ctx.fsm_osaka_active = false;
            } else if config.enable_ec_sensor
                && config.enable_water_level_sensor
                && config.auto_dilute_enabled
                && sensors.ec_value > (config.ec_target + config.ec_tolerance)
            {
                let target = (sensors.water_level - config.dilute_drain_amount_cm)
                    .max(config.water_level_min);
                ctx.current_state = SystemState::WaterDraining {
                    target_level: target,
                    start_time: current_time_ms,
                };
                let _ = pump_ctrl.set_water_pump(WaterDirection::Out);
                ctx.pump_status.water_pump_out = true;
                ctx.pump_status.water_pump_in = false;
                ctx.fsm_osaka_active = false;
            } else {
                let mut is_dosing_active = false;

                // 🟢 BƠM ĐỊNH KỲ THEO LỊCH CRON
                if config.scheduled_dosing_enabled && !config.scheduled_dosing_cron.is_empty() {
                    if ctx.current_cron_expr != config.scheduled_dosing_cron {
                        ctx.current_cron_expr = config.scheduled_dosing_cron.clone();
                        if let Ok(schedule) = Schedule::from_str(&ctx.current_cron_expr) {
                            if let Some(next) = schedule.upcoming(Local).next() {
                                ctx.next_cron_trigger_sec = Some(next.timestamp() as u64);
                                info!("⏰ Cập nhật lịch Dosing Cron: {}", next);
                            }
                        } else {
                            warn!("⚠️ Biểu thức Cron Dosing không hợp lệ!");
                            ctx.next_cron_trigger_sec = None;
                        }
                    }

                    if let Some(next_trigger) = ctx.next_cron_trigger_sec {
                        if current_time_sec >= next_trigger {
                            info!("⏰ Đã đến giờ BƠM DINH DƯỠNG theo lịch CRON!");

                            if let Ok(schedule) = Schedule::from_str(&ctx.current_cron_expr) {
                                let future = Local::now() + chrono::Duration::seconds(1);
                                if let Some(next) = schedule.after(&future).next() {
                                    ctx.next_cron_trigger_sec = Some(next.timestamp() as u64);
                                }
                            }

                            ctx.last_scheduled_dose_time_sec = current_time_sec;
                            if let Some(flash) = nvs.as_mut() {
                                let _ = flash.set_u64("last_sched_dose", current_time_sec);
                            }

                            let safe_pwm = config.dosing_pwm_percent.clamp(1, 100);
                            if config.scheduled_dose_a_ml > 0.0 || config.scheduled_dose_b_ml > 0.0
                            {
                                ctx.current_state = SystemState::StartingOsakaPump {
                                    finish_time: current_time_ms + config.soft_start_duration,
                                    pending_action: PendingDose::ScheduledDose {
                                        dose_a_ml: config.scheduled_dose_a_ml,
                                        dose_b_ml: config.scheduled_dose_b_ml,
                                        pwm_percent: safe_pwm,
                                    },
                                };
                                ctx.fsm_osaka_active = true;
                                is_dosing_active = true;
                            }
                        }
                    }
                }

                // 🟢 BÙ EC TỰ ĐỘNG
                if config.enable_ec_sensor
                    && !is_dosing_active
                    && sensors.ec_value < (config.ec_target - config.ec_tolerance)
                {
                    if ctx.ec_retry_count >= 3 {
                        ctx.stop_all_pumps(pump_ctrl);
                        ctx.current_state =
                            SystemState::SystemFault("EC_DOSING_FAILED".to_string());
                        is_dosing_active = true;
                    } else {
                        let safe_pwm = config.dosing_pwm_percent.clamp(1, 100);
                        let dose_ml = ((config.ec_target - sensors.ec_value)
                            / config.ec_gain_per_ml
                            * config.ec_step_ratio)
                            .clamp(0.0, config.max_dose_per_cycle);

                        if dose_ml > 0.0 {
                            ctx.last_ec_before_dosing = Some(sensors.ec_value);
                            ctx.current_state = SystemState::StartingOsakaPump {
                                finish_time: current_time_ms + config.soft_start_duration,
                                pending_action: PendingDose::EC {
                                    dose_ml,
                                    target_ec: config.ec_target,
                                    pwm_percent: safe_pwm,
                                },
                            };
                            ctx.fsm_osaka_active = true;
                            is_dosing_active = true;
                        }
                    }
                }

                // 🟢 BÙ PH TỰ ĐỘNG
                if config.enable_ph_sensor
                    && !is_dosing_active
                    && (sensors.ph_value - config.ph_target).abs() > config.ph_tolerance
                {
                    if ctx.ph_retry_count >= 3 {
                        ctx.stop_all_pumps(pump_ctrl);
                        ctx.current_state =
                            SystemState::SystemFault("PH_DOSING_FAILED".to_string());
                        is_dosing_active = true;
                    } else {
                        let is_ph_up = sensors.ph_value < config.ph_target;
                        let diff = (sensors.ph_value - config.ph_target).abs();
                        let ratio = if is_ph_up {
                            config.ph_shift_up_per_ml
                        } else {
                            config.ph_shift_down_per_ml
                        };
                        let safe_pwm = config.dosing_pwm_percent.clamp(1, 100);

                        let base_capacity = if is_ph_up {
                            config.pump_ph_up_capacity_ml_per_sec
                        } else {
                            config.pump_ph_down_capacity_ml_per_sec
                        };

                        let active_capacity = base_capacity * (safe_pwm as f32 / 100.0);

                        let dose_ml = (diff / ratio * config.ph_step_ratio)
                            .clamp(0.0, config.max_dose_per_cycle);
                        let duration_ms = ((dose_ml / active_capacity) * 1000.0) as u64;

                        if duration_ms > 0 {
                            ctx.last_ph_before_dosing = Some(sensors.ph_value);
                            ctx.last_ph_dosing_is_up = Some(is_ph_up);

                            ctx.current_state = SystemState::StartingOsakaPump {
                                finish_time: current_time_ms + config.soft_start_duration,
                                pending_action: PendingDose::PH {
                                    is_up: is_ph_up,
                                    dose_ml,
                                    duration_ms,
                                    target_ph: config.ph_target,
                                    pwm_percent: safe_pwm,
                                },
                            };
                            ctx.fsm_osaka_active = true;
                            is_dosing_active = true;
                        }
                    }
                }

                if !is_dosing_active {
                    ctx.fsm_osaka_active = false;
                }
            }
        }

        SystemState::WaterRefilling {
            target_level,
            start_time,
        } => {
            if sensors.water_level >= target_level
                || current_time_ms.saturating_sub(start_time)
                    > (config.max_refill_duration_sec as u64 * 1000)
            {
                let _ = pump_ctrl.set_water_pump(WaterDirection::Stop);
                ctx.pump_status.water_pump_in = false;
                ctx.pump_status.water_pump_out = false;
                ctx.fsm_osaka_active = true;
                ctx.current_state = SystemState::ActiveMixing {
                    finish_time: current_time_ms + (config.active_mixing_sec as u64 * 1000),
                };
            }
        }

        SystemState::WaterDraining {
            target_level,
            start_time,
        } => {
            if sensors.water_level <= target_level
                || current_time_ms.saturating_sub(start_time)
                    > (config.max_drain_duration_sec as u64 * 1000)
            {
                let _ = pump_ctrl.set_water_pump(WaterDirection::Stop);
                ctx.pump_status.water_pump_in = false;
                ctx.pump_status.water_pump_out = false;
                ctx.fsm_osaka_active = false;
                ctx.current_state = SystemState::Stabilizing {
                    finish_time: current_time_ms + (config.sensor_stabilize_sec as u64 * 1000),
                };
            }
        }

        SystemState::StartingOsakaPump {
            finish_time,
            ref pending_action,
        } => {
            if current_time_ms >= finish_time {
                let action = pending_action.clone();
                match action {
                    PendingDose::ScheduledDose {
                        dose_a_ml,
                        dose_b_ml,
                        pwm_percent,
                    } => {
                        if dose_a_ml > 0.0 {
                            let _ = pump_ctrl.set_dosing_pump_pwm(
                                PumpType::NutrientA,
                                true,
                                pwm_percent,
                            );
                            ctx.pump_status.pump_a = true;
                            let active_capacity_a =
                                config.pump_a_capacity_ml_per_sec * (pwm_percent as f32 / 100.0);
                            let duration_ms_a = ((dose_a_ml / active_capacity_a) * 1000.0) as u64;

                            ctx.current_state = SystemState::DosingPumpA {
                                finish_time: current_time_ms + duration_ms_a,
                                dose_a_ml,
                                dose_b_ml,
                                target_ec: sensors.ec_value,
                                start_ec: sensors.ec_value,
                                start_ph: sensors.ph_value,
                            };
                        } else if dose_b_ml > 0.0 {
                            ctx.current_state = SystemState::WaitingBetweenDose {
                                finish_time: current_time_ms,
                                dose_b_ml,
                                target_ec: sensors.ec_value,
                                start_ec: sensors.ec_value,
                                start_ph: sensors.ph_value,
                                dose_a_ml_reported: 0.0,
                            };
                        } else {
                            ctx.current_state = SystemState::ActiveMixing {
                                finish_time: current_time_ms
                                    + (config.active_mixing_sec as u64 * 1000),
                            };
                        }
                    }
                    PendingDose::EC {
                        dose_ml,
                        target_ec,
                        pwm_percent,
                    } => {
                        let _ =
                            pump_ctrl.set_dosing_pump_pwm(PumpType::NutrientA, true, pwm_percent);
                        ctx.pump_status.pump_a = true;
                        let active_capacity_a =
                            config.pump_a_capacity_ml_per_sec * (pwm_percent as f32 / 100.0);
                        let duration_ms_a = ((dose_ml / active_capacity_a) * 1000.0) as u64;

                        ctx.current_state = SystemState::DosingPumpA {
                            finish_time: current_time_ms + duration_ms_a,
                            dose_a_ml: dose_ml,
                            dose_b_ml: dose_ml,
                            target_ec,
                            start_ec: sensors.ec_value,
                            start_ph: sensors.ph_value,
                        };
                    }
                    PendingDose::PH {
                        is_up,
                        dose_ml,
                        duration_ms,
                        target_ph,
                        pwm_percent,
                    } => {
                        let _ = pump_ctrl.set_dosing_pump_pwm(
                            if is_up {
                                PumpType::PhUp
                            } else {
                                PumpType::PhDown
                            },
                            true,
                            pwm_percent,
                        );
                        if is_up {
                            ctx.pump_status.ph_up = true;
                        } else {
                            ctx.pump_status.ph_down = true;
                        }

                        ctx.current_state = SystemState::DosingPH {
                            finish_time: current_time_ms + duration_ms,
                            is_up,
                            dose_ml,
                            target_ph,
                            start_ec: sensors.ec_value,
                            start_ph: sensors.ph_value,
                        };
                    }
                }
            }
        }

        SystemState::DosingPumpA {
            finish_time,
            dose_a_ml,
            dose_b_ml,
            target_ec,
            start_ec,
            start_ph,
        } => {
            if current_time_ms >= finish_time {
                let _ = pump_ctrl.set_dosing_pump_pwm(PumpType::NutrientA, false, 0);
                ctx.pump_status.pump_a = false;
                ctx.current_state = SystemState::WaitingBetweenDose {
                    finish_time: current_time_ms + (config.delay_between_a_and_b_sec as u64 * 1000),
                    dose_b_ml,
                    target_ec,
                    start_ec,
                    start_ph,
                    dose_a_ml_reported: dose_a_ml,
                };
            }
        }

        SystemState::WaitingBetweenDose {
            finish_time,
            dose_b_ml,
            target_ec,
            start_ec,
            start_ph,
            dose_a_ml_reported,
        } => {
            if current_time_ms >= finish_time {
                if dose_b_ml > 0.0 {
                    let safe_pwm = config.dosing_pwm_percent.clamp(1, 100);
                    let _ = pump_ctrl.set_dosing_pump_pwm(PumpType::NutrientB, true, safe_pwm);
                    ctx.pump_status.pump_b = true;
                    let active_capacity_b =
                        config.pump_b_capacity_ml_per_sec * (safe_pwm as f32 / 100.0);
                    let duration_ms_b = ((dose_b_ml / active_capacity_b) * 1000.0) as u64;

                    ctx.current_state = SystemState::DosingPumpB {
                        finish_time: current_time_ms + duration_ms_b,
                        dose_b_ml,
                        target_ec,
                        start_ec,
                        start_ph,
                        dose_a_ml_reported,
                    };
                } else {
                    let report_json = format!(
                        r#"{{"start_ec":{:.2},"start_ph":{:.2},"pump_a_ml":{:.2},"pump_b_ml":0.0,"ph_up_ml":0.0,"ph_down_ml":0.0,"target_ec":{:.2},"target_ph":{:.2}}}"#,
                        start_ec, start_ph, dose_a_ml_reported, target_ec, config.ph_target
                    );
                    let _ = dosing_report_tx.send(report_json);
                    ctx.current_state = SystemState::ActiveMixing {
                        finish_time: current_time_ms + (config.active_mixing_sec as u64 * 1000),
                    };
                }
            }
        }

        SystemState::DosingPumpB {
            finish_time,
            dose_b_ml,
            target_ec,
            start_ec,
            start_ph,
            dose_a_ml_reported,
        } => {
            if current_time_ms >= finish_time {
                let _ = pump_ctrl.set_dosing_pump_pwm(PumpType::NutrientB, false, 0);
                ctx.pump_status.pump_b = false;
                let report_json = format!(
                    r#"{{"start_ec":{:.2},"start_ph":{:.2},"pump_a_ml":{:.2},"pump_b_ml":{:.2},"ph_up_ml":0.0,"ph_down_ml":0.0,"target_ec":{:.2},"target_ph":{:.2}}}"#,
                    start_ec, start_ph, dose_a_ml_reported, dose_b_ml, target_ec, config.ph_target
                );
                let _ = dosing_report_tx.send(report_json);
                ctx.current_state = SystemState::ActiveMixing {
                    finish_time: current_time_ms + (config.active_mixing_sec as u64 * 1000),
                };
            }
        }

        SystemState::DosingPH {
            finish_time,
            is_up,
            dose_ml,
            target_ph,
            start_ec,
            start_ph,
        } => {
            if current_time_ms >= finish_time {
                let _ = pump_ctrl.set_dosing_pump_pwm(PumpType::PhUp, false, 0);
                ctx.pump_status.ph_up = false;
                let _ = pump_ctrl.set_dosing_pump_pwm(PumpType::PhDown, false, 0);
                ctx.pump_status.ph_down = false;

                let ph_up_ml = if is_up { dose_ml } else { 0.0 };
                let ph_down_ml = if !is_up { dose_ml } else { 0.0 };
                let report_json = format!(
                    r#"{{"start_ec":{:.2},"start_ph":{:.2},"pump_a_ml":0.0,"pump_b_ml":0.0,"ph_up_ml":{:.2},"ph_down_ml":{:.2},"target_ec":{:.2},"target_ph":{:.2}}}"#,
                    start_ec, start_ph, ph_up_ml, ph_down_ml, config.ec_target, target_ph
                );
                let _ = dosing_report_tx.send(report_json);
                ctx.current_state = SystemState::ActiveMixing {
                    finish_time: current_time_ms + (config.active_mixing_sec as u64 * 1000),
                };
            }
        }

        SystemState::ActiveMixing { finish_time } => {
            if current_time_ms >= finish_time {
                ctx.fsm_osaka_active = false;
                ctx.current_state = SystemState::Stabilizing {
                    finish_time: current_time_ms + (config.sensor_stabilize_sec as u64 * 1000),
                };
            }
        }

        SystemState::Stabilizing { finish_time } => {
            if current_time_ms >= finish_time {
                ctx.current_state = SystemState::Monitoring;
            }
        }

        SystemState::EmergencyStop(_) => {} // Không làm gì cả, chờ user FORCE hoặc Reset lỗi
    }
}

