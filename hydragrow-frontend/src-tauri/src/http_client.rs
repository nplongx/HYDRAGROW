use crate::models::AppSettings;
use serde::{de::DeserializeOwned, Serialize};
use tauri::AppHandle;
use tauri_plugin_store::StoreExt;

/// Helper để lấy cấu hình kết nối từ tauri-plugin-store
pub fn get_settings(app: &AppHandle) -> Result<AppSettings, String> {
    let store = app.store("settings.json").map_err(|e| e.to_string())?;

    let backend_url = store
        .get("backend_url")
        .and_then(|v| v.as_str().map(|s| s.to_string()))
        .unwrap_or("http://192.168.1.3:8080".to_string())
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
