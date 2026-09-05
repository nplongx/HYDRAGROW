// src/core/optimizer.rs
use hydragrow_shared::ControllerConfig;
use log::warn;

use crate::WaterDirection;
use crate::core::adaptive::matrix::ControlVector;

/// Hàm này nhận đầu vào là ControlVector do Ma trận tính toán ra, duyệt qua các ranh giới
/// sinh tồn vật lý của cây trồng và thiết bị để cắt tỉa (truncate) hoặc khóa (interlock) dòng lệnh.
pub fn apply_safety_guardrails(
    control: &mut ControlVector,
    current_ec: f32,
    current_ph: f32,
    current_water_level: f32,
    config: &ControllerConfig,
    ec_a_gain_per_ml: f32,
    ec_b_gain_per_ml: f32,
) {
    // =========================================================================
    // LƯỚI 1: BẢO VỆ MỰC NƯỚC SINH TỒN & CHỐNG TRÀN BỒN (ƯU TIÊN CAO NHẤT)
    // =========================================================================
    // 1.1. Mực nước cạn nguy hiểm -> Cháy bơm, khóa toàn bộ hóa chất, ép cấp nước
    if current_water_level <= config.water_level_critical_min {
        warn!(
            "🚨 [GUARDRAIL] Mực nước cạn nguy hiểm ({:.1}cm <= {:.1}cm)! Khóa toàn bộ kênh châm hóa chất, ép chạy bơm cấp nước.",
            current_water_level, config.water_level_critical_min
        );
        control.nutrient_a_ml = 0.0;
        control.nutrient_b_ml = 0.0;
        control.ph_up_ml = 0.0;
        control.ph_down_ml = 0.0;
        control.water_out_sec = 0.0;
        control.misting_sec = 0.0;
        control.mixing_sec = 0.0;
        control.water_in_sec = config.max_refill_duration_sec as f32;
        return;
    }

    // 1.2. Mực nước đã đầy hoặc vượt trần -> Cấm tuyệt đối cấp thêm nước chống tràn
    if current_water_level >= config.water_level_max && control.water_in_sec > 0.0 {
        warn!(
            "⚠️ [GUARDRAIL] Mực nước bồn đã đầy ({:.1}cm >= {:.1}cm), triệt tiêu lệnh cấp nước chống tràn.",
            current_water_level, config.water_level_max
        );
        control.water_in_sec = 0.0;
    }

    // =========================================================================
    // LƯỚI 2: CHỐNG ĐỘC TÍNH DINH DƯỠNG & SỐC EC (EC OVERDOSE GUARD)
    // =========================================================================
    let total_nutrient_ml = control.nutrient_a_ml + control.nutrient_b_ml;
    let safe_ec_a_gain = ec_a_gain_per_ml.max(0.0001);
    let safe_ec_b_gain = ec_b_gain_per_ml.max(0.0001);
    let predicted_ec_gain =
        control.nutrient_a_ml * safe_ec_a_gain + control.nutrient_b_ml * safe_ec_b_gain;

    // 2.1. Đã vượt trần độc tính hoặc dự báo sau châm sẽ vượt trần
    if (current_ec >= config.max_ec_limit || (current_ec + predicted_ec_gain) > config.max_ec_limit)
        && total_nutrient_ml > 0.0
    {
        warn!(
            "⚠️ [GUARDRAIL] Chặn châm phân A/B: EC hiện tại ({:.2}) + dự tăng ({:.2}) vượt ngưỡng độc tính ({:.2})",
            current_ec, predicted_ec_gain, config.max_ec_limit
        );
        control.nutrient_a_ml = 0.0;
        control.nutrient_b_ml = 0.0;
    }

    // 2.2. Kiểm tra mức tăng vượt ngưỡng sốc tối đa trong 1 chu kỳ (max_ec_delta)
    if predicted_ec_gain > config.max_ec_delta && total_nutrient_ml > 0.0 {
        let weighted_avg_gain = ((control.nutrient_a_ml * safe_ec_a_gain)
            + (control.nutrient_b_ml * safe_ec_b_gain))
            / total_nutrient_ml.max(0.0001);
        let safe_total_ml = (config.max_ec_delta / weighted_avg_gain.max(0.0001)).max(0.0);
        let scale = safe_total_ml / total_nutrient_ml;
        warn!(
            "⚠️ [GUARDRAIL] Thu nhỏ liều phân bón từ {:.1}ml xuống {:.1}ml để khống chế ΔEC <= {:.2}",
            total_nutrient_ml, safe_total_ml, config.max_ec_delta
        );
        control.nutrient_a_ml *= scale;
        control.nutrient_b_ml *= scale;
    }

    // =========================================================================
    // LƯỚI 3: CHỐNG SỐC AXIT / KIỀM (pH BORDER & DELTA INTERLOCK)
    // =========================================================================
    // 3.1. Bảo vệ pH Down (Axit)
    if control.ph_down_ml > 0.0 {
        let predicted_ph_drop = control.ph_down_ml * config.ph_shift_down_per_ml;
        if current_ph <= config.min_ph_limit
            || (current_ph - predicted_ph_drop) < config.min_ph_limit
        {
            warn!(
                "🚨 [GUARDRAIL] Chặn pH Down: pH hiện tại ({:.2}) - dự giảm ({:.2}) chạm sàn an toàn ({:.2})",
                current_ph, predicted_ph_drop, config.min_ph_limit
            );
            control.ph_down_ml = 0.0;
        } else if predicted_ph_drop > config.max_ph_delta {
            let safe_ph_down_ml = config.max_ph_delta / config.ph_shift_down_per_ml.max(0.0001);
            warn!(
                "⚠️ [GUARDRAIL] Cắt tỉa pH Down từ {:.1}ml xuống {:.1}ml để khống chế ΔpH <= {:.2}",
                control.ph_down_ml, safe_ph_down_ml, config.max_ph_delta
            );
            control.ph_down_ml = safe_ph_down_ml;
        }
    }

    // 3.2. Bảo vệ pH Up (Kiềm)
    if control.ph_up_ml > 0.0 {
        let predicted_ph_rise = control.ph_up_ml * config.ph_shift_up_per_ml;
        if current_ph >= config.max_ph_limit
            || (current_ph + predicted_ph_rise) > config.max_ph_limit
        {
            warn!(
                "🚨 [GUARDRAIL] Chặn pH Up: pH hiện tại ({:.2}) + dự tăng ({:.2}) chạm trần an toàn ({:.2})",
                current_ph, predicted_ph_rise, config.max_ph_limit
            );
            control.ph_up_ml = 0.0;
        } else if predicted_ph_rise > config.max_ph_delta {
            let safe_ph_up_ml = config.max_ph_delta / config.ph_shift_up_per_ml.max(0.0001);
            warn!(
                "⚠️ [GUARDRAIL] Cắt tỉa pH Up từ {:.1}ml xuống {:.1}ml để khống chế ΔpH <= {:.2}",
                control.ph_up_ml, safe_ph_up_ml, config.max_ph_delta
            );
            control.ph_up_ml = safe_ph_up_ml;
        }
    }

    // =========================================================================
    // LƯỚI 4: TRIỆT TIÊU HÀNH VI ĐỐI KHÁNG THỦY LỰC & HÓA CHẤT
    // =========================================================================
    // 4.1. Triệt tiêu xung đột vừa cấp nước vừa xả nước
    if control.water_in_sec > 0.0 && control.water_out_sec > 0.0 {
        if control.water_in_sec >= control.water_out_sec {
            control.water_in_sec -= control.water_out_sec;
            control.water_out_sec = 0.0;
        } else {
            control.water_out_sec -= control.water_in_sec;
            control.water_in_sec = 0.0;
        }
    }

    // 4.2. Triệt tiêu xung đột vừa châm pH Up vừa châm pH Down
    if control.ph_up_ml > 0.0 && control.ph_down_ml > 0.0 {
        if control.ph_up_ml >= control.ph_down_ml {
            control.ph_up_ml -= control.ph_down_ml;
            control.ph_down_ml = 0.0;
        } else {
            control.ph_down_ml -= control.ph_up_ml;
            control.ph_up_ml = 0.0;
        }
    }

    // =========================================================================
    // LƯỚI 5: RÀNG BUỘC CÔNG SUẤT VẬT LÝ VÀ THỜI GIAN CHẠY TỐI ĐA
    // =========================================================================
    let total_ab = control.nutrient_a_ml + control.nutrient_b_ml;
    if total_ab > config.max_dose_per_cycle && total_ab > 0.0 {
        let scale = config.max_dose_per_cycle / total_ab;
        control.nutrient_a_ml *= scale;
        control.nutrient_b_ml *= scale;
    }

    // Chặn chống âm (phòng ngừa lỗi số học)
    control.nutrient_a_ml = control.nutrient_a_ml.max(0.0);
    control.nutrient_b_ml = control.nutrient_b_ml.max(0.0);

    control.ph_up_ml = control.ph_up_ml.clamp(0.0, config.max_dose_per_cycle);
    control.ph_down_ml = control.ph_down_ml.clamp(0.0, config.max_dose_per_cycle);

    control.water_in_sec = control
        .water_in_sec
        .clamp(0.0, config.max_refill_duration_sec as f32);
    control.water_out_sec = control
        .water_out_sec
        .clamp(0.0, config.max_drain_duration_sec as f32);
    control.mixing_sec = control.mixing_sec.clamp(0.0, 3600.0);
    control.misting_sec = control.misting_sec.clamp(0.0, 300.0);
}

pub const DEFAULT_WATER_RATE_CM_PER_SEC: f32 = 0.1;

#[derive(Debug, Clone, PartialEq)]
pub struct WaterPlan {
    pub direction: WaterDirection,
    pub target_level: f32,
    pub duration_sec: f32,
    pub amount_cm: f32,
}

/// Chuyển đổi hướng + lượng nước + tốc độ/hiệu chuẩn + thời gian tối đa thành một kế hoạch khả thi đã qua kiểm tra an toàn.
pub fn plan_water_operation(
    direction: WaterDirection,
    amount_cm: f32,
    current_water_level: f32,
    flow_cm_per_sec: Option<f32>,
    config: &ControllerConfig,
) -> Option<WaterPlan> {
    let flow_rate = flow_cm_per_sec
        .unwrap_or(DEFAULT_WATER_RATE_CM_PER_SEC)
        .max(0.001);

    match direction {
        WaterDirection::Out => {
            if current_water_level <= config.water_level_critical_min {
                warn!(
                    "🚨 [WATER_PLAN] Không thể xả nước: mực nước ({:.1}cm) <= ngưỡng nguy hiểm ({:.1}cm)",
                    current_water_level, config.water_level_critical_min
                );
                return None;
            }

            let max_safe_drain = (current_water_level - config.water_level_critical_min).max(0.0);
            if max_safe_drain <= 0.0 {
                return None;
            }

            let effective_amount = if amount_cm > 0.0 {
                amount_cm.min(max_safe_drain)
            } else {
                max_safe_drain
            };

            let target_level =
                (current_water_level - effective_amount).max(config.water_level_critical_min);
            let calculated_duration = if amount_cm > 0.0 {
                effective_amount / flow_rate
            } else {
                config.max_drain_duration_sec as f32
            };
            let duration_sec = calculated_duration.clamp(0.0, config.max_drain_duration_sec as f32);

            if duration_sec <= 0.0 {
                return None;
            }

            Some(WaterPlan {
                direction: WaterDirection::Out,
                target_level,
                duration_sec,
                amount_cm: effective_amount,
            })
        }
        WaterDirection::In => {
            if current_water_level >= config.water_level_max {
                warn!(
                    "⚠️ [WATER_PLAN] Không thể cấp nước: mực nước ({:.1}cm) >= trần bồn ({:.1}cm)",
                    current_water_level, config.water_level_max
                );
                return None;
            }

            let max_safe_refill = (config.water_level_max - current_water_level).max(0.0);
            if max_safe_refill <= 0.0 {
                return None;
            }

            let effective_amount = if amount_cm > 0.0 {
                amount_cm.min(max_safe_refill)
            } else {
                max_safe_refill
            };

            let target_level = (current_water_level + effective_amount).min(config.water_level_max);
            let calculated_duration = if amount_cm > 0.0 {
                effective_amount / flow_rate
            } else {
                config.max_refill_duration_sec as f32
            };
            let duration_sec =
                calculated_duration.clamp(0.0, config.max_refill_duration_sec as f32);

            if duration_sec <= 0.0 {
                return None;
            }

            Some(WaterPlan {
                direction: WaterDirection::In,
                target_level,
                duration_sec,
                amount_cm: effective_amount,
            })
        }
        WaterDirection::Stop => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn total_nutrient_dose_per_cycle_limit_preserves_ratio() {
        let mut config = ControllerConfig::default();
        config.max_dose_per_cycle = 10.0;
        config.max_ec_delta = 5.0; // High enough not to trigger delta guardrail
        config.max_ec_limit = 5.0;
        config.water_level_critical_min = 5.0;
        config.water_level_max = 30.0;

        let mut control = ControlVector {
            nutrient_a_ml: 6.0,
            nutrient_b_ml: 6.0,
            ..Default::default()
        };

        apply_safety_guardrails(
            &mut control,
            1.0,  // current_ec
            6.0,  // current_ph
            20.0, // current_water_level
            &config,
            0.01, // ec_a_gain_per_ml
            0.01, // ec_b_gain_per_ml
        );

        let total_ab = control.nutrient_a_ml + control.nutrient_b_ml;
        assert!(
            total_ab <= 10.0 + 1e-4,
            "Total A+B must be <= max_dose_per_cycle (10.0), got {total_ab}"
        );
        assert!(
            (control.nutrient_a_ml - 5.0).abs() < 1e-4,
            "Nutrient A should be scaled to 5.0, got {}",
            control.nutrient_a_ml
        );
        assert!(
            (control.nutrient_b_ml - 5.0).abs() < 1e-4,
            "Nutrient B should be scaled to 5.0, got {}",
            control.nutrient_b_ml
        );
    }

    #[test]
    fn total_nutrient_dose_per_cycle_limit_asymmetric_ratio() {
        let mut config = ControllerConfig::default();
        config.max_dose_per_cycle = 9.0;
        config.max_ec_delta = 5.0;
        config.max_ec_limit = 5.0;
        config.water_level_critical_min = 5.0;
        config.water_level_max = 30.0;

        let mut control = ControlVector {
            nutrient_a_ml: 12.0,
            nutrient_b_ml: 6.0,
            ..Default::default()
        };

        apply_safety_guardrails(&mut control, 1.0, 6.0, 20.0, &config, 0.01, 0.01);

        let total_ab = control.nutrient_a_ml + control.nutrient_b_ml;
        assert!(
            total_ab <= 9.0 + 1e-4,
            "Total A+B must be <= 9.0, got {total_ab}"
        );
        let ratio = control.nutrient_a_ml / control.nutrient_b_ml;
        assert!(
            (ratio - 2.0).abs() < 1e-4,
            "Ratio A:B must remain 2.0, got {ratio}"
        );
        assert!((control.nutrient_a_ml - 6.0).abs() < 1e-4);
        assert!((control.nutrient_b_ml - 3.0).abs() < 1e-4);
    }
}
