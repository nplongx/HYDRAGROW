// use crate::models::{PumpStatus, ValveState};
// use std::sync::Mutex;
//
// /// Struct lưu trữ trạng thái mới nhất của hệ thống bơm/van
// /// Sẽ được inject vào Tauri qua `app.manage(ValveGuardState::default())`
// #[derive(Default)]
// pub struct ValveGuardState {
//     pub latest_status: Mutex<PumpStatus>,
// }
//
// impl ValveGuardState {
//     /// Hàm này sẽ được gọi liên tục bởi luồng WebSocket khi nhận được tin nhắn mới từ backend
//     pub fn update_status(&self, new_status: PumpStatus) {
//         if let Ok(mut status) = self.latest_status.lock() {
//             *status = new_status;
//         }
//     }
//
//     /// Kiểm tra tính an toàn trước khi thực hiện lệnh điều khiển Van thủ công
//     pub fn check_safety(
//         &self,
//         target_valve: &str,
//         requested_action: &ValveState,
//     ) -> Result<(), String> {
//         // Nguyên tắc 1: Lệnh ĐÓNG luôn luôn an toàn và được phép thực thi ngay lập tức
//         if *requested_action == ValveState::Closed {
//             return Ok(());
//         }
//
//         // Lấy trạng thái hiện tại (nếu lỗi lock mutex thì chặn luôn cho an toàn)
//         let current_status = self.latest_status.lock().map_err(|_| {
//             "Hệ thống bận, không thể đọc trạng thái van. Đã hủy lệnh để đảm bảo an toàn."
//         })?;
//
//         // Nguyên tắc 2: Kiểm tra chéo (Interlock) khi có lệnh MỞ
//         match target_valve {
//             "VAN_IN" => {
//                 if current_status.van_out == ValveState::Open {
//                     return Err("⛔ XUNG ĐỘT AN TOÀN: Không thể mở VAN_IN (Cấp nước) do VAN_OUT (Xả nước) đang mở!".to_string());
//                 }
//             }
//             "VAN_OUT" => {
//                 if current_status.van_in == ValveState::Open {
//                     return Err("⛔ XUNG ĐỘT AN TOÀN: Không thể mở VAN_OUT (Xả nước) do VAN_IN (Cấp nước) đang mở!".to_string());
//                 }
//             }
//             _ => {
//                 return Err(format!("Tên van không hợp lệ: {}", target_valve));
//             }
//         }
//
//         // Nếu vượt qua mọi bài test -> an toàn
//         Ok(())
//     }
// }
