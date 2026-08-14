import gleam/option.{None, Some}
import gleeunit/should
import settings/calibration
import settings/cron
import settings/validation

pub fn validate_dosing_config_test() {
  // Dữ liệu hợp lệ -> Trả về danh sách lỗi rỗng
  validation.validate_dosing_config("50", "20", "1.2", "1.2", "1.2", "1.2")
  |> should.equal([])

  // PWM vượt quá 100% -> Trả về lỗi
  let errors =
    validation.validate_dosing_config("150", "20", "1.2", "1.2", "1.2", "1.2")
  case errors {
    [err, ..] -> err.field |> should.equal("dosing_pwm_percent")
    _ -> panic as "Kỳ vọng có lỗi validation PWM"
  }
}

pub fn parse_cron_safe_test() {
  // Parse chuỗi Cron 6 trường hợp lệ
  let schedule = cron.parse_cron_safe("0 30 14 * * MON,WED")
  schedule.minute |> should.equal(30)
  schedule.hour |> should.equal(14)
  schedule.is_every_day |> should.equal(False)
  schedule.days_str |> should.equal("MON,WED")

  // Chuỗi Cron hỏng -> Fallback về 08:00 hằng ngày
  let fallback = cron.parse_cron_safe("invalid_cron_string")
  fallback.hour |> should.equal(8)
  fallback.minute |> should.equal(0)
  fallback.is_every_day |> should.equal(True)
}

pub fn calculate_calibration_summary_test() {
  // Khoảng chênh áp giữa pH 7 và pH 4 là 1.0V (>= 0.2V) -> Được cộng 15 điểm thưởng
  let summary = calibration.calculate_summary(Some(2.5), Some(1.5), 80)
  summary.reliability |> should.equal(95)
}
