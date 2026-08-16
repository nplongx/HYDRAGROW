import * as $list from "../gleam_stdlib/gleam/list.mjs";
import * as $string from "../gleam_stdlib/gleam/string.mjs";

/**
 * Escape một ô dữ liệu trong CSV (Thêm ngoặc kép & nhân đôi ngoặc kép bên trong)
 */
export function escape_field(val) {
  let escaped = $string.replace(val, "\"", "\"\"");
  return ("\"" + escaped) + "\"";
}

/**
 * Chuyển danh sách các ô thành 1 hàng CSV hoàn chỉnh
 */
export function build_row(fields) {
  let _pipe = fields;
  let _pipe$1 = $list.map(_pipe, escape_field);
  return $string.join(_pipe$1, ",");
}

/**
 * Dành cho JS: Nhận chuỗi chứa các trường cách nhau bằng ngoặc vuông hoặc phân cách
 */
export function escape_field_str(val) {
  return escape_field(val);
}
