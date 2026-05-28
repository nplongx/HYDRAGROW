use actix_web::{Error, HttpRequest, HttpResponse, web};
use actix_ws::Message;
use futures_util::StreamExt as _;
use hydragrow_shared::events::AppEvent;
use tokio::sync::broadcast::error::RecvError;
use tracing::{info, warn};

use crate::AppState;

pub async fn ws_handler(
    req: HttpRequest,
    body: web::Payload,
    app_state: web::Data<AppState>,
) -> Result<HttpResponse, Error> {
    let (response, mut session, mut msg_stream) = actix_ws::handle(&req, body)?;

    let mut event_rx = app_state.event_bus.subscribe();

    let client_ip = req
        .connection_info()
        .realip_remote_addr()
        .unwrap_or("unknown")
        .to_string();

    info!(
        "New WebSocket connection established from IP: {}",
        client_ip
    );

    actix_web::rt::spawn(async move {
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
