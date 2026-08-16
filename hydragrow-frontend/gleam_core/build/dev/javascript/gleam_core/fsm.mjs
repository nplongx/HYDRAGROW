import * as $int from "../gleam_stdlib/gleam/int.mjs";
import * as $option from "../gleam_stdlib/gleam/option.mjs";
import { Some, Option$None$const } from "../gleam_stdlib/gleam/option.mjs";
import * as $string from "../gleam_stdlib/gleam/string.mjs";
import { Empty as $Empty, CustomType as $CustomType } from "./gleam.mjs";

export class FriendlyState extends $CustomType {
  constructor(label, description, tone) {
    super();
    this.label = label;
    this.description = description;
    this.tone = tone;
  }
}
export const FriendlyState$FriendlyState = (label, description, tone) =>
  new FriendlyState(label, description, tone);
export const FriendlyState$isFriendlyState = (value) =>
  value instanceof FriendlyState;
export const FriendlyState$FriendlyState$label = (value) => value.label;
export const FriendlyState$FriendlyState$0 = (value) => value.label;
export const FriendlyState$FriendlyState$description = (value) =>
  value.description;
export const FriendlyState$FriendlyState$1 = (value) => value.description;
export const FriendlyState$FriendlyState$tone = (value) => value.tone;
export const FriendlyState$FriendlyState$2 = (value) => value.tone;

export class ComputedHealth extends $CustomType {
  constructor(score, label, color, description) {
    super();
    this.score = score;
    this.label = label;
    this.color = color;
    this.description = description;
  }
}
export const ComputedHealth$ComputedHealth = (score, label, color, description) =>
  new ComputedHealth(score, label, color, description);
export const ComputedHealth$isComputedHealth = (value) =>
  value instanceof ComputedHealth;
export const ComputedHealth$ComputedHealth$score = (value) => value.score;
export const ComputedHealth$ComputedHealth$0 = (value) => value.score;
export const ComputedHealth$ComputedHealth$label = (value) => value.label;
export const ComputedHealth$ComputedHealth$1 = (value) => value.label;
export const ComputedHealth$ComputedHealth$color = (value) => value.color;
export const ComputedHealth$ComputedHealth$2 = (value) => value.color;
export const ComputedHealth$ComputedHealth$description = (value) =>
  value.description;
export const ComputedHealth$ComputedHealth$3 = (value) => value.description;

/**
 * Helper phân tích chuỗi JSON đơn giản như {"Fault":"PhDosingFailed"} không cần thư viện ngoài
 * 
 * @ignore
 */
function parse_json_fault(json_str) {
  let $ = $string.split(json_str, "\"Fault\"");
  if ($ instanceof $Empty) {
    return Option$None$const;
  } else {
    let $1 = $.tail;
    if ($1 instanceof $Empty) {
      return Option$None$const;
    } else {
      let $2 = $1.tail;
      if ($2 instanceof $Empty) {
        let rest = $1.head;
        let $3 = $string.split(rest, "\"");
        if ($3 instanceof $Empty) {
          return Option$None$const;
        } else {
          let $4 = $3.tail;
          if ($4 instanceof $Empty) {
            return Option$None$const;
          } else {
            let fault_name = $4.head;
            return new Some($string.trim(fault_name));
          }
        }
      } else {
        return Option$None$const;
      }
    }
  }
}

/**
 * Trích xuất mã lỗi từ chuỗi FSM State (Hỗ trợ định dạng SystemFault:X, Fault:X, hoặc JSON {"Fault":"X"})
 */
export function extract_fault_code(state_str) {
  let trimmed = $string.trim(state_str);
  let $ = $string.starts_with(trimmed, "SystemFault:");
  if ($) {
    return new Some($string.replace(trimmed, "SystemFault:", ""));
  } else {
    let $1 = $string.starts_with(trimmed, "Fault:");
    if ($1) {
      return new Some($string.replace(trimmed, "Fault:", ""));
    } else {
      let $2 = $string.starts_with(trimmed, "{");
      if ($2) {
        return parse_json_fault(trimmed);
      } else {
        return Option$None$const;
      }
    }
  }
}

/**
 * Hàm dành riêng cho JS: Trả về String rỗng "" thay vì Option nếu không có lỗi
 */
export function extract_fault_code_str(state_str) {
  let $ = extract_fault_code(state_str);
  if ($ instanceof Some) {
    let code = $[0];
    return code;
  } else {
    return "";
  }
}

export function compute_health(is_online, raw_score) {
  if (is_online) {
    let score = $option.unwrap(raw_score, 100);
    let s = score;
    if (s >= 90) {
      return new ComputedHealth(
        s,
        "Hoàn hảo",
        "bg-emerald-500",
        "Mạng, rơ-le và vi xử lý hoạt động hoàn hảo.",
      );
    } else {
      let s = score;
      if (s >= 60) {
        return new ComputedHealth(
          s,
          "Cần chú ý",
          "bg-amber-500",
          "Phát hiện chỉ số chênh lệch, đang khắc phục.",
        );
      } else {
        let s = score;
        return new ComputedHealth(
          s,
          "Yếu / Cần kiểm tra",
          "bg-rose-500",
          "Phát hiện bơm bị nghẽn hoặc cạn dung dịch.",
        );
      }
    }
  } else {
    return new ComputedHealth(
      0,
      "Mất kết nối",
      "bg-rose-500",
      "Không có tín hiệu từ thiết bị ngoại vi.",
    );
  }
}

/**
 * Hàm dành riêng cho JS: Nhận score dưới dạng Int (truyền -1 nếu null/undefined)
 */
export function compute_health_safe(is_online, score_num) {
  let _block;
  let $ = score_num < 0;
  if ($) {
    _block = Option$None$const;
  } else {
    _block = new Some(score_num);
  }
  let opt_score = _block;
  return compute_health(is_online, opt_score);
}

export function friendly_state(state_str, is_online) {
  if (is_online) {
    let trimmed = $string.trim(state_str);
    let $ = extract_fault_code(trimmed);
    if ($ instanceof Some) {
      let fault_code = $[0];
      return new FriendlyState(
        "Sự cố: " + fault_code,
        "Hệ thống kích hoạt chế độ an toàn. Vui lòng kiểm tra thiết bị.",
        "danger",
      );
    } else {
      if (trimmed === "Booting") {
        return new FriendlyState(
          "Đang khởi động",
          "Thiết bị đang tải cấu hình và kiểm tra cảm biến.",
          "info",
        );
      } else if (trimmed === "SystemBooting") {
        return new FriendlyState(
          "Đang khởi động",
          "Thiết bị đang tải cấu hình và kiểm tra cảm biến.",
          "info",
        );
      } else if (trimmed === "Monitoring") {
        return new FriendlyState(
          "Đang giám sát",
          "Mô hình thông minh đang theo dõi sinh trưởng cây trồng.",
          "success",
        );
      } else if (trimmed === "MimoDosing") {
        return new FriendlyState(
          "Đang bổ sung vi chất",
          "Hệ thống tự động châm EC/pH vào bồn chứa.",
          "mist",
        );
      } else if (trimmed === "ActiveMixing") {
        return new FriendlyState(
          "Đang sục trộn phân",
          "Bơm trộn tuần hoàn đang hòa tan hóa chất.",
          "info",
        );
      } else if (trimmed === "Stabilizing") {
        return new FriendlyState(
          "Đang lắng đọng",
          "Chờ ổn định nước trước khi đọc cảm biến.",
          "warn",
        );
      } else if (trimmed === "Cooldown") {
        return new FriendlyState(
          "Đang nghỉ hồi bồn",
          "Ngắt tạm thời để dinh dưỡng thẩm thấu an toàn.",
          "warn",
        );
      } else if (trimmed === "ManualMode") {
        return new FriendlyState(
          "Chế độ thủ công",
          "Người dùng điều khiển rơ-le bằng tay.",
          "warn",
        );
      } else if (trimmed === "Offline") {
        return new FriendlyState(
          "Ngoại tuyến",
          "Trạm điều khiển đang mất kết nối mạng.",
          "danger",
        );
      } else {
        let s = trimmed;
        let $1 = $string.starts_with(s, "EmergencyStop:");
        if ($1) {
          return new FriendlyState(
            "Dừng khẩn cấp",
            "Phát lệnh ngắt toàn bộ hệ thống do sự cố.",
            "danger",
          );
        } else {
          return new FriendlyState(
            s,
            "Hệ thống đang thực thi tiến trình.",
            "default",
          );
        }
      }
    }
  } else {
    return new FriendlyState(
      "Ngoại tuyến",
      "Trạm điều khiển đang mất kết nối mạng.",
      "danger",
    );
  }
}
