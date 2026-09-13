//! traits that abstract across f32 / f64.

use std::fmt::LowerHex;
use std::num::FpCategory;
use std::ops::Shl;

#[doc(hidden)]
pub trait MantissaOps: Shl<u32, Output = Self> + LowerHex + TryFrom<u64> {}

#[doc(hidden)]
pub trait ExponentOps: Into<i32> + TryFrom<i32> {}

impl MantissaOps for u32 {}
impl ExponentOps for u8 {}
impl MantissaOps for u64 {}
impl ExponentOps for u16 {}

#[doc(hidden)]
pub trait FloatBits {
    const BITS: u32;
    const EXPONENT_BITS: u8;
    const MANTISSA_BITS: u8;
    const EXPONENT_BIAS: u16;
    const INFINITY: Self;
    const NEG_INFINITY: Self;
    const NAN: Self;

    type IntegerBits;
    type ExponentType: ExponentOps;
    type MantissaType: MantissaOps;

    fn zero(sign: bool) -> Self;
    fn from_parts(sign: bool, exponent: Self::ExponentType, mantissa: Self::MantissaType) -> Self;
    fn to_parts(&self) -> (bool, Self::ExponentType, Self::MantissaType);
    fn category(&self) -> FpCategory;
}

impl FloatBits for f32 {
    const BITS: u32 = 32;
    const EXPONENT_BITS: u8 = 8;
    const MANTISSA_BITS: u8 = 23;
    const EXPONENT_BIAS: u16 = 127;
    const INFINITY: Self = f32::INFINITY;
    const NEG_INFINITY: Self = f32::NEG_INFINITY;
    const NAN: Self = f32::NAN;
    type IntegerBits = u32;
    type ExponentType = u8;
    type MantissaType = u32;

    /// Return a zero.
    ///
    /// If sign is `true`, return a negative zero.
    fn zero(sign: bool) -> Self {
        if sign {
            -0.0
        } else {
            0.0
        }
    }

    /// Create the f32 bit layout.
    ///
    /// Inputs should be right-aligned (i.e. LSB is the 0 bit).
    /// The caller must verify that the inputs are within the proper bounds.
    ///
    fn from_parts(sign: bool, exponent: u8, mantissa: u32) -> Self {
        let bits: u32 = ((sign as u32) << 31) + ((exponent as u32) << 23) + mantissa;
        f32::from_bits(bits)
    }

    fn to_parts(&self) -> (bool, u8, u32) {
        let bits = self.to_bits();
        let sign: bool = (bits >> 31) == 1;
        let exponent = (bits >> 23) as u8;
        let mantissa = bits & 0x7F_FFFF;
        (sign, exponent, mantissa)
    }

    fn category(&self) -> FpCategory {
        self.classify()
    }
}

impl FloatBits for f64 {
    const BITS: u32 = 64;
    const EXPONENT_BITS: u8 = 11;
    const MANTISSA_BITS: u8 = 52;
    const EXPONENT_BIAS: u16 = 1023;
    const INFINITY: Self = f64::INFINITY;
    const NEG_INFINITY: Self = f64::NEG_INFINITY;
    const NAN: Self = f64::NAN;
    type IntegerBits = u64;
    type ExponentType = u16;
    type MantissaType = u64;

    /// Return a zero.
    ///
    /// If sign is `true`, return a negative zero.
    fn zero(sign: bool) -> Self {
        if sign {
            -0.0
        } else {
            0.0
        }
    }

    /// Create the f64 bit layout.
    ///
    /// Inputs should be right-aligned (i.e. LSB is the 0 bit).
    /// The caller must verify that the inputs are within the proper bounds.
    ///
    fn from_parts(sign: bool, exponent: u16, mantissa: u64) -> Self {
        let bits: u64 = ((sign as u64) << 63) + ((exponent as u64) << 52) + mantissa;
        f64::from_bits(bits)
    }

    fn to_parts(&self) -> (bool, u16, u64) {
        let bits = self.to_bits();
        let sign: bool = (bits >> 63) == 1;
        let exponent = ((bits >> 52) & 0x7FF) as u16;
        let mantissa = bits & 0x000F_FFFF_FFFF_FFFF;
        (sign, exponent, mantissa)
    }

    fn category(&self) -> FpCategory {
        self.classify()
    }
}
