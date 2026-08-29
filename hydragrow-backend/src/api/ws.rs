// api/ws.rs
use actix_web::{Error, HttpRequest, HttpResponse, web};
use actix_ws::Message;
use futures_util::StreamExt as _;
use hydragrow_shared::events::AppEvent;
use tokio::{
    sync::broadcast::error::RecvError,
    time::{Duration, timeout},
};
use tracing::{info, warn};

use crate::AppState;
use crate::metrics::ACTIVE_WS_CONNECTIONS;

#[derive(serde::Deserialize)]
struct WsAuthMessage {
    #[serde(rename = "type")]
    message_type: String,
    api_key: String,
}

#[derive(serde::Deserialize)]
struct WsQuery {
    api_key: Option<String>,
}

/// RAII guard để đảm bảo active WebSocket luôn được giảm
/// khi connection kết thúc.
struct WsConnectionGuard;

impl WsConnectionGuard {
    fn new() -> Self {
        ACTIVE_WS_CONNECTIONS.inc();
        Self
    }
}

impl Drop for WsConnectionGuard {
    fn drop(&mut self) {
        ACTIVE_WS_CONNECTIONS.dec();
    }
}

pub async fn ws_handler(
    req: HttpRequest,
    body: web::Payload,
    app_state: web::Data<AppState>,
    path: web::Path<String>,
) -> Result<HttpResponse, Error> {
    let scoped_device_id = path.into_inner();

    // Gọi trực tiếp actix_ws::handle để trả về Handshake Body nguyên bản
    let (response, mut session, mut msg_stream) = actix_ws::handle(&req, body)?;

    let client_ip = req
        .connection_info()
        .realip_remote_addr()
        .unwrap_or("unknown")
        .to_string();

    info!(
        client_ip = %client_ip,
        "New WebSocket connection established"
    );

    let expected_api_key = app_state.api_key.clone();
    let event_bus = app_state.event_bus.clone();

    // Kiểm tra API key nếu client truyền trên URL (?api_key=...)
    let query_api_key = web::Query::<WsQuery>::from_query(req.query_string())
        .ok()
        .and_then(|q| q.api_key.clone());

    let pre_authorized = query_api_key.as_deref() == Some(&expected_api_key);

    actix_web::rt::spawn(async move {
        let is_authorized = if pre_authorized {
            true
        } else {
            let auth_result = timeout(Duration::from_secs(10), msg_stream.next()).await;

            match auth_result {
                Ok(Some(Ok(Message::Text(text)))) => serde_json::from_str::<WsAuthMessage>(&text)
                    .map(|auth| auth.message_type == "auth" && auth.api_key == expected_api_key)
                    .unwrap_or(false),

                Ok(Some(Ok(Message::Close(reason)))) => {
                    let _ = session.close(reason).await;
                    return;
                }

                _ => false,
            }
        };

        // Không được tính connection chưa authenticate
        if !is_authorized {
            warn!(
                client_ip = %client_ip,
                "Rejected unauthenticated WebSocket connection"
            );

            let _ = session.close(None).await;
            return;
        }

        // Từ thời điểm này connection được xem là ACTIVE.
        //
        // Guard sẽ tự động gọi:
        // ACTIVE_WS_CONNECTIONS.inc()
        //
        // và khi task kết thúc:
        // ACTIVE_WS_CONNECTIONS.dec()
        let _ws_connection_guard = WsConnectionGuard::new();

        info!(
            client_ip = %client_ip,
            active_connections = ACTIVE_WS_CONNECTIONS.get(),
            "WebSocket client authenticated"
        );

        let mut event_rx = event_bus.subscribe();

        loop {
            tokio::select! {
                event_result = event_rx.recv() => {
                    match event_result {
                        Ok(event) => {
                            // Only forward events for this connection's device_id.
                            // Events with no device_id (e.g. broad system events) pass through.
                            let event_device_id = event_device_id_for_filter(&event);
                            if let Some(dev_id) = event_device_id
                                && dev_id != scoped_device_id {
                                    continue;
                                }

                            let ws_msg = match event {
                                AppEvent::SystemAlert(alert_msg) => {
                                    serde_json::json!({
                                        "type": "alert",
                                        "payload": alert_msg
                                    })
                                }

                                AppEvent::SensorUpdate(sensor_data) => {
                                    serde_json::json!({
                                        "type": "sensor_update",
                                        "payload": sensor_data
                                    })
                                }

                                AppEvent::DeviceStatus(status) => {
                                    serde_json::json!({
                                        "type": "device_status",
                                        "payload": {
                                            "device_id": status.device_id,
                                            "is_online": status.is_online,
                                            "last_seen": chrono::Utc::now().to_rfc3339()
                                        }
                                    })
                                }

                                AppEvent::FsmTransition(fsm) => {
                                    serde_json::json!({
                                        "type": "fsm_transition",
                                        "payload": fsm
                                    })
                                }

                                AppEvent::DosingCycle(report) => {
                                    serde_json::json!({
                                        "type": "dosing_report",
                                        "payload": report
                                    })
                                }

                                AppEvent::CalibrationUpdate(payload) => {
                                    serde_json::json!({
                                        "type": "calibration_update",
                                        "payload": payload
                                    })
                                }

                                AppEvent::WaterCycle(payload) => {
                                    serde_json::json!({
                                        "type": "water_cycle",
                                        "payload": payload
                                    })
                                }

                                AppEvent::HealthSnapshot(snapshot) => {
                                    serde_json::json!({
                                        "type": "health_snapshot",
                                        "payload": snapshot
                                    })
                                }

                                AppEvent::FsmStateUpdate(snapshot) => {
                                    serde_json::json!({
                                        "type": "fsm_state_update",
                                        "payload": snapshot
                                    })
                                }

                                AppEvent::ControllerStatus(payload) => {
                                    serde_json::json!({
                                        "type": "controller_status",
                                        "payload": payload
                                    })
                                }
                            };

                            if let Ok(json_str) = serde_json::to_string(&ws_msg)
                                && session.text(json_str).await.is_err() {
                                    break;
                                }
                        }

                        Err(RecvError::Lagged(skipped)) => {
                            warn!(
                                client_ip = %client_ip,
                                skipped,
                                "WebSocket client is too slow and missed events"
                            );
                        }

                        Err(RecvError::Closed) => {
                            break;
                        }
                    }
                }

                Some(Ok(msg)) = msg_stream.next() => {
                    match msg {
                        Message::Ping(bytes) => {
                            if session.pong(&bytes).await.is_err() {
                                break;
                            }
                        }

                        Message::Close(reason) => {
                            let _ = session.close(reason).await;
                            break;
                        }

                        _ => {}
                    }
                }

                else => {
                    break;
                }
            }
        }

        info!(
            client_ip = %client_ip,
            active_connections = ACTIVE_WS_CONNECTIONS.get(),
            "WebSocket connection closed"
        );

        // Không cần ACTIVE_WS_CONNECTIONS.dec() ở đây.
        //
        // Khi scope kết thúc, _ws_connection_guard bị drop
        // và tự động gọi ACTIVE_WS_CONNECTIONS.dec().
    });

    Ok(response)
}

pub fn init_routes(cfg: &mut web::ServiceConfig) {
    cfg.route("/ws", web::get().to(ws_handler));
}

fn event_device_id_for_filter(event: &AppEvent) -> Option<&str> {
    match event {
        AppEvent::DeviceStatus(p) => Some(p.device_id.as_str()),
        AppEvent::SensorUpdate(p) => Some(p.device_id.as_str()),
        AppEvent::FsmTransition(p) => Some(p.device_id.as_str()),
        AppEvent::SystemAlert(a) => Some(a.device_id.as_str()),
        AppEvent::DosingCycle(c) => Some(c.device_id.as_str()),
        AppEvent::WaterCycle(c) => Some(c.device_id.as_str()),
        AppEvent::HealthSnapshot(s) => Some(s.device_id.as_str()),
        AppEvent::CalibrationUpdate(c) => Some(c.device_id.as_str()),
        AppEvent::ControllerStatus(payload) => payload.get("device_id").and_then(|v| v.as_str()),
        AppEvent::FsmStateUpdate(_) => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::alert::AlertMessage;
    use hydragrow_shared::events::AppEvent;

    fn make_alert(device_id: &str) -> AppEvent {
        AppEvent::SystemAlert(AlertMessage {
            level: "warning".to_string(),
            category: "dosing".to_string(),
            title: "Test".to_string(),
            message: "msg".to_string(),
            device_id: device_id.to_string(),
            timestamp: 0,
            reason: None,
            metadata: None,
        })
    }

    #[test]
    fn alert_for_device_a_filtered_out_for_device_b_connection() {
        let event = make_alert("device_A");
        let scoped = "device_B";
        let event_did = event_device_id_for_filter(&event);
        assert!(event_did.is_some());
        assert_ne!(
            event_did,
            Some(scoped),
            "Alert of device_A must be filtered for device_B WS"
        );
    }

    #[test]
    fn alert_for_device_a_passes_for_device_a_connection() {
        let event = make_alert("device_A");
        let scoped = "device_A";
        let event_did = event_device_id_for_filter(&event);
        assert_eq!(event_did, Some(scoped));
    }
}
