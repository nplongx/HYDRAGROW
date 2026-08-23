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

use crate::command_queue::{CommandQueue, QueuedCommand};

/// Frontend gọi khi muốn gửi lệnh điều khiển.
/// Nếu WS đang connected (is_online = true): gửi qua HTTP API bình thường.
/// Nếu offline: lưu vào queue, emit event để UI thông báo.
#[tauri::command]
pub async fn send_device_command(
    app: tauri::AppHandle,
    payload_json: String,
    command_id: String,
) -> Result<String, String> {
    use tauri::Manager;
    let queue = app.state::<CommandQueue>();
    let settings = get_settings(&app)?;

    // Thử gửi HTTP trực tiếp
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(5))
        .build()
        .map_err(|e| e.to_string())?;

    let result = client
        .post(format!("{}/api/devices/{}/control", settings.backend_url, settings.device_id))
        .header("X-API-Key", &settings.api_key)
        .header("Content-Type", "application/json")
        .body(payload_json.clone())
        .send()
        .await;

    match result {
        Ok(resp) if resp.status().is_success() => {
            Ok(serde_json::json!({"status": "sent", "id": command_id}).to_string())
        }
        _ => {
            use tauri::Emitter;
            // Offline → queue
            queue.enqueue(QueuedCommand {
                id: command_id.clone(),
                payload: payload_json,
            });
            let queue_len = queue.len();
            let _ = app.emit("command_queued", serde_json::json!({
                "id": command_id,
                "queue_length": queue_len,
            }));
            Ok(serde_json::json!({
                "status": "queued",
                "id": command_id,
                "queue_length": queue_len,
            }).to_string())
        }
    }
}

/// Gọi tự động khi WS reconnect (từ ws_client.rs sau khi nhận WifiConnected).
#[tauri::command]
pub async fn flush_command_queue(app: tauri::AppHandle) -> Result<usize, String> {
    use tauri::Manager;
    let queue = app.state::<CommandQueue>();
    if queue.is_empty() { return Ok(0); }

    let settings = get_settings(&app)?;
    let commands = queue.drain();
    let total = commands.len();
    let client = reqwest::Client::new();

    for cmd in commands {
        let result = client
            .post(format!("{}/api/devices/{}/control", settings.backend_url, settings.device_id))
            .header("X-API-Key", &settings.api_key)
            .header("Content-Type", "application/json")
            .body(cmd.payload.clone())
            .send()
            .await;

        if result.is_err() || !result.unwrap().status().is_success() {
            // Re-enqueue nếu vẫn fail
            queue.enqueue(cmd);
        }
    }

    let remaining = queue.len();
    use tauri::Emitter;
    let _ = app.emit("queue_flushed", serde_json::json!({
        "attempted": total,
        "remaining": remaining,
    }));
    Ok(total - remaining)
}

/// Lấy danh sách lệnh đang chờ (cho UI hiển thị badge).
#[tauri::command]
pub fn get_pending_commands(app: tauri::AppHandle) -> Vec<QueuedCommand> {
    use tauri::Manager;
    app.state::<CommandQueue>().peek_all()
}
