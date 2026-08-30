//! Pure-logic core của HYDRAGROW controller: FSM, Kalman filter, MIMO adaptive,
//! dosing/water actors. Không phụ thuộc esp-idf — test được 100% bằng `cargo test`
//! trên host, không cần phần cứng ESP32.

#![allow(clippy::field_reassign_with_default)]

pub mod core;
pub mod pump_types;
pub mod utils;

pub use hydragrow_shared;
pub use pump_types::{PumpType, WaterDirection};

#[cfg(any(feature = "test-support", test))]
pub mod test_support;
