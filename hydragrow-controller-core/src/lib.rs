//! Pure-logic core của HYDRAGROW controller: FSM, Kalman filter, MIMO adaptive,
//! dosing/water actors. Không phụ thuộc esp-idf — test được 100% bằng `cargo test`
//! trên host, không cần phần cứng ESP32.

pub mod pump_types;
pub mod utils;
pub mod core;

pub use hydragrow_shared;
pub use pump_types::{PumpType, WaterDirection};
