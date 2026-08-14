import gleam/float
import gleam/int
import gleam/list
import gleam/string

// --- TYPES ---

pub type SensorEvaluation {
  SensorEvaluation(label: String, tone: String)
}

pub type HourlyDose {
  HourlyDose(ec_ml: Float, ph_ml: Float)
}

// --- 1. ĐÁNH GIÁ TRẠNG THÁI CẢM BIẾN ---

pub fn eval_sensor_status(
  has_error: Bool,
  value: Float,
  has_min: Bool,
  min_val: Float,
  has_max: Bool,
  max_val: Float,
) -> SensorEvaluation {
  case has_error {
    True -> SensorEvaluation(label: "Cần kiểm tra", tone: "danger")
    False -> {
      let is_low = has_min && value <. min_val
      let is_high = has_max && value >. max_val

      case is_low, is_high {
        True, _ -> SensorEvaluation(label: "Thấp", tone: "warn")
        _, True -> SensorEvaluation(label: "Cao", tone: "warn")
        False, False -> SensorEvaluation(label: "Ổn định", tone: "good")
      }
    }
  }
}

pub fn eval_sensor_status_safe(
  has_error: Bool,
  value_str: String,
  min_str: String,
  max_str: String,
) -> SensorEvaluation {
  case float.parse(value_str) {
    Error(_) -> SensorEvaluation(label: "Cần kiểm tra", tone: "danger")
    Ok(val) -> {
      let #(has_min, min_val) = case float.parse(min_str) {
        Ok(m) -> #(True, m)
        Error(_) -> #(False, 0.0)
      }
      let #(has_max, max_val) = case float.parse(max_str) {
        Ok(m) -> #(True, m)
        Error(_) -> #(False, 0.0)
      }

      eval_sensor_status(has_error, val, has_min, min_val, has_max, max_val)
    }
  }
}

// --- 2. TÍNH PHẦN TRĂM HẠN NGẠCH ---

pub fn calc_budget_percent(used: Float, limit: Float) -> Int {
  case limit <=. 0.0 {
    True -> 0
    False -> {
      let pct = float.round({ used /. limit } *. 100.0)
      int.min(100, int.max(0, pct))
    }
  }
}

pub fn calc_budget_percent_safe(used_num: Float, limit_num: Float) -> Int {
  let used = case used_num <. 0.0 {
    True -> 0.0
    False -> used_num
  }
  let limit = case limit_num <=. 0.0 {
    True -> 300.0
    False -> limit_num
  }
  calc_budget_percent(used, limit)
}

// --- 3. CỘNG DỒN SỰ KIỆN CHÂM PHÂN (PARSE TỪ CHUỖI PRIMITIVE) ---

/// Nhận chuỗi định dạng "ts1,ec1,ph1;ts2,ec2,ph2" từ JS
pub fn calc_hourly_dose_str(events_str: String, now_ms: Float) -> HourlyDose {
  let one_hour_ms = 3_600_000.0

  events_str
  |> string.split(";")
  |> list.filter_map(fn(item_str) {
    case string.split(item_str, ",") {
      [ts_str, ec_str, ph_str] -> {
        case float.parse(ts_str), float.parse(ec_str), float.parse(ph_str) {
          Ok(ts), Ok(ec), Ok(ph) -> Ok(#(ts, ec, ph))
          _, _, _ -> Error(Nil)
        }
      }
      _ -> Error(Nil)
    }
  })
  |> list.filter(fn(item) {
    let #(ts, _, _) = item
    let diff = now_ms -. ts
    diff <=. one_hour_ms && diff >=. 0.0
  })
  |> list.fold(HourlyDose(ec_ml: 0.0, ph_ml: 0.0), fn(acc, item) {
    let #(_, ec, ph) = item
    HourlyDose(ec_ml: acc.ec_ml +. ec, ph_ml: acc.ph_ml +. ph)
  })
}
