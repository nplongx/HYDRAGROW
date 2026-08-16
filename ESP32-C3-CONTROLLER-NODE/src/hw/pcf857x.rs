// src/hw/pcf857x.rs
use esp_idf_hal::i2c::I2cDriver;
use pcf857x::{Error, Pcf8574, PinFlag, SlaveAddr};
use serde::{Deserialize, Serialize};

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

#[repr(u8)]
pub enum ExpanderPin {
    TankA = 0,
    TankB = 1,
    TankPHDown = 2,
    TankPHUp = 3,
    ValveMist = 4,
    ValveMix = 5,
}

impl ExpanderPin {
    pub fn mask(self) -> u8 {
        1 << (self as u8)
    }

    pub fn flag(&self) -> PinFlag {
        match self {
            Self::TankA => PinFlag::P0,
            Self::TankB => PinFlag::P1,
            Self::TankPHDown => PinFlag::P2,
            Self::TankPHUp => PinFlag::P3,
            Self::ValveMist => PinFlag::P4,
            Self::ValveMix => PinFlag::P5,
        }
    }
}

pub struct I2cExpander<'d> {
    pcf: Pcf8574<I2cDriver<'d>>,
    state: u8,
}

impl<'d> I2cExpander<'d> {
    pub fn new(i2c_driver: I2cDriver<'d>) -> Self {
        Self {
            pcf: pcf857x::Pcf8574::new(i2c_driver, SlaveAddr::Default),
            state: 0x00,
        }
    }

    pub fn set_high(
        &mut self,
        pin: u8,
    ) -> Result<(), Error<<I2cDriver<'d> as embedded_hal::i2c::ErrorType>::Error>> {
        self.state |= pin;
        self.pcf.set(self.state)
    }

    pub fn set_low(
        &mut self,
        pin: u8,
    ) -> Result<(), Error<<I2cDriver<'d> as embedded_hal::i2c::ErrorType>::Error>> {
        self.state &= !pin;
        self.pcf.set(self.state)
    }

    pub fn read_all_input(
        &mut self,
    ) -> Result<u8, Error<<I2cDriver<'d> as embedded_hal::i2c::ErrorType>::Error>> {
        let mut buffer = [0u8; 1];
        // 4 chân Input: P0 (A), P1 (B), P2 (pH Down), P3 (pH Up)
        let input_mask = PinFlag::P0 | PinFlag::P1 | PinFlag::P2 | PinFlag::P3;
        self.pcf.read_array(input_mask, &mut buffer)?;
        Ok(buffer[0])
    }

    pub fn parse_tank_alert(
        &mut self,
    ) -> Result<TankAlert, Error<<I2cDriver<'d> as embedded_hal::i2c::ErrorType>::Error>> {
        let raw_byte = self.read_all_input()?;
        // Active-HIGH: Chân ở mức HIGH (bit = 1) là có cảnh báo
        Ok(TankAlert {
            tank_a_low: (raw_byte & ExpanderPin::TankA.mask()) != 0,
            tank_b_low: (raw_byte & ExpanderPin::TankB.mask()) != 0,
            tank_ph_down_low: (raw_byte & ExpanderPin::TankPHDown.mask()) != 0,
            tank_ph_up_low: (raw_byte & ExpanderPin::TankPHUp.mask()) != 0,
        })
    }
}
