//! Cầu nối MỘT CHIỀU: event_bus (AppEvent) → MQTT topic ngoài cho Node-RED.
//! KHÔNG bao giờ subscribe ngược lại — Node-RED không được phép gửi lệnh làm thay đổi
//! FSM/stage. Nếu tương lai cần Node-RED trigger hành động, hành động đó phải đi qua
//! API có auth (giống mọi client khác), không phải qua topic này.

use rumqttc::QoS;
use tracing::warn;

use crate::AppState;
use hydragrow_shared::{events::AppEvent, topics::topic_integration_events};

pub async fn run(app_state: std::sync::Arc<AppState>) {
    let mut event_rx = app_state.event_bus.subscribe();
    loop {
        match event_rx.recv().await {
            Ok(AppEvent::SystemAlert(alert)) => {
                let topic = topic_integration_events(&alert.device_id);
                let payload = match serde_json::to_vec(&serde_json::json!({
                    "type": "alert",
                    "payload": alert,
                })) {
                    Ok(p) => p,
                    Err(e) => {
                        warn!(error = %e, "Không serialize được alert cho integration bridge");
                        continue;
                    }
                };
                if let Err(e) = app_state
                    .mqtt_client
                    .publish(topic, QoS::AtMostOnce, false, payload)
                    .await
                {
                    warn!(error = %e, "Không publish được integration event — Node-RED có thể đang offline");
                }
            }
            Ok(_) => {} // Chỉ fan-out alert ở giai đoạn đầu — mở rộng dần theo nhu cầu tích hợp thực tế.
            Err(tokio::sync::broadcast::error::RecvError::Lagged(skipped)) => {
                warn!(skipped, "integration_bridge bị lag trên event_bus, một số alert có thể đã bị bỏ qua");
            }
            Err(tokio::sync::broadcast::error::RecvError::Closed) => break,
        }
    }
}
