use std::borrow::Cow;

use super::cbordiagnostic;

#[derive(Debug, Clone, PartialEq)]
/// A string with the invariant (under penalty of panics) is that it matches the `number` ABNF rule
/// of EDN.
pub(super) struct Number<'a>(pub(super) Cow<'a, str>);

impl<'a> Number<'a> {
    pub(super) fn new_float(value: f64) -> Self {
        if value.is_nan() {
            return Number(Cow::from("NaN"));
        }
        if value.is_infinite() {
            if value > 0.0 {
                return Number(Cow::from("Infinity"));
            } else {
                return Number(Cow::from("-Infinity"));
            }
        }
        let mut formatted = format!("{}", value);
        if !formatted.contains('.') && !formatted.contains('e') {
            formatted.push_str(".0")
        }
        Number(Cow::Owned(formatted))
    }
    pub(super) fn value(&self) -> NumberValue {
        if let Ok(numval) = cbordiagnostic::hexfloat(&self.0)
            .or_else(|_| cbordiagnostic::hexint(&self.0))
            .or_else(|_| cbordiagnostic::octint(&self.0))
            .or_else(|_| cbordiagnostic::binint(&self.0))
            .or_else(|_| cbordiagnostic::decnumber(&self.0))
        {
            match numval {
                NumberParts {
                    base,
                    sign,
                    predot,
                    postdot: None,
                    exponent: None,
                    ..
                } => {
                    // Parsing into a u128 is the easiest way to ensure that the minimum value of a
                    // negative 64bit integer is also parsed correctly
                    u128::from_str_radix(predot, base.into())
                        .ok()
                        .and_then(|value| match sign {
                            Some(Sign::Plus) | None => {
                                value.try_into().ok().map(NumberValue::Positive)
                            }
                            Some(Sign::Minus) => match value.checked_sub(1) {
                                None => Some(NumberValue::Positive(0)),
                                Some(offset) => offset.try_into().ok().map(NumberValue::Negative),
                            },
                        })
                        .unwrap_or_else(|| {
                            NumberValue::Big(
                                num_bigint::BigInt::parse_bytes(self.0.as_bytes(), base.into())
                                    .expect("Parser verified success"),
                            )
                        })
                }
                NumberParts { base: 10, .. } => {
                    // An f64 is the highest precision we can get. As parsing a float is
                    // difficult especially when base mismatches mean we could get caught in
                    // precision loss crossfire, let's just do the simple thing and rely on the
                    // built-in parser.

                    NumberValue::Float(
                        self.0
                            .parse()
                            .expect("Parser and construction guarantee this to succeed"),
                    )
                }
                NumberParts { base: 16, .. } => NumberValue::Float(
                    hexfloat2::parse(&self.0)
                        .expect("Parser and construction guarantee this to succeed"),
                ),
                _ => unreachable!("Syntax does not produce floats outside base 10/16"),
            }
        } else {
            match self.0.as_ref() {
                "Infinity" => NumberValue::Float(f64::INFINITY),
                "-Infinity" => NumberValue::Float(f64::NEG_INFINITY),
                // FIXME: Should we pick any particular one?
                "NaN" => NumberValue::Float(f64::NAN),
                _ => unreachable!(
                    "Number's invariant is that it matches the ABNF, found {:?}",
                    self.0.as_ref()
                ),
            }
        }
    }

    pub(super) fn with_spec(self, spec: Option<super::Spec>) -> super::Item<'a> {
        super::InnerItem::Number(self, spec).into()
    }
}

#[derive(Debug)]
pub(super) enum Sign {
    Plus,
    Minus,
}

#[derive(Debug)]
pub(super) struct NumberParts<'a> {
    pub(super) base: u8,
    pub(super) sign: Option<Sign>,
    pub(super) predot: &'a str,
    pub(super) postdot: Option<&'a str>,
    pub(super) exponent: Option<(Option<Sign>, &'a str)>,
}

/// Expressible numeric values (without resorting to bignums, of which it is unclear whether they
/// are even represented by EDN `number` expressions)
pub(super) enum NumberValue {
    Positive(u64),
    /// Note that Negative(n) has the numeric value -1-n
    Negative(u64),
    Float(f64),
    Big(num_bigint::BigInt),
}

/// Visitor for [`visit_tag`](super::StandaloneItem::visit_tag) that converts tag 2/3 bignums into plain EDN.
///
/// As parsing CBOR does not do this by default, this is provided as an extra step (currently in
/// the [crate::application] module – it is not an application-oriented literal, but fits the
/// style, even though it is a bit special in that it does use crate internals to access encoding
/// indicators).
///
/// If a value has encoding indicators, it is left unmodified, as encoding indicators can not be
/// expressed in EDN for bignums.
pub fn tag23_to_edn_integer(tag: u64, item: &mut super::Item) -> Result<(), String> {
    use num_bigint::{BigInt, Sign};

    // FIXME: If any reason is present to not encode it but it's a tag 2/3 and has bytes we can
    // get, maybe we should place the number in a comment.

    let crate::InnerItem::Tagged(2 | 3, None, ref tagged) = item.inner() else {
        // not a bignum or encoding indicator present
        return Ok(());
    };
    let tagged = tagged.item();

    let crate::InnerItem::String(crate::CborString { ref items, .. }) = tagged.inner() else {
        // ruling out indefinite length strings whose encoding indicators we can't represent
        return Ok(());
    };
    if items
        .iter()
        .any(|e| !matches!(e, crate::String1e::TextChunk(_, None)))
    {
        // Encoding indicators present, or embeddec chunk, which gut feeling says we should
        // preserve as well
        return Ok(());
    }
    let Ok(bytes) = tagged.get_bytes() else {
        // Might have still been some weird application oriented literal
        return Ok(());
    };

    if bytes.first() == Some(&0) {
        // Not preferred representation
        return Ok(());
    }

    let value = BigInt::from_bytes_be(if tag == 2 { Sign::Plus } else { Sign::Minus }, &bytes);

    if (-BigInt::from(u64::MAX)..=BigInt::from(u64::MAX)).contains(&value) {
        // Not preferred representation
        return Ok(());
    }

    *item = crate::InnerItem::Number(Number(format!("{}", value).into()), None).into();
    Ok(())
}
