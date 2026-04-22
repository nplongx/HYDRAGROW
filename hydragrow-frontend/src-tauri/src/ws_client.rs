// src-tauri/src/ws_client.rs

use futures_util::StreamExt;
use serde::Deserialize;
use tauri::{AppHandle, Emitter, Manager};
use tokio::time::{sleep, Duration};
use tokio_tungstenite::{connect_async, tungstenite::protocol::Message};

use crate::{
    commands::get_settings,
    models::{AlertPayload, SensorData, StatusPayload},
};

/// Giả định cấu trúc message từ Backend gửi qua WS có chứa trường `type` để phân biệt
#[derive(Debug, Deserialize)]
#[serde(tag = "type", content = "payload")]
pub enum WsMessage {
    #[serde(rename = "sensor_update")]
    SensorUpdate(SensorData),
    #[serde(rename = "alert")]
    Alert(AlertPayload),
    #[serde(rename = "device_status")]
    DeviceStatus(StatusPayload),
}

pub async fn start_ws_listener(app: AppHandle, device_id: String) {
    // Chạy trong một luồng nền độc lập (background task)
    tokio::spawn(async move {
        loop {
            // 1. Lấy cấu hình kết nối mới nhất (phòng trường hợp user đổi backend URL trong Settings)
            let settings = match get_settings(&app) {
                Ok(s) => s,
                Err(_) => {
                    sleep(Duration::from_secs(5)).await;
                    continue;
                }
            };

            // Chuyển đổi HTTP URL sang WS URL
            let ws_base = settings
                .backend_url
                .replace("http://", "ws://")
                .replace("https://", "wss://");
            // Gắn API Key và Device ID vào URL hoặc Header tùy thiết kế backend của bạn
            let ws_url = format!(
                "{}/ws?device_id={}&api_key={}",
                ws_base, device_id, settings.api_key
            );

            println!("[WebSocket] Đang cố gắng kết nối tới: {}", ws_url);

            match connect_async(&ws_url).await {
                Ok((ws_stream, _)) => {
                    println!("[WebSocket] Kết nối thành công!");
                    let (_, mut read) = ws_stream.split();

                    // Báo cho React biết thiết bị đã online
                    let _ = app.emit(
                        "device_status",
                        StatusPayload {
                            is_online: true,
                            last_seen: chrono::Utc::now().to_rfc3339(),
                        },
                    );

                    // Lắng nghe tin nhắn liên tục
                    while let Some(msg) = read.next().await {
                        match msg {
                            Ok(Message::Text(text)) => {
                                // Cố gắng parse theo Envelope có type
                                if let Ok(ws_msg) = serde_json::from_str::<WsMessage>(&text) {
                                    match ws_msg {
                                        WsMessage::SensorUpdate(data) => {
                                            // Bắn event cho React
                                            let _ = app.emit("sensor_update", data);
                                        }
                                        WsMessage::Alert(alert) => {
                                            let _ = app.emit("alert", alert.clone());

                                            // 2. Nếu lỗi nghiêm trọng, bắn luôn Notification của hệ điều hành
                                            if alert.level == "critical" {
                                                crate::notification::show_critical_alert(
                                                    &app,
                                                    "Cảnh báo hệ thống Thủy Canh!",
                                                    &format!(
                                                        "Lỗi: {}. Vui lòng kiểm tra ngay!",
                                                        alert.message
                                                    ),
                                                );
                                            }
                                        }
                                        WsMessage::DeviceStatus(status) => {
                                            let _ = app.emit("device_status", status);
                                        }
                                    }
                                } else if let Ok(sensor_data) =
                                    serde_json::from_str::<SensorData>(&text)
                                {
                                    // Fallback: Nếu backend chỉ gửi thẳng cục JSON SensorData mà không có Wrapper
                                    let _ = app.emit("sensor_update", sensor_data);
                                } else {
                                    println!("[WebSocket] Không thể parse bản tin: {}", text);
                                }
                            }
                            Ok(Message::Close(_)) => {
                                println!("[WebSocket] Server chủ động đóng kết nối.");
                                break;
                            }
                            Err(e) => {
                                println!("[WebSocket] Lỗi đọc tin nhắn: {}", e);
                                break;
                            }
                            _ => {} // Bỏ qua các tin nhắn Ping/Pong/Binary
                        }
                    }
                }
                Err(e) => {
                    println!("[WebSocket] Lỗi kết nối: {}. Thử lại sau 5 giây...", e);
                }
            }

            // Nếu vòng lặp đến được đây nghĩa là WS đã bị ngắt. Báo cho UI thiết bị offline.
            let _ = app.emit(
                "device_status",
                StatusPayload {
                    is_online: false,
                    last_seen: chrono::Utc::now().to_rfc3339(),
                },
            );

            // Chờ 5s trước khi tự động Reconnect
            sleep(Duration::from_secs(5)).await;
        }
    });
}
