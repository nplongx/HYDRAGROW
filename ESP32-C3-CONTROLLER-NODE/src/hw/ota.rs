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
            "timestamp_ms": crate::utils::get_current_time_sec() * 1000,
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
    let mut buf = [0u8; 4096];
    let bytes_read = http_client.read(&mut buf)?;
    let json_str = std::str::from_utf8(&buf[..bytes_read])?;

    // Parse JSON tìm browser_download_url
    let parsed: serde_json::Value = serde_json::from_str(json_str)?;
    let tag_name = parsed["tag_name"].as_str().unwrap_or("");

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

    Ok(())
}
