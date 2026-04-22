// src/services/fcm.rs
use reqwest::Client;
use serde_json::json;
use tracing::{error, info, warn};
use yup_oauth2::{ServiceAccountAuthenticator, read_service_account_key};

// 🔴 THAY BẰNG PROJECT ID FIREBASE CỦA BẠN
const PROJECT_ID: &str = "hydragrow-iot";

async fn get_oauth_token() -> Result<String, Box<dyn std::error::Error>> {
    // File này tải từ Firebase Console, để ở thư mục gốc ngang với Cargo.toml
    let secret = read_service_account_key("firebase-service-account.json").await?;
    let auth = ServiceAccountAuthenticator::builder(secret).build().await?;
    let scopes = &["https://www.googleapis.com/auth/firebase.messaging"];
    let token = auth.token(scopes).await?;

    Ok(token.token().unwrap().to_string())
}

pub async fn send_push_notification(title: &str, body: &str, tokens: Vec<String>) {
    if tokens.is_empty() {
        return;
    }

    let oauth_token = match get_oauth_token().await {
        Ok(t) => t,
        Err(e) => {
            error!("❌ Lỗi lấy OAuth2 Token từ Google Service Account: {}", e);
            return;
        }
    };

    let client = Client::new();
    let url = format!(
        "https://fcm.googleapis.com/v1/projects/{}/messages:send",
        PROJECT_ID
    );

    for token in tokens {
        let payload = json!({
            "message": {
                "token": token,
                "notification": { "title": title, "body": body },
                "android": {
                    "priority": "high",
                    "notification": {
                        "sound": "default",
                        "channel_id": "agitech_alarms",
                        "default_vibrate_timings": true
                    }
                }
            }
        });

        match client
            .post(&url)
            .bearer_auth(&oauth_token)
            .json(&payload)
            .send()
            .await
        {
            Ok(res) => {
                if res.status().is_success() {
                    info!("🚀 Đã đẩy Push Notification thành công tới điện thoại!");
                } else {
                    error!(
                        "❌ Lỗi từ FCM Server: {}",
                        res.text().await.unwrap_or_default()
                    );
                }
            }
            Err(e) => error!("❌ Lỗi Network FCM API: {}", e),
        }
    }
}
