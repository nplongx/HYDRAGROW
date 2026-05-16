use actix_web::web;
use hydragrow_shared::events::AppEvent;
use tokio::sync::broadcast;

use crate::AppState;

pub async fn run(mut rx: broadcast::Receiver<AppEvent>, app_state: web::Data<AppState>) {
    while let Ok(event) = rx.recv().await {
        match event {
            AppEvent::SensorUpdate(sensor_data) => { let _ = app_state.sensor_sender.send(sensor_data); }
            AppEvent::SystemAlert(alert) => { let _ = app_state.alert_sender.send(alert); }
            AppEvent::DeviceStatus(status) => { let _ = app_state.health_sender.send(serde_json::json!({"_msg_type":"device_status","device_id":status.device_id,"is_online":status.is_online,"last_seen": chrono::Utc::now().to_rfc3339()})); }
            AppEvent::FsmTransition(fsm) => { let _ = app_state.health_sender.send(serde_json::json!({"_msg_type":"fsm_status","device_id":fsm.device_id,"fsm_state":fsm.state,"pump_status":fsm.pump_status.unwrap_or_default()})); }
            _ => {}
        }
    }
}
