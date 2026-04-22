use crate::controller::SystemState;
use esp_idf_hal::spi::SpiDeviceDriver;
use std::sync::mpsc::Receiver;
use std::time::Duration;

// Hàm "hack" SPI thành tín hiệu WS2812B
// Ở tốc độ SPI 3.33MHz, 1 bit SPI mất 300ns.
// - WS2812 Bit 0: Cần ~300ns High, ~900ns Low -> Tương đương SPI gửi 0b1000
// - WS2812 Bit 1: Cần ~900ns High, ~300ns Low -> Tương đương SPI gửi 0b1110
fn encode_ws2812(r: u8, g: u8, b: u8) -> [u8; 12] {
    let mut out = [0u8; 12];

    // (Tùy chọn) Giảm độ sáng xuống 30% để khỏi chói mắt
    let r = (r as f32 * 0.3) as u8;
    let g = (g as f32 * 0.3) as u8;
    let b = (b as f32 * 0.3) as u8;

    // WS2812 sử dụng thứ tự truyền là Xanh lá (G) - Đỏ (R) - Xanh dương (B)
    let colors = [g, r, b];
    for (c_idx, &color) in colors.iter().enumerate() {
        for i in 0..4 {
            let mut byte = 0;
            for j in 0..2 {
                let bit = (color >> (7 - (i * 2 + j))) & 1;
                byte |= if bit == 1 { 0b1110 } else { 0b1000 } << (4 - j * 4);
            }
            out[c_idx * 4 + i] = byte;
        }
    }
    out
}

pub fn start_led_task(
    rx: Receiver<SystemState>,
    mut spi: SpiDeviceDriver<'static, esp_idf_hal::spi::SpiDriver<'static>>,
) {
    std::thread::Builder::new()
        .name("led_thread".to_string())
        .spawn(move || {
            let mut current_state = SystemState::Monitoring;
            let mut blink_toggle = false;

            loop {
                // Nhận state mới từ FSM, timeout 500ms để tạo nhịp nháy đèn
                if let Ok(new_state) = rx.recv_timeout(Duration::from_millis(500)) {
                    current_state = new_state;
                }

                blink_toggle = !blink_toggle;

                // Quy định màu sắc (R, G, B)
                let (r, g, b) = match current_state {
                    SystemState::Monitoring => (0, 255, 0), // Xanh lá

                    SystemState::EmergencyStop | SystemState::SystemFault(_) => {
                        if blink_toggle {
                            (255, 0, 0)
                        } else {
                            (0, 0, 0)
                        } // Đỏ chớp
                    }

                    SystemState::WaterRefilling { .. } | SystemState::WaterDraining { .. } => {
                        if blink_toggle {
                            (0, 0, 255)
                        } else {
                            (0, 0, 0)
                        } // Xanh biển chớp
                    }

                    SystemState::StartingOsakaPump { .. }
                    | SystemState::DosingPumpA { .. }
                    | SystemState::WaitingBetweenDose { .. }
                    | SystemState::DosingPumpB { .. } => {
                        if blink_toggle {
                            (255, 255, 0)
                        } else {
                            (0, 0, 0)
                        } // Vàng chớp
                    }

                    SystemState::DosingPH { .. } => {
                        if blink_toggle {
                            (128, 0, 128)
                        } else {
                            (0, 0, 0)
                        } // Tím chớp
                    }

                    SystemState::ActiveMixing { .. } | SystemState::Stabilizing { .. } => {
                        (0, 255, 255) // Lục lam tĩnh
                    }
                };

                let spi_data = encode_ws2812(r, g, b);
                let _ = spi.write(&spi_data); // Đẩy gói tin ra LED
            }
        })
        .expect("❌ Lỗi khởi tạo luồng LED");
}

