//! Danh sách scope chuẩn dùng trong toàn hệ thống.
//! Khi thêm scope mới, thêm vào KNOWN_SCOPES để validation tự động bắt.

/// Toàn bộ scope hợp lệ trong hệ thống.
/// `*` = full access (chỉ dùng cho server internal hoặc root admin).
pub const KNOWN_SCOPES: &[&str] = &[
    // Đọc dữ liệu
    "read:telemetry",        // Đọc dữ liệu cảm biến, metrics

    // Ghi / điều khiển cơ bản
    "write:config",          // Cập nhật cấu hình thiết bị (recipe, cài đặt)
    "control:pump",          // Điều khiển bơm thủ công (force_on/off)
    "control:emergency",     // Lệnh khẩn cấp (reset_fault, enter_calibration)

    // Quản lý thiết bị (Device Admin)
    "device:ota",            // Trigger OTA firmware update
    "device:network",        // Cập nhật WiFi priority list
    "device:admin",          // Reboot, factory reset — quyền cao nhất cho thiết bị

    // User script APIs
    "script:read",           // Đọc / validate user scripts
    "script:write",          // Tạo, cập nhật, xóa user scripts

    // Wildcard (root only)
    "*",
];

/// Kiểm tra xem scope có nằm trong KNOWN_SCOPES không.
pub fn is_valid_scope(scope: &str) -> bool {
    KNOWN_SCOPES.contains(&scope)
}

/// Mô tả cho từng scope (hiển thị trong UI quản lý).
pub fn scope_description(scope: &str) -> &'static str {
    match scope {
        "read:telemetry"     => "Đọc dữ liệu cảm biến và metrics theo thời gian thực",
        "write:config"       => "Cập nhật cấu hình thiết bị, recipe, lịch tưới",
        "control:pump"       => "Điều khiển bơm thủ công (bật/tắt từng bơm)",
        "control:emergency"  => "Lệnh khẩn cấp: dừng khẩn cấp, reset lỗi, hiệu chuẩn",
        "device:ota"         => "Cập nhật firmware thiết bị qua OTA",
        "device:network"     => "Cập nhật danh sách WiFi trên thiết bị",
        "device:admin"       => "Reboot và factory reset thiết bị (toàn quyền quản trị)",
        "script:read"        => "Đọc và kiểm tra (validate) các user scripts",
        "script:write"       => "Tạo, cập nhật và xóa các user scripts",
        "*"                  => "Toàn quyền truy cập (chỉ dành cho admin hệ thống)",
        _                    => "Scope không xác định",
    }
}
