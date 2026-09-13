//! Internal helpers for float conversion
//!
//! Generally, three kinds of operations are covered:
//! * Change size of a float. These operations are implemented manually rather than going through
//!   Rust's numeric conversions in order to preserve NaN payloads as per CBOR's semantics.
//!   * Change it up ([f16_bits_to_f64], [f32_to_f64]): This is lossless.
//!   * Change it down ([f64_to_f16_bits], [f64_to_f32]): This loses data, but we only us it in
//!     practice if conversion back up yields the same result. (If we wanted to use it to shorten
//!     data, we'd have to implement proper rounding).
//! * Encode a float according to a spec. This tries the down/up-conversion where it refuses to
//!   lose data, and prepends the appropriate CBOR byte.
//!
//! A lot is done manually rather than trusting Rust's conversions, because CBOR wants payloads
//! preserved. Given that Rust's conversion preserves payloads with the exception of the signaling
//! bit, it may be worth revisiting this and turning this module into a largely tests-only module,
//! if we just special-case the conversion with "if is_nan ... pick out the signaling bit and set
//! it right again after the conversion".

use super::{InconsistentEdn, Spec};

/// Produce a number where the n lowest bits are set
fn low_bits(n: u8) -> u64 {
    if n == 64 {
        u64::MAX
    } else {
        !(u64::MAX << n)
    }
}

#[test]
fn test_low_bits() {
    assert_eq!(low_bits(0), 0);
    assert_eq!(low_bits(1), 1);
    assert_eq!(low_bits(12), 0xfff);
    assert_eq!(low_bits(15), 0x7fff);
    assert_eq!(low_bits(15), 0x7fff);
    assert_eq!(low_bits(64), 0xffffffffffffffff);
}

pub(crate) fn encode(
    f: f64,
    spec: Option<Spec>,
) -> Result<impl Iterator<Item = u8>, InconsistentEdn> {
    // We go through attempted conversion and back-conversion because that's way easier than
    // analytically determining whether the conversion might still be exact if we tried
    // denormalized numbers. We don't use Rust's casting because that doesn't treat NaN payloads
    // right (and we don't have an f16 type in Rust anyway).

    let f16_reconstructed = f64_to_f16_bits(f);
    let equivalent_f64 = f16_bits_to_f64(f16_reconstructed).to_bits();
    // Comparing bits to avoid NaN trouble -- we want exact conversion even there
    let precise_f16 = if equivalent_f64 == f.to_bits() {
        Some(f16_reconstructed)
    } else {
        None
    };

    let f32_reconstructed = f64_to_f32(f);
    let equivalent_f64 = f32_to_f64(f32_reconstructed).to_bits();
    // Comparing bits to avoid NaN trouble -- we want exact conversion even there
    let precise_f32 = if equivalent_f64 == f.to_bits() {
        Some(f32_reconstructed)
    } else {
        None
    };

    match (spec, precise_f32, precise_f16) {
        (Some(Spec::S_1) | None, _, Some(precise_f16)) => {
            let head_byte = 0xf9;
            let encoded = u64::from(precise_f16).to_be_bytes();
            Ok(core::iter::once(head_byte).chain(encoded.into_iter().skip(6)))
        }
        // All later `| None` rely on earlier items having caught on if the number can be expressed
        // more compactly
        (Some(Spec::S_2) | None, Some(precise_f32), _) => {
            let head_byte = 0xfa;
            let encoded = u64::from(precise_f32.to_bits()).to_be_bytes();
            Ok(core::iter::once(head_byte).chain(encoded.into_iter().skip(4)))
        }
        (Some(Spec::S_3) | None, _, _) => {
            let head_byte = 0xfb;
            let encoded = f.to_be_bytes();
            #[allow(clippy::iter_skip_zero)]
            // reason: false positive, see https://github.com/rust-lang/rust-clippy/issues/11761
            Ok(core::iter::once(head_byte).chain(encoded.into_iter().skip(0)))
        }
        (Some(Spec::S_1), _, None) | (Some(Spec::S_2), None, _) => {
            // FIXME: Allow it still? Then we should have proper rounding.
            Err(InconsistentEdn(
                "Float can not be encoded with that spec losslessly",
            ))
        }
        (Some(Spec::S_) | Some(Spec::S_i) | Some(Spec::S_0), _, _) => Err(InconsistentEdn(
            "Encoding indicators _, _i and _0 do not apply to floats",
        )),
    }
}

#[test]
fn test_encode() {
    // reference values from cbor.me
    assert_eq!(
        encode(1.0, None).unwrap().collect::<Vec<_>>(),
        vec![0xf9, 0x3c, 0x00]
    );
    assert_eq!(
        encode(100.0, None).unwrap().collect::<Vec<_>>(),
        vec![0xf9, 0x56, 0x40]
    );
    assert_eq!(
        encode(f64::INFINITY, None).unwrap().collect::<Vec<_>>(),
        vec![0xf9, 0x7c, 0x00]
    );
    assert_eq!(
        encode(f64::NEG_INFINITY, None).unwrap().collect::<Vec<_>>(),
        vec![0xf9, 0xfc, 0x00]
    );

    let mut max_encoded = vec![0xfb];
    max_encoded.extend(f64::MAX.to_be_bytes());
    assert_eq!(
        encode(f64::MAX, None).unwrap().collect::<Vec<_>>(),
        max_encoded
    );

    let mut max32_encoded = vec![0xfa];
    max32_encoded.extend(f32::MAX.to_be_bytes());
    assert_eq!(
        encode(f32::MAX.into(), None).unwrap().collect::<Vec<_>>(),
        max32_encoded
    );

    // constructed manually and verified with cbor.me
    assert_eq!(
        encode(1.0, Some(Spec::S_3)).unwrap().collect::<Vec<_>>(),
        vec![0xfb, 0x3f, 0xf0, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00]
    );

    let mut min32_encoded = vec![0xfa];
    min32_encoded.extend(f32::MIN_POSITIVE.to_be_bytes());
    // This roundtrips it from denormalized numbers to normalized and manually back
    assert_eq!(
        encode(f32::MIN_POSITIVE.into(), None)
            .unwrap()
            .collect::<Vec<_>>(),
        min32_encoded
    );
}

/// Type of a float
// Generally, we use u8 to index into numbers' bits, u64 for fields' values, and i32 for the
// numeric (unbiased) form of the exponent
#[derive(Copy, Clone)]
struct FloatCharacterization {
    bit_length: u8,
    mantissa_length: u8,
}

impl FloatCharacterization {
    const fn sign_shift(&self) -> u8 {
        self.bit_length - 1
    }

    const fn exponent_length(&self) -> u8 {
        self.bit_length - 1 - self.mantissa_length
    }

    const fn exponent_bias(&self) -> u64 {
        (1 << self.exponent_length()) / 2 - 1
    }

    /// Numeric value of the exponent used to indicate zero or denormalized numbers; this is also
    /// the negative bias.
    const fn denormal_numeric_exponent(&self) -> i32 {
        // Using as-casts rather than try_into.expect because try_into does not work in const yet
        assert!(
            self.exponent_bias() < i32::MAX as u64,
            "All floats' bias fits in an i32"
        );
        let bias = self.exponent_bias() as i32;
        -bias
    }

    /// Numeric value of the exponent used to indicate infinite and NaN values.
    const fn infnan_numeric_exponent(&self) -> i32 {
        // Using as-casts rather than try_into.expect because try_into does not work in const yet
        assert!(
            self.exponent_bias() < i32::MAX as u64,
            "All floats' bias fits in an i32"
        );
        let bias = self.exponent_bias() as i32;
        bias + 1
    }

    /// Quick and easy internal debug function
    #[cfg(test)]
    fn debug_value(&self, value: u64) -> impl core::fmt::Display + 'static {
        struct FormattedFloat(FloatCharacterization, u64);

        impl core::fmt::Display for FormattedFloat {
            fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
                let FormattedFloat(fc, value) = *self;
                assert!(value & low_bits(fc.bit_length) == value, "Extra bits");
                let exponent = (value & low_bits(fc.bit_length - 1)) >> fc.mantissa_length;
                let numeric_exponent = i32::try_from(exponent).expect("constructed from bit mask")
                    + fc.denormal_numeric_exponent();
                let mantissa = value & low_bits(fc.mantissa_length);
                write!(
                    f,
                    "Sign {}, exponent 0x{:x} (",
                    value >> (fc.bit_length - 1),
                    exponent
                )?;
                match exponent {
                    e if e == low_bits(fc.exponent_length()) => match mantissa {
                        0 => return write!(f, "infinity)"),
                        _ => write!(f, "nan), nan payload")?,
                    },
                    0 => write!(f, "denormalized), no implicit bit, mantissa ")?,
                    _ => write!(
                        f,
                        "numeric 2^{}), implicit bit 1, mantissa ",
                        numeric_exponent
                    )?,
                }
                write!(
                    f,
                    " 0x{:x} (mask 0x{:x})",
                    mantissa,
                    low_bits(fc.mantissa_length)
                )?;
                Ok(())
            }
        }

        FormattedFloat(*self, value)
    }
}

const F16: FloatCharacterization = FloatCharacterization {
    bit_length: 16,
    mantissa_length: 10,
};

const F32: FloatCharacterization = FloatCharacterization {
    bit_length: 32,
    mantissa_length: 23,
};

#[cfg(test)]
const F64: FloatCharacterization = FloatCharacterization {
    bit_length: 64,
    mantissa_length: 52,
};

/// Extend an f16 (represented as u16 for lack of f16 in Rust) into an f64.
///
/// This preserves all values as well as (signalling and quiet) NaN payloads
pub(crate) fn f16_bits_to_f64(input: u16) -> f64 {
    short_float_bits_to_f64(input.into(), F16)
}

/// Extend an f32 into an f64.
///
/// This preserves all values as well as (signalling and quiet) NaN payloads
pub(crate) fn f32_to_f64(input: f32) -> f64 {
    short_float_bits_to_f64(input.to_bits().into(), F32)
}

/// Generalization of f16_bits_to_f64 and f32_to_f64.
///
/// The original value is passed in the lowest bits of input.
fn short_float_bits_to_f64(input: u64, floatchar: FloatCharacterization) -> f64 {
    let sign = input >> floatchar.sign_shift();
    let exponent = (input >> floatchar.mantissa_length) & low_bits(floatchar.exponent_length());
    let fraction = input & low_bits(floatchar.mantissa_length);
    if exponent == 0 {
        if fraction == 0 {
            f64::from_bits(sign << 63)
        } else {
            // denormalized -- we shift it into normal range and pick a suitable exponent
            let leading_zeros = fraction.leading_zeros();
            let shift = i64::from(leading_zeros) - (64 - 52) + 1;
            let fraction = fraction << shift;
            debug_assert_eq!(
                fraction & !low_bits(52),
                1 << 52,
                "Shift is supposed shift mantissa into normalized range (and it was checked to be non-zero)"
            );
            let numeric_exponent = floatchar.denormal_numeric_exponent()
                - i32::try_from(shift).expect("Fits by construction")
                + (52 - i32::from(floatchar.mantissa_length))
                + 1;
            let new_exponent =
                u64::try_from(numeric_exponent + 1023).expect("Value is positive by construction");
            let recomposed = (sign << 63) | (new_exponent << 52) | (fraction & low_bits(52));
            f64::from_bits(recomposed)
        }
    } else {
        let numeric_exponent = i32::try_from(exponent).expect("constructed from bit mask")
            + floatchar.denormal_numeric_exponent();
        let new_exponent = if numeric_exponent == floatchar.infnan_numeric_exponent() {
            2047
        } else {
            // input is truly betwe the denormalized and infnan exponent, but we checked that by
            // construction already.
            numeric_exponent + 1023
        };
        let new_exponent: u64 = new_exponent
            .try_into()
            .expect("Value is positive by construction");

        let recomposed =
            (sign << 63) | (new_exponent << 52) | (fraction << (52 - floatchar.mantissa_length));

        f64::from_bits(recomposed)
    }
}

/// Shrink an 64 into a smaller float value as per the characterization of the target.
///
/// The value is returned in the lowest bits of the result.
// FIXME: Convert it also to the correctly-rounded one
fn f64_to_smaller(f: f64, floatchar: FloatCharacterization) -> u64 {
    let as_int = f.to_bits();

    let sign = as_int >> 63;
    let exponent = (as_int >> 52) & low_bits(11);
    let is_inf_nan = exponent == low_bits(11);
    // Bias correction
    let numeric_exponent = i32::try_from(exponent).expect("constructed from bit mask") - 1023;
    let fraction = as_int & low_bits(52);

    // Add the implicit bit right away
    // FIXME: If we added rounding, we'd have to know whether rounding applies or whether we're
    // truncating a NaN payload (or are NaN payloads rounded too?)
    let mut fraction = (fraction | (1 << 52)) >> (52 - floatchar.mantissa_length);
    let exponent;
    if is_inf_nan {
        // Keep NaN as is, bits are already truncated
        exponent = low_bits(floatchar.exponent_length());
    } else if numeric_exponent > floatchar.infnan_numeric_exponent() {
        // Overshoot, going to Inf
        exponent = low_bits(floatchar.exponent_length());
        fraction = 0;
    } else if numeric_exponent <= floatchar.denormal_numeric_exponent() {
        // can't use -15 (that's our exponent 0), let alone anything below it (inexpressible)
        let undershoot = floatchar.denormal_numeric_exponent() - numeric_exponent + 1;
        if undershoot < 64 {
            // Might become 0 as we're even shifting out the implicit bit
            fraction >>= undershoot;
        } else {
            // Undershoot, going to 0
            fraction = 0;
        }
        exponent = 0;
    } else {
        exponent = u64::try_from(numeric_exponent - floatchar.denormal_numeric_exponent())
            .expect("Positive by construction");
    }
    (sign << floatchar.sign_shift())
        | (exponent << floatchar.mantissa_length)
        // Might have the implicit bit still set
        | (fraction & low_bits(floatchar.mantissa_length))
}

/// Convert an f64 to some f16 that has the same value if that is possible
fn f64_to_f16_bits(f: f64) -> u16 {
    f64_to_smaller(f, F16)
        .try_into()
        .expect("Fits by construction")
}

/// Convert an f64 to some f32 that has the same value if that is possible
fn f64_to_f32(f: f64) -> f32 {
    f32::from_bits(
        f64_to_smaller(f, F32)
            .try_into()
            .expect("Fits by construction"),
    )
}

/// For a given bit length, generate numeric values ranging from 0 to 0b111..11, exercising corner
/// cases.
#[cfg(test)]
fn generate_ux_test_patterns(n: u8) -> impl Iterator<Item = u64> + Clone {
    let highbits = if n >= 4 { 1..4 } else { 0..0 };

    if n < 4 {
        // Excluding all-bits: that's chained to the end anyway
        0..low_bits(n)
    } else {
        0..3
    }
    .chain(
        highbits
            .clone()
            .map(move |highbits| highbits << n.saturating_sub(2)),
    )
    .chain(
        highbits
            .clone()
            .map(move |highbits| (highbits << n.saturating_sub(2)) | 1),
    )
    .chain(core::iter::once(low_bits(n)))
}

#[test]
fn test_generage_ux_test_patterns() {
    for n in 1..=6 {
        assert!(generate_ux_test_patterns(n).any(|p| p == 0));
        assert!(generate_ux_test_patterns(n).any(|p| p == 1));
        if n > 1 {
            assert!(generate_ux_test_patterns(n).any(|p| p == 2));
        }
        assert!(generate_ux_test_patterns(n).any(|p| p == low_bits(n) - low_bits(n - 1)));
        assert!(generate_ux_test_patterns(n).any(|p| p == low_bits(n)));
        assert!(!generate_ux_test_patterns(n).any(|p| p & !low_bits(n) != 0));
    }
}

#[cfg(test)]
fn generate_float_test_patterns(floatchar: FloatCharacterization) -> impl Iterator<Item = u64> {
    let sign_bits = generate_ux_test_patterns(1);
    let exponent_bits = generate_ux_test_patterns(floatchar.exponent_length());
    let mantissa_bits = generate_ux_test_patterns(floatchar.mantissa_length);

    itertools::iproduct!(sign_bits, exponent_bits, mantissa_bits).map(
        move |(sign_bits, exponent_bits, mantissa_bits)| {
            (sign_bits << floatchar.sign_shift())
                | (exponent_bits << floatchar.mantissa_length)
                | mantissa_bits
        },
    )
}

/// Take many relevant f16, convert them to f64 and assert that encoding them (which involves a
/// downconversion) produces the same value.
///
/// This does not contain test vectors for the values.
#[test]
fn test_many_f16() {
    for i in generate_float_test_patterns(F16) {
        let i = u16::try_from(i).expect("Matches by construction");
        let binary64 = f16_bits_to_f64(i);
        let encoded: Vec<_> = encode(binary64, None).unwrap().collect();
        assert_eq!(
            encoded[1..],
            i.to_be_bytes(),
            "{i:#06x} got interpreted as {:#018x} {binary64} and encoded as {encoded:x?}",
            binary64.to_bits()
        );
    }
}

/// Take many relevant f32, convert them to f64 and assert that encoding them (which involves a
/// downconversion) produces the same value.
///
/// This does not contain test vectors for the values.
#[test]
fn test_many_f32() {
    for i in generate_float_test_patterns(F32) {
        let i = u32::try_from(i).expect("Matches by construction");
        let binary64 = f32_to_f64(f32::from_bits(i));
        let encoded: Vec<_> = encode(binary64, None).unwrap().collect();
        if let [0xf9, _, _] = encoded[..] {
            // Nothing to test here -- that it got converted down shows that it did a round-trip
            // through u64.
        } else {
            assert_eq!(
                encoded[1..],
                i.to_be_bytes(),
                "{i:#06x} got interpreted as {:#018x} {binary64} and encoded as {encoded:x?}",
                binary64.to_bits()
            );
        }
    }
}

/// Take some known f16, convert them to f64 and assert their value is as it should be.
///
/// Reference values were obtained from https://lukaskollmer.de/ieee-754-visualizer/ and checked
/// manually for plausibility.
#[test]
fn test_known_f16() {
    assert_eq!(f16_bits_to_f64(0x0000), 0.0);
    assert_eq!(f16_bits_to_f64(0x8000), -0.0);

    // Smallest denormalized number
    assert_eq!(f16_bits_to_f64(0x0001), 5.960464477539063e-8);
    assert_eq!(f16_bits_to_f64(0x8001), -5.960464477539063e-8);
    // Largest denormalized number
    assert_eq!(f16_bits_to_f64(0x03ff), 0.00006097555160522461);
    // Smallest positive
    assert_eq!(f16_bits_to_f64(0x0400), 0.00006103515625);
    // Next larger
    assert_eq!(f16_bits_to_f64(0x0401), 0.00006109476089477539);
    // Full range of mantissa bits
    assert_eq!(f16_bits_to_f64(0x0601), 0.00009161233901977539);

    // Largest finite
    assert_eq!(f16_bits_to_f64(0x7bff), 65504.0);
    // Unset some bits
    assert_eq!(f16_bits_to_f64(0x7b55), 60064.0);

    assert_eq!(f16_bits_to_f64(0x7c00), f64::INFINITY);

    // The canonical NaN form RFC8949 -- positive, signalling bit is set, all others are zero
    assert!(f16_bits_to_f64(0x7e00).is_nan());
    // Same in negative
    assert!(f16_bits_to_f64(0xfe00).is_nan());
    // A non-signaling NaN with the most significant bit after the signaling bit
    assert!(f16_bits_to_f64(0x7d00).is_nan());
}
///
/// Take some known f32, convert them to f64 and assert their value is as it should be.
///
/// Reference values were obtained from https://lukaskollmer.de/ieee-754-visualizer/ and checked
/// manually for plausibility.
///
/// This is less thorough than the f16 case because it's the same mechanism, and fewer data points
/// suffice.
#[test]
fn test_known_f32() {
    assert_eq!(f32_to_f64(f32::from_bits(0x00000000)), 0.0);
    assert_eq!(f32_to_f64(f32::from_bits(0x80000000)), -0.0);

    // Smallest denormalized number
    assert_eq!(
        f32_to_f64(f32::from_bits(0x00000001)),
        1.401298464324817e-45
    );
    // Smallest positive
    assert_eq!(
        f32_to_f64(f32::from_bits(0x00800000)),
        1.1754943508222875e-38
    );

    assert_eq!(f32_to_f64(f32::from_bits(0x7f800000)), f64::INFINITY);
}

/// This verifies that Rust's casts force the signalling bit. If this test starts failing, it may
/// mean that Rust has learned to do CBOR style conversions.
#[test]
#[cfg_attr(miri, ignore = "Miri sometimes also does not preserve other payloads")]
fn rust_normalizes_nans_like_cbor() {
    // This gets altered in Rust:
    let positive_signalbitclear_with_payload = f32::from_bits(0x7f911111);

    println!(
        "Starting from {}",
        F32.debug_value(positive_signalbitclear_with_payload.to_bits().into())
    );

    let bitclear_through_rust = f64::from(positive_signalbitclear_with_payload).to_bits();
    let bitclear_through_ours = f32_to_f64(positive_signalbitclear_with_payload).to_bits();
    println!(
        "Rust converts to {}",
        F64.debug_value(bitclear_through_rust)
    );
    println!(
        "CBOR converts to {}",
        F64.debug_value(bitclear_through_ours)
    );
    assert!(
        bitclear_through_rust != bitclear_through_ours,
        "We acknowledge that Rust sets the bit here"
    );

    // This gets preserved
    let positive_signalbitset_with_payload = f32::from_bits(0x7fc11111);
    println!(
        "Starting from {}",
        F32.debug_value(positive_signalbitset_with_payload.to_bits().into())
    );
    let bitset_through_rust = f64::from(positive_signalbitset_with_payload).to_bits();
    let bitset_through_ours = f32_to_f64(positive_signalbitset_with_payload).to_bits();
    println!("Rust converts to {}", F64.debug_value(bitset_through_rust));
    println!("CBOR converts to {}", F64.debug_value(bitset_through_ours));
    assert_eq!(
        bitset_through_rust, bitset_through_ours,
        "For these, Rust conversion has so far worked out"
    );
}
