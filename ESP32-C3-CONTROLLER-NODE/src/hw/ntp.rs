// src/hw/ntp.rs
use esp_idf_svc::sntp::{EspSntp, OperatingMode, SntpConf, SyncStatus};
use log::{info, warn};
use std::thread;
use std::time::{Duration, Instant};

pub fn sync_sntp_time() -> anyhow::Result<EspSntp<'static>> {
    info!("⏰ Khởi tạo SNTP và thiết lập múi giờ (UTC+7)...");

    // Cấu hình các server NTP uy tín & nhanh
    let conf = SntpConf {
        operating_mode: OperatingMode::Poll,
        servers: ["time.google.com"],
        ..Default::default()
    };

    let sntp = EspSntp::new(&conf)?;

    unsafe {
        esp_idf_svc::sys::setenv(
            b"TZ\0".as_ptr() as *const _,
            b"ICT-7\0".as_ptr() as *const _,
            1,
        );
        esp_idf_svc::sys::tzset();
    }

    info!("⏳ Đang chờ đồng bộ thời gian từ Internet (tối đa 8 giây)...");

    let start = Instant::now();
    let timeout = Duration::from_secs(8); // Giới hạn tối đa 8 giây

    while sntp.get_sync_status() != SyncStatus::Completed {
        if start.elapsed() >= timeout {
            warn!("⚠️ Chưa thể đồng bộ NTP ngay lúc này (Timeout). SNTP sẽ tự cập nhật ngầm khi có mạng. Bắt đầu FSM...");
            return Ok(sntp);
        }
        thread::sleep(Duration::from_millis(500));
    }

    info!("✅ Đồng bộ thời gian NTP thành công!");
    Ok(sntp)
}
