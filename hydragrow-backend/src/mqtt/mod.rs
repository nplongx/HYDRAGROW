use actix_web::web;
use rumqttc::Publish;
use tracing::{debug, instrument, warn};

use crate::{
    AppState,
    metrics::{MQTT_MESSAGES_RECEIVED_TOTAL, SENSOR_UPDATES_TOTAL},
};

use hydragrow_shared::topics::parse_agitech_topic;

pub mod handlers; // Import thư mục con

#[instrument(skip(app_state, publish), fields(topic = %publish.topic))]
pub async fn process_message(publish: Publish, app_state: web::Data<AppState>) {
    let topic = publish.topic.clone();
    let payload_bytes = publish.payload;

    let parsed = match parse_agitech_topic(&topic) {
        Some(v) => v,
        None => {
            warn!(
                topic = %topic,
                "Bỏ qua topic không đúng chuẩn"
            );
            return;
        }
    };

    let device_id = parsed.device_id.to_string();
    let suffix = format!("/{}", parsed.suffix);

    // ============================================================
    // MQTT metrics
    // ============================================================

    MQTT_MESSAGES_RECEIVED_TOTAL
        .with_label_values(&[suffix.as_str()])
        .inc();

    // ============================================================
    // Message routing
    // ============================================================

    match suffix.as_str() {
        "/sensors" => {
            SENSOR_UPDATES_TOTAL.inc();

            handlers::sensors::handle(device_id, &payload_bytes, app_state).await
        }

        "/status" => {
            handlers::status::handle_device(device_id, "Trạm Điều Khiển", &payload_bytes, app_state)
                .await
        }

        "/sensor/status" => {
            handlers::status::handle_device(device_id, "Mạch Cảm Biến", &payload_bytes, app_state)
                .await
        }

        "/controller/status" => {
            debug!(device_id = %device_id, "Routing controller_status message");
            handlers::status::handle_controller(device_id, &payload_bytes, app_state).await
        }

        "/fsm/state" => handlers::fsm::handle_state(device_id, &payload_bytes, app_state).await,

        "/fsm/events" => handlers::events::handle(device_id, &payload_bytes, app_state).await,

        "/system_log" => handlers::system_log::handle(device_id, &payload_bytes, app_state).await,

        "/calibration" => {
            let payload_json = match serde_json::from_slice::<serde_json::Value>(&payload_bytes) {
                Ok(v) => v,

                Err(e) => {
                    warn!(
                        error = ?e,
                        topic = %topic,
                        device_id = %device_id,
                        "Lỗi parse calibration payload"
                    );

                    return;
                }
            };

            handlers::fsm::handle_calibration_update(&device_id, &payload_json, app_state).await;
        }

        "/dosing_cycle" => {
            handlers::dosing_cycle::handle_dosing_cycle(device_id, &payload_bytes, app_state).await
        }

        "/water_cycle" => handlers::water_cycle::handle(device_id, &payload_bytes, app_state).await,

        "/fsm/transition" => {
            handlers::fsm::handle_fsm_transition(device_id, &payload_bytes, app_state).await
        }

        _ => {
            debug!(
                topic = %topic,
                "Nhận được topic không quản lý"
            );
        }
    }
}
