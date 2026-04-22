use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AlertMessage {
    pub level: String, // "info", "warning", "critical", "success"
    pub title: String,
    pub message: String,
    pub device_id: String,
    pub timestamp: u64,

    pub reason: Option<String>,
    pub metadata: Option<serde_json::Value>,
}
