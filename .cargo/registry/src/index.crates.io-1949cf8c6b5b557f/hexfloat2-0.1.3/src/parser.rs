use std::iter::Peekable;
use std::str::{Chars, FromStr};

use crate::float::FloatBits;
use crate::HexFloat;

#[derive(Debug)]
pub struct ParseError;

impl<F> FromStr for HexFloat<F>
where
    F: FloatBits,
{
    type Err = ParseError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "inf" => Ok(Self(F::INFINITY)),
            "-inf" => Ok(Self(F::NEG_INFINITY)),
            "NaN" => Ok(Self(F::NAN)),
            _ => Ok(HexFloat(parse_hex(s)?)),
        }
    }
}

fn parse_hex<F>(s: &str) -> Result<F, ParseError>
where
    F: FloatBits,
{
    let mut sign = false;
    let mut chars = s.chars().peekable();

    if let Some(sign_char) = chars.next_if(|c| matches!(c, '-' | '+')) {
        if sign_char == '-' {
            sign = true;
        }
    }
    // The leading `"0x"` is required.
    if !matches!((chars.next(), chars.next()), (Some('0'), Some('x' | 'X'))) {
        return Err(ParseError);
    }
    // This will consume all the hex digits.
    let hexpoint = take_hex(&mut chars)?;

    let mut exponent = match chars.next() {
        None => {
            // Exponent is missing; assume zero.
            0
        }
        Some('p' | 'P') => {
            // Parse the exponent value
            take_decimal(&mut chars)?
        }
        _ => {
            return Err(ParseError);
        }
    };

    // We should have consumed all the characters by now.
    if chars.next().is_some() {
        return Err(ParseError);
    }

    // We successfully parsed everything; if the mantissa is 0,
    // then return an exponent of 0.
    if hexpoint.value == 0 {
        return Ok(F::zero(sign));
    }

    // FIXME: this comment is wrong, and I don't know why.
    // The comment implies exponent += (point_lo - 1) * 4
    // but the working code is point_loc * 4 + 1.
    //
    // If point_loc is 0, then we should subtract 4 from the exponent.
    // .1p0   <- incorrect
    // 1.0p-4 <- correct
    //
    // If point_loc is 1, then the exponent is correct.
    // 1.8 <- correct mantissa format.
    //
    // If point_loc is 2, then we should add 4 to the exponent.
    // 0x10.p0 <- incorrect
    // 0x1.0p4 <- correct

    // This adjustment corrects the location of the decimal point.
    exponent += (hexpoint.point_loc as i32) * 4 - 1;

    // Next, we need to adjust the mantissa so that it either:
    // - has just shifted a 1 out of the MSB, or
    // - would result in the most-negative exponent, or
    // - would overflow the most-positive exponent (error)
    let highest_bit = hexpoint.value.ilog2();
    let mut shift_left = u64::BITS - highest_bit;
    // For each bit shifted left, we should subtract 1 from the exponent.
    // EXCEPT! That's not true for the first bit, which we assume to
    // always be the case: 1.0p0 is well-formed.
    let mut exponent_adjust = (shift_left as i32) - 1;

    let too_far = exponent - exponent_adjust + i32::from(F::EXPONENT_BIAS - 1);
    if too_far < 0 {
        // Don't shift so far; try generating a subnormal
        // value instead.
        shift_left -= (-too_far) as u32;
        // We adjust the exponent by 1 more, which should make it -127 (f32) or -1023 (f64)
        exponent_adjust += too_far + 1;
    }

    // Go ahead and execute the adjustments.
    // (Correctly handle shift by the total number of bits)
    let mantissa = if shift_left == 64 {
        0
    } else {
        hexpoint.value << shift_left
    };
    exponent -= exponent_adjust;

    if exponent < -(F::EXPONENT_BIAS as i32) || exponent > F::EXPONENT_BIAS as i32 {
        return Err(ParseError);
    }

    // The mantissa is currently in bits 63..?, goes in bits 22..0,
    // that's a right shift of 41. That's the same as 64 - MANTISSA_BITS.
    // FIXME: don't parse all these bits if we're just going to throw them away.
    let mantissa_shift = 64 - F::MANTISSA_BITS;
    let mantissa: F::MantissaType = (mantissa >> mantissa_shift).try_into().unwrap_or_else(|_| {
        panic!("mantissa shift failed");
    });

    // IEEE754 floating point adds a bias to the exponent. For f32 the bias is 127 (for f64 it's 1023).
    let exponent: F::ExponentType = (exponent + i32::from(F::EXPONENT_BIAS))
        .try_into()
        .map_err(|_| ParseError)?;

    Ok(F::from_parts(sign, exponent, mantissa))
}

/// A hex value with a `.` divider.
#[derive(Debug, PartialEq)]
struct HexPoint {
    // All of the hex digits.
    value: u64,
    // The number of digits to the left of the point character.
    point_loc: u8,
}

/// Consume a run of hex digits, with an optional decimal point.
///
/// If the digits left of the decimal point don't fit into a u64 then
/// an error will be returned. If there are more digits to the right
/// of the decimal point than will fit, the rightmost digits will be
/// silently ignored.
///
/// The output is the u64 containing all hex digits, along with the
/// location of the decimal.
///
fn take_hex(chars: &mut Peekable<Chars>) -> Result<HexPoint, ParseError> {
    let mut leading_zero = false;
    // Number of digits in the output value.
    let mut count = 0u8;
    // Number of digits to the left of the decimal point.
    let mut point_loc = None;
    let mut value = 0u64;

    // Consume all the leading zeros.
    while chars.next_if_eq(&'0').is_some() {
        leading_zero = true;
    }

    loop {
        // Don't consume the 'p' character
        if matches!(chars.peek(), Some('p' | 'P')) {
            // Fail if input starts with 'p'
            if count == 0 && !leading_zero {
                return Err(ParseError);
            }
            break;
        }
        let nibble = match chars.next() {
            Some('.') => {
                if point_loc.replace(count).is_some() {
                    // Multiple `.` characters found
                    return Err(ParseError);
                }
                continue;
            }
            Some('_') => continue,
            Some(d @ '0'..='9') => (d as u8) - b'0',
            Some(d @ 'a'..='f') => (d as u8) - b'a' + 10,
            Some(d @ 'A'..='F') => (d as u8) - b'A' + 10,
            None => {
                if count == 0 && !leading_zero {
                    return Err(ParseError);
                }
                break;
            }
            _ => return Err(ParseError),
        };
        // We will only put up to 16 digits into the hex value.
        // If we see too many digits to the left of the `.`, we
        // will return an error. If we see extra digits to the
        // right of the `.`, we will ignore them.
        if count >= 16 {
            if point_loc.is_none() {
                return Err(ParseError);
            }
            continue;
        }
        value <<= 4;
        value += nibble as u64;
        count += 1;
    }

    // If we never saw the `.`, it's implicitly all the way
    // to the right.
    let mut point_loc = point_loc.unwrap_or(count);

    // Adjust point_loc to compensate for empty space on the left
    // I.e. "0x10.0" is effectively 0x0000_0000_0000_010.0 (13 implicit leading zeros).
    // The point_loc should be adjusted from 2 to 15.
    point_loc += 16 - count;

    Ok(HexPoint { value, point_loc })
}

/// Parse a decimal value from the input iterator.
///
/// Allows a leading `+` or `-`.
/// Stops at end of string; returns an error at any nondigit character.
/// (because the decimal exponent is always the last part of the hexfloat
/// value.)
fn take_decimal(chars: &mut Peekable<Chars>) -> Result<i32, ParseError> {
    let mut negative = false;
    let mut value = 0i32;
    let mut count = 0;

    match chars.peek() {
        Some(&'-') => {
            negative = true;
            chars.next();
        }
        Some(&'+') => {
            chars.next();
        }
        _ => {}
    }

    loop {
        match chars.peek().copied() {
            Some(d) if d.is_ascii_digit() => {
                chars.next();
                // If we overflow while computing this integer,
                // there's no way the result will fit in an f64.
                value = value.checked_mul(10).ok_or(ParseError)?;
                value = value
                    .checked_add(((d as u8) - b'0') as i32)
                    .ok_or(ParseError)?;
                count += 1;
            }
            None => {
                if count == 0 {
                    return Err(ParseError);
                }
                break;
            }
            _ => return Err(ParseError),
        }
    }

    if negative {
        value = -value
    };
    Ok(value)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[track_caller]
    fn check_parse_f32(s: &str, val: f32) {
        let result = s.parse::<HexFloat<f32>>().unwrap();
        assert_eq!(result.0, val);
        let s_pos = format!("+{s}");
        let result = s_pos.parse::<HexFloat<f32>>().unwrap();
        assert_eq!(result.0, val);

        let s_neg = format!("-{s}");
        let result = s_neg.parse::<HexFloat<f32>>().unwrap();
        assert_eq!(result.0, -val);
    }

    #[track_caller]
    fn check_parse_f64(s: &str, val: f64) {
        let result = s.parse::<HexFloat<f64>>().unwrap();
        assert_eq!(result.0, val);
        let s_pos = format!("+{s}");
        let result = s_pos.parse::<HexFloat<f64>>().unwrap();
        assert_eq!(result.0, val);

        let s_neg = format!("-{s}");
        let result = s_neg.parse::<HexFloat<f64>>().unwrap();
        assert_eq!(result.0, -val);
    }

    #[track_caller]
    fn check_parse(s: &str, val: f64) {
        check_parse_f32(s, val as f32);
        check_parse_f64(s, val);
    }

    #[test]
    fn test_parse() {
        check_parse("0x1", 1.0);
        check_parse("0x1.8", 1.5);
        check_parse("0x1.0", 1.0);
        check_parse("0x8.8", 8.5);
        check_parse("0x.8", 0.5);
        check_parse("0x000.08", 0.03125);
        check_parse("0x100.08", 256.03125);

        check_parse("0x1p0", 1.0);
        check_parse("0x1p+0", 1.0);
        check_parse("0x1p-0", 1.0);
        check_parse("0x2p0", 2.0);
        check_parse("0x0.8p0", 0.5);
        check_parse("0x.8p0", 0.5);

        check_parse("0x1.0p0", 1.0);
        check_parse("0x2.0p0", 2.0);
        check_parse("0x4.0p0", 4.0);
        check_parse("0x8.0p0", 8.0);
        check_parse("0x10.0p0", 16.0);
        check_parse("0x001p0", 1.0);
        check_parse("0x1.8p0", 1.5);
        check_parse("0x1.0p-2", 0.25);
        check_parse("0x1.0p2", 4.0);
    }

    #[test]
    fn test_subnormal_f32() {
        // These values were computed by hand, so hopefully they're correct.
        let smallest_normal_f32 = 0.000000000000000000000000000000000000011754943508222875079687365372222456778186655567720875215087517062784172594547271728515625f32;
        let smallest_subnormal_f32 = 0.00000000000000000000000000000000000000000000140129846432481707092372958328991613128026194187651577175706828388979108268586060148663818836212158203125f32;
        assert_eq!(smallest_subnormal_f32.to_bits(), 0x0000_0001);

        check_parse_f32("0x1.0p-126", smallest_normal_f32); // smallest normal f32

        check_parse_f32("0x1.0p-127", smallest_normal_f32 / 2.0); // biggest power of 2 subnormal
        check_parse_f32("0x0.8p-126", smallest_normal_f32 / 2.0);

        check_parse_f32("0x1.0p-128", smallest_normal_f32 / 4.0);
        check_parse_f32("0x0.8p-127", smallest_normal_f32 / 4.0);
        check_parse_f32("0x0.4p-126", smallest_normal_f32 / 4.0);

        check_parse_f32("0x1.0p-129", smallest_normal_f32 / 8.0);
        check_parse_f32("0x0.8p-128", smallest_normal_f32 / 8.0);
        check_parse_f32("0x0.4p-127", smallest_normal_f32 / 8.0);
        check_parse_f32("0x0.2p-126", smallest_normal_f32 / 8.0);
        check_parse_f32("0x0.1p-125", smallest_normal_f32 / 8.0);
        check_parse_f32("0x0.08p-124", smallest_normal_f32 / 8.0);
        check_parse_f32("0x0.008p-124", smallest_normal_f32 / 128.0);
        check_parse_f32("0x0.0008p-124", smallest_normal_f32 / 2048.0);
        check_parse_f32("0x0.00008p-124", smallest_normal_f32 / 32768.0);
        check_parse_f32("0x0.000008p-124", smallest_normal_f32 / 524288.0);
        check_parse_f32("0x0.000004p-124", smallest_normal_f32 / 1048576.0);
        check_parse_f32("0x0.000002p-124", smallest_normal_f32 / 2097152.0);
        check_parse_f32("0x0.000002p-125", smallest_normal_f32 / 4194304.0);

        check_parse_f32("0x0.000001p-125", smallest_subnormal_f32);
        check_parse_f32("0x0.000002p-126", smallest_subnormal_f32);
        check_parse_f32("0x0.000004p-127", smallest_subnormal_f32);

        check_parse_f32("0x0.000001p-126", 0.0); // loss of precision, rounds to 0
        check_parse_f32("0x0.000002p-127", 0.0);
        check_parse_f32("0x0.000004p-128", 0.0);

        check_parse_f32("0x1.000001p0", 1.0); // loss of precision, rounds to 1
    }

    #[test]
    fn test_subnormal_f64() {
        check_parse_f64("0x0.00000000000008p-1021", 5e-324); // smallest subnormal f64
        check_parse_f64("0x0.00000000000004p-1021", 0.0); // loss of precision, rounds to 0
        check_parse_f64("0x1.00000000000004p0", 1.0); // loss of precision, rounds to 1
    }

    #[track_caller]
    fn check_hex(s: &str, value: u64, point_loc: u8) {
        assert_eq!(
            take_hex(&mut s.chars().peekable()).unwrap(),
            HexPoint { value, point_loc }
        );
    }

    #[track_caller]
    fn bad_hex(s: &str) {
        take_hex(&mut s.chars().peekable()).unwrap_err();
    }

    #[test]
    fn test_hex() {
        check_hex("0", 0, 16);
        check_hex("a", 0xa, 16);
        check_hex("A", 0xa, 16);
        check_hex("f", 0xf, 16);
        check_hex("F", 0xF, 16);
        check_hex("012569abdefp", 0x12569abdef, 16);
        check_hex("ffffffffffffffffp", 0xffff_ffff_ffff_ffff, 16);
        check_hex("1.000_0000_0000_00001", 0x1000_0000_0000_0000, 1);
        check_hex("1000_0000_0000_0000.1", 0x1000_0000_0000_0000, 16);
        // Ignore a lot of leading zeroes.
        check_hex("00000000000000000001p", 1, 16);

        bad_hex("");
        bad_hex("p");
        bad_hex("1.0.0");
        // Too many digits
        bad_hex("1000_0000_0000_00001");
        bad_hex("fffffffffffffffffp");
    }

    #[track_caller]
    fn check_decimal(s: &str, val: i32) {
        assert_eq!(take_decimal(&mut s.chars().peekable()).unwrap(), val);
    }

    #[track_caller]
    fn bad_decimal(s: &str) {
        take_decimal(&mut s.chars().peekable()).unwrap_err();
    }

    #[test]
    fn test_decimal() {
        check_decimal("1", 1);
        check_decimal("+1", 1);
        check_decimal("-1", -1);
        check_decimal("100", 100);
        check_decimal("9999", 9999);
        check_decimal("-2147483647", -2_147_483_647);
        check_decimal("2147483647", 2_147_483_647);

        bad_decimal("");
        bad_decimal("a");
        bad_decimal("p");
        bad_decimal("-");
        bad_decimal("+");
        bad_decimal("++1");
        bad_decimal("--1");
        bad_decimal("+-1");
    }
}
