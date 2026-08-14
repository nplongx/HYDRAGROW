import gleam/option.{type Option, None, Some}

pub type FaultGuide {
  FaultGuide(
    code: String,
    short: String,
    reason: String,
    action: String,
    recovery: String,
  )
}

pub fn get_fault_guide(code: String) -> Option(FaultGuide) {
  case code {
    "MAX_HOURLY_DOSE_EC" ->
      Some(FaultGuide(
        code: "MAX_HOURLY_DOSE_EC",
        short: "Đạt giới hạn châm EC theo giờ",
        reason: "Đã châm nhiều phân EC trong vòng 1 giờ qua.",
        action: "Chờ 1 giờ hoặc thực hiện Reset thủ công.",
        recovery: "Tự khôi phục khi hết giới hạn rate-limit.",
      ))
    "MAX_HOURLY_DOSE_PH" ->
      Some(FaultGuide(
        code: "MAX_HOURLY_DOSE_PH",
        short: "Đạt giới hạn châm pH theo giờ",
        reason: "Đã châm nhiều dung dịch pH trong 1 giờ qua.",
        action: "Kiểm tra cảm biến pH, sau đó nhấn Reset.",
        recovery: "Tự khôi phục khi hết giới hạn rate-limit.",
      ))
    "EC_DOSING_FAILED" ->
      Some(FaultGuide(
        code: "EC_DOSING_FAILED",
        short: "Châm EC thất bại sau 3 lần thử",
        reason: "Bơm châm đã chạy nhưng chỉ số EC không tăng.",
        action: "Kiểm tra bình A/B còn dung dịch không hoặc bơm bị nghẽn.",
        recovery: "Khắc phục nguyên nhân vật lý sau đó nhấn Reset.",
      ))
    "PH_DOSING_FAILED" ->
      Some(FaultGuide(
        code: "PH_DOSING_FAILED",
        short: "Châm pH thất bại sau 3 lần thử",
        reason: "Bơm pH đã chạy nhưng chỉ số pH không đổi.",
        action: "Kiểm tra bình pH Up/Down và van một chiều.",
        recovery: "Khắc phục nguyên nhân vật lý sau đó nhấn Reset.",
      ))
    "WATER_REFILL_FAILED" ->
      Some(FaultGuide(
        code: "WATER_REFILL_FAILED",
        short: "Cấp nước thất bại sau 3 lần thử",
        reason: "Bơm cấp nước đã bật nhưng mực nước không tăng.",
        action: "Kiểm tra phao báo mực nước, nguồn nước cấp và van.",
        recovery: "Khắc phục nguồn nước và nhấn Reset.",
      ))
    _ -> None
  }
}
