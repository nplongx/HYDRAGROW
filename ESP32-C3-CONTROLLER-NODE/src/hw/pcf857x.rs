use esp_idf_hal::i2c::{I2c, I2cDriver, I2cError};
use pcf857x::{pcf8574::Parts, Error, OutputPin, Pcf8574, PinFlag, SlaveAddr};

use crate::core::adaptive::solver::select_solver;

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
            state: 0x00000000,
        }
    }
}

impl<'d> I2cExpander<'d> {
    pub fn set_high(
        &mut self,
        pin: u8, //mask
    ) -> Result<(), Error<<I2cDriver<'d> as embedded_hal::i2c::ErrorType>::Error>> {
        self.state |= pin;
        self.pcf.set(self.state)
    }

    pub fn set_low(
        &mut self,
        pin: u8, //mask
    ) -> Result<(), Error<<I2cDriver<'d> as embedded_hal::i2c::ErrorType>::Error>> {
        self.state &= !pin;
        self.pcf.set(self.state)
    }

    pub fn is_high(
        &mut self,
        pin: ExpanderPin,
    ) -> Result<bool, Error<<I2cDriver<'d> as embedded_hal::i2c::ErrorType>::Error>> {
        let value = self.pcf.get(pin.flag())?;
        Ok(value != 0)
    }
}
