use tauri::AppHandle;
use tauri_plugin_store::StoreExt;

use crate::{models::AppSettings, ws_client};

// ==========================================
// 1. HTTP HELPERS (Đọc từ tauri-plugin-store)
// ==========================================

pub fn get_settings(app: &AppHandle) -> Result<AppSettings, String> {
    let store = app.store("settings.json").map_err(|e| e.to_string())?;

    let backend_url = store
        .get("backend_url")
        .and_then(|v| v.as_str().map(|s| s.to_string()))
        .unwrap_or("https://hydragrow-backend.onrender.com".to_string())
        .trim_end_matches('/')
        .to_string();

    let api_key = store
        .get("api_key")
        .and_then(|v| v.as_str().map(|s| s.to_string()))
        .unwrap_or("".to_string())
        .to_string();

    let device_id = store
        .get("device_id")
        .and_then(|v| v.as_str().map(|s| s.to_string()))
        .unwrap_or("".to_string())
        .to_string();

    Ok(AppSettings {
        api_key,
        backend_url,
        device_id,
    })
}

#[tauri::command]
pub async fn save_settings(
    app: AppHandle,
    api_key: String,
    backend_url: String,
    device_id: String,
) -> Result<(), String> {
    let store = app.store("settings.json").map_err(|e| e.to_string())?;
    store.set("api_key", serde_json::json!(api_key));
    store.set("backend_url", serde_json::json!(backend_url));
    store.set("device_id", serde_json::json!(device_id));
    let _ = store.save(); // Bỏ qua lỗi save nếu có

    Ok(())
}

#[tauri::command]
pub async fn load_settings(app: AppHandle) -> Result<AppSettings, String> {
    get_settings(&app)
}

#[tauri::command]
pub async fn start_ws_listener(app: AppHandle, device_id: String) -> Result<(), String> {
    // Chạy background task, không block luồng chính
    ws_client::start_ws_listener(app, device_id).await;
    Ok(())
}
