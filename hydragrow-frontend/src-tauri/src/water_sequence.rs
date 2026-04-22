// // src-tauri/src/water_sequence.rs
//
// use crate::http_client;
// use crate::models::{SensorData, WaterConfig};
// use serde::Serialize;
// use tauri::AppHandle;
// use tokio::time::{sleep, Duration, Instant};
//
// // Helper struct cho lệnh điều khiển bơm
// #[derive(Serialize)]
// struct PumpCommandReq {
//     pub pump: String,
//     pub action: String, // Trạng thái bơm: "forward" (bơm vào), "reverse" (hút ra), "off" (tắt)
//     pub duration_sec: Option<u32>,
// }
//
// // ── Helpers gọi API ───────────────────────────────────────────────────
//
// async fn set_pump(
//     app: &AppHandle,
//     device_id: &str,
//     pump: &str,
//     action: &str,
// ) -> Result<(), String> {
//     let payload = PumpCommandReq {
//         pump: pump.to_string(),
//         action: action.to_string(),
//         duration_sec: None,
//     };
//     http_client::post(
//         app,
//         &format!("/api/devices/{}/control/pump", device_id),
//         &payload,
//     )
//     .await
// }
//
// async fn get_latest_sensor(app: &AppHandle, device_id: &str) -> Result<SensorData, String> {
//     http_client::get(app, &format!("/api/devices/{}/sensors/latest", device_id)).await
// }
//
// // ── Sequences ─────────────────────────────────────────────────────────
//
// /// Sequence CẤP NƯỚC TỰ ĐỘNG (Bơm chiều Forward)
// pub async fn run_refill_sequence(app: AppHandle, device_id: String) -> Result<(), String> {
//     // 1. Lấy WaterConfig để biết target
//     let config: WaterConfig =
//         http_client::get(&app, &format!("/api/devices/{}/config/water", device_id)).await?;
//
//     // Thực thi logic chính trong một block để dễ quản lý lỗi và cleanup
//     let result = async {
//         // Mức nước đã đạt mục tiêu rồi thì không bơm nữa
//         let initial_sensor = get_latest_sensor(&app, &device_id).await?;
//         if initial_sensor.water_level >= config.water_level_target {
//             return Ok(());
//         }
//
//         // Bước 1: Bật BƠM ĐẢO CHIỀU chạy tới (Forward) để hút nước từ bồn dự trữ vào bồn chính
//         // Van 1 chiều sẽ tự động mở do áp lực nước.
//         set_pump(&app, &device_id, "WATER_PUMP", "forward").await?;
//
//         // Bước 2: Poll sensor liên tục cho đến khi đạt target
//         let timeout = Duration::from_secs(30 * 60); // Max 30 phút safety timeout
//         let start_time = Instant::now();
//
//         loop {
//             if start_time.elapsed() > timeout {
//                 return Err(
//                     "Timeout: Quá thời gian cấp nước an toàn (30 phút). Đã ngắt hệ thống!"
//                         .to_string(),
//                 );
//             }
//
//             match get_latest_sensor(&app, &device_id).await {
//                 Ok(sensor) => {
//                     // Cấp nước: dừng khi chạm mốc target
//                     if sensor.water_level >= config.water_level_target {
//                         break;
//                     }
//                 }
//                 Err(e) => {
//                     println!(
//                         "[Sequence Warn] Lỗi đọc sensor khi Refill, thử lại sau... ({})",
//                         e
//                     );
//                 }
//             }
//             sleep(Duration::from_secs(3)).await; // Poll mỗi 3s
//         }
//         Ok(())
//     }
//     .await;
//
//     // BƯỚC DỌN DẸP AN TOÀN (Luôn chạy dù thành công hay có lỗi)
//     // Tắt bơm. Áp lực giảm, van 1 chiều sẽ tự động khóa lại chống rò rỉ ngược.
//     let _ = set_pump(&app, &device_id, "WATER_PUMP", "off").await;
//
//     result
// }
//
// /// Sequence XẢ NƯỚC TỰ ĐỘNG (Bơm chiều Reverse)
// pub async fn run_drain_sequence(app: AppHandle, device_id: String) -> Result<(), String> {
//     // 1. Lấy WaterConfig để biết mốc xả
//     let config: WaterConfig =
//         http_client::get(&app, &format!("/api/devices/{}/config/water", device_id)).await?;
//
//     let result = async {
//         let initial_sensor = get_latest_sensor(&app, &device_id).await?;
//         if initial_sensor.water_level <= config.water_level_drain {
//             return Ok(());
//         }
//
//         // Bước 1: Bật BƠM ĐẢO CHIỀU chạy ngược (Reverse) để hút nước từ bồn chính xả ra ngoài
//         // (Lưu ý: Ống xả cần có nhánh riêng hoặc cấu trúc van 1 chiều phù hợp để xả đúng chỗ)
//         set_pump(&app, &device_id, "WATER_PUMP", "reverse").await?;
//
//         // Bước 2: Poll sensor liên tục
//         let timeout = Duration::from_secs(30 * 60);
//         let start_time = Instant::now();
//
//         loop {
//             if start_time.elapsed() > timeout {
//                 return Err(
//                     "Timeout: Quá thời gian xả nước an toàn (30 phút). Đã ngắt hệ thống!"
//                         .to_string(),
//                 );
//             }
//
//             match get_latest_sensor(&app, &device_id).await {
//                 Ok(sensor) => {
//                     // Xả nước: dừng khi chạm mốc drain (mốc cạn)
//                     if sensor.water_level <= config.water_level_drain {
//                         break;
//                     }
//                 }
//                 Err(e) => {
//                     println!(
//                         "[Sequence Warn] Lỗi đọc sensor khi Drain, thử lại sau... ({})",
//                         e
//                     );
//                 }
//             }
//             sleep(Duration::from_secs(3)).await;
//         }
//         Ok(())
//     }
//     .await;
//
//     // DỌN DẸP
//     let _ = set_pump(&app, &device_id, "WATER_PUMP", "off").await;
//
//     result
// }
