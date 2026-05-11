use actix_web::web;
use rumqttc::Publish;
use tracing::{debug, instrument, warn};

use crate::AppState;
mod handlers; // Import thư mục con

#[inline]
fn parse_agitech_topic(topic: &str) -> Option<(String, String)> {
    // ... (Giữ nguyên logic của bạn) ...
    let prefix = "AGITECH/";
    if !topic.starts_with(prefix) {
        return None;
    }
    let rest = &topic[prefix.len()..];
    let slash = rest.find('/')?;
    Some((rest[..slash].to_string(), rest[slash..].to_string()))
}

#[instrument(skip(app_state, publish), fields(topic = %publish.topic))]
pub async fn process_message(publish: Publish, app_state: web::Data<AppState>) {
    let topic = publish.topic.clone();
    let payload_bytes = publish.payload;

    let (device_id, suffix) = match parse_agitech_topic(&topic) {
        Some(v) => v,
        None => {
            warn!("Bỏ qua topic không đúng chuẩn: {}", topic);
            return;
        }
    };

    match suffix.as_str() {
        "/sensors" => handlers::sensors::handle(device_id, &payload_bytes, app_state).await,
        "/status" => {
            handlers::status::handle_device(device_id, "Trạm Điều Khiển", &payload_bytes, app_state)
                .await
        }
        "/sensor/status" => {
            handlers::status::handle_device(device_id, "Mạch Cảm Biến", &payload_bytes, app_state)
                .await
        }
        "/controller/status" => {
            handlers::status::handle_controller(device_id, &payload_bytes, app_state).await
        }
        "/fsm/state" => handlers::fsm::handle_state(device_id, &payload_bytes, app_state).await,
        "/dosing_report" => {
            handlers::dosing::handle_report(device_id, &payload_bytes, app_state).await
        }

        // 👇 ĐÂY LÀ ĐIỂM THAY ĐỔI LỚN NHẤT CỦA GIAI ĐOẠN 2
        // Gộp chung /fsm/events và /calibration vào một topic duy nhất
        "/system_log" => handlers::system_log::handle(device_id, &payload_bytes, app_state).await,

        _ => debug!("Nhận được topic không quản lý: {}", topic),
    }
}
