import gleam/list
import gleam/string

/// Escape một ô dữ liệu trong CSV (Thêm ngoặc kép & nhân đôi ngoặc kép bên trong)
pub fn escape_field(val: String) -> String {
  let escaped = string.replace(val, each: "\"", with: "\"\"")
  "\"" <> escaped <> "\""
}

/// Chuyển danh sách các ô thành 1 hàng CSV hoàn chỉnh
pub fn build_row(fields: List(String)) -> String {
  fields
  |> list.map(escape_field)
  |> string.join(",")
}

/// Dành cho JS: Nhận chuỗi chứa các trường cách nhau bằng ngoặc vuông hoặc phân cách
pub fn escape_field_str(val: String) -> String {
  escape_field(val)
}
