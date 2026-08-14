import gleam/int
import gleam/option.{type Option, None, Some}
import gleam/string

// --- TYPES ---

pub type FriendlyState {
  FriendlyState(label: String, description: String, tone: String)
}

pub type ComputedHealth {
  ComputedHealth(score: Int, label: String, color: String, description: String)
}

// --- 1. TRÍCH XUẤT MÃ LỖI (FAULT CODE PARSER) ---

/// Trích xuất mã lỗi từ chuỗi FSM State (Hỗ trợ định dạng SystemFault:X, Fault:X, hoặc JSON {"Fault":"X"})
pub fn extract_fault_code(state_str: String) -> Option(String) {
  let trimmed = string.trim(state_str)

  case string.starts_with(trimmed, "SystemFault:") {
    True -> Some(string.replace(trimmed, "SystemFault:", ""))
    False ->
      case string.starts_with(trimmed, "Fault:") {
        True -> Some(string.replace(trimmed, "Fault:", ""))
        False ->
          case string.starts_with(trimmed, "{") {
            True -> parse_json_fault(trimmed)
            False -> None
          }
      }
  }
}

/// Helper phân tích chuỗi JSON đơn giản như {"Fault":"PhDosingFailed"} không cần thư viện ngoài
fn parse_json_fault(json_str: String) -> Option(String) {
  case string.split(json_str, "\"Fault\"") {
    [_, rest] ->
      case string.split(rest, "\"") {
        [_, fault_name, ..] -> Some(string.trim(fault_name))
        _ -> None
      }
    _ -> None
  }
}

/// Hàm dành riêng cho JS: Trả về String rỗng "" thay vì Option nếu không có lỗi
pub fn extract_fault_code_str(state_str: String) -> String {
  case extract_fault_code(state_str) {
    Some(code) -> code
    None -> ""
  }
}

// --- 2. TÍNH TOÁN ĐIỂM SỨC KHỎE TRẠM (HEALTH COMPUTATION) ---

pub fn compute_health(
  is_online: Bool,
  raw_score: Option(Int),
) -> ComputedHealth {
  case is_online {
    False ->
      ComputedHealth(
        score: 0,
        label: "Mất kết nối",
        color: "bg-rose-500",
        description: "Không có tín hiệu từ thiết bị ngoại vi.",
      )
    True -> {
      let score = option.unwrap(raw_score, 100)
      case score {
        s if s >= 90 ->
          ComputedHealth(
            score: s,
            label: "Hoàn hảo",
            color: "bg-emerald-500",
            description: "Mạng, rơ-le và vi xử lý hoạt động hoàn hảo.",
          )
        s if s >= 60 ->
          ComputedHealth(
            score: s,
            label: "Cần chú ý",
            color: "bg-amber-500",
            description: "Phát hiện chỉ số chênh lệch, đang khắc phục.",
          )
        s ->
          ComputedHealth(
            score: s,
            label: "Yếu / Cần kiểm tra",
            color: "bg-rose-500",
            description: "Phát hiện bơm bị nghẽn hoặc cạn dung dịch.",
          )
      }
    }
  }
}

/// Hàm dành riêng cho JS: Nhận score dưới dạng Int (truyền -1 nếu null/undefined)
pub fn compute_health_safe(is_online: Bool, score_num: Int) -> ComputedHealth {
  let opt_score = case score_num < 0 {
    True -> None
    False -> Some(score_num)
  }
  compute_health(is_online, opt_score)
}

// --- 3. DỊCH TRẠNG THÁI THÂN THIỆN NGUYÊN BẢN (FRIENDLY STATE) ---

pub fn friendly_state(state_str: String, is_online: Bool) -> FriendlyState {
  case is_online {
    False ->
      FriendlyState(
        label: "Ngoại tuyến",
        description: "Trạm điều khiển đang mất kết nối mạng.",
        tone: "danger",
      )
    True -> {
      let trimmed = string.trim(state_str)
      case extract_fault_code(trimmed) {
        Some(fault_code) ->
          FriendlyState(
            label: "Sự cố: " <> fault_code,
            description: "Hệ thống kích hoạt chế độ an toàn. Vui lòng kiểm tra thiết bị.",
            tone: "danger",
          )
        None ->
          case trimmed {
            "Booting" | "SystemBooting" ->
              FriendlyState(
                label: "Đang khởi động",
                description: "Thiết bị đang tải cấu hình và kiểm tra cảm biến.",
                tone: "info",
              )
            "Monitoring" ->
              FriendlyState(
                label: "Đang giám sát",
                description: "Mô hình thông minh đang theo dõi sinh trưởng cây trồng.",
                tone: "success",
              )
            "MimoDosing" ->
              FriendlyState(
                label: "Đang bổ sung vi chất",
                description: "Hệ thống tự động châm EC/pH vào bồn chứa.",
                tone: "mist",
              )
            "ActiveMixing" ->
              FriendlyState(
                label: "Đang sục trộn phân",
                description: "Bơm trộn tuần hoàn đang hòa tan hóa chất.",
                tone: "info",
              )
            "Stabilizing" ->
              FriendlyState(
                label: "Đang lắng đọng",
                description: "Chờ ổn định nước trước khi đọc cảm biến.",
                tone: "warn",
              )
            "Cooldown" ->
              FriendlyState(
                label: "Đang nghỉ hồi bồn",
                description: "Ngắt tạm thời để dinh dưỡng thẩm thấu an toàn.",
                tone: "warn",
              )
            "ManualMode" ->
              FriendlyState(
                label: "Chế độ thủ công",
                description: "Người dùng điều khiển rơ-le bằng tay.",
                tone: "warn",
              )
            "Offline" ->
              FriendlyState(
                label: "Ngoại tuyến",
                description: "Trạm điều khiển đang mất kết nối mạng.",
                tone: "danger",
              )
            s ->
              case string.starts_with(s, "EmergencyStop:") {
                True ->
                  FriendlyState(
                    label: "Dừng khẩn cấp",
                    description: "Phát lệnh ngắt toàn bộ hệ thống do sự cố.",
                    tone: "danger",
                  )
                False ->
                  FriendlyState(
                    label: s,
                    description: "Hệ thống đang thực thi tiến trình.",
                    tone: "default",
                  )
              }
          }
      }
    }
  }
}
