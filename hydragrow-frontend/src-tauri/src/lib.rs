mod commands;
mod http_client;
mod models;
mod notification;
mod valve_guard;
mod water_sequence;
mod ws_client;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        // 1. Khởi tạo các Plugin cần thiết (Lưu trữ file và Thông báo OS)
        .plugin(tauri_plugin_store::Builder::new().build())
        .plugin(tauri_plugin_notification::init())
        .plugin(tauri_plugin_http::init()) // <--- Thêm dòng này 3. Đăng ký TOÀN BỘ các hàm đã expose ở file commands.rs và notification.rs
        .plugin(tauri_plugin_fs::init())
        .plugin(tauri_plugin_dialog::init())
        .invoke_handler(tauri::generate_handler![
            commands::save_settings,
            commands::load_settings,
            commands::start_ws_listener,
            // Tiện ích (Thông báo)
            notification::trigger_os_notification,
        ])
        .setup(|app| {
            println!("🚀 Core Backend Rust đã khởi động thành công!");
            // Nếu bạn muốn tự động kết nối WS ngay khi mở app, có thể đọc settings và gọi start_ws_listener tại đây.
            // Nhưng thiết kế hiện tại đang để cho React chủ động gọi lệnh `start_ws_listener` khi component mount.
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("Lỗi trong quá trình chạy ứng dụng Tauri!");
}
