import gleam/float

/// Quyết định xem có giữ điểm dữ liệu hiện tại hay bỏ qua để giảm tải biểu đồ
pub fn should_keep_sample(
  current_ts: Float,
  last_ts: Float,
  interval_ms: Float,
  is_first: Bool,
  is_last: Bool,
) -> Bool {
  case is_first || is_last {
    True -> True
    False -> {
      case interval_ms <=. 0.0 {
        True -> True
        False -> current_ts -. last_ts >=. interval_ms
      }
    }
  }
}
