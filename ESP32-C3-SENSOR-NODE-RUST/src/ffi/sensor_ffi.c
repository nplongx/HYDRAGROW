#include "sensor_ffi.h"
#include <math.h>
#include <stdbool.h>
#include <string.h>

/* ─── Arduino/IDF compat includes ─── */
#include "driver/gpio.h"
#include "driver/i2c.h"
#include "freertos/FreeRTOS.h"
#include "freertos/task.h"
#include "esp_log.h"
#include "rom/ets_sys.h" /* ets_delay_us */

static const char* TAG = "sensor_ffi";

/* ═══════════════════════════════════════
DS18B20 — dùng esp-idf-hal qua Rust.
C wrapper chỉ cung cấp placeholder;
Rust gọi one-wire-bus crate trực tiếp.
Xem temp_sensor.rs — C wrapper KHÔNG dùng DallasTemperature.h.
═══════════════════════════════════════ */

/* NOTE: DS18B20 được đọc hoàn toàn từ Rust (ds18b20 crate).
Các hàm ds18b20_* ở đây là stub để build.rs không lỗi;
chúng không được gọi ở runtime. */

static int _ds18b20_pin = -1;

void ds18b20_init(int pin) {
    _ds18b20_pin = pin;
    ESP_LOGI(TAG, "DS18B20 init stub (handled by Rust ds18b20 crate), pin=%d", pin);
}

float ds18b20_read_c(void) {
    /* Không dùng — Rust gọi trực tiếp qua ds18b20 crate */
    return NAN;
}

/* ═══════════════════════════════════════
ADS1115 — dùng ESP-IDF I2C master API.
═══════════════════════════════════════ */

/* ADS1115 registers */
#define ADS1115_REG_CONVERT 0x00
#define ADS1115_REG_CONFIG 0x01

/* Config bits */
#define ADS1115_OS_SINGLE (1 << 15)
#define ADS1115_MUX_DIFF_01 (0x0 << 12)
#define ADS1115_MUX_AIN0 (0x4 << 12)
#define ADS1115_MUX_AIN1 (0x5 << 12)
#define ADS1115_MUX_AIN2 (0x6 << 12)
#define ADS1115_MUX_AIN3 (0x7 << 12)
#define ADS1115_PGA_6_144V (0x0 << 9) /* GAIN_TWOTHIRDS */
#define ADS1115_PGA_4_096V (0x1 << 9) /* GAIN_ONE */
#define ADS1115_MODE_SINGLE (1 << 8)
#define ADS1115_DR_128SPS (0x4 << 5)
#define ADS1115_COMP_DISABLE 0x0003

#define I2C_PORT I2C_NUM_0
#define I2C_SDA_PIN 6
#define I2C_SCL_PIN 7
#define I2C_FREQ_HZ 100000
#define I2C_TIMEOUT_MS 100

/* Track gain per address */
static float _ads_pga_mv[256] = {0};
static int _ads_initialized[256] = {0};

static esp_err_t ads1115_write_reg(uint8_t addr, uint8_t reg, uint16_t value) {
    uint8_t data[3] = {reg, (uint8_t)(value >> 8), (uint8_t)(value & 0xFF)};
    i2c_cmd_handle_t cmd = i2c_cmd_link_create();
    i2c_master_start(cmd);
    i2c_master_write_byte(cmd, (addr << 1) | I2C_MASTER_WRITE, true);
    i2c_master_write(cmd, data, 3, true);
    i2c_master_stop(cmd);
    esp_err_t ret = i2c_master_cmd_begin(I2C_PORT, cmd, pdMS_TO_TICKS(I2C_TIMEOUT_MS));
    i2c_cmd_link_delete(cmd);
    return ret;
}

static esp_err_t ads1115_read_reg(uint8_t addr, uint8_t reg, int16_t* out) {
    uint8_t buf[2];
    i2c_cmd_handle_t cmd = i2c_cmd_link_create();
    i2c_master_start(cmd);
    i2c_master_write_byte(cmd, (addr << 1) | I2C_MASTER_WRITE, true);
    i2c_master_write_byte(cmd, reg, true);
    i2c_master_start(cmd);
    i2c_master_write_byte(cmd, (addr << 1) | I2C_MASTER_READ, true);
    i2c_master_read(cmd, buf, 2, I2C_MASTER_LAST_NACK);
    i2c_master_stop(cmd);
    esp_err_t ret = i2c_master_cmd_begin(I2C_PORT, cmd, pdMS_TO_TICKS(I2C_TIMEOUT_MS));
    i2c_cmd_link_delete(cmd);
    if (ret == ESP_OK) {
        *out = (int16_t)((buf[0] << 8) | buf[1]);
    }
    return ret;
}

int ads1115_init(uint8_t addr, int gain_code) {
    /* gain_code: 0=TWOTHIRDS(±6.144V), 1=ONE(±4.096V) */
    _ads_pga_mv[addr] = (gain_code == 0) ? 6144.0f : 4096.0f;

    /* Probe: thử đọc config register */
    int16_t tmp;
    esp_err_t ret = ads1115_read_reg(addr, ADS1115_REG_CONFIG, &tmp);
    if (ret != ESP_OK) {
        ESP_LOGE(TAG, "ADS1115 @ 0x%02X not found (err %d)", addr, ret);
        return 0;
    }
    _ads_initialized[addr] = 1;
    ESP_LOGI(TAG, "ADS1115 @ 0x%02X OK, pga=%.0f mV", addr, _ads_pga_mv[addr]);
    return 1;
}

static float ads1115_raw_to_mv(uint8_t addr, int16_t raw) {
    /* fullscale = pga_mv, 32767 counts = fullscale */
    return (float)raw * _ads_pga_mv[addr] / 32767.0f;
}

static uint16_t _mux_for_single(int ch) {
    switch (ch) {
        case 0: return ADS1115_MUX_AIN0;
        case 1: return ADS1115_MUX_AIN1;
        case 2: return ADS1115_MUX_AIN2;
        case 3: return ADS1115_MUX_AIN3;
        default: return ADS1115_MUX_AIN0;
    }
}

static int16_t ads1115_one_shot(uint8_t addr, uint16_t mux) {
    uint16_t gain_bits = (_ads_pga_mv[addr] > 4100.0f) ? ADS1115_PGA_6_144V : ADS1115_PGA_4_096V;
    uint16_t config = ADS1115_OS_SINGLE | mux | gain_bits |
                      ADS1115_MODE_SINGLE | ADS1115_DR_128SPS | ADS1115_COMP_DISABLE;
    ads1115_write_reg(addr, ADS1115_REG_CONFIG, config);
    /* Wait ~8ms for 128SPS conversion */
    vTaskDelay(pdMS_TO_TICKS(10));
    int16_t raw = 0;
    ads1115_read_reg(addr, ADS1115_REG_CONVERT, &raw);
    return raw;
}

float ads1115_read_differential_mv(uint8_t addr, int samples) {
    if (!_ads_initialized[addr]) return NAN;
    int32_t sum = 0;
    for (int i = 0; i < samples; i++) {
        sum += ads1115_one_shot(addr, ADS1115_MUX_DIFF_01);
    }
    float raw_avg = (float)sum / samples;
    return ads1115_raw_to_mv(addr, (int16_t)raw_avg);
}

float ads1115_read_single_mv(uint8_t addr, int ch, int samples) {
    if (!_ads_initialized[addr]) return NAN;
    uint16_t mux = _mux_for_single(ch);
    int32_t sum = 0;
    for (int i = 0; i < samples; i++) {
        sum += ads1115_one_shot(addr, mux);
    }
    float raw_avg = (float)sum / samples;
    return ads1115_raw_to_mv(addr, (int16_t)raw_avg);
}

/* ═══════════════════════════════════════
HC-SR04 Ultrasonic
═══════════════════════════════════════ */

static int _trig_pin = -1;
static int _echo_pin = -1;

void hcsr04_init(int trig_pin, int echo_pin) {
    _trig_pin = trig_pin;
    _echo_pin = echo_pin;

    gpio_config_t trig_cfg = {
        .pin_bit_mask = (1ULL << trig_pin),
        .mode = GPIO_MODE_OUTPUT,
        .pull_up_en = GPIO_PULLUP_DISABLE,
        .pull_down_en = GPIO_PULLDOWN_DISABLE,
        .intr_type = GPIO_INTR_DISABLE,
    };
    gpio_config(&trig_cfg);

    gpio_config_t echo_cfg = {
        .pin_bit_mask = (1ULL << echo_pin),
        .mode = GPIO_MODE_INPUT,
        .pull_up_en = GPIO_PULLUP_DISABLE,
        .pull_down_en = GPIO_PULLDOWN_ENABLE,
        .intr_type = GPIO_INTR_DISABLE,
    };
    gpio_config(&echo_cfg);

    gpio_set_level(trig_pin, 0);
}

float hcsr04_read_cm(void) {
    if (_trig_pin < 0 || _echo_pin < 0) return 0.0f;

    /* Trigger pulse 20µs */
    gpio_set_level(_trig_pin, 0);
    ets_delay_us(2);
    gpio_set_level(_trig_pin, 1);
    ets_delay_us(20);
    gpio_set_level(_trig_pin, 0);

    /* Wait for echo HIGH */
    uint32_t timeout = 20000; /* 20ms timeout */
    while (gpio_get_level(_echo_pin) == 0 && timeout--) {
        ets_delay_us(1);
    }
    if (timeout == 0) return 0.0f;

    /* Measure HIGH duration */
    uint32_t duration = 0;
    timeout = 20000;
    while (gpio_get_level(_echo_pin) == 1 && timeout--) {
        ets_delay_us(1);
        duration++;
    }
    if (timeout == 0) return 0.0f;

    /* distance = duration(µs) / 2 * 0.0343 cm/µs */
    return (float)duration * 0.01715f;
}
