//! FFI declarations — kết nối với sensor_ffi.c qua cc crate.

use std::os::raw::{c_float, c_int};

extern "C" {
    // DS18B20 — stub (Rust dùng ds18b20 crate trực tiếp)
    pub fn ds18b20_init(pin: c_int);

    // ADS1115
    pub fn ads1115_init(addr: u8, gain_code: c_int) -> c_int;
    pub fn ads1115_read_differential_mv(addr: u8, samples: c_int) -> c_float;
    pub fn ads1115_read_single_mv(addr: u8, ch: c_int, samples: c_int) -> c_float;

    // HC-SR04
    pub fn hcsr04_init(trig_pin: c_int, echo_pin: c_int);
    pub fn hcsr04_read_cm() -> c_float;
}

/// ADS1115 I2C addresses (khớp SENSOR-NODE C++)
pub const ADS_PH_ADDR: u8 = 0x48; // ADDR -> GND
pub const ADS_TDS_ADDR: u8 = 0x49; // ADDR -> VCC

/// Gain codes cho ads1115_init
pub const GAIN_TWOTHIRDS: i32 = 0; // +/-6.144V — dùng cho pH
pub const GAIN_ONE: i32 = 1; // +/-4.096V — dùng cho TDS
