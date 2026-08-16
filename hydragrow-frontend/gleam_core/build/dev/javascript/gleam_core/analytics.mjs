import * as $float from "../gleam_stdlib/gleam/float.mjs";

/**
 * Quyết định xem có giữ điểm dữ liệu hiện tại hay bỏ qua để giảm tải biểu đồ
 */
export function should_keep_sample(
  current_ts,
  last_ts,
  interval_ms,
  is_first,
  is_last
) {
  let $ = is_first || is_last;
  if ($) {
    return $;
  } else {
    let $1 = interval_ms <= 0.0;
    if ($1) {
      return $1;
    } else {
      return (current_ts - last_ts) >= interval_ms;
    }
  }
}
