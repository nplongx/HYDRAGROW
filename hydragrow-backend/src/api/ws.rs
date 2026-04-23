use actix_web::{Error, HttpRequest, HttpResponse, web};
use actix_ws::Message;
use futures_util::StreamExt as _;
use tokio::sync::broadcast::error::RecvError;
use tracing::{info, warn};

use crate::AppState;

pub async fn ws_handler(
    req: HttpRequest,
    body: web::Payload,
    app_state: web::Data<AppState>,
) -> Result<HttpResponse, Error> {
    let (response, mut session, mut msg_stream) = actix_ws::handle(&req, body)?;

    let mut alert_rx = app_state.alert_sender.subscribe();
    let mut sensor_rx = app_state.sensor_sender.subscribe();
    let mut health_rx = app_state.health_sender.subscribe();

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
                // LUỒNG NGHE CÁC BẢN TIN TỔNG HỢP (Alert, FSM, Blockchain)
                alert_result = alert_rx.recv() => {
                    match alert_result {
                        Ok(alert_msg) => {
                            let ws_msg = serde_json::json!({
                                "type": "alert",
                                "payload": alert_msg
                            });

                            if let Ok(json_str) = serde_json::to_string(&ws_msg) {
                                if session.text(json_str).await.is_err() {
                                    break;
                                }
                            }
                        }
                        Err(RecvError::Lagged(_)) => {
                            warn!("WS Client {} is too slow, missed some alerts", client_ip);
                        }
                        Err(RecvError::Closed) => break,
                    }
                }

                // LUỒNG NGHE SENSOR DATA
                sensor_result = sensor_rx.recv() => {
                    match sensor_result {
                        Ok(sensor_data) => {
                            let ws_msg = serde_json::json!({
                                "type": "sensor_update",
                                "payload": sensor_data
                            });

                            if let Ok(json_str) = serde_json::to_string(&ws_msg) {
                                if session.text(json_str).await.is_err() {
                                    break;
                                }
                            }
                        }
                        Err(RecvError::Lagged(_)) => {
                            warn!("WS Client {} is too slow, missed some sensor data", client_ip);
                        }
                        Err(RecvError::Closed) => break,
                    }
                }

                health_result = health_rx.recv() => {
                    match health_result {
                        Ok(health_data) => {
                            let is_status_msg = health_data.get("_msg_type")
                                .and_then(|v| v.as_str())
                                .map_or(false, |s| s == "device_status");

                            let ws_msg = if is_status_msg {
                                serde_json::json!({
                                    "type": "device_status",
                                    "payload": {
                                        "is_online": health_data["is_online"],
                                        "last_seen": health_data["last_seen"]
                                    }
                                })
                            } else {
                                serde_json::json!({
                                    "type": "device_health",
                                    "payload": health_data
                                })
                            };

                            if let Ok(json_str) = serde_json::to_string(&ws_msg) {
                                if session.text(json_str).await.is_err() {
                                    break;
                                }
                            }
                        }
                        Err(RecvError::Lagged(_)) => {
                            warn!("WS Client {} is too slow, missed some health data", client_ip);
                        }
                        Err(RecvError::Closed) => break,
                    }
                }

                // LUỒNG XỬ LÝ CÁC EVENT WEBSOCKET KHÁC
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
