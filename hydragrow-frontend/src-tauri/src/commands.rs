use tauri::AppHandle;
use tauri_plugin_store::StoreExt;

use crate::{models::AppSettings, secret_store, ws_client};

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

    let legacy_api_key = store
        .get("api_key")
        .and_then(|v| v.as_str().map(|s| s.trim().to_string()))
        .unwrap_or_default();

    if !legacy_api_key.is_empty() {
        secret_store::save_api_key(&legacy_api_key)?;
        store.delete("api_key");
        let _ = store.save();
    }

    let api_key = secret_store::load_api_key()?;

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
    secret_store::save_api_key(&api_key)?;
    store.delete("api_key");
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

#[tauri::command]
pub async fn forget_api_key(app: AppHandle) -> Result<(), String> {
    let store = app.store("settings.json").map_err(|e| e.to_string())?;
    secret_store::save_api_key("")?;
    store.delete("api_key");
    let _ = store.save();
    Ok(())
}

/// Preflight manual water commands. This is deliberately a separate command
/// because commands are sent directly from the React control client rather
/// than through a Tauri-side HTTP proxy.
#[tauri::command]
pub fn check_valve_safety<R: tauri::Runtime>(app: tauri::AppHandle<R>, target_pump: String, is_on: bool) -> Result<(), String> {
    use tauri::Manager;

    app.state::<crate::valve_guard::ValveGuardState>()
        .check_safety(&target_pump, is_on)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::valve_guard::ValveGuardState;
    use crate::models::PumpStatus;
    use tauri::Manager;

    fn setup_app() -> tauri::AppHandle<tauri::test::MockRuntime> {
        let app = tauri::test::mock_app();
        app.manage(ValveGuardState::default());
        app.app_handle().clone()
    }

    #[test]
    fn check_valve_safety_allowed() {
        let app = setup_app();

        let result = check_valve_safety(app, "WATER_PUMP_IN".to_string(), true);
        assert!(result.is_ok());
    }

    #[test]
    fn check_valve_safety_blocked() {
        let app = setup_app();
        let guard = app.state::<ValveGuardState>();
        guard.update_status(PumpStatus {
            water_pump_out: true,
            ..Default::default()
        });

        let result = check_valve_safety(app.clone(), "WATER_PUMP_IN".to_string(), true);
        assert!(result.is_err());
        assert_eq!(
            result.unwrap_err(),
            "⛔ XUNG ĐỘT AN TOÀN: Không thể mở VAN_IN do bơm/van xả đang hoạt động!"
        );
    }

    #[test]
    fn check_valve_safety_turn_off_always_allowed() {
        let app = setup_app();
        let guard = app.state::<ValveGuardState>();
        guard.update_status(PumpStatus {
            water_pump_out: true,
            ..Default::default()
        });

        let result = check_valve_safety(app.clone(), "WATER_PUMP_IN".to_string(), false);
        assert!(result.is_ok());
    }
}
