use actix_web::web;
use hydragrow_shared::events::AppEvent;
use serde_json::json;
use tokio::sync::broadcast;
use tracing::{info, warn};

use crate::AppState;

pub async fn run(mut rx: broadcast::Receiver<AppEvent>, app_state: web::Data<AppState>) {
    info!("event_dispatcher started");

    loop {
        match rx.recv().await {
            Ok(event) => dispatch_event(event, &app_state),
            Err(broadcast::error::RecvError::Lagged(skipped)) => {
                warn!(skipped, "event_dispatcher lagged and dropped events");
            }
            Err(broadcast::error::RecvError::Closed) => {
                warn!("event_dispatcher stopped because event_bus channel is closed");
                break;
            }
        }
    }
}

fn dispatch_event(event: AppEvent, app_state: &web::Data<AppState>) {
    match event {
        AppEvent::SensorUpdate(sensor_data) => {
            let _ = app_state.sensor_sender.send(sensor_data);
        }
        AppEvent::SystemAlert(alert) => {
            let _ = app_state.alert_sender.send(alert);
        }
        AppEvent::DeviceStatus(status) => {
            let _ = app_state.health_sender.send(json!({
                "_msg_type": "device_status",
                "device_id": status.device_id,
                "is_online": status.is_online,
                "last_seen": chrono::Utc::now().to_rfc3339(),
            }));
        }
        AppEvent::FsmTransition(fsm) => {
            let _ = app_state.health_sender.send(json!({
                "_msg_type": "fsm_status",
                "device_id": fsm.device_id,
                "fsm_state": fsm.state,
                "pump_status": fsm.pump_status.unwrap_or_default(),
            }));
        }
        AppEvent::DosingReport(report) => {
            let _ = app_state.health_sender.send(json!({
                "_msg_type": "dosing_report",
                "payload": report,
            }));
        }
        AppEvent::WaterEvent(payload) => {
            let _ = app_state.health_sender.send(json!({
                "_msg_type": "water_event",
                "payload": payload,
            }));
        }
        AppEvent::CalibrationUpdate(payload) => {
            let _ = app_state.health_sender.send(json!({
                "_msg_type": "calibration_update",
                "payload": payload,
            }));
        }
    }
}
