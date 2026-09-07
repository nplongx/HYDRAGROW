// src/hw/ota.rs
use esp_idf_svc::http::client::{Configuration, EspHttpConnection};
use esp_idf_svc::http::Method;
use esp_idf_svc::ota::EspOta;
use esp_idf_sys::esp_crt_bundle_attach;
use hydragrow_controller_core::ota_verify;
use log::{error, info, warn};
use sha2::{Digest, Sha256};
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

    // GitHub Release redirects sang host khác; tăng HTTP receive buffer để chứa
    // response header/Location dài mà esp_http_client mặc định có thể không chứa nổi.
    let http_config = Configuration {
        crt_bundle_attach: Some(esp_crt_bundle_attach),
        timeout: Some(Duration::from_secs(60)),
        buffer_size: Some(8192),
        buffer_size_tx: Some(2048),
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

    // Parse toàn bộ metadata trong scope riêng để giải phóng JSON buffer
    // và HTTP/TLS client trước khi mở TLS connection thứ 2 để tải firmware.
    let (tag_name, download_url, sha256_url) = {
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

        // Anti-rollback: chỉ chấp nhận bản mới hơn bản đang chạy một cách
        // nghiêm ngặt. Cùng version, downgrade, và tag sai định dạng đều bị
        // từ chối trước khi chạm vào flash.
        if let Err(reason) = ota_verify::is_upgrade(CURRENT_VERSION, tag_name) {
            info!(
                "✅ [OTA] Từ chối cập nhật ({}); giữ firmware hiện tại {}.",
                reason, CURRENT_VERSION
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

        let release = ota_verify::parse_release_metadata(&parsed).map_err(|e| {
            error!("❌ [OTA] Metadata release không hợp lệ: {}", e);
            anyhow::anyhow!(e)
        })?;
        let download_url = release.firmware_url;
        let sha256_url = match release.sha256_url {
            Some(url) => url,
            None => {
                error!("❌ [OTA] Release thiếu asset 'firmware.bin.sha256'; từ chối cập nhật không kiểm chứng.");
                publish_ota_event(
                    &mqtt_tx,
                    device_id,
                    "Critical",
                    "Cập nhật firmware thất bại",
                    "Bản phát hành thiếu checksum SHA-256.",
                );
                return Err(anyhow::anyhow!("firmware checksum asset missing"));
            }
        };

        (tag_name.to_string(), download_url, sha256_url)
    };

    // Giải phóng TLS connection của GitHub API trước khi tạo connection TLS thứ 2.
    drop(http_client);

    // Tải checksum SHA-256 kỳ vọng trước khi ghi bất kỳ byte firmware nào.
    let expected_sha256 = {
        let mut digest_client = EspHttpConnection::new(&http_config)?;
        digest_client.initiate_request(Method::Get, &sha256_url, &headers)?;
        digest_client.initiate_response()?;
        let mut digest_buf = [0u8; 256];
        let mut digest_body: Vec<u8> = Vec::with_capacity(128);
        loop {
            let n = digest_client.read(&mut digest_buf)?;
            if n == 0 {
                break;
            }
            digest_body.extend_from_slice(&digest_buf[..n]);
            if digest_body.len() >= 256 {
                break;
            }
        }
        drop(digest_client);
        let body = std::str::from_utf8(&digest_body)
            .map_err(|e| anyhow::anyhow!("Checksum response không phải UTF-8: {}", e))?;
        ota_verify::parse_checksum_file(body)
            .ok_or_else(|| anyhow::anyhow!("Không parse được SHA-256 từ checksum asset"))?
    };

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

    // Khởi tạo HTTP request mới để tải file Binary.
    // GitHub releases thường redirect (302) tới máy chủ AWS;
    // buffer 8KB giúp esp_http_client chứa header/redirect dài.
    let mut download_client = EspHttpConnection::new(&http_config)?;
    download_client.initiate_request(Method::Get, &download_url, &headers)?;
    download_client.initiate_response()?;

    let mut binary_buf = [0u8; 2048];
    let mut total_bytes = 0;
    let mut hasher = Sha256::new();

    // 4. Vòng lặp đọc stream và ghi thẳng xuống Flash
    loop {
        let n = download_client.read(&mut binary_buf)?;
        if n == 0 {
            break; // Hết file
        }
        hasher.update(&binary_buf[..n]);
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
        "✅ [OTA] Hoàn tất tải firmware ({} bytes). Kiểm chứng SHA-256...",
        total_bytes
    );

    // Chỉ commit khi digest khớp. Nếu sai, slot OTA không được commit và
    // firmware cũ vẫn là bản bootable — thiết bị không brick.
    let actual_sha256 = hex::encode(hasher.finalize());
    if !actual_sha256.eq_ignore_ascii_case(&expected_sha256) {
        error!("❌ [OTA] SHA-256 mismatch; từ chối commit firmware.");
        publish_ota_event(
            &mqtt_tx,
            device_id,
            "Critical",
            "Cập nhật firmware thất bại",
            "Checksum firmware không khớp; giữ bản cũ.",
        );
        return Err(anyhow::anyhow!("firmware SHA-256 mismatch"));
    }
    info!("✅ [OTA] SHA-256 khớp. Chuyển phân vùng boot...");

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

#[cfg(test)]
mod tests {
    #[test]
    fn detects_truncated_json() {
        let truncated =
            r#"{"tag_name":"v1.2.3","assets":[{"name":"firmware.bin","browser_download_"#;
        let result = serde_json::from_str::<serde_json::Value>(truncated);
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
