// src-tauri/src/notification.rs

use tauri::AppHandle;

// Dành cho Tauri v2 (sử dụng tauri-plugin-notification)
use tauri_plugin_notification::NotificationExt;

/// Hàm tiện ích để gọi từ các module Rust khác (ví dụ: từ ws_client khi nhận được Alert)
pub fn show_critical_alert(app: &AppHandle, title: &str, body: &str) {
    // Gọi API thông báo của hệ điều hành
    let result = app.notification().builder().title(title).body(body).show();

    if let Err(e) = result {
        println!("[Notification] Không thể hiển thị thông báo OS: {}", e);
    }
}

/// Expose thành command để React frontend cũng có thể chủ động gọi khi cần
#[tauri::command]
pub fn trigger_os_notification(app: AppHandle, title: String, body: String) {
    show_critical_alert(&app, &title, &body);
}
