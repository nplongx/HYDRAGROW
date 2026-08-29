//! Mirror thuần-Rust của rule "hourly dose budget" mà firmware enforce trong
//! `hydragrow-controller-core::core::actors::safety_guard::SafetyGuard::check_hourly_dose`.
//! Sống ở đây (không copy sang backend riêng) theo module-rules/shared.md rule #2:
//! "Không đặt logic dùng chung ở 2 nơi." Thuần dữ liệu vào/ra, không I/O — test được
//! trên host mà không cần DB hay MQTT (theo đúng tinh thần của controller-core.md
//! cho module `adaptive/`).

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct DoseSafetyLimits {
    pub max_dose_per_cycle_ml: f32,
    pub max_dose_per_hour_ml: f32,
    pub cooldown_sec: u64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum DoseSafetyViolation {
    ExceedsPerCycleLimit {
        requested_ml: f32,
        max_ml: f32,
    },
    ExceedsHourlyBudget {
        requested_ml: f32,
        already_dosed_ml: f32,
        max_ml: f32,
    },
    CooldownActive {
        seconds_remaining: u64,
    },
}

/// Kiểm tra một liều đề xuất so với giới hạn an toàn. Gọi TRƯỚC khi publish bất kỳ
/// lệnh dosing nào — caller (backend) chịu trách nhiệm truyền đúng lịch sử liều
/// trong 1 giờ gần nhất (`hourly_history_ml`); hàm này không tự lưu trạng thái.
pub fn check_dose(
    limits: &DoseSafetyLimits,
    hourly_history_ml: &[(u64, f32)],
    now_sec: u64,
    last_dose_at_sec: Option<u64>,
    requested_ml: f32,
) -> Result<(), DoseSafetyViolation> {
    if requested_ml > limits.max_dose_per_cycle_ml {
        return Err(DoseSafetyViolation::ExceedsPerCycleLimit {
            requested_ml,
            max_ml: limits.max_dose_per_cycle_ml,
        });
    }

    if let Some(last) = last_dose_at_sec {
        let elapsed = now_sec.saturating_sub(last);
        if elapsed < limits.cooldown_sec {
            return Err(DoseSafetyViolation::CooldownActive {
                seconds_remaining: limits.cooldown_sec - elapsed,
            });
        }
    }

    let already_dosed_ml: f32 = hourly_history_ml
        .iter()
        .filter(|(ts, _)| now_sec.saturating_sub(*ts) <= 3600)
        .map(|(_, ml)| *ml)
        .sum();

    if already_dosed_ml + requested_ml > limits.max_dose_per_hour_ml {
        return Err(DoseSafetyViolation::ExceedsHourlyBudget {
            requested_ml,
            already_dosed_ml,
            max_ml: limits.max_dose_per_hour_ml,
        });
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn limits() -> DoseSafetyLimits {
        DoseSafetyLimits {
            max_dose_per_cycle_ml: 10.0,
            max_dose_per_hour_ml: 30.0,
            cooldown_sec: 60,
        }
    }

    #[test]
    fn allows_dose_within_all_limits() {
        let result = check_dose(&limits(), &[], 1_000, None, 5.0);
        assert!(result.is_ok());
    }

    #[test]
    fn denies_dose_exceeding_per_cycle_limit() {
        let result = check_dose(&limits(), &[], 1_000, None, 15.0);
        assert_eq!(
            result,
            Err(DoseSafetyViolation::ExceedsPerCycleLimit {
                requested_ml: 15.0,
                max_ml: 10.0
            })
        );
    }

    #[test]
    fn denies_dose_during_cooldown() {
        let result = check_dose(&limits(), &[], 1_000, Some(970), 5.0); // 30s trôi qua, cooldown 60s
        assert_eq!(
            result,
            Err(DoseSafetyViolation::CooldownActive {
                seconds_remaining: 30
            })
        );
    }

    #[test]
    fn denies_dose_exceeding_hourly_budget() {
        let history = vec![(500u64, 25.0f32)]; // đã bơm 25ml trong giờ qua
        let result = check_dose(&limits(), &history, 1_000, None, 8.0); // 25+8 > 30
        assert_eq!(
            result,
            Err(DoseSafetyViolation::ExceedsHourlyBudget {
                requested_ml: 8.0,
                already_dosed_ml: 25.0,
                max_ml: 30.0
            })
        );
    }

    #[test]
    fn ignores_hourly_history_entries_older_than_one_hour() {
        let now_sec = 10_000u64;
        let history = vec![(now_sec.saturating_sub(3601), 25.0f32)]; // ngoài cửa sổ 1h
        let result = check_dose(&limits(), &history, now_sec, None, 8.0);
        assert!(result.is_ok());
    }
}
