import gleam/float
import gleam/int
import gleam/option.{type Option, None, Some}

/// Type chứa thông tin tóm tắt kết quả hiệu chuẩn pH 2 điểm
pub type CalibrationSummary {
  CalibrationSummary(
    ph_v7: Option(Float),
    ph_v4: Option(Float),
    reliability: Int,
  )
}

/// Tính toán tóm tắt hiệu chuẩn và điểm tin cậy (reliability)
pub fn calculate_summary(
  v7: Option(Float),
  v4: Option(Float),
  avg_confidence: Int,
) -> CalibrationSummary {
  // 1. Tính điểm thưởng dựa trên độ chênh lệch điện áp giữa pH 7 và pH 4
  let spread_bonus = case v7, v4 {
    Some(val7), Some(val4) -> {
      let spread = float.absolute_value(val7 -. val4)
      case spread {
        s if s >=. 0.2 -> 15
        s if s >=. 0.1 -> 8
        _ -> 0
      }
    }
    _, _ -> 0
  }

  // 2. Điểm tin cậy = Độ tin cậy trung bình + Điểm thưởng spread (giới hạn trong 0 -> 100)
  let raw_reliability = avg_confidence + spread_bonus
  let reliability = int.min(100, int.max(0, raw_reliability))

  CalibrationSummary(ph_v7: v7, ph_v4: v4, reliability: reliability)
}

/// Kiểm tra xem kết quả hiệu chuẩn đã đủ điều kiện để áp dụng hay chưa
pub fn is_calibration_valid(summary: CalibrationSummary) -> Bool {
  case summary.ph_v7, summary.ph_v4 {
    Some(_), Some(_) -> True
    _, _ -> False
  }
}
