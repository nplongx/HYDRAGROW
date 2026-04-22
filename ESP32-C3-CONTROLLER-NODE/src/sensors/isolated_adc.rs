use crate::config::DeviceConfig;
use esp_idf_hal::gpio::{Output, PinDriver};
use log::debug;
use std::thread;
use std::time::Duration;

// Struct giờ đây vô cùng nhẹ nhàng, triệt tiêu toàn bộ ADC Generics
pub struct IsolatedPhEcReader<'a> {
    ph_mosfet: PinDriver<'a, Output>,
    ec_mosfet: PinDriver<'a, Output>,
}

impl<'a> IsolatedPhEcReader<'a> {
    pub fn new(
        mut ph_mosfet: PinDriver<'a, Output>,
        mut ec_mosfet: PinDriver<'a, Output>,
    ) -> anyhow::Result<Self> {
        // Đảm bảo cả 2 MOSFET đều bị ngắt khi vừa khởi động
        ph_mosfet.set_low()?;
        ec_mosfet.set_low()?;

        Ok(Self {
            ph_mosfet,
            ec_mosfet,
        })
    }
}

// Trả về thẳng giá trị Millivolt (mV)

pub fn convert_voltage_to_ec(voltage_mv: f32, current_temp_c: f32, config: &DeviceConfig) -> f32 {
    // 🟢 SỬA LẠI CÔNG THỨC EC (Dành cho Cảm biến Analog chuẩn)
    // voltage_mv thường từ 0 đến 3000mV.
    // Hệ số ec_factor thường là 1.0 (Không phải 880.0). Tạm thời chia cho 1000 để ra Volt
    let voltage_v = voltage_mv / 1000.0;

    // Công thức tính EC thô (Tùy thuộc mạch, đây là công thức tuyến tính cơ bản)
    // Nếu bạn dùng ec_factor=880 trên Backend, hãy giảm nó xuống 1.0 trên UI React.
    let mut raw_ec = (voltage_v * config.ec_factor) + config.ec_offset;

    // Bù trừ nhiệt độ (Temperature Compensation)
    let temp_coefficient = 1.0 + config.temp_compensation_beta * (current_temp_c - 25.0);
    let mut ec_compensated = raw_ec / temp_coefficient;

    // Chặn khoảng an toàn (Clamp)
    if ec_compensated < 0.0 {
        ec_compensated = 0.0;
    }

    // Đổi thành info! để nhìn thấy sự thật
    log::info!(
        "🧮 EC Calc | ADC đọc được: {:.1}mV -> Tính ra EC: {:.2} mS/cm (Factor={:.2})",
        voltage_mv,
        ec_compensated,
        config.ec_factor
    );

    ec_compensated
}

// =========================================
// CÁC HÀM TÍNH TOÁN HIỆU CHUẨN (Calibration)
// =========================================

pub fn convert_voltage_to_ph(voltage_mv: f32, config: &DeviceConfig) -> f32 {
    // TẠM THỜI BỎ ĐIỀU KIỆN CHẶN VOLTAGE_MV ĐỂ NHÌN THẤY ĐIỆN ÁP THẬT

    // Tránh lỗi chia cho 0 nếu chưa config đúng ph_v4 và ph_v7
    let diff = config.ph_v4 - config.ph_v7;
    let slope = if diff.abs() < 0.1 {
        -0.006 // Giá trị Slope ước lượng mặc định nếu config chưa chuẩn
    } else {
        (4.0 - 7.0) / diff
    };

    // Tính pH
    let mut ph = 7.0 + slope * (voltage_mv - config.ph_v7);

    // Chặn khoảng an toàn (Clamp)
    if ph < 0.0 {
        ph = 0.0;
    } else if ph > 14.0 {
        ph = 14.0;
    }

    // Đổi thành info! để bạn LUÔN LUÔN thấy log này mà không cần cấu hình RUST_LOG
    log::info!(
        "🧮 pH Calc | ADC đọc được: {:.1}mV -> Tính ra pH: {:.2} (Config: v7={:.1}mV, v4={:.1}mV)",
        voltage_mv,
        ph,
        config.ph_v7,
        config.ph_v4
    );

    ph
}
