// hydragrow-shared/src/fsm.rs — version mới hoàn chỉnh
use crate::PumpStatus;
use serde::{Deserialize, Serialize};

/// Phase của FSM — có Serialize/Deserialize để gửi qua MQTT và lưu DB
/// Serde sẽ serialize "Monitoring" -> "Monitoring", "Fault(EcDosingFailed)" -> {"Fault":"EC_DOSING_FAILED"}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum SystemPhase {
    Booting,
    Monitoring,
    ManualMode,
    WaterRefilling,
    WaterDraining,
    MimoDosing,
    ActiveMixing,
    Stabilizing,
    Cooldown,
    SensorCalibration,
    Fault(FaultCode),
    EmergencyStop(String),
}

impl SystemPhase {
    pub fn as_str(&self) -> &'static str {
        match self {
            SystemPhase::Booting => "Booting",
            SystemPhase::Monitoring => "Monitoring",
            SystemPhase::ManualMode => "ManualMode",
            SystemPhase::WaterRefilling => "WaterRefilling",
            SystemPhase::WaterDraining => "WaterDraining",
            SystemPhase::MimoDosing => "MimoDosing",
            SystemPhase::ActiveMixing => "ActiveMixing",
            SystemPhase::Stabilizing => "Stabilizing",
            SystemPhase::Cooldown => "Cooldown",
            SystemPhase::SensorCalibration => "SensorCalibration",
            SystemPhase::Fault(_) => "Fault",
            SystemPhase::EmergencyStop(_) => "EmergencyStop",
        }
    }

    /// Trả về true nếu phase hoạt động cần Osaka pump chạy
    pub fn requires_mixing(&self) -> bool {
        matches!(
            self,
            Self::MimoDosing | Self::ActiveMixing | Self::Stabilizing
        )
    }

    /// Trả về true nếu là phase lỗi cần dừng toàn bộ actuator
    pub fn is_fault(&self) -> bool {
        matches!(self, Self::Fault(_) | Self::EmergencyStop(_))
    }

    /// Lấy fault code nếu đang ở trạng thái Fault
    pub fn fault_code(&self) -> Option<&FaultCode> {
        match self {
            Self::Fault(code) => Some(code),
            _ => None,
        }
    }
}

impl core::fmt::Display for SystemPhase {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::Fault(code) => write!(f, "Fault:{}", code.as_str()),
            Self::EmergencyStop(reason) => write!(f, "EmergencyStop:{}", reason),
            other => write!(f, "{}", other.as_str()),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum FaultCode {
    EcDosingFailed,
    PhDosingFailed,
    WaterRefillFailed,
    WaterDrainFailed,
    TooManyRefills,
    TooManyDrains,
    MaxHourlyDoseEc,
    MaxHourlyDosePh,
    SensorTimeout,
    EcStagnant,
    PhOscillating,
    WaterLevelCritical,
    EmergencyStop,
    OsakaRunningWithoutValve,
}

impl FaultCode {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::EcDosingFailed => "EC_DOSING_FAILED",
            Self::PhDosingFailed => "PH_DOSING_FAILED",
            Self::WaterRefillFailed => "WATER_REFILL_FAILED",
            Self::WaterDrainFailed => "WATER_DRAIN_FAILED",
            Self::TooManyRefills => "TOO_MANY_REFILLS",
            Self::TooManyDrains => "TOO_MANY_DRAINS",
            Self::MaxHourlyDoseEc => "MAX_HOURLY_DOSE_EC",
            Self::MaxHourlyDosePh => "MAX_HOURLY_DOSE_PH",
            Self::SensorTimeout => "SENSOR_TIMEOUT",
            Self::EcStagnant => "EC_STAGNANT",
            Self::PhOscillating => "PH_OSCILLATING",
            Self::WaterLevelCritical => "WATER_LEVEL_CRITICAL",
            Self::EmergencyStop => "EMERGENCY_STOP",
            Self::OsakaRunningWithoutValve => "OSAKA_RUNNING_WITHOUT_VALVE",
        }
    }
}

impl core::fmt::Display for FaultCode {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

/// Snapshot trạng thái FSM — type-safe, gửi qua MQTT topic `fsm/state`
/// Thay thế `FsmStatePayload` cũ với `current_state: String`
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FsmSnapshot {
    pub online: bool,
    /// Phase hiện tại — đã typed, backend có thể match trực tiếp
    pub current_phase: SystemPhase,
    /// Phase ngay trước đó — để frontend có thể hiện animation transition
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub previous_phase: Option<SystemPhase>,
    pub pump_status: PumpStatus,
    pub budgets: FsmBudgets,
    /// Dữ liệu chẩn đoán từ `LocalHealthAndDiagnostic` (optional để backward compat)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub diagnostics: Option<FsmDiagnostics>,
}

/// Thông tin chẩn đoán edge AI nhúng trong snapshot FSM
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FsmDiagnostics {
    pub health_score_percent: u32,
    pub ec_pump_streak: u32,
    pub ph_pump_streak: u32,
    pub water_hydraulics_streak: u32,
    pub adaptive_mixing_sec: u32,
    pub adaptive_stabilize_sec: u32,
    /// Số lần log bị drop do channel đầy
    #[serde(default)]
    pub log_drop_count: u32,

    // --- Các trường dữ liệu phục vụ tính toán nội bộ (Không gửi qua MQTT) ---
    #[serde(skip)]
    pub mixing_time_history_sec: [u32; 4],
    #[serde(skip)]
    pub stabilize_time_history_sec: [u32; 4],
    #[serde(skip)]
    pub history_head: usize,
    #[serde(skip)]
    pub history_count: usize,
}

impl Default for FsmDiagnostics {
    fn default() -> Self {
        Self {
            health_score_percent: 100,
            ec_pump_streak: 0,
            ph_pump_streak: 0,
            water_hydraulics_streak: 0,
            // Giá trị an toàn khởi tạo mặc định ban đầu
            adaptive_mixing_sec: 15,
            adaptive_stabilize_sec: 10,
            log_drop_count: 0,

            mixing_time_history_sec: [15; 4],
            stabilize_time_history_sec: [10; 4],
            history_head: 0,
            history_count: 0,
        }
    }
}

impl FsmDiagnostics {
    /// TỰ CHẨN ĐOÁN LỖI PHẦN CỨNG VẬT LÝ TOÀN DIỆN (RESIDUAL DIAGNOSTIC)
    #[allow(clippy::too_many_arguments)]
    pub fn diagnose_hardware_fault(
        &mut self,
        total_nutrient_ml: f32, // Dose A + Dose B
        total_ph_agent_ml: f32, // pH Up + pH Down
        water_in_sec: f32,
        water_out_sec: f32,
        actual_delta_ec: f32,
        actual_delta_ph: f32,
        actual_delta_water: f32,
        config: &crate::ControllerConfig,
    ) -> Result<(), FaultCode> {
        // --- 1. KIỂM TRA MẠCH CHÂM PHÂN EC ---
        if total_nutrient_ml > 1.0 {
            if actual_delta_ec < config.ec_ack_threshold {
                self.ec_pump_streak += 1;
                log::warn!(
                    "⚠️ [DIAGNOSTIC] Bất thường mạch dinh dưỡng lần {}! Bơm chạy {:.1}ml nhưng EC không nhích.",
                    self.ec_pump_streak,
                    total_nutrient_ml
                );

                if self.ec_pump_streak >= 3 {
                    log::error!(
                        "🚨 [HARDWARE FAULT] Khóa máy! Xác nhận nghẹt ống phân bón hoặc hết bình chứa thuốc."
                    );
                    return Err(FaultCode::EcDosingFailed);
                }
            } else {
                self.ec_pump_streak = 0; // Đạt mục tiêu -> Xóa trắng bộ đếm lỗi
            }
        }

        // --- 2. KIỂM TRA MẠCH HÓA CHẤT HIỆU CHỈNH pH ---
        if total_ph_agent_ml > 0.5 {
            if actual_delta_ph.abs() < config.ph_ack_threshold {
                self.ph_pump_streak += 1;
                log::warn!(
                    "⚠️ [DIAGNOSTIC] Bất thường mạch pH lần {}! Bơm chạy {:.1}ml nhưng pH đứng im.",
                    self.ph_pump_streak,
                    total_ph_agent_ml
                );

                if self.ph_pump_streak >= 3 {
                    log::error!(
                        "🚨 [HARDWARE FAULT] Khóa máy! Xác nhận bơm pH bị tụt áp, e-khí hoặc hết dung dịch axit/kiềm."
                    );
                    return Err(FaultCode::PhDosingFailed);
                }
            } else {
                self.ph_pump_streak = 0;
            }
        }

        // --- 3. KIỂM TRA MẠCH THỦY LỰC NƯỚC (WATER LEVEL UP/DOWN) ---
        if water_in_sec > 1.0 {
            if actual_delta_water < config.water_ack_threshold {
                self.water_hydraulics_streak += 1;
                log::warn!(
                    "⚠️ [DIAGNOSTIC] Bất thường cấp nước lần {}! Bơm chạy {:.1}s nhưng mực nước không tăng.",
                    self.water_hydraulics_streak,
                    water_in_sec
                );

                if self.water_hydraulics_streak >= 3 {
                    log::error!(
                        "🚨 [HARDWARE FAULT] Khóa máy! Xác nhận mất nước nguồn cấp hoặc cháy bơm cấp nước."
                    );
                    return Err(FaultCode::WaterRefillFailed);
                }
            } else {
                self.water_hydraulics_streak = 0;
            }
        } else if water_out_sec > 1.0 {
            if actual_delta_water > -config.water_ack_threshold {
                self.water_hydraulics_streak += 1;
                log::warn!(
                    "⚠️ [DIAGNOSTIC] Bất thường xả nước lần {}! Bơm xả chạy {:.1}s nhưng mực nước giữ nguyên.",
                    self.water_hydraulics_streak,
                    water_out_sec
                );

                if self.water_hydraulics_streak >= 3 {
                    log::error!(
                        "🚨 [HARDWARE FAULT] Khóa máy! Xác nhận tắc đường ống xả hoặc hỏng bơm thoát nước."
                    );
                    return Err(FaultCode::WaterDrainFailed);
                }
            } else {
                self.water_hydraulics_streak = 0;
            }
        }

        // Cập nhật lại Health Score sau mỗi lần chẩn đoán
        self.update_health_score();
        Ok(())
    }

    /// 🎯 CƠ CHẾ 2: TỰ HỌC THỂ TÍCH BỒN NƯỚC ĐỂ ĐIỀU CHỈNH THỜI GIAN KHUẤY/ĐỢI ĐỘNG
    pub fn learn_fluid_dynamics(&mut self, actual_mixing_ms: u64, actual_stabilize_ms: u64) {
        let mix_sec = (actual_mixing_ms / 1000) as u32;
        let stabilize_sec = (actual_stabilize_ms / 1000) as u32;

        self.mixing_time_history_sec[self.history_head] = mix_sec;
        self.stabilize_time_history_sec[self.history_head] = stabilize_sec;

        self.history_head = (self.history_head + 1) % 4;

        if self.history_count < 4 {
            self.history_count += 1;
        }

        let sum_mix: u32 = self.mixing_time_history_sec[0..self.history_count]
            .iter()
            .sum();
        let sum_stab: u32 = self.stabilize_time_history_sec[0..self.history_count]
            .iter()
            .sum();

        let avg_mix = sum_mix / self.history_count as u32;
        let avg_stab = sum_stab / self.history_count as u32;

        self.adaptive_mixing_sec = avg_mix.clamp(15, 120);
        self.adaptive_stabilize_sec = avg_stab.clamp(10, 90);

        log::info!(
            "🧠 [EDGE AI] Fluid-dynamics update: ActiveMixing = {}s, Stabilizing = {}s",
            self.adaptive_mixing_sec,
            self.adaptive_stabilize_sec
        );
    }

    /// TÍNH TOÁN VÀ LƯU LẠI ĐIỂM SỨC KHỎE
    pub fn update_health_score(&mut self) {
        let penalties = (self.ec_pump_streak * 33)
            + (self.ph_pump_streak * 33)
            + (self.water_hydraulics_streak * 33);

        self.health_score_percent = 100_u32.saturating_sub(penalties);
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct FsmBudgets {
    pub ec_ml: f32,
    pub ph_ml: f32,
    pub refill_count: u32,
    pub drain_count: u32,
}

// /// Legacy struct — giữ lại để backward compat với code backend cũ
// /// Dùng `FsmSnapshot` cho code mới
// #[deprecated(note = "Use FsmSnapshot instead — current_state is now typed as SystemPhase")]
// #[derive(Debug, Clone, Serialize, Deserialize)]
// pub struct FsmStatePayload {
//     pub online: bool,
//     pub current_state: String,
//     pub pump_status: PumpStatus,
//     pub budgets: FsmBudgets,
// }
