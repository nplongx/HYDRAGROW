// use actix_web::web;
// use sqlx::Row; // Thêm thư viện Row để map dữ liệu
// use std::time::{Duration, SystemTime, UNIX_EPOCH};
// use tokio::time::interval;
// use tracing::{error, info, instrument, warn};
//
// use crate::AppState;
// use crate::services::tuya;
//
// /// Vòng lặp Scheduler chạy ngầm kiểm tra định kỳ mỗi 30 giây
// #[instrument(skip(app_state))]
// pub async fn start_tuya_scheduler(app_state: web::Data<AppState>, target_device_id: String) {
//     tokio::spawn(async move {
//         let mut ticker = interval(Duration::from_secs(30));
//         let mut is_pump_currently_on = false;
//
//         info!(
//             "🚀 Khởi động Scheduler thông minh (30s/tick) cho: {}",
//             target_device_id
//         );
//
//         loop {
//             ticker.tick().await;
//
//             // ==========================================
//             // 1. ĐỌC CẤU HÌNH TỪ POSTGRES (Real-time)
//             // ==========================================
//             // Dùng sqlx::query thay vì query! để tránh check compile-time
//             // Đổi `?` thành `$1` cho Postgres
//             let config_result = sqlx::query(
//                 r#"
//                 SELECT circulation_mode, circulation_on_sec, circulation_off_sec
//                 FROM water_config
//                 WHERE device_id = $1
//                 "#,
//             )
//             .bind(&target_device_id)
//             .fetch_optional(&app_state.pg_pool) // Đã đổi sang pg_pool
//             .await;
//
//             // Xử lý default values nếu DB chưa có dữ liệu
//             let (mode, on_sec, off_sec) = match config_result {
//                 Ok(Some(row)) => (
//                     row.get::<String, _>("circulation_mode"),
//                     row.get::<i32, _>("circulation_on_sec") as i64, // Ép i32 của DB sang i64
//                     row.get::<i32, _>("circulation_off_sec") as i64,
//                 ),
//                 _ => ("auto".to_string(), 900, 2700), // Fallback an toàn
//             };
//
//             // ==========================================
//             // 2. TÍNH TOÁN LỊCH TRÌNH (Bằng Toán học)
//             // ==========================================
//             let is_scheduled_time = if mode == "always_on" {
//                 true
//             } else if mode == "always_off" || (on_sec == 0 && off_sec == 0) {
//                 false
//             } else {
//                 // Chế độ "auto" -> Áp dụng trick Modulo chia lấy dư
//                 let now_sec = SystemTime::now()
//                     .duration_since(UNIX_EPOCH)
//                     .unwrap_or_default()
//                     .as_secs();
//
//                 let cycle_total = on_sec + off_sec;
//                 (now_sec as i64 % cycle_total) < on_sec
//             };
//
//             // ==========================================
//             // 3. ĐÁNH GIÁ AN TOÀN TỪ ESP32
//             // ==========================================
//             let is_safe_to_pump = {
//                 let states = app_state.device_states.read().await;
//                 match states.get(&target_device_id) {
//                     Some(state) => state == "Monitoring", // Chỉ an toàn khi FSM đang rảnh
//                     None => true, // Nếu chưa có data ESP32, tạm cho là an toàn
//                 }
//             };
//
//             // ==========================================
//             // 4. RA QUYẾT ĐỊNH & GỌI TUYA
//             // ==========================================
//             let target_state = is_scheduled_time && is_safe_to_pump;
//
//             // Chỉ gọi API Tuya nếu trạng thái mong muốn KHÁC trạng thái hiện tại
//             if target_state != is_pump_currently_on {
//                 if target_state {
//                     info!(
//                         "🕒 Chế độ {}: Đúng lịch ({}s/{}s) & ESP32 an toàn. BẬT BƠM!",
//                         mode, on_sec, off_sec
//                     );
//                     match tuya::send_tuya_command(true).await {
//                         Ok(_) => is_pump_currently_on = true,
//                         Err(e) => error!("❌ Lỗi bật bơm Tuya: {:?}", e),
//                     }
//                 } else {
//                     if is_scheduled_time && !is_safe_to_pump {
//                         warn!(
//                             "🚨 Đang trong giờ tưới nhưng ESP32 đang bận pha phân! NGẮT BƠM ĐỂ BẢO VỆ RỄ."
//                         );
//                     } else {
//                         info!("🕒 Chế độ {}: Hết chu kỳ tưới. TẮT BƠM.", mode);
//                     }
//
//                     match tuya::send_tuya_command(false).await {
//                         Ok(_) => is_pump_currently_on = false,
//                         Err(e) => error!("❌ Lỗi tắt bơm Tuya: {:?}", e),
//                     }
//                 }
//             }
//         }
//     });
// }
