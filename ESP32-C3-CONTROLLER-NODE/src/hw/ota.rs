// src/hw/ota.rs
use esp_idf_svc::http::client::{Configuration, EspHttpConnection};
use esp_idf_svc::http::Method;
use esp_idf_svc::ota::EspOta;
use esp_idf_sys::esp_crt_bundle_attach;
use log::{error, info, warn};
use std::time::Duration;

const GITHUB_API_URL: &str = "https://api.github.com/repos/nplongx/HYDRAGROW/releases/latest";
const CURRENT_VERSION: &str = concat!("v", env!("CARGO_PKG_VERSION"));

pub fn perform_ota_update() -> anyhow::Result<()> {
    info!("🔄 [OTA] Bắt đầu tiến trình kiểm tra cập nhật từ GitHub...");

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
        error!("❌ [OTA] Lỗi khi gọi GitHub API. HTTP Status: {}", http_client.status());
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
        info!("✅ [OTA] Đang ở phiên bản mới nhất ({}). Không cần cập nhật.", CURRENT_VERSION);
        return Ok(());
    }

    info!("🚀 [OTA] Tìm thấy phiên bản mới: {}. Đang tìm link tải...", tag_name);

    let mut download_url = String::new();
    if let Some(assets) = parsed["assets"].as_array() {
        for asset in assets {
            if asset["name"].as_str().unwrap_or("") == "firmware.bin" {
                download_url = asset["browser_download_url"].as_str().unwrap_or("").to_string();
                break;
            }
        }
    }

    if download_url.is_empty() {
        error!("❌ [OTA] Không tìm thấy file 'firmware.bin' trong Release Assets.");
        return Err(anyhow::anyhow!("Binary not found"));
    }

    info!("⬇️ [OTA] Bắt đầu tải firmware từ: {}", download_url);

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
        }
    }

    info!("✅ [OTA] Hoàn tất tải firmware ({} bytes). Chuyển phân vùng boot...", total_bytes);

    // 5. Commit bản cập nhật và khởi động lại
    ota_update.complete()?;
    
    info!("🔄 [OTA] Đang khởi động lại thiết bị với firmware mới...");
    std::thread::sleep(Duration::from_secs(2));
    unsafe {
        esp_idf_sys::esp_restart();
    }

    Ok(())
}