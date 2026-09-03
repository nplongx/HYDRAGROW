// src/hw/pcf857x.rs

use esp_idf_hal::i2c::I2cDriver;
use pcf857x::{Error, Pcf8574, PinFlag, SlaveAddr};
use serde::{Deserialize, Serialize};

// Tank alert state
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct TankAlert {
    pub tank_a_low: bool,
    pub tank_b_low: bool,
    pub tank_ph_down_low: bool,
    pub tank_ph_up_low: bool,
}

impl TankAlert {
    pub fn has_alert(&self) -> bool {
        self.tank_a_low || self.tank_b_low || self.tank_ph_down_low || self.tank_ph_up_low
    }
}

// PCF8574 pins
// P0..P3 = INPUT from TTP223 touch sensors
// P4..P5 = OUTPUT for valves
// P6    = OUTPUT for water pump IN
// P7    = OUTPUT for water pump OUT
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExpanderPin {
    // Inputs - TTP223
    TankA = 0,
    TankB = 1,
    TankPHDown = 2,
    TankPHUp = 3,

    // Outputs - valves
    ValveMist = 4,
    ValveMix = 5,

    // Outputs - water pumps
    WaterPumpIn = 6,
    WaterPumpOut = 7,
}

#[allow(dead_code)]
impl ExpanderPin {
    pub fn mask(self) -> u8 {
        1u8 << (self as u8)
    }

    pub fn flag(self) -> PinFlag {
        match self {
            Self::TankA => PinFlag::P0,
            Self::TankB => PinFlag::P1,
            Self::TankPHDown => PinFlag::P2,
            Self::TankPHUp => PinFlag::P3,
            Self::ValveMist => PinFlag::P4,
            Self::ValveMix => PinFlag::P5,
            Self::WaterPumpIn => PinFlag::P6,
            Self::WaterPumpOut => PinFlag::P7,
        }
    }

    /// True nếu pin này là input từ TTP223.
    pub fn is_input(self) -> bool {
        matches!(
            self,
            Self::TankA | Self::TankB | Self::TankPHDown | Self::TankPHUp
        )
    }

    /// True nếu pin này là output điều khiển valve/bơm nước.
    pub fn is_output(self) -> bool {
        matches!(
            self,
            Self::ValveMist | Self::ValveMix | Self::WaterPumpIn | Self::WaterPumpOut
        )
    }
}

// Masks
// P0..P3 là INPUT.
// Với PCF8574, muốn dùng pin làm input thì phải ghi HIGH vào latch.
// HIGH ở đây có nghĩa là "release" chân, không phải ép tín hiệu HIGH.
const INPUT_MASK: u8 = 0b0000_1111;

// P4..P7 là OUTPUT.
#[allow(dead_code)]
pub const OUTPUT_MASK: u8 = 0b1111_0000;

// ---------------------------------------------------------------------------
// I2C Expander
// ---------------------------------------------------------------------------

pub struct I2cExpander<'d> {
    pcf: Pcf8574<I2cDriver<'d>>,

    /// Shadow state của output latch PCF8574.
    ///
    /// P0..P3 luôn phải là 1.
    /// P4..P7 được điều khiển bởi application.
    state: u8,
}

impl<'d> I2cExpander<'d> {
    /// Tạo PCF8574.
    ///
    /// Initial state:
    ///
    /// P0..P3 = 1 -> input/released
    /// P4..P7 = 0 -> outputs OFF
    pub fn new(i2c_driver: I2cDriver<'d>) -> Self {
        Self {
            pcf: Pcf8574::new(i2c_driver, SlaveAddr::Default),

            // P0..P3 = 1 (input/released)
            // P4..P7 = 0 (outputs LOW)
            state: INPUT_MASK,
        }
    }

    /// Ghi trạng thái ban đầu vào PCF8574.
    ///
    /// PHẢI gọi hàm này sau khi tạo I2cExpander.
    pub fn init(
        &mut self,
    ) -> Result<(), Error<<I2cDriver<'d> as embedded_hal::i2c::ErrorType>::Error>> {
        self.pcf.set(self.state)
    }

    // -----------------------------------------------------------------------
    // Output control
    // -----------------------------------------------------------------------

    /// Bật một output.
    ///
    /// Chỉ cho phép P4..P7.
    pub fn set_high(
        &mut self,
        pin: ExpanderPin,
    ) -> Result<(), Error<<I2cDriver<'d> as embedded_hal::i2c::ErrorType>::Error>> {
        assert!(
            pin.is_output(),
            "Attempted to drive a PCF8574 input pin as output"
        );

        self.state |= pin.mask();

        // Đảm bảo P0..P3 luôn HIGH/released.
        self.state |= INPUT_MASK;

        self.pcf.set(self.state)
    }

    /// Tắt một output.
    ///
    /// Chỉ cho phép P4..P7.
    pub fn set_low(
        &mut self,
        pin: ExpanderPin,
    ) -> Result<(), Error<<I2cDriver<'d> as embedded_hal::i2c::ErrorType>::Error>> {
        assert!(
            pin.is_output(),
            "Attempted to drive a PCF8574 input pin as output"
        );

        self.state &= !pin.mask();

        // Đảm bảo P0..P3 luôn HIGH/released.
        self.state |= INPUT_MASK;

        self.pcf.set(self.state)
    }

    /// Bật/tắt output theo bool.
    #[allow(dead_code)]
    pub fn set_output(
        &mut self,
        pin: ExpanderPin,
        high: bool,
    ) -> Result<(), Error<<I2cDriver<'d> as embedded_hal::i2c::ErrorType>::Error>> {
        if high {
            self.set_high(pin)
        } else {
            self.set_low(pin)
        }
    }

    // Input
    /// Đọc P0..P3.
    ///
    /// TTP223 là active-HIGH:
    ///     Không chạm -> OUT = LOW  -> bit = 0
    ///     Chạm       -> OUT = HIGH -> bit = 1
    pub fn read_all_input(
        &mut self,
    ) -> Result<u8, Error<<I2cDriver<'d> as embedded_hal::i2c::ErrorType>::Error>> {
        let mut buffer = [0u8; 1];

        let input_mask = PinFlag::P0 | PinFlag::P1 | PinFlag::P2 | PinFlag::P3;

        self.pcf.read_array(input_mask, &mut buffer)?;

        // Chỉ trả về P0..P3.
        Ok(buffer[0] & INPUT_MASK)
    }

    /// Đọc một input cụ thể.
    ///
    /// Chỉ cho phép TankA/TankB/TankPHDown/TankPHUp.
    #[allow(dead_code)]
    pub fn read_input(
        &mut self,
        pin: ExpanderPin,
    ) -> Result<bool, Error<<I2cDriver<'d> as embedded_hal::i2c::ErrorType>::Error>> {
        assert!(
            pin.is_input(),
            "Attempted to read a PCF8574 output pin as input"
        );

        let raw = self.read_all_input()?;

        Ok((raw & pin.mask()) != 0)
    }

    // Tank alerts
    /// Đọc toàn bộ 4 cảm biến TTP223.
    ///
    /// Active-HIGH:
    ///     bit = 1 -> cảm biến đang HIGH -> có cảnh báo
    ///     bit = 0 -> cảm biến LOW       -> không cảnh báo
    pub fn parse_tank_alert(
        &mut self,
    ) -> Result<TankAlert, Error<<I2cDriver<'d> as embedded_hal::i2c::ErrorType>::Error>> {
        let raw_byte = self.read_all_input()?;

        Ok(TankAlert {
            tank_a_low: (raw_byte & ExpanderPin::TankA.mask()) != 0,
            tank_b_low: (raw_byte & ExpanderPin::TankB.mask()) != 0,
            tank_ph_down_low: (raw_byte & ExpanderPin::TankPHDown.mask()) != 0,
            tank_ph_up_low: (raw_byte & ExpanderPin::TankPHUp.mask()) != 0,
        })
    }

    // Debug
    /// Trả về shadow state hiện tại của PCF8574.
    #[allow(dead_code)]
    pub fn state(&self) -> u8 {
        self.state
    }
}
