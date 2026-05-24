use hydragrow_shared::{fsm::FaultCode, ControllerConfig};
use log::{error, info, warn};

use crate::fsm::PendingCalibrationSample;

#[derive(Debug, Clone)]
pub struct LocalHealthAndDiagnostic {
    // Bộ đếm lỗi liên tiếp để tránh báo động giả do bọt khí hoặc nhiễu tức thời
    pub consecutive_ec_anomalies: u32,
    pub consecutive_ph_anomalies: u32,
    pub consecutive_water_anomalies: u32,

    // Tầng tự học thời gian (Adaptive Time Learning) thay thế cho hằng số config tĩnh
    pub adaptive_mixing_sec: u32,
    pub adaptive_stabilize_sec: u32,

    // Mảng phẳng lưu trữ lịch sử thời gian đạt trạng thái bão hòa phẳng của 4 chu kỳ gần nhất
    pub mixing_time_history_sec: [u32; 4],
    pub stabilize_time_history_sec: [u32; 4],
    pub history_head: usize,
    pub history_count: usize,
}

impl Default for LocalHealthAndDiagnostic {
    fn default() -> Self {
        Self {
            consecutive_ec_anomalies: 0,
            consecutive_ph_anomalies: 0,
            consecutive_water_anomalies: 0,
            // Giá trị an toàn khởi tạo mặc định ban đầu
            adaptive_mixing_sec: 15,
            adaptive_stabilize_sec: 10,
            mixing_time_history_sec: [15; 4],
            stabilize_time_history_sec: [10; 4],
            history_head: 0,
            history_count: 0,
        }
    }
}

impl LocalHealthAndDiagnostic {
    /// 🛡️ CƠ CHẾ 1: TỰ CHẨN ĐOÁN LỖI PHẦN CỨNG VẬT LÝ TOÀN DIỆN (RESIUDAL DIAGNOSTIC)
    /// Đối chiếu kết quả đo bão hòa thực tế với lượng ml/giây mà rơ-le đã kích hoạt để phát hiện
    /// các sự cố: Hết dung dịch, nghẹt kim bơm, đứt ống silicon, e-khí hoặc hỏng van một chiều.
    pub fn diagnose_hardware_fault(
        &mut self,
        sample: &PendingCalibrationSample,
        actual_delta_ec: f32,
        actual_delta_ph: f32,
        actual_delta_water: f32, // Bổ sung trục nước để xử lý trọn vẹn MIMO
        config: &ControllerConfig,
    ) -> Result<(), FaultCode> {
        // --- 1. KIỂM TRA MẠCH CHÂM PHÂN EC ---
        let total_nutrient_ml = sample.dose_a_ml + sample.dose_b_ml;
        if total_nutrient_ml > 1.0 {
            // Chỉ chẩn đoán khi liều lượng châm đủ lớn để tạo ra biến thiên lý thuyết
            if actual_delta_ec < config.ec_ack_threshold {
                self.consecutive_ec_anomalies += 1;
                warn!("⚠️ [DIAGNOSTIC] Bất thường mạch dinh dưỡng lần {}! Bơm chạy {:.1}ml nhưng EC không nhích.", 
                    self.consecutive_ec_anomalies, total_nutrient_ml);

                if self.consecutive_ec_anomalies >= 3 {
                    error!("🚨 [HARDWARE FAULT] Khóa máy! Xác nhận nghẹt ống phân bón hoặc hết bình chứa thuốc.");
                    return Err(FaultCode::EcDosingFailed);
                }
            } else {
                self.consecutive_ec_anomalies = 0; // Đạt mục tiêu -> Xóa trắng bộ đếm lỗi
            }
        }

        // --- 2. KIỂM TRA MẠCH HÓA CHẤT HIỆU CHỈNH pH ---
        let total_ph_agent_ml = sample.dose_ph_up_ml + sample.dose_ph_down_ml;
        if total_ph_agent_ml > 0.5 {
            if actual_delta_ph.abs() < config.ph_ack_threshold {
                self.consecutive_ph_anomalies += 1;
                warn!(
                    "⚠️ [DIAGNOSTIC] Bất thường mạch pH lần {}! Bơm chạy {:.1}ml nhưng pH đứng im.",
                    self.consecutive_ph_anomalies, total_ph_agent_ml
                );

                if self.consecutive_ph_anomalies >= 3 {
                    error!("🚨 [HARDWARE FAULT] Khóa máy! Xác nhận bơm pH bị tụt áp, e-khí hoặc hết dung dịch axit/kiềm.");
                    return Err(FaultCode::PhDosingFailed);
                }
            } else {
                self.consecutive_ph_anomalies = 0;
            }
        }

        // --- 3. KIỂM TRA MẠCH THỦY LỰC NƯỚC (WATER LEVEL UP/DOWN) ---
        // Nếu bơm cấp nước In hoạt động (>1 giây) nhưng mực nước tăng không đạt ngưỡng tối thiểu
        if sample.water_in_sec > 1.0 {
            if actual_delta_water < config.water_ack_threshold {
                self.consecutive_water_anomalies += 1;
                warn!("⚠️ [DIAGNOSTIC] Bất thường cấp nước lần {}! Bơm chạy {:.1}s nhưng mực nước không tăng.", 
                    self.consecutive_water_anomalies, sample.water_in_sec);

                if self.consecutive_water_anomalies >= 3 {
                    error!("🚨 [HARDWARE FAULT] Khóa máy! Xác nhận mất nước nguồn cấp hoặc cháy bơm cấp nước.");
                    return Err(FaultCode::WaterRefillFailed);
                }
            } else {
                self.consecutive_water_anomalies = 0;
            }
        }
        // Nếu bơm xả nước Out hoạt động (>1 giây) nhưng mực nước không chịu tụt giảm
        else if sample.water_out_sec > 1.0 {
            if actual_delta_water > -config.water_ack_threshold {
                self.consecutive_water_anomalies += 1;
                warn!("⚠️ [DIAGNOSTIC] Bất thường xả nước lần {}! Bơm xả chạy {:.1}s nhưng mực nước giữ nguyên.", 
                    self.consecutive_water_anomalies, sample.water_out_sec);

                if self.consecutive_water_anomalies >= 3 {
                    error!("🚨 [HARDWARE FAULT] Khóa máy! Xác nhận tắc đường ống xả hoặc hỏng bơm thoát nước.");
                    return Err(FaultCode::WaterDrainFailed);
                }
            } else {
                self.consecutive_water_anomalies = 0;
            }
        }

        Ok(())
    }

    /// 🎯 CƠ CHẾ 2: TỰ HỌC THỂ TÍCH BỒN NƯỚC ĐỂ ĐIỀU CHỈNH THỜI GIAN KHUẤY/ĐỢI ĐỘNG (CIRCULAR BUFFER LEARNING)
    pub fn learn_fluid_dynamics(&mut self, actual_mixing_ms: u64, actual_stabilize_ms: u64) {
        let mix_sec = (actual_mixing_ms / 1000) as u32;
        let stabilize_sec = (actual_stabilize_ms / 1000) as u32;

        self.mixing_time_history_sec[self.history_head] = mix_sec;
        self.stabilize_time_history_sec[self.history_head] = stabilize_sec;

        // Tiến con trỏ mảng vòng tròn
        self.history_head = (self.history_head + 1) % 4;

        if self.history_count < 4 {
            self.history_count += 1;
        }

        // Tính toán toán học mượt bằng đường trung bình động (Moving Average)
        let sum_mix: u32 = self.mixing_time_history_sec[0..self.history_count]
            .iter()
            .sum();
        let sum_stab: u32 = self.stabilize_time_history_sec[0..self.history_count]
            .iter()
            .sum();

        let avg_mix = sum_mix / self.history_count as u32;
        let avg_stab = sum_stab / self.history_count as u32;

        // Ghi đè cấu hình động, bọc ranh giới cứng bảo vệ chống kịch trần bộ nhớ đệm
        self.adaptive_mixing_sec = avg_mix.clamp(15, 120);
        self.adaptive_stabilize_sec = avg_stab.clamp(10, 90);

        info!(
            "🧠 [EDGE AI] Fluid-dynamics update: ActiveMixing = {}s, Stabilizing = {}s",
            self.adaptive_mixing_sec, self.adaptive_stabilize_sec
        );
    }

    /// TÍNH TOÁN ĐIỂM SỨC KHỎE HỆ THỐNG ĐỂ TRUYỀN THÔNG (EDGE HEALTH SCORE)
    /// Xuất ra phần trăm sống sót từ 0% đến 100% của thiết bị dựa trên các bất thường tích lũy.
    pub fn calculate_health_score(&self) -> u32 {
        let penalties = (self.consecutive_ec_anomalies * 33)
            + (self.consecutive_ph_anomalies * 33)
            + (self.consecutive_water_anomalies * 33);

        100_u32.saturating_sub(penalties)
    }

    /// Đóng gói nhanh trạng thái chẩn đoán để nhúng vào MQTT Payload chính
    pub fn to_telemetry_json(&self) -> serde_json::Value {
        serde_json::json!({
            "health_score_percent": self.calculate_health_score(),
            "anomalies": {
                "ec_pump_streak": self.consecutive_ec_anomalies,
                "ph_pump_streak": self.consecutive_ph_anomalies,
                "water_hydraulics_streak": self.consecutive_water_anomalies
            },
            "edge_learned_timers": {
                "active_mixing_duration_sec": self.adaptive_mixing_sec,
                "sensor_stabilize_duration_sec": self.adaptive_stabilize_sec
            }
        })
    }
}
