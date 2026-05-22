use crate::fsm::matrix::ControlVector;
use crate::fsm::utils::soft_deadband_scale;
use hydragrow_shared::ControllerConfig;
use log::warn;

pub fn apply_deadband(delta: f32, tolerance: f32) -> f32 {
    soft_deadband_scale(delta.max(0.0), tolerance.max(0.0))
}

// pub fn confidence_from_error_ratio(target: f32, predicted: f32) -> f32 {
//     let denom = target.abs().max(0.0001);
//     (1.0 - ((target - predicted).abs() / denom)).clamp(0.0, 1.0)
// }

/// Hàm này nhận đầu vào là ControlVector do Ma trận tính toán ra, duyệt qua các ranh giới
/// sinh tồn vật lý của cây trồng và thiết bị để cắt tỉa (truncate) hoặc khóa (interlock) dòng lệnh.
pub fn apply_safety_guardrails(
    control: &mut ControlVector,
    current_ec: f32,
    current_ph: f32,
    current_water_level: f32,
    config: &ControllerConfig,
) {
    // --- LƯỚI 1: BẢO VỆ MỰC NƯỚC SINH TỒN (MỨC ƯU TIÊN CAO NHẤT) ---
    // Nếu mực nước chạm ranh giới critical_min, nguy cơ cháy bơm Osaka/bơm sục rất cao.
    // Toàn bộ các cơ cấu châm hóa chất, dinh dưỡng, phun sương phải bị KHÓA CHẶT VỀ 0.
    // Hệ thống chỉ cho phép duy nhất kênh water_in_sec hoạt động hết công suất.
    if current_water_level <= config.water_level_critical_min {
        warn!("🚨 [GUARDRAIL] Mực nước chạm ngưỡng nguy hiểm ({:.1}cm)! Khóa toàn bộ các kênh châm, ép chạy bơm cấp.", current_water_level);
        control.nutrient_a_ml = 0.0;
        control.nutrient_b_ml = 0.0;
        control.ph_up_ml = 0.0;
        control.ph_down_ml = 0.0;
        control.water_out_sec = 0.0;
        control.misting_sec = 0.0;
        control.mixing_sec = 0.0;
        // Bơm nước vào hết công suất cấu hình
        control.water_in_sec = config.max_refill_duration_sec as f32;
        return;
    }

    // --- LƯỚI 2: CHỐNG ĐỘC TÍNH DINH DƯỠNG (EC OVERDOSE GUARD) ---
    // Nếu chỉ số EC thực tế đo được đã vượt ngưỡng max_ec_limit an toàn của cây,
    // hoặc kết quả châm ma trận tính toán cộng dồn làm vượt quá ranh giới, khóa lập tức bơm phân A, B.
    if current_ec >= config.max_ec_limit
        || (current_ec + (control.nutrient_a_ml * config.ec_gain_per_ml)) > config.max_ec_limit
    {
        if control.nutrient_a_ml > 0.0 {
            warn!("⚠️ [GUARDRAIL] Chặn lệnh châm phân dinh dưỡng do EC hiện tại ({:.2}) tiến sát giới hạn độc tính ({:.2})", current_ec, config.max_ec_limit);
            control.nutrient_a_ml = 0.0;
            control.nutrient_b_ml = 0.0;
        }
    }

    // --- LƯỚI 3: CHỐNG SỐC AXIT / KIỀM (pH BORDER INTERLOCK) ---
    // Kiểm tra ranh giới pH sinh tồn [min_ph_limit -> max_ph_limit].
    if current_ph <= config.min_ph_limit && control.ph_down_ml > 0.0 {
        warn!(
            "🚨 [GUARDRAIL] Cực đoan: pH quá thấp ({:.2})! Cấm tuyệt đối châm thêm pH Down.",
            current_ph
        );
        control.ph_down_ml = 0.0;
    }
    if current_ph >= config.max_ph_limit && control.ph_up_ml > 0.0 {
        warn!(
            "🚨 [GUARDRAIL] Cực đoan: pH quá cao ({:.2})! Cấm tuyệt đối châm thêm pH Up.",
            current_ph
        );
        control.ph_up_ml = 0.0;
    }

    // --- LƯỚI 4: TRIỆT TIÊU HÀNH VI ĐỐI KHÁNG THỦY LỰC ---
    // Ma trận trong quá trình học (chưa hội tụ hoàn toàn) có thể đưa ra nghiệm đồng thời:
    // Vừa bật bơm nước vào (Water In) vừa bật bơm xả nước (Water Out).
    // Bộ lọc này sẽ lấy hiệu số để giữ lại duy nhất một hướng dòng chảy tối ưu.
    if control.water_in_sec > 0.0 && control.water_out_sec > 0.0 {
        if control.water_in_sec >= control.water_out_sec {
            control.water_in_sec -= control.water_out_sec;
            control.water_out_sec = 0.0;
        } else {
            control.water_out_sec -= control.water_in_sec;
            control.water_in_sec = 0.0;
        }
    }

    // Triệt tiêu hành vi châm đối kháng cả pH Up và pH Down
    if control.ph_up_ml > 0.0 && control.ph_down_ml > 0.0 {
        if control.ph_up_ml >= control.ph_down_ml {
            control.ph_up_ml -= control.ph_down_ml;
            control.ph_down_ml = 0.0;
        } else {
            control.ph_down_ml -= control.ph_up_ml;
            control.ph_up_ml = 0.0;
        }
    }
}
