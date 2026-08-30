//! Công thức thể tích ↔ thời lượng bơm định lượng, thuần Rust — mirror của công
//! thức đã chạy thật trong `hydragrow-backend/src/api/control.rs::
//! validate_manual_dose_safety` (`capacity_ml_per_sec * (pwm/100) * duration_sec`).
//! Đặt ở đây để Action blocks (script `action_command`) và endpoint manual-control
//! dùng CHUNG một công thức, không lệch nhau theo module-rules/shared.md rule #2.
//! `api/control.rs` hiện vẫn giữ bản copy riêng (không refactor trong Phase này để
//! tránh đụng vào code manual-control đã test kỹ) — ghi nhận là tech-debt nhỏ.

/// Ước lượng ml sẽ được bơm ra với PWM và thời lượng cho trước.
pub fn estimate_ml(capacity_ml_per_sec: f32, pwm_percent: u32, duration_sec: u64) -> f32 {
    capacity_ml_per_sec * (pwm_percent as f32 / 100.0) * duration_sec as f32
}

/// Nghịch đảo của `estimate_ml`: cần bơm bao nhiêu giây để đạt `target_ml` ở PWM
/// cho trước. Làm tròn LÊN (ceil) — thà bơm dư một chút thời lượng còn hơn bơm
/// thiếu so với yêu cầu (an toàn hơn cho phía "không đủ liều" chứ không phải phía
/// ngược lại; `check_dose` ở `safety.rs` vẫn chặn nếu tổng vượt ngưỡng).
/// Trả `None` nếu capacity hoặc pwm bằng 0 (không thể bơm được gì).
pub fn ml_to_duration_sec(
    capacity_ml_per_sec: f32,
    pwm_percent: u32,
    target_ml: f32,
) -> Option<u64> {
    if capacity_ml_per_sec <= 0.0 || pwm_percent == 0 {
        return None;
    }
    let rate_ml_per_sec = capacity_ml_per_sec * (pwm_percent as f32 / 100.0);
    if rate_ml_per_sec <= 0.0 {
        return None;
    }
    Some((target_ml / rate_ml_per_sec).ceil() as u64)
}

/// Mirror của `normalize_dosing_pump_name` (private) trong `api/control.rs` —
/// đặt lại ở đây vì action_command dispatch (backend) cần cùng logic mà không
/// được phép import 1 hàm private từ module khác.
pub fn normalize_dosing_pump_name(pump: &str) -> Option<&'static str> {
    match pump {
        "A" | "PUMP_A" => Some("PUMP_A"),
        "B" | "PUMP_B" => Some("PUMP_B"),
        "PH_UP" => Some("PH_UP"),
        "PH_DOWN" => Some("PH_DOWN"),
        _ => None,
    }
}

/// Nhận 4 field capacity rời (không nhận `DosingCalibration` struct — struct đó
/// sống ở `hydragrow-backend`, và `hydragrow-shared` không được phép phụ thuộc
/// ngược lại backend theo module-rules).
pub fn capacity_ml_per_sec_for_pump(
    pump_a: f32,
    pump_b: f32,
    ph_up: f32,
    ph_down: f32,
    normalized_pump: &str,
) -> f32 {
    match normalized_pump {
        "PUMP_A" => pump_a,
        "PUMP_B" => pump_b,
        "PH_UP" => ph_up,
        "PH_DOWN" => ph_down,
        _ => 0.0,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn estimate_ml_matches_manual_control_formula() {
        // capacity=1.2ml/s, pwm=50%, 10s → 1.2 * 0.5 * 10 = 6.0ml
        // (đúng công thức trong api/control.rs::validate_manual_dose_safety)
        assert!((estimate_ml(1.2, 50, 10) - 6.0).abs() < 1e-4);
    }

    #[test]
    fn ml_to_duration_sec_is_inverse_of_estimate_ml() {
        let duration = ml_to_duration_sec(1.2, 50, 6.0).unwrap();
        assert_eq!(duration, 10);
    }

    #[test]
    fn ml_to_duration_sec_rounds_up_so_dose_is_never_under_delivered() {
        // 1.2 * 1.0 * duration = 5.0 → duration = 4.1666...s → phải làm tròn LÊN thành 5s
        let duration = ml_to_duration_sec(1.2, 100, 5.0).unwrap();
        assert_eq!(duration, 5);
    }

    #[test]
    fn ml_to_duration_sec_returns_none_for_zero_capacity() {
        assert_eq!(ml_to_duration_sec(0.0, 100, 5.0), None);
    }

    #[test]
    fn ml_to_duration_sec_returns_none_for_zero_pwm() {
        assert_eq!(ml_to_duration_sec(1.2, 0, 5.0), None);
    }

    #[test]
    fn normalize_dosing_pump_name_accepts_legacy_and_canonical_aliases() {
        assert_eq!(normalize_dosing_pump_name("A"), Some("PUMP_A"));
        assert_eq!(normalize_dosing_pump_name("PUMP_A"), Some("PUMP_A"));
        assert_eq!(normalize_dosing_pump_name("PH_DOWN"), Some("PH_DOWN"));
        assert_eq!(normalize_dosing_pump_name("NOT_A_PUMP"), None);
    }

    #[test]
    fn capacity_ml_per_sec_for_pump_picks_the_right_field() {
        assert_eq!(
            capacity_ml_per_sec_for_pump(1.0, 2.0, 3.0, 4.0, "PH_DOWN"),
            4.0
        );
        assert_eq!(
            capacity_ml_per_sec_for_pump(1.0, 2.0, 3.0, 4.0, "UNKNOWN"),
            0.0
        );
    }
}
