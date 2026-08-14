import gleam/int
import gleam/list
import gleam/result
import gleam/string

pub type CronSchedule {
  CronSchedule(minute: Int, hour: Int, is_every_day: Bool, days_str: String)
}

/// Parse chuỗi Cron chuẩn
pub fn parse_cron(cron_str: String) -> Result(CronSchedule, Nil) {
  let parts =
    cron_str
    |> string.trim
    |> string.split(" ")
    |> list.filter(fn(s) { s != "" })

  case parts {
    [_sec, min_str, hour_str, _dom, _month, dow_str, ..] -> {
      use minute <- result.try(int.parse(min_str))
      use hour <- result.try(int.parse(hour_str))

      let #(is_every_day, days_str) = case dow_str {
        "*" -> #(True, "")
        _ -> #(False, dow_str)
      }

      Ok(CronSchedule(
        minute: minute,
        hour: hour,
        is_every_day: is_every_day,
        days_str: days_str,
      ))
    }
    _ -> Error(Nil)
  }
}

/// HÀM DÀNH RIÊNG CHO JS/REACT:
/// Tự động fallback về giá trị mặc định khi gặp lỗi.
/// Trả về Object trực tiếp (không bọc trong Result) giúp React đọc thuộc tính an toàn.
pub fn parse_cron_safe(cron_str: String) -> CronSchedule {
  case parse_cron(cron_str) {
    Ok(schedule) -> schedule
    Error(_) ->
      CronSchedule(minute: 0, hour: 8, is_every_day: True, days_str: "")
  }
}

/// Convert ngược lại thành chuỗi Cron từ thông số JS
pub fn to_cron_string(
  minute: Int,
  hour: Int,
  is_every_day: Bool,
  days_str: String,
) -> String {
  let dow_part = case is_every_day, days_str {
    True, _ -> "*"
    False, "" -> "*"
    False, days -> days
  }

  "0 "
  <> int.to_string(minute)
  <> " "
  <> int.to_string(hour)
  <> " * * "
  <> dow_part
}
