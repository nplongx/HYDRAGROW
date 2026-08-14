import dashboard
import gleeunit/should

pub fn eval_sensor_status_test() {
  // Cảm biến bình thường
  let status = dashboard.eval_sensor_status_safe(False, "1.5", "0.5", "3.0")
  status.label |> should.equal("Ổn định")
  status.tone |> should.equal("good")

  // Giá trị vượt ngưỡng cao
  let high_status =
    dashboard.eval_sensor_status_safe(False, "4.0", "0.5", "3.0")
  high_status.label |> should.equal("Cao")
  high_status.tone |> should.equal("warn")

  // Cảm biến bị báo lỗi phần cứng
  let err_status = dashboard.eval_sensor_status_safe(True, "1.5", "0.5", "3.0")
  err_status.label |> should.equal("Cần kiểm tra")
  err_status.tone |> should.equal("danger")
}

pub fn calc_hourly_dose_str_test() {
  let now = 10_000_000.0
  let ts_recent = "9900000.0"
  // 100,000 ms trước (< 1 giờ) -> ĐƯỢC TÍNH
  let ts_old = "5000000.0"

  // 5,000,000 ms trước (> 1 giờ) -> BỊ LỌC BỎ
  let events_str = ts_recent <> ",10.0,5.0;" <> ts_old <> ",20.0,10.0"

  let result = dashboard.calc_hourly_dose_str(events_str, now)
  result.ec_ml |> should.equal(10.0)
  result.ph_ml |> should.equal(5.0)
}
