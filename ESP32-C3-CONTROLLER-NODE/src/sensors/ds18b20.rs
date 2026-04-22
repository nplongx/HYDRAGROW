use ds18b20::Ds18b20;
use esp_idf_hal::delay::Ets;
use esp_idf_hal::gpio::{InputOutput, PinDriver};
use one_wire_bus::{Address, OneWire};
use std::thread;
use std::time::Duration;

/// Struct quản lý cảm biến nhiệt độ DS18B20
pub struct HydroponicTempSensor<'a> {
    one_wire: OneWire<PinDriver<'a, InputOutput>>,
    device_address: Option<Address>,
}

impl<'a> HydroponicTempSensor<'a> {
    /// Khởi tạo bus OneWire và tìm kiếm cảm biến DS18B20
    pub fn new(mut pin_driver: PinDriver<'a, InputOutput>) -> anyhow::Result<Self> {
        // Đảm bảo chân GPIO ở trạng thái High trước khi khởi tạo
        pin_driver.set_high()?;

        let mut one_wire = OneWire::new(pin_driver).unwrap();
        let mut delay = Ets; // Ets delay cung cấp độ trễ micro-giây chính xác cho phần cứng ESP

        // Tìm kiếm thiết bị đầu tiên trên OneWire bus
        let mut search_state = None;
        let mut device_address = None;

        log::info!("Đang quét OneWire bus để tìm DS18B20...");

        // one_wire.device_search trả về (Address, State)
        if let Ok(Some((addr, _))) =
            one_wire.device_search(search_state.as_ref(), false, &mut delay)
        {
            if addr.family_code() == ds18b20::FAMILY_CODE {
                log::info!("Tìm thấy DS18B20 với ROM: {:?}", addr);
                device_address = Some(addr);
            } else {
                log::warn!("Tìm thấy thiết bị OneWire nhưng không phải DS18B20.");
            }
        }

        if device_address.is_none() {
            log::error!("Cảnh báo: Không tìm thấy cảm biến DS18B20! Kiểm tra lại dây hoặc điện trở Pull-up 4.7k.");
        }

        Ok(Self {
            one_wire,
            device_address,
        })
    }

    /// Yêu cầu đo và đọc nhiệt độ
    pub fn read_temperature(&mut self) -> anyhow::Result<Option<f32>> {
        let Some(address) = self.device_address else {
            log::warn!("Bỏ qua đọc nhiệt độ do chưa kết nối cảm biến.");
            return Ok(None);
        };

        let mut delay = Ets;
        let sensor = Ds18b20::new::<()>(address).unwrap();

        // 1. Gửi lệnh bắt đầu đo nhiệt độ (Start Temperature Conversion)
        // Việc này tốn rất ít thời gian
        sensor
            .start_temp_measurement(&mut self.one_wire, &mut delay)
            .map_err(|_| anyhow::anyhow!("Lỗi khi gửi lệnh đo nhiệt độ"))?;

        // 2. Chờ cảm biến xử lý (Non-blocking cho các thread khác)
        // Độ phân giải mặc định 12-bit cần tối đa 750ms.
        // Ta dùng thread::sleep để nhường CPU cho MQTT/WiFi chạy ngầm.
        thread::sleep(Duration::from_millis(750));

        // 3. Đọc dữ liệu từ Scratchpad của cảm biến
        let sensor_data = sensor
            .read_data(&mut self.one_wire, &mut delay)
            .map_err(|_| anyhow::anyhow!("Lỗi khi đọc dữ liệu DS18B20"))?;

        Ok(Some(sensor_data.temperature))
    }
}
