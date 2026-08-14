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

pub async fn ws_handler(
    req: HttpRequest,
    body: web::Payload,
    app_state: web::Data<AppState>,
) -> Result<HttpResponse, Error> {
    // Gọi trực tiếp actix_ws::handle để trả về Handshake Body nguyên bản
    let (response, mut session, mut msg_stream) = actix_ws::handle(&req, body)?;

    let client_ip = req
        .connection_info()
        .realip_remote_addr()
        .unwrap_or("unknown")
        .to_string();

    info!(
        "New WebSocket connection established from IP: {}",
        client_ip
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

        if !is_authorized {
            warn!(
                "Rejected unauthenticated WebSocket connection from IP: {}",
                client_ip
            );
            let _ = session.close(None).await;
            return;
        }

        let mut event_rx = event_bus.subscribe();
        loop {
            tokio::select! {
                event_result = event_rx.recv() => {
                    match event_result {
                        Ok(event) => {
                            let ws_msg = match event {
                                AppEvent::SystemAlert(alert_msg) => serde_json::json!({"type":"alert","payload":alert_msg}),
                                AppEvent::SensorUpdate(sensor_data) => serde_json::json!({"type":"sensor_update","payload":sensor_data}),
                                AppEvent::DeviceStatus(status) => serde_json::json!({"type":"device_status","payload":{"is_online":status.is_online,"last_seen":chrono::Utc::now().to_rfc3339()}}),
                                AppEvent::FsmTransition(fsm) => serde_json::json!({"type":"fsm_transition","payload":fsm}),
                                AppEvent::DosingCycle(report) => serde_json::json!({"type":"dosing_report","payload":report}),
                                AppEvent::CalibrationUpdate(payload) => serde_json::json!({"type":"calibration_update","payload":payload}),
                                AppEvent::WaterCycle(payload) => serde_json::json!({"type":"water_cycle", "payload":payload}),
                                AppEvent::HealthSnapshot(snapshot) => serde_json::json!({"type":"health_snapshot", "payload":snapshot}),
                                AppEvent::FsmStateUpdate(snapshot) => serde_json::json!({"type":"fsm_state_update", "payload":snapshot}),
                                AppEvent::ControllerStatus(payload) => serde_json::json!({"type":"controller_status", "payload":payload}),
                            };

                            if let Ok(json_str) = serde_json::to_string(&ws_msg) {
                                if session.text(json_str).await.is_err() {
                                    break;
                                }
                            }
                        }
                        Err(RecvError::Lagged(_)) => {
                            warn!("WS Client {} is too slow, missed some events", client_ip);
                        }
                        Err(RecvError::Closed) => break,
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

                else => break,
            }
        }
        info!("WebSocket connection closed for IP: {}", client_ip);
    });

    Ok(response)
}

pub fn init_routes(cfg: &mut web::ServiceConfig) {
    cfg.route("/ws", web::get().to(ws_handler));
}
