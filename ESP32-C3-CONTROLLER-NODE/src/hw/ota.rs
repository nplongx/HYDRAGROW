// src/hw/ota.rs
use esp_idf_svc::http::client::{Configuration, EspHttpConnection};
use esp_idf_svc::http::Method;
use esp_idf_svc::ota::EspOta;
use esp_idf_sys::esp_crt_bundle_attach;
use log::{error, info, warn};
use std::sync::mpsc::Sender;
use std::time::Duration;

const GITHUB_API_URL: &str = "https://api.github.com/repos/nplongx/HYDRAGROW/releases/latest";
pub const CURRENT_VERSION: &str = concat!("v", env!("CARGO_PKG_VERSION"));

fn publish_ota_event(
    mqtt_tx: &Option<Sender<String>>,
    device_id: &str,
    level: &str,
    title: &str,
    message: &str,
) {
    if let Some(tx) = mqtt_tx {
        let payload = serde_json::json!({
            "type": "system_alert", "device_id": device_id, "level": level,
            "category": "system", "title": title, "message": message,
            "timestamp_ms": hydragrow_controller_core::utils::get_current_time_sec() * 1000,
        });
        let _ = tx.send(payload.to_string());
    }
}

pub fn perform_ota_update(device_id: &str, mqtt_tx: Option<Sender<String>>) -> anyhow::Result<()> {
    info!("🔄 [OTA] Bắt đầu tiến trình kiểm tra cập nhật từ GitHub...");
    publish_ota_event(
        &mqtt_tx,
        device_id,
        "Info",
        "Bắt đầu cập nhật firmware",
        "Đang kiểm tra phiên bản mới trên GitHub...",
    );

    // 1. Cấu hình HTTP Client với mảng chứng chỉ SSL có sẵn của ESP-IDF
    let http_config = Configuration {
        crt_bundle_attach: Some(esp_crt_bundle_attach),
        timeout: Some(Duration::from_secs(10)),
        ..Default::default()
    };

    let mut http_client = EspHttpConnection::new(&http_config)?;

    // 2. Gọi API GitHub để lấy thông tin Release
    // GitHub API bắt buộc phải có header User-Agent
    let headers = [
        ("User-Agent", "Hydragrow-ESP32"),
        ("Accept", "application/vnd.github.v3+json"),
    ];

    http_client.initiate_request(Method::Get, GITHUB_API_URL, &headers)?;
    http_client.initiate_response()?;

    if http_client.status() != 200 {
        error!(
            "❌ [OTA] Lỗi khi gọi GitHub API. HTTP Status: {}",
            http_client.status()
        );
        publish_ota_event(
            &mqtt_tx,
            device_id,
            "Critical",
            "Cập nhật firmware thất bại",
            "Không thể kiểm tra GitHub Releases.",
        );
        return Err(anyhow::anyhow!("GitHub API failed"));
    }

    // Đọc JSON response
    let mut response_buf: Vec<u8> = Vec::with_capacity(8192);
    let mut chunk = [0u8; 1024];
    loop {
        let n = http_client.read(&mut chunk)?;
        if n == 0 {
            break;
        }
        response_buf.extend_from_slice(&chunk[..n]);
        if response_buf.len() >= 32768 {
            warn!("⚠️ [OTA] GitHub API response > 32KB, cắt bớt để tiết kiệm heap");
            break;
        }
    }
    let json_str = std::str::from_utf8(&response_buf)
        .map_err(|e| anyhow::anyhow!("GitHub API response không phải UTF-8: {}", e))?;

    // Parse JSON tìm browser_download_url
    let parsed: serde_json::Value = serde_json::from_str(json_str).map_err(|e| {
        error!(
            "❌ [OTA] Không parse được GitHub API response ({} bytes): {}",
            json_str.len(),
            e
        );
        anyhow::anyhow!("JSON parse error: {}", e)
    })?;

    let tag_name = parsed["tag_name"].as_str().unwrap_or("");
    if tag_name.is_empty() {
        error!(
            "❌ [OTA] GitHub API không trả về tag_name. Response preview: {}",
            &json_str[..json_str.len().min(200)]
        );
        return Err(anyhow::anyhow!("tag_name missing in GitHub response"));
    }

    if tag_name == CURRENT_VERSION {
        info!(
            "✅ [OTA] Đang ở phiên bản mới nhất ({}). Không cần cập nhật.",
            CURRENT_VERSION
        );
        publish_ota_event(
            &mqtt_tx,
            device_id,
            "Info",
            "Firmware đã mới nhất",
            CURRENT_VERSION,
        );
        return Ok(());
    }

    info!(
        "🚀 [OTA] Tìm thấy phiên bản mới: {}. Đang tìm link tải...",
        tag_name
    );

    let mut download_url = String::new();
    if let Some(assets) = parsed["assets"].as_array() {
        for asset in assets {
            if asset["name"].as_str().unwrap_or("") == "firmware.bin" {
                download_url = asset["browser_download_url"]
                    .as_str()
                    .unwrap_or("")
                    .to_string();
                break;
            }
        }
    }

    if download_url.is_empty() {
        error!("❌ [OTA] Không tìm thấy file 'firmware.bin' trong Release Assets.");
        publish_ota_event(
            &mqtt_tx,
            device_id,
            "Critical",
            "Cập nhật firmware thất bại",
            "Không tìm thấy firmware.bin trong bản phát hành.",
        );
        return Err(anyhow::anyhow!("Binary not found"));
    }

    info!("⬇️ [OTA] Bắt đầu tải firmware từ: {}", download_url);
    publish_ota_event(
        &mqtt_tx,
        device_id,
        "Info",
        "Đang tải firmware",
        &format!("Tìm thấy {}", tag_name),
    );

    // 3. Khởi tạo tiến trình OTA
    let mut ota = EspOta::new()?;
    let mut ota_update = ota.initiate_update()?;

    // Khởi tạo HTTP request mới để tải file Binary
    // Lưu ý: GitHub releases thường redirect (Mã 302) tới máy chủ AWS,
    // EspHttpConnection mặc định sẽ tự động follow redirect.
    let mut download_client = EspHttpConnection::new(&http_config)?;
    download_client.initiate_request(Method::Get, &download_url, &headers)?;
    download_client.initiate_response()?;

    let expected_len: Option<u64> = download_client
        .header("Content-Length")
        .and_then(|s| s.parse::<u64>().ok());
    if expected_len.is_none() {
        warn!("⚠️ [OTA] Server không trả Content-Length — sẽ không thể kiểm tra file bị cắt cụt.");
    }

    let mut binary_buf = [0u8; 2048];
    let mut total_bytes = 0;

    // 4. Vòng lặp đọc stream và ghi thẳng xuống Flash
    loop {
        let n = download_client.read(&mut binary_buf)?;
        if n == 0 {
            break; // Hết file
        }
        ota_update.write(&binary_buf[..n])?;
        total_bytes += n;

        // In log mỗi 100KB tải được để theo dõi
        if total_bytes % 102400 < 2048 {
            info!("⏳ [OTA] Đã tải và ghi: {} KB", total_bytes / 1024);
            publish_ota_event(
                &mqtt_tx,
                device_id,
                "Info",
                "Đang cập nhật firmware",
                &format!("Đã tải {} KB", total_bytes / 1024),
            );
        }
    }

    if let Err(e) = validate_download_length(total_bytes, expected_len) {
        if let Some(expected) = expected_len {
            error!(
                "❌ [OTA] Firmware bị cắt cụt: tải {} bytes, kỳ vọng {} bytes. Huỷ cập nhật.",
                total_bytes, expected
            );
            publish_ota_event(
                &mqtt_tx,
                device_id,
                "Critical",
                "Cập nhật firmware thất bại",
                &format!(
                    "File tải về không đủ ({} / {} bytes) — không commit firmware cụt.",
                    total_bytes, expected
                ),
            );
            return Err(e);
        }
    }

    info!(
        "✅ [OTA] Hoàn tất tải firmware ({} bytes). Chuyển phân vùng boot...",
        total_bytes
    );

    // 5. Commit bản cập nhật và khởi động lại
    ota_update.complete()?;
    publish_ota_event(
        &mqtt_tx,
        device_id,
        "Success",
        "Cập nhật firmware thành công",
        &format!("Đã cập nhật lên {}, đang khởi động lại...", tag_name),
    );

    info!("🔄 [OTA] Đang khởi động lại thiết bị với firmware mới...");
    std::thread::sleep(Duration::from_secs(2));
    unsafe {
        esp_idf_sys::esp_restart();
    }
}

fn validate_download_length(total_bytes: usize, expected_len: Option<u64>) -> anyhow::Result<()> {
    if let Some(expected) = expected_len {
        if total_bytes as u64 != expected {
            return Err(anyhow::anyhow!(
                "OTA download truncated: got {} bytes, expected {}",
                total_bytes,
                expected
            ));
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn validates_download_length_matching() {
        let res = validate_download_length(1024, Some(1024));
        assert!(res.is_ok());
    }

    #[test]
    fn validates_download_length_truncated() {
        let res = validate_download_length(500, Some(1024));
        assert!(res.is_err());
        assert_eq!(
            res.unwrap_err().to_string(),
            "OTA download truncated: got 500 bytes, expected 1024"
        );
    }

    #[test]
    fn validates_download_length_none_expected() {
        let res = validate_download_length(500, None);
        assert!(res.is_ok());
    }

    #[test]
    fn detects_truncated_json() {
        // Mô phỏng JSON bị cắt giữa chừng
        let truncated =
            r#"{"tag_name":"v1.2.3","assets":[{"name":"firmware.bin","browser_download_"#;
        let result = serde_json::from_str::<serde_json::Value>(truncated);
        // Xác nhận parse fail với JSON không đầy đủ
        assert!(result.is_err(), "Truncated JSON phải fail parse");
    }

    #[test]
    fn extracts_tag_and_download_url_from_full_response() {
        let full_json = serde_json::json!({
            "tag_name": "v1.3.0",
            "assets": [
                {
                    "name": "firmware.bin",
                    "browser_download_url": "https://github.com/nplongx/HYDRAGROW/releases/download/v1.3.0/firmware.bin"
                }
            ]
        });
        let tag = full_json["tag_name"].as_str().unwrap_or("");
        assert_eq!(tag, "v1.3.0");

        let url = full_json["assets"]
            .as_array()
            .unwrap()
            .iter()
            .find(|a| a["name"].as_str() == Some("firmware.bin"))
            .and_then(|a| a["browser_download_url"].as_str())
            .unwrap_or("");
        assert!(!url.is_empty());
        assert!(url.ends_with("firmware.bin"));
    }
}
