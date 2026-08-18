
// src/core/optimizer.rs
use hydragrow_shared::ControllerConfig;
use log::warn;

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
    if current_ec >= config.max_ec_limit || (current_ec + predicted_ec_gain) > config.max_ec_limit {
        if total_nutrient_ml > 0.0 {
            warn!(
                "⚠️ [GUARDRAIL] Chặn châm phân A/B: EC hiện tại ({:.2}) + dự tăng ({:.2}) vượt ngưỡng độc tính ({:.2})",
                current_ec, predicted_ec_gain, config.max_ec_limit
            );
            control.nutrient_a_ml = 0.0;
            control.nutrient_b_ml = 0.0;
        }
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
        if current_ph <= config.min_ph_limit || (current_ph - predicted_ph_drop) < config.min_ph_limit {
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
        if current_ph >= config.max_ph_limit || (current_ph + predicted_ph_rise) > config.max_ph_limit {
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
    control.nutrient_a_ml = control.nutrient_a_ml.clamp(0.0, config.max_dose_per_cycle);
    control.nutrient_b_ml = control.nutrient_b_ml.clamp(0.0, config.max_dose_per_cycle);
    control.ph_up_ml = control.ph_up_ml.clamp(0.0, config.max_dose_per_cycle);
    control.ph_down_ml = control.ph_down_ml.clamp(0.0, config.max_dose_per_cycle);

    control.water_in_sec = control.water_in_sec.clamp(0.0, config.max_refill_duration_sec as f32);
    control.water_out_sec = control.water_out_sec.clamp(0.0, config.max_drain_duration_sec as f32);
    control.mixing_sec = control.mixing_sec.clamp(0.0, 3600.0);
    control.misting_sec = control.misting_sec.clamp(0.0, 300.0);
}
