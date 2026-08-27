#pragma once
#include <stdint.h>

#ifdef __cplusplus
extern "C" {
#endif

/* ─── DS18B20 / OneWire ─── */
/**
 * Khởi tạo OneWire + DallasTemperature trên GPIO `pin`.
 * Gọi một lần trong setup.
 */
void ds18b20_init(int pin);

/**
 * Đọc nhiệt độ (°C). Trả về NaN nếu thiết bị bị ngắt kết nối.
 */
float ds18b20_read_c(void);

/* ─── ADS1115 (I2C) ─── */
/**
 * Init ADS1115 trên địa chỉ I2C `addr` với GAIN đã chọn.
 * gain_code: 0 = GAIN_TWOTHIRDS, 1 = GAIN_ONE
 * Trả về 1 nếu thành công, 0 nếu không tìm thấy thiết bị.
 */
int ads1115_init(uint8_t addr, int gain_code);

/**
 * Đọc differential ADC (A0-A1) trên ADS tại địa chỉ `addr`, trung bình `samples` lần.
 * Trả về điện áp mV. Trả về NaN nếu lỗi.
 */
float ads1115_read_differential_mv(uint8_t addr, int samples);

/**
 * Đọc single-ended ADC channel `ch` (0..3) trên ADS tại `addr`.
 * Trả về điện áp mV. Trả về NaN nếu lỗi.
 */
float ads1115_read_single_mv(uint8_t addr, int ch, int samples);

/* ─── HC-SR04 Ultrasonic ─── */
/**
 * Khởi tạo GPIO trig/echo.
 */
void hcsr04_init(int trig_pin, int echo_pin);

/**
 * Đo khoảng cách (cm). Trả về 0.0 nếu timeout.
 */
float hcsr04_read_cm(void);

#ifdef __cplusplus
}
#endif
