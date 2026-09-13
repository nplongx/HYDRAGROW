//! # Tools for processing CBOR Diagnostic Notation (EDN)
//!
//! The parser used by this crate is a PEG (Parsing Expression Grammer) parser built from the ABNF
//! used in the [EDN specification].
//!
//! The crate's main types represent not only the parsed items but also all the parts that have no
//! bearing on the translation to CBOR (spaces, commas, comments) and
//! choices that may or may not influence the CBOR (encoding indicators). This allows detailed
//! manipulation (for example inside comments) and a delayed processing of application oriented
//! literals.
//!
//! Parsed values are expected to round-trip to identical representations when serialized. Most
//! manipulations of the values will ensure that their serialization output can also be
//! round-tripped from the internal format to the EDN serialization and back into the internal
//! format, but this can not be provided by all. (For example, removing all optional commas
//! while retaining comments would make the previous distinction between whether a comment was
//! before or after a comma indistinguishable).
//!
//! Correct parsing does not guarantee that the value can also be encoded into CBOR. While there
//! are aspects that could be handled at parsing time and are not (eg. tag numbers exceeding the
//! encodable number space), there are cases that can not be handled by a library without further
//! context or privileges (eg. the e'' application oriented literal that needs application context,
//! or the ref'' application oriented literal that defers to relative files, accessing which can
//! involve file or network access). Consequentially, conversion to CBOR through the various
//! `.to_cbor()` methods is inherently fallible.
//!
//! [EDN specification]: https://www.ietf.org/archive/id/draft-ietf-cbor-edn-literals-15.html
//!
//! ## Completeness
//!
//! Known limitations are:
//!
//! * Support for inspecting and constructing CBOR items is incomplete. The most common types can
//!   be constructed; contructing or inspecting more exotic items is possible through parsing
//!   hand-crafted EDN/CBOR and using the generated serializations, respectively.
//!
//! * Options for attaching comments and space are limited and immature:
//!
//!   * [`Item::with_comment()`] & [`StandaloneItem::set_comment`] can be used to add comments, but
//!     mainly produce [top-level items][StandaloneItem]. Deeper items are not configurable that
//!     way, as the comments don't live in the item but its container.
//!
//!   * Comments can be added to items through visitors such as [`Item::visit_map_elements`]; both
//!     the success and the error path of a visiting function can set comments around a tag.
//!
//!   * Replacing an item with hand-crafted EDN (possibly from serialized item) is always an
//!     option.
//!
//! * Indenting EDN works for the easy cases, but more exotic cases such as overflowing the limited
//!   width, long keys, or hash comments, easily disrupt the visual result.
//!
//! ## Security
//!
//! This library does not access network or file system in any surprising ways and does not
//! endanger memory safety on its own. The main threat in using it is not resource bound: even
//! without packed CBOR, heavy nesting can easily overflow the stack, and the float conversions are
//! costly in time. Unless resource usage per user is limited, it is recommended to limit untrusted
//! user input to the length of repeated `{` characters that do not yet overflow the stack.
//!
//! The crate has not been audited internally or externally. As the
//! [licenses](https://spdx.org/licenses/MIT.html)
//! [state](https://spdx.org/licenses/Apache-2.0.html), the software is provided "as is".
//!
//! ## CLI application
//!
//! Some functionality is available through a binary included with this crate:
//!
//! <!-- See https://github.com/assert-rs/snapbox/issues/172 -->
//! ```console
//! $ echo "[1, 2, 'x', ip'2001:db1::/64']" | cbor-edn diag2diag
//! [1, 2, 'x', ip'2001:db1::/64']
//! ```
#![forbid(unsafe_code)]

use std::borrow::Cow;

mod visitor;
use visitor::{
    ApplicationLiteralsVisitor, ArrayElementVisitor, MapElementVisitor, MapValueHandler,
    ProcessResult, TagVisitor, Visitor,
};

pub mod application;
pub mod error;
mod float;
mod space;
use space::{Comment, SDetails, MS, MSC, S, SOC};
mod number;
use number::{Number, NumberParts, NumberValue, Sign};
mod string;
use string::{CborString, PreprocessedStringComponent, String1e};

#[cfg(test)]
mod tests;

use error::*;

const U8MAX: u64 = u8::MAX as _;
const U16MAX: u64 = u16::MAX as _;
const U32MAX: u64 = u32::MAX as _;

/// A CBOR Item, including any space and comments surrounding it in a serialization.
#[derive(Debug, Clone, PartialEq)]
pub struct StandaloneItem<'a>(S<'a>, Item<'a>, S<'a>);

/// A CBOR Item.
///
/// This type represents a CBOR item in EDN. By virtue of EDN's expressiveness, it is capable not
/// only of expressing any well-formed CBOR, but also to preserve encoding details that are not
/// preferred (eg. a small integer encoded in more bytes than necessary). Some transformations on
/// the EDN may lose such details; components that perform a translation such as recoding `(_
/// h'18', h'6402')` into `<<100, 2>>` have a choice to either not perform the translation or to
/// discard some encoding details.
#[derive(Debug, Clone, PartialEq)]
pub struct Item<'a>(InnerItem<'a>);

/// # Conversion between the in-memory format and serializations
impl<'a> StandaloneItem<'a> {
    /// Ingests CBOR Diagnostic Notation (EDN) representing a single CBOR item
    ///
    /// Note that this will only return syntactic errors. Content errors that make it impossible to
    /// produce this as CBOR, such as non-matching encoding indicators or unknown application
    /// oriented literals, are not reported.
    pub fn parse(s: &'a str) -> Result<Self, ParseError> {
        cbordiagnostic::one_item(s).map_err(ParseError)
    }

    /// Produce an EDN String from the item
    pub fn serialize(&self) -> String {
        Unparse::serialize(self)
    }

    /// Parse a complete CBOR item.
    ///
    /// Providing excessive data results in an error.
    pub fn from_cbor(cbor: &[u8]) -> Result<Self, CborError> {
        Ok(Self(S::default(), Item::from_cbor(cbor)?, S::default()))
    }

    /// Parse a complete CBOR item.
    ///
    /// Any remaining byts are returned as part of the result.
    pub fn from_cbor_with_rest(cbor: &[u8]) -> Result<(Self, &[u8]), CborError> {
        let (item, rest) = Item::from_cbor_with_rest(cbor)?;
        Ok((Self(S::default(), item, S::default()), rest))
    }

    /// Encode into a binary CBOR representation
    pub fn to_cbor(&self) -> Result<Vec<u8>, InconsistentEdn> {
        Ok(Unparse::to_cbor(self)?.collect())
    }
}

/// # Helpers for conversion between standalone and bare items
impl<'a> StandaloneItem<'a> {
    // pub?
    fn item(&self) -> &Item<'a> {
        &self.1
    }

    // pub?
    fn item_mut(&mut self) -> &mut Item<'a> {
        &mut self.1
    }

    fn inner(&self) -> &InnerItem<'a> {
        self.1.inner()
    }
}

/// # Conversion between the in-memory format and serializations
///
/// Note that unlike [`StandaloneItem`], this does not provide EDN parsing: Any standalone EDN CBOR
/// item may contain outer blank space or comments, which can only be represented in a
/// [`StandaloneItem`].
impl Item<'_> {
    /// Produce an EDN String from the item
    pub fn serialize(&self) -> String {
        Unparse::serialize(self)
    }

    /// Parse a complete CBOR item.
    ///
    /// Providing excessive data results in an error.
    pub fn from_cbor(cbor: &[u8]) -> Result<Self, CborError> {
        match Self::from_cbor_with_rest(cbor) {
            Ok((s, &[])) => Ok(s),
            Ok(_) => Err(CborError("Data after item")),
            Err(e) => Err(e),
        }
    }

    /// Parse a complete CBOR item.
    ///
    /// Any remaining byts are returned as part of the result.
    pub fn from_cbor_with_rest(cbor: &[u8]) -> Result<(Self, &[u8]), CborError> {
        let (major, argument, spec, mut tail) = process_cbor_major_argument(cbor)?;

        let mut s = match (major, argument, spec) {
            (Major::Unsigned, Some(argument), spec) => Self::new_integer_decimal_with_spec(
                argument,
                spec.or_none_if_default_for_arg(argument),
            ),
            (Major::Negative, Some(argument), spec) => Self::new_integer_decimal_with_spec(
                -1i128 - i128::from(argument),
                spec.or_none_if_default_for_arg(argument),
            ),
            (Major::FloatSimple, Some(n @ 0..=19), Spec::S_i) => {
                Simple::Numeric(Box::new(Self::new_integer_decimal(n).into())).into()
            }
            (Major::FloatSimple, Some(20), Spec::S_i) => Simple::False.into(),
            (Major::FloatSimple, Some(21), Spec::S_i) => Simple::True.into(),
            (Major::FloatSimple, Some(22), Spec::S_i) => Simple::Null.into(),
            (Major::FloatSimple, Some(23), Spec::S_i) => Simple::Undefined.into(),
            (Major::FloatSimple, Some(n @ 32..=255), Spec::S_0) => {
                Simple::Numeric(Box::new(Self::new_integer_decimal(n).into())).into()
            }
            // 0..=31 in S_0 or 24..=31 in S_i
            (Major::FloatSimple, _, Spec::S_i | Spec::S_0) => {
                return Err(CborError("Invalid simple value"))
            }
            (Major::FloatSimple, Some(0x7c00), Spec::S_1) => {
                Number(Cow::from("Infinity")).with_spec(Some(Spec::S_1))
            }
            (Major::FloatSimple, Some(0xfc00), Spec::S_1) => {
                Number(Cow::from("-Infinity")).with_spec(Some(Spec::S_1))
            }
            (Major::FloatSimple, Some(0x7e00), Spec::S_1) => {
                Number(Cow::from("NaN")).with_spec(Some(Spec::S_1))
            }
            (Major::FloatSimple, Some(n), Spec::S_1) => {
                let f =
                    float::f16_bits_to_f64(n.try_into().expect("Range limited by construction"));
                Number::new_float(f).with_spec(Some(Spec::S_1))
            }
            (Major::FloatSimple, Some(n), Spec::S_2) => {
                let n: u32 = n.try_into().expect("Range limited by construction");
                let f = f64::from(f32::from_bits(n));
                Number::new_float(f).with_spec(Some(Spec::S_2))
            }
            (Major::FloatSimple, Some(n), Spec::S_3) => {
                let f = f64::from_bits(n);
                Number::new_float(f).with_spec(Some(Spec::S_3))
            }
            (Major::FloatSimple, None, _ /* S_ not written for exhaustiveness */)
            | (Major::FloatSimple, _ /* None not written for exhaustiveness */, Spec::S_) => {
                return Err(CborError(
                    "Break code only expected at end of indefinte length items",
                ))
            }
            (Major::Tagged, Some(n), s) => {
                // FIXME this is recursing on the stack rather than on the heap
                let (item, new_tail) = StandaloneItem::from_cbor_with_rest(tail)?;
                tail = new_tail;
                item.tagged_with_spec(n, s.or_none_if_default_for_arg(n))
            }
            (Major::Unsigned | Major::Negative | Major::Tagged, None, _) => {
                return Err(CborError(
                    "Integer/Tag with indefinite length encoding is not well-formed",
                ))
            }
            (Major::ByteString, Some(n), spec) => {
                let data = n
                    .try_into()
                    .ok()
                    .and_then(|n| tail.get(..n))
                    .ok_or(CborError("Announced bytes unavailable"))?;
                tail = &tail[data.len()..];
                Self::new_bytes_hex_with_spec(data, spec.or_none_if_default_for_arg(n))
            }
            (Major::TextString, Some(n), spec) => {
                let data = n
                    .try_into()
                    .ok()
                    .and_then(|n| tail.get(..n))
                    .ok_or(CborError("Announced bytes unavailable"))?;
                let data = core::str::from_utf8(data)
                    .map_err(|_| CborError("Text string must be valid UTF-8"))?;
                tail = &tail[data.len()..];
                Self::new_text_with_spec(data, spec.or_none_if_default_for_arg(n))
            }
            (
                Major::ByteString | Major::TextString,
                None,
                _, /* S_ not written for exhaustiveness */
            ) => {
                let mut items = vec![];
                while tail.first() != Some(&0xff) {
                    let (inner_major, argument, spec, new_tail) =
                        process_cbor_major_argument(tail)?;
                    let Some(argument) = argument.and_then(|a| usize::try_from(a).ok()) else {
                        return Err(CborError(
                            "Indefinite length strings can only contain definite lengths and must fit in data",
                        ));
                    };
                    if inner_major != major {
                        return Err(CborError(
                            "Indefinite length strings can only contain matching items",
                        ));
                    }
                    if new_tail.len() < argument {
                        return Err(CborError(
                            "Announced bytes unavailable inside indefinite length byte string",
                        ));
                    }
                    // with split_at_checked, we could combine the checkinto the split
                    let (item_data, new_tail) = new_tail.split_at(argument);
                    tail = new_tail;
                    items.push(match major {
                        Major::ByteString => {
                            CborString::new_bytes_hex_with_spec(item_data, Some(spec))
                        }
                        Major::TextString => CborString::new_text_with_spec(
                            core::str::from_utf8(item_data)
                                .map_err(|_| CborError("Text string must be valid UTF-8"))?,
                            Some(spec),
                        ),
                        _ => unreachable!(),
                    });
                }
                if tail.is_empty() {
                    return Err(CborError(
                        "Indefinite length byte string terminated after item",
                    ));
                }
                tail = &tail[1..];

                let mut items = items.drain(..);
                if let Some(first_item) = items.next() {
                    InnerItem::StreamString(
                        Default::default(),
                        NonemptyMscVec::new(first_item, items),
                    )
                    .into()
                } else {
                    todo!()
                }
            }
            (Major::Array, mut length, spec) => {
                // FIXME this is recursing on the stack rather than on the heap
                let mut items = vec![];
                while length != Some(0) && tail.first() != Some(&0xff) {
                    let (item, new_tail) = Self::from_cbor_with_rest(tail)?;
                    items.push(item);
                    tail = new_tail;
                    if let Some(ref mut n) = &mut length {
                        *n -= 1;
                    }
                }
                if length.is_none() {
                    if tail.is_empty() {
                        return Err(CborError(
                            "Indefinite length byte string terminated after item",
                        ));
                    }
                    tail = &tail[1..];
                }
                let spec = match length {
                    Some(l) => spec.or_none_if_default_for_arg(l),
                    None => Some(spec), // which is always indefinite length
                };
                InnerItem::Array(SpecMscVec::new(spec, items.into_iter())).into()
            }
            (Major::Map, mut length, spec) => {
                // FIXME this is recursing on the stack rather than on the heap
                let mut items = vec![];
                while length != Some(0) && tail.first() != Some(&0xff) {
                    let (key, new_tail) = Self::from_cbor_with_rest(tail)?;
                    tail = new_tail;
                    let (value, new_tail) = Self::from_cbor_with_rest(tail)?;
                    tail = new_tail;
                    items.push(Kp::new(key, value));
                    if let Some(ref mut n) = &mut length {
                        *n -= 1;
                    }
                }
                if length.is_none() {
                    if tail.is_empty() {
                        return Err(CborError(
                            "Indefinite length byte string terminated after item",
                        ));
                    }
                    tail = &tail[1..];
                }
                let spec = match length {
                    Some(l) => spec.or_none_if_default_for_arg(l),
                    None => Some(spec), // which is always indefinite length
                };
                InnerItem::Map(SpecMscVec::new(spec, items.into_iter())).into()
            }
        };

        s.set_delimiters(DelimiterPolicy::SingleLineRegularSpacing);
        Ok((s, tail))
    }

    fn visit(&mut self, visitor: &mut impl Visitor) -> ProcessResult {
        let mut result = visitor.process(self);
        if result.take_recurse() {
            self.0.visit(visitor);
        }
        result
    }
}

/// # Conversion between the in-memory format and serializations
impl<'a> Item<'a> {
    fn inner(&self) -> &InnerItem<'a> {
        &self.0
    }

    fn inner_mut(&mut self) -> &mut InnerItem<'a> {
        &mut self.0
    }
}

/// # Creating items from data or by wrapping other items
impl<'a> StandaloneItem<'a> {
    fn tagged_with_spec(self, tag: u64, spec: Option<Spec>) -> Item<'a> {
        InnerItem::Tagged(tag, spec, Box::new(self)).into()
    }

    /// Wrap the item into a CBOR tag.
    pub fn tagged(self, tag: u64) -> Item<'a> {
        InnerItem::Tagged(tag, None, Box::new(self)).into()
    }
}

/// # Creating items from data or by wrapping other items
impl<'a> Item<'a> {
    fn new_integer_decimal_with_spec(value: impl Into<i128>, spec: Option<Spec>) -> Self {
        Number(format!("{}", value.into()).into()).with_spec(spec)
    }

    /// Create a new item that is integer valued in CBOR and expressed in decimal in EDN.
    ///
    /// Note that while values exceeding i65 are accepted, they can not be encoded into CBOR.
    pub fn new_integer_decimal(value: impl Into<i128>) -> Self {
        Self::new_integer_decimal_with_spec(value, None)
    }

    /// Create a new item that is float valued in CBOR and expressed in decimal in EDN.
    pub fn new_float_decimal(value: f64) -> Self {
        Number::new_float(value).with_spec(None)
    }

    /// Create a new item that is integer valued in CBOR and expressed in hexadecimal in EDN.
    ///
    /// Negative values have not been implemented in this constructor.
    pub fn new_integer_hex(value: impl Into<u64>) -> Self {
        InnerItem::Number(Number(format!("0x{:x}", value.into()).into()), None).into()
    }

    fn new_bytes_hex_with_spec(value: &[u8], spec: Option<Spec>) -> Self {
        InnerItem::String(CborString::new_bytes_hex_with_spec(value, spec)).into()
    }

    /// Create a new item that is a byte string in CBOR (identical to the passed in value) and
    /// expressed as a `h'...'` string in EDN.
    pub fn new_bytes_hex(value: &[u8]) -> Self {
        Self::new_bytes_hex_with_spec(value, None)
    }

    fn new_text_with_spec(value: &str, spec: Option<Spec>) -> Self {
        InnerItem::String(CborString::new_text_with_spec(value, spec)).into()
    }

    /// Create a new item that is a text string in CBOR (identical to the passed in value) and
    /// expressed as a single double-quoted string in EDN.
    ///
    /// ```rust
    /// # use cbor_edn::*;
    /// assert_eq!(
    ///     Item::new_text("Hello \"World\"\0").serialize(),
    ///     r#""Hello \"World\"\u{0}""#,
    /// );
    /// ```
    pub fn new_text(value: &str) -> Self {
        Self::new_text_with_spec(value, None)
    }

    pub fn new_application_literal(identifier: &str, value: &str) -> Result<Self, InconsistentEdn> {
        if cbordiagnostic::app_prefix(identifier).is_err() {
            // FIXME bad error type
            return Err(InconsistentEdn(
                "Identifier is not a valid application string identifier",
            ));
        };
        Ok(InnerItem::String(CborString::new_application_literal(identifier, value, None)).into())
    }

    /// Create a CBOR array out of the items
    pub fn new_array(items: impl Iterator<Item = Item<'a>>) -> Self {
        InnerItem::Array(SpecMscVec::new(None, items)).into()
    }

    /// Create a CBOR map out of the keys-value pairs
    pub fn new_map(items: impl Iterator<Item = (Item<'a>, Item<'a>)>) -> Self {
        InnerItem::Map(SpecMscVec::new(
            None,
            items.map(|(key, value)| Kp::new(key, value)),
        ))
        .into()
    }

    /// Wrap the item into a CBOR tag.
    pub fn tagged(self, tag: u64) -> Item<'a> {
        StandaloneItem::from(self).tagged(tag)
    }
}

/// # Accessing and modifying an item in place
impl StandaloneItem<'_> {
    /// Replace any comment before the item with the new comment
    pub fn with_comment(self, comment: &str) -> Self {
        let wrapped_comment = if comment.contains('/') {
            format!("# {}\n", comment.replace('\n', "\n# "))
        } else {
            format!("/ {} /", comment)
        };
        Self(S(wrapped_comment.into()), self.1, self.2)
    }

    /// Replace any comment before the item with the new comment
    pub fn set_comment(&mut self, comment: &str) {
        let wrapped_comment = if comment.contains('/') {
            format!("# {}\n", comment.replace('\n', "\n# "))
        } else {
            format!("/ {} /", comment)
        };
        self.0 = S(wrapped_comment.into());
    }

    /// Alters how space and comments are placed inside the item.
    ///
    /// See the policy values for details.
    pub fn set_delimiters(&mut self, policy: DelimiterPolicy) {
        // On the top level, let's not add the leading \n, because that would cause an empty line
        // above the sole element formatted like this.
        self.0.set_delimiters(policy, false);
        self.1.set_delimiters(policy);
        self.2.set_delimiters(policy, false);
    }

    fn visit(&mut self, visitor: &mut impl Visitor) {
        self.1
            .visit(visitor)
            .use_space_before(&mut self.0)
            .use_space_after(&mut self.2)
            .done();
    }

    /// For each item in the tree that is a single application literal, call a callback.
    ///
    /// This is primarily used to apply custom EDN filtering:
    ///
    /// ```rust
    /// # use cbor_edn::*;
    /// let mut full = StandaloneItem::parse("[0 /unmodified/, german'zweiundvierzig']").unwrap();
    /// full.visit_application_literals(&mut |id, value: String, item: &mut cbor_edn::Item| {
    ///     if id == "german" {
    ///         let numeric = match value.as_str() {
    ///             "dreiundzwanzig" => 23,
    ///             "zweiundvierzig" => 42,
    ///             _ => todo!(),
    ///         };
    ///         *item = Item::new_integer_decimal(numeric).into();
    ///     }
    ///     Ok(())
    /// });
    /// assert_eq!(full.serialize(), "[0 /unmodified/, 42]");
    /// ```
    pub fn visit_application_literals<F, RF>(&mut self, mut f: RF)
    where
        F: for<'b> FnMut(String, String, &mut Item<'b>) -> Result<(), String> + ?Sized,
        RF: std::ops::DerefMut<Target = F>,
    {
        self.visit(&mut ApplicationLiteralsVisitor {
            user_fn: f.deref_mut(),
        });
    }

    /// For each item in the full tree (including embedded representations) that is tagged, call a
    /// callback.
    ///
    /// Any error string is placed in a comment next to the item. The function should return Ok(())
    /// on any tags it is not interested in visiting.
    ///
    /// This is primarily used to apply custom EDN application; see [application::dt_tag_to_aol] for an
    /// example.
    pub fn visit_tag<F, RF>(&mut self, mut f: RF)
    where
        F: for<'b> FnMut(u64, &mut Item<'b>) -> Result<(), String> + ?Sized,
        RF: std::ops::DerefMut<Target = F>,
    {
        self.visit(&mut TagVisitor {
            user_fn: f.deref_mut(),
        });
    }
}

/// # Accessing and modifying an item in place
impl<'a> Item<'a> {
    /// Access application-extension identifier and string value
    ///
    /// This only succeeds if the item is expressed using a single application oriented literal.
    pub fn get_application_literal(&self) -> Result<(String, String), TypeMismatch> {
        let InnerItem::String(CborString { ref items, .. }) = self.inner() else {
            return Err(TypeMismatch::expecting("application-oriented literal"));
        };
        let [chunk] = items.as_slice() else {
            return Err(TypeMismatch::expecting(
                "single application-oriented literal",
            ));
        };
        let PreprocessedStringComponent::AppString(identifier, value) = chunk
            .preprocess()
            // The only reason this would err is if there is embedded CBOR in there, and then
            // that'd just mean it's not what we requested
            .map_err(|_| TypeMismatch::expecting("application-oriented literal"))?
        else {
            return Err(TypeMismatch::expecting("application-oriented literal"));
        };

        Ok((identifier, value))
    }

    /// Access a byte literal value
    ///
    /// This only succeeds if the item is expressed using a single byte string, no matter how many
    /// EDN concatenations or even chunks.
    pub fn get_bytes(&self) -> Result<Vec<u8>, TypeMismatch> {
        let mut result = vec![];

        let mut append_items = |items: &Vec<String1e>| -> Result<(), TypeMismatch> {
            for item in items {
                if item
                    .encoded_major_type()
                    .map_err(|_| TypeMismatch::expecting("byte literal"))?
                    != Major::ByteString
                {
                    return Err(TypeMismatch::expecting("byte literal"));
                }
                result.extend(
                    item.bytes_value()
                        .map_err(|_| TypeMismatch::expecting("byte literal"))?,
                );
            }
            Ok(())
        };

        match self.inner() {
            InnerItem::String(CborString { ref items, .. }) => append_items(items)?,
            InnerItem::StreamString(_, ref chunks) => {
                for CborString { ref items, .. } in chunks.iter() {
                    append_items(items)?;
                }
            }
            _ => return Err(TypeMismatch::expecting("byte literal")),
        }

        Ok(result)
    }

    /// Access the tag number
    ///
    /// This only succeeds if the item is a tagged item. Use [`Self::get_tagged()`] to get the
    /// corresponding tagged item.
    pub fn get_tag(&self) -> Result<u64, TypeMismatch> {
        let InnerItem::Tagged(tag, _, _) = self.inner() else {
            return Err(TypeMismatch::expecting("tagged item"));
        };
        Ok(*tag)
    }

    /// Access the inner item of a tag
    ///
    /// This only succeeds if the item is a tagged item. Use [`Self::get_tag()`] to get the
    /// corresponding tag number.
    pub fn get_tagged(&self) -> Result<&StandaloneItem<'a>, TypeMismatch> {
        let InnerItem::Tagged(_, _, ref item) = self.inner() else {
            return Err(TypeMismatch::expecting("tagged item"));
        };
        Ok(item)
    }

    /// Mutably ccess the inner item of a tag
    ///
    /// This only succeeds if the item is a tagged item. Use [`Self::get_tag()`] to get the
    /// corresponding tag number.
    pub fn get_tagged_mut(&mut self) -> Result<&mut StandaloneItem<'a>, TypeMismatch> {
        let InnerItem::Tagged(_, _, ref mut item) = self.inner_mut() else {
            return Err(TypeMismatch::expecting("tagged item"));
        };
        Ok(item)
    }

    /// Access the integer value of an item
    ///
    /// This only succeeds if the item is integer valued; the returned range is an i65 (expressed
    /// as an i128 for simplicity).
    pub fn get_integer(&self) -> Result<i128, TypeMismatch> {
        let InnerItem::Number(ref number, _) = self.inner() else {
            return Err(TypeMismatch::expecting("integer"));
        };
        match number.value() {
            NumberValue::Float(_) => Err(TypeMismatch::expecting("integer")),
            NumberValue::Positive(n) => Ok(n.into()),
            NumberValue::Negative(n) => Ok(-1 - i128::from(n)),
            // FIXME: that's definitely not a type mismatch
            NumberValue::Big(n) => n
                .try_into()
                .map_err(|_| TypeMismatch::expecting("integer in i128 range")),
        }
    }

    /// Access the float value of an item
    ///
    /// This only succeeds if the item is float valued.
    pub fn get_float(&self) -> Result<f64, TypeMismatch> {
        let InnerItem::Number(ref number, _) = self.inner() else {
            return Err(TypeMismatch::expecting("float"));
        };
        match number.value() {
            NumberValue::Float(f) => Ok(f),
            NumberValue::Positive(_) => Err(TypeMismatch::expecting("float (not integer)")),
            NumberValue::Negative(_) => Err(TypeMismatch::expecting("float (not integer)")),
            NumberValue::Big(_) => Err(TypeMismatch::expecting("float (not integer)")),
        }
    }

    /// Access the items inside an array
    ///
    /// This only succeeds if the item is an array.
    pub fn get_array_items(&self) -> Result<impl Iterator<Item = &Item<'a>>, TypeMismatch> {
        let InnerItem::Array(smv) = self.inner() else {
            return Err(TypeMismatch::expecting("array"));
        };

        Ok(smv.iter())
    }

    /// Mutably access the items inside an array
    ///
    /// This only succeeds if the item is an array.
    pub fn get_array_items_mut(
        &mut self,
    ) -> Result<impl Iterator<Item = &mut Item<'a>>, TypeMismatch> {
        let InnerItem::Array(smv) = self.inner_mut() else {
            return Err(TypeMismatch::expecting("array"));
        };

        Ok(smv.iter_mut())
    }

    /// Access the items inside a map
    ///
    /// This only succeeds if the item is a map.
    pub fn get_map_items(
        &self,
    ) -> Result<impl Iterator<Item = (&Item<'a>, &Item<'a>)>, TypeMismatch> {
        let InnerItem::Map(smv) = self.inner() else {
            return Err(TypeMismatch::expecting("map"));
        };

        Ok(smv.iter().map(|kp| (&kp.key, &kp.value)))
    }

    /// Access the items inside a map
    ///
    /// This only succeeds if the item is a map.
    pub fn get_map_items_mut(
        &mut self,
    ) -> Result<impl Iterator<Item = (&mut Item<'a>, &mut Item<'a>)>, TypeMismatch> {
        let InnerItem::Map(smv) = self.inner_mut() else {
            return Err(TypeMismatch::expecting("map"));
        };

        Ok(smv.iter_mut().map(|kp| (&mut kp.key, &mut kp.value)))
    }

    /// Removes any encoding indicators present in the item.
    ///
    /// This does not affect space or comments; in particular, an item containing only the
    /// necessary space may be left with extraneous (but harmless) space that was previously needed
    /// to set an encoding indicator apart from a value.
    pub fn discard_encoding_indicators(&mut self) {
        self.inner_mut().discard_encoding_indicators();
    }

    /// Alters how space and comments are placed inside the item.
    ///
    /// Being a plain [`Item`], this only affects inner space; it can not have any around itself.
    ///
    /// See the policy values for details.
    pub fn set_delimiters(&mut self, policy: DelimiterPolicy) {
        self.0.set_delimiters(policy);
    }

    /// Turn the item into a [`StandaloneItem`] and add a single new comment
    pub fn with_comment(self, comment: &str) -> StandaloneItem<'a> {
        let wrapped_comment = if comment.contains('/') {
            format!("# {}\n", comment.replace('\n', "\n# "))
        } else {
            format!("/ {} /", comment)
        };
        StandaloneItem(S(wrapped_comment.into()), self, S::default())
    }

    /// Calls a callback on any key item inside the map.
    ///
    /// Calling this on a non-map item returns a [type mismatch error][TypeMismatch].
    ///
    /// An error string returned by the callback is stored in the tree as a comment next to the
    /// key. A successful result may also contain text that gets placed next to the key, and may
    /// contain a callback that gets applied in the same fashion to the value after the key.
    ///
    /// # Example
    ///
    /// The [`application::comment_ccs`] method is an exampel of a callback function.
    ///
    /// # Future development
    ///
    /// Once `feature(try_trait)` is usable, those return types can be simplified; until then,
    /// using a [`Result`] enables easy propagation of errors out of the callbacks.
    pub fn visit_map_elements<F, RF>(&mut self, mut f: RF) -> Result<(), TypeMismatch>
    where
        F: for<'b> FnMut(
                &mut Item<'b>,
            ) -> Result<(Option<String>, Option<MapValueHandler>), String>
            + ?Sized,
        RF: std::ops::DerefMut<Target = F>,
    {
        if !matches!(self.0, InnerItem::Map(_)) {
            return Err(TypeMismatch::expecting("map"));
        }
        let f = f.deref_mut();
        self.visit(&mut MapElementVisitor::new(f)).done();
        Ok(())
    }

    /// Calls a callback on any key item inside the array.
    ///
    /// Calling this on a non-array item returns a [type mismatch error][TypeMismatch].
    ///
    /// An error string returned by the callback is stored in the tree as a comment next to the
    /// item, as is the string in the successful variant.
    ///
    /// # Example
    ///
    /// The [`application::comment_lang_tag`] method is an exampel of a callback function. It is
    /// relatively complex (see below).
    ///
    /// # Future development
    ///
    /// Once `feature(try_trait)` is usable, those return types can be simplified; until then,
    /// using a [`Result`] enables easy propagation of errors out of the callbacks.
    ///
    /// This function is relatively impractical to use: When a callback needs to know its position
    /// in the array (which is a frequent occurrence in inhomogenous arrays), it needs to use
    /// internal state to count up; in doing so it needs to be a closure rather than a function,
    /// and due to [suboptimal lifetimes](https://codeberg.org/chrysn/cbor-edn/issues/9) that means
    /// that the callback may easily need to be boxed.
    pub fn visit_array_elements<F, RF>(&mut self, mut f: RF) -> Result<(), TypeMismatch>
    where
        F: for<'b> FnMut(&mut Item<'b>) -> Result<Option<String>, String> + ?Sized,
        RF: std::ops::DerefMut<Target = F>,
    {
        if !matches!(self.0, InnerItem::Array(_)) {
            return Err(TypeMismatch::expecting("array"));
        }
        let f = f.deref_mut();
        self.visit(&mut ArrayElementVisitor::new(f)).done();
        Ok(())
    }
}

impl Unparse for StandaloneItem<'_> {
    fn serialize_write(&self, formatter: &mut core::fmt::Formatter) -> core::fmt::Result {
        self.0.serialize_write(formatter)?;
        self.1.serialize_write(formatter)?;
        self.2.serialize_write(formatter)?;
        Ok(())
    }

    fn to_cbor(&self) -> Result<impl Iterator<Item = u8>, InconsistentEdn> {
        self.1.to_cbor()
    }
}

impl Unparse for Item<'_> {
    fn serialize_write(&self, formatter: &mut core::fmt::Formatter) -> core::fmt::Result {
        self.0.serialize_write(formatter)
    }

    fn to_cbor(&self) -> Result<impl Iterator<Item = u8>, InconsistentEdn> {
        self.0.to_cbor()
    }
}

impl<'a> From<InnerItem<'a>> for StandaloneItem<'a> {
    fn from(inner: InnerItem<'a>) -> Self {
        Item::from(inner).into()
    }
}

impl<'a> From<Item<'a>> for StandaloneItem<'a> {
    fn from(inner: Item<'a>) -> Self {
        Self(S::default(), inner, S::default())
    }
}

impl<'a> From<InnerItem<'a>> for Item<'a> {
    fn from(inner: InnerItem<'a>) -> Self {
        Item(inner)
    }
}

/// A CBOR Sequence.
#[derive(Debug, Clone, PartialEq)]
pub struct Sequence<'a> {
    s0: S<'a>,
    items: Option<NonemptyMscVec<'a, Item<'a>>>,
}

impl<'a> Sequence<'a> {
    /// Ingests CBOR Diagnostic Notation (EDN) representing a CBOR sequence
    ///
    /// Note that this will only return syntactic errors. Content errors that make it impossible to
    /// produce this as CBOR, such as non-matching encoding indicators or unknown application
    /// oriented literals, are not reported.
    pub fn parse(s: &'a str) -> Result<Self, ParseError> {
        cbordiagnostic::seq(s).map_err(ParseError)
    }

    /// Produce an EDN String from the sequence
    pub fn serialize(&self) -> String {
        Unparse::serialize(self)
    }

    pub fn from_cbor(cbor: &[u8]) -> Result<Self, CborError> {
        let mut tail = cbor;
        // Could this be more efficient if we returned an iterator? Yes. Would it be easier to
        // maintain? Probably not.
        let mut items = vec![];
        while !tail.is_empty() {
            let (item, new_tail) = Item::from_cbor_with_rest(tail)?;
            items.push(item);
            tail = new_tail;
        }
        let mut s = Self::new(items.into_iter());
        s.set_delimiters(DelimiterPolicy::SingleLineRegularSpacing);
        Ok(s)
    }

    /// Encode into a binary CBOR representation
    pub fn to_cbor(&self) -> Result<Vec<u8>, InconsistentEdn> {
        Ok(Unparse::to_cbor(self)?.collect())
    }

    /// Construct a CBOR sequence from items
    pub fn new(mut items: impl Iterator<Item = Item<'a>>) -> Self {
        Sequence {
            s0: Default::default(),
            items: items.next().map(|first| NonemptyMscVec::new(first, items)),
        }
    }

    /// For each item in the tree that is any element of the squence, call a callback.
    ///
    /// This is primarily used to apply custom EDN filtering:
    ///
    /// ```rust
    /// # use cbor_edn::*;
    /// let mut full = Sequence::parse("0 /unmodified/, german'zweiundvierzig'").unwrap();
    /// full.visit_application_literals(&mut |id, value: String, item: &mut cbor_edn::Item| {
    ///     if id == "german" {
    ///         let numeric = match value.as_str() {
    ///             "dreiundzwanzig" => 23,
    ///             "zweiundvierzig" => 42,
    ///             _ => todo!(),
    ///         };
    ///         *item = Item::new_integer_decimal(numeric).into();
    ///     }
    ///     Ok(())
    /// });
    /// assert_eq!(full.serialize(), "0 /unmodified/, 42");
    /// ```
    pub fn visit_application_literals<F, RF>(&mut self, mut f: RF)
    where
        F: for<'b> FnMut(String, String, &mut Item<'b>) -> Result<(), String> + ?Sized,
        RF: std::ops::DerefMut<Target = F>,
    {
        self.visit(&mut ApplicationLiteralsVisitor {
            user_fn: f.deref_mut(),
        });
    }

    /// For each item in the full tree of any element (including embedded representations) that is
    /// tagged, call a callback.
    ///
    /// Any error string is placed in a comment next to the item. The function should return Ok(())
    /// on any tags it is not interested in visiting.
    ///
    /// This is primarily used to apply custom EDN application; see [application::dt_tag_to_aol] for an
    /// example.
    pub fn visit_tag<F, RF>(&mut self, mut f: RF)
    where
        F: for<'b> FnMut(u64, &mut Item<'b>) -> Result<(), String> + ?Sized,
        RF: std::ops::DerefMut<Target = F>,
    {
        self.visit(&mut TagVisitor {
            user_fn: f.deref_mut(),
        });
    }

    /// Mutably access the items of the sequence
    pub fn get_items_mut(&mut self) -> impl Iterator<Item = &mut Item<'a>> {
        self.items
            .as_mut()
            .map(|i| i.iter_mut())
            .into_iter()
            .flatten()
    }

    /// Removes any encoding indicators present in the sequence.
    ///
    /// This does not affect space or comments; in particular, an item containing only the
    /// necessary space may be left with extraneous (but harmless) space that was previously needed
    /// to set an encoding indicator apart from a value.
    pub fn discard_encoding_indicators(&mut self) {
        for i in self.get_items_mut() {
            i.discard_encoding_indicators()
        }
    }

    /// Alters how space and comments are placed inside the sequence.
    ///
    /// See the policy values for details.
    pub fn set_delimiters(&mut self, policy: DelimiterPolicy) {
        // On the top level, let's not add the leading \n, because that would cause an empty line
        // above the sole element formatted like this.
        self.s0.set_delimiters(policy, false);
        for i in self.get_items_mut() {
            i.set_delimiters(policy);
        }
    }

    fn visit(&mut self, visitor: &mut impl Visitor) {
        if let Some(nmv) = self.items.as_mut() {
            nmv.visit(visitor).use_space_after(&mut self.s0).done();
        }
    }
}

impl Unparse for Sequence<'_> {
    fn serialize_write(&self, formatter: &mut core::fmt::Formatter) -> core::fmt::Result {
        self.s0.serialize_write(formatter)?;
        if let Some(items) = self.items.as_ref() {
            items.serialize_write(formatter)?;
        }
        Ok(())
    }

    fn to_cbor(&self) -> Result<impl Iterator<Item = u8>, InconsistentEdn> {
        let chain = self.items.as_ref().map(|items| items.to_cbor());
        let chain = chain.transpose();
        chain.map(|optit| optit.into_iter().flatten())
    }
}

/// Rule set for the `set_delimiters()` family of methods
#[derive(Copy, Clone, Debug, PartialEq)]
#[non_exhaustive]
pub enum DelimiterPolicy {
    /// Remove all comments, optional space and commas; place commas exactly where in there absence there
    /// would need to be space instead.
    DiscardAll,
    /// Like [`DiscardAll`][DelimiterPolicy::DiscardAll], but leave comments in place.
    DiscardAllButComments,
    /// Set commas where separation is mandatory, followed by a single space; set a single space after colons of key-value pairs.
    ///
    /// All other space and commas are removed. Comments are retained, including space between
    /// adjacent comments.
    SingleLineRegularSpacing,
    /// Replace all space with automated indentation. Comments are left in place, including line
    /// breaks, space and commas inside or between adjacent comments.
    ///
    /// For an easy default construction, see the [`.indented()`](Self::indented) method.
    IndentedRegularSpacing {
        /// Indentation level at the start
        base_indent: usize,
        /// Indentation added per nesting level
        indent_level: usize,
        /// Maximum width of lines that is left as a single item.
        ///
        /// If zero, this will wrap all nested structures; otherwise, it will leave small items
        /// with `SingleLineRegularSpacing`.
        ///
        /// Note that this measures line width in bytes; this is not exact if non-ASCII characters
        /// are involved, but a good enough estimate for most EDN content.
        max_width: usize,
    },
    /// Set a single space wherever one is allowed.
    ///
    /// This is not a practical policy over-all, but some functions may set this for their
    /// downstream items.
    SingleSpace,
}

impl DelimiterPolicy {
    /// Constructor for [`Self::IndentedRegularSpacing`] with default settings
    pub fn indented() -> Self {
        Self::IndentedRegularSpacing {
            base_indent: 0,
            indent_level: 4,
            max_width: 80,
        }
    }
}

/// Trait through which a parsed CBOR diagnostic notation item can be turned back into a string
trait Unparse: Sized {
    /// Write the full item into a given formatter
    ///
    /// This is mainly used to implement this trait, but rarely called from the outside.
    fn serialize_write(&self, formatter: &mut core::fmt::Formatter) -> core::fmt::Result;

    /// Produce a String from the full item
    ///
    /// No reason is known to not use the provided method; this is what is usually called on an
    /// item implemlenting this trait.
    fn serialize(&self) -> String {
        struct Unparsed<'a, T: Unparse>(&'a T);
        impl<T: Unparse> core::fmt::Display for Unparsed<'_, T> {
            fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
                self.0.serialize_write(f)
            }
        }

        format!("{}", Unparsed(self))
    }

    fn to_cbor(&self) -> Result<impl Iterator<Item = u8>, InconsistentEdn>;
}

/// This represents a `T *(MSC T) SOC` sequence.
///
/// This type is common to CBOR sequences, streamstrings and array/map, with different mechsnisms of
/// optionality around them ("just have the whole thing None", "there must be at least one" and
/// "the empty variant has a different type (specms vs. spec) next to it").
#[derive(Debug, Clone, PartialEq)]
struct NonemptyMscVec<'a, T: Unparse> {
    // Most users of this are somehow inside Item, and T is usally an item itself -- so we box the
    // T here to avoid recursively sized types.
    first: Box<T>,
    tail: Vec<(MSC<'a>, T)>,
    soc: SOC<'a>,
}

impl<'a, T: Unparse> NonemptyMscVec<'a, T> {
    /// Creates a new instance from just the items, with default space.
    fn new(first: T, tail: impl Iterator<Item = T>) -> Self {
        Self {
            first: Box::new(first),
            tail: tail.map(|i| (Default::default(), i)).collect(),
            soc: Default::default(),
        }
    }

    /// Creates a new instance, taking explicitly all space components (as used in a parser).
    fn new_parsing(first: T, tail: Vec<(MSC<'a>, T)>, soc: SOC<'a>) -> Self {
        Self {
            first: Box::new(first),
            tail,
            soc,
        }
    }

    fn len(&self) -> usize {
        1 + self.tail.len()
    }

    fn iter(&self) -> impl Iterator<Item = &T> {
        core::iter::once(&*self.first).chain(self.tail.iter().map(|(_msc, t)| t))
    }
}

impl<'a> NonemptyMscVec<'a, Item<'a>> {
    fn visit(&mut self, visitor: &mut impl Visitor) -> ProcessResult {
        let mut own_result = self.first.visit(visitor);
        let mut last_result: Option<ProcessResult> = None;
        for (msc, item) in self.tail.iter_mut() {
            if let Some(result) = last_result.take() {
                result.use_space_after(msc).done();
            } else {
                own_result = own_result.use_space_after(msc);
            }
            let item_result = item.visit(visitor);
            let replaced = last_result.replace(item_result.use_space_before(msc));
            assert!(replaced.is_none());
        }
        if let Some(result) = last_result.take() {
            result.use_space_after(&mut self.soc).done();
        } else {
            own_result = own_result.use_space_after(&mut self.soc);
        }

        own_result
    }
}
// Those ↑ and ↓ are identical, but we don't have a trait for being visit'able … should we?
impl<'a> NonemptyMscVec<'a, Kp<'a>> {
    fn visit(&mut self, visitor: &mut impl Visitor) -> ProcessResult {
        let mut own_result = self.first.visit(visitor);
        let mut last_result: Option<ProcessResult> = None;
        for (msc, item) in self.tail.iter_mut() {
            if let Some(result) = last_result.take() {
                result.use_space_after(msc).done();
            } else {
                own_result = own_result.use_space_after(msc);
            }
            let item_result = item.visit(visitor);
            let replaced = last_result.replace(item_result.use_space_before(msc));
            assert!(replaced.is_none());
        }
        if let Some(result) = last_result.take() {
            result.use_space_after(&mut self.soc).done();
        } else {
            own_result = own_result.use_space_after(&mut self.soc);
        }

        own_result
    }
}

// With feature(precise_capturing), we can use the impl … + use syntax, and unify over T.
// fn iter_mut(&mut self) -> impl Iterator<Item = &mut T> + use<'_, 'a, T> {
macro_rules! nmv_concrete_impl {
    ($t:ident) => {
        impl<'a> NonemptyMscVec<'a, $t<'a>> {
            fn iter_mut(&mut self) -> impl Iterator<Item = &mut $t<'a>> {
                let first: &mut $t<'a> = &mut self.first;
                let tail = &mut self.tail;
                core::iter::once(first).chain(tail.iter_mut().map(|(_msc, i)| i))
            }
        }
    };
}
nmv_concrete_impl!(Item);
nmv_concrete_impl!(CborString);

impl<T: Unparse> Unparse for NonemptyMscVec<'_, T> {
    fn serialize_write(&self, formatter: &mut core::fmt::Formatter) -> core::fmt::Result {
        self.first.serialize_write(formatter)?;
        for (msc, item) in self.tail.iter() {
            msc.serialize_write(formatter)?;
            item.serialize_write(formatter)?;
        }
        self.soc.serialize_write(formatter)?;
        Ok(())
    }

    fn to_cbor(&self) -> Result<impl Iterator<Item = u8>, InconsistentEdn> {
        // Collecting in a vec of inner iterators to flush out the error early
        let collected: Result<Vec<_>, _> = self.iter().map(Unparse::to_cbor).collect();
        Ok(collected?.into_iter().flatten())
    }
}

/// An empty-allowing extension of [`NonemptyMscVec`] where the empty and nonempty versions differ
/// in that the empty version has a spec and the nonempty version has a specms.
///
/// Note that this is a bit funny in that the space after specms is always empty when there is
/// Some spec (because then its inner MS consumes them), whereas when there is no spec, the space
/// lands in the `.s`.
#[derive(Debug, Clone, PartialEq)]
enum SpecMscVec<'a, T: Unparse> {
    Present {
        spec: Option<(Spec, MS<'a>)>,
        s: S<'a>,
        items: NonemptyMscVec<'a, T>,
    },
    Absent {
        spec: Option<Spec>,
        s: S<'a>,
    },
}

impl<T: Unparse> SpecMscVec<'_, T> {
    /// Construct a new list from a spec and items
    fn new(spec: Option<Spec>, mut items: impl Iterator<Item = T>) -> Self {
        if let Some(first) = items.next() {
            // The Some is a bit weird here because the type of SpecMscVec expects Spec to
            // non-nullable; we'll see how this develops once that is removed)
            SpecMscVec::Present {
                spec: spec.map(|spec| (spec, Default::default())),
                s: Default::default(),
                items: NonemptyMscVec::new(first, items),
            }
        } else {
            SpecMscVec::Absent {
                spec,
                s: Default::default(),
            }
        }
    }

    fn len(&self) -> usize {
        match self {
            SpecMscVec::Present { items, .. } => items.len(),
            SpecMscVec::Absent { .. } => 0,
        }
    }

    fn spec(&self) -> Option<Spec> {
        match self {
            SpecMscVec::Present {
                spec: Some((spec, _ms)),
                ..
            } => Some(*spec),
            SpecMscVec::Present { spec: None, .. } => None,
            SpecMscVec::Absent { spec, .. } => *spec,
        }
    }

    fn iter(&self) -> impl Iterator<Item = &T> {
        let (first, tail) = match self {
            SpecMscVec::Absent { .. } => (None, None),
            SpecMscVec::Present {
                items: NonemptyMscVec { first, tail, .. },
                ..
            } => (Some(first.as_ref()), Some(tail)),
        };
        first
            .into_iter()
            .chain(tail.into_iter().flatten().map(|(_msc, i)| i))
    }

    /// Discards the own spec.
    ///
    /// On presence, this discards a single blank character from the MS that becomes the S (for the
    /// common case of the MS just having that mandatory space), but retains any other space
    /// including comments.
    fn discard_own_encoding_indicator(&mut self) {
        match self {
            SpecMscVec::Absent { spec, .. } => *spec = None,
            SpecMscVec::Present { spec, s, .. } => {
                if let Some((_spec, ms)) = spec.take() {
                    if ms != Default::default() {
                        // Most of the time, s is already empty, but during manipulation, it can
                        // get some value too.
                        s.prefix(ms.0);
                    }
                }
            }
        }
    }
}

// With feature(precise_capturing), we can use the impl … + use syntax, and unify over T. When
// restoring the generic form, beware that this will require an explicit lifetime on the impl
// (instead of `impl<T: …> SpecMscVec<'_, T>`).
//
// fn iter_mut(&mut self) -> impl Iterator<Item = &mut T> + use<'_, 'a, T> {
macro_rules! smv_concrete_impl {
    ($t:ident) => {
        impl<'a> SpecMscVec<'a, $t<'a>> {
            fn iter_mut(&mut self) -> impl Iterator<Item = &mut $t<'a>> {
                let (first, tail) = match self {
                    SpecMscVec::Absent { .. } => (None, None),
                    SpecMscVec::Present {
                        items: NonemptyMscVec { first, tail, .. },
                        ..
                    } => (Some(first.as_mut()), Some(tail)),
                };
                first
                    .into_iter()
                    .chain(tail.into_iter().flatten().map(|(_msc, i)| i))
            }
        }
    };
}
smv_concrete_impl!(Item);
smv_concrete_impl!(Kp);

impl<'a> SpecMscVec<'a, Item<'a>> {
    fn visit(&mut self, visitor: &mut impl Visitor) {
        match self {
            SpecMscVec::Present { spec: _, s, items } => {
                // anything to after the last item is processed internally
                items.visit(visitor).use_space_before(s).done();
            }
            SpecMscVec::Absent { spec: _, s: _ } => (),
        }
    }
}
// Those ↑ and ↓ are identical, but we don't have a trait for being visit'able … should we?
impl<'a> SpecMscVec<'a, Kp<'a>> {
    fn visit(&mut self, visitor: &mut impl Visitor) {
        match self {
            SpecMscVec::Present { spec: _, s, items } => {
                // anything to after the last item is processed internally
                items.visit(visitor).use_space_before(s).done();
            }
            SpecMscVec::Absent { spec: _, s: _ } => (),
        }
    }
}

impl<T: Unparse> Unparse for SpecMscVec<'_, T> {
    fn serialize_write(&self, formatter: &mut core::fmt::Formatter) -> core::fmt::Result {
        match self {
            SpecMscVec::Present { spec, s, items } => {
                if let Some((spec, msc)) = spec {
                    spec.serialize_write(formatter)?;
                    msc.serialize_write(formatter)?;
                }
                s.serialize_write(formatter)?;
                items.serialize_write(formatter)?;
                Ok(())
            }
            SpecMscVec::Absent { spec, s } => {
                if let Some(spec) = spec {
                    spec.serialize_write(formatter)?;
                }
                s.serialize_write(formatter)?;
                Ok(())
            }
        }
    }

    // This writes just the CBOR items; it is up to the caller to process the spec.
    fn to_cbor(&self) -> Result<impl Iterator<Item = u8>, InconsistentEdn> {
        // FIXME: Or is this just now the point to split Unparse and not implement the CBOR side?

        // Collecting in a vec of inner iterators to flush out the error early
        let collected: Result<Vec<_>, _> = self.iter().map(Unparse::to_cbor).collect();
        Ok(collected?.into_iter().flatten())
    }
}

/// A key-value pair of CBOR items, both surrounded by [S]pace, separated by a ":"
#[derive(Debug, Clone, PartialEq)]
struct Kp<'a> {
    key: Item<'a>,
    s0: S<'a>,
    s1: S<'a>,
    value: Item<'a>,
}

impl<'a> Kp<'a> {
    fn new(key: Item<'a>, value: Item<'a>) -> Self {
        Self {
            key,
            s0: Default::default(),
            s1: Default::default(),
            value,
        }
    }

    fn visit(&mut self, visitor: &mut impl Visitor) -> ProcessResult {
        let key_result = self.key.visit(visitor);
        let value_result = self.value.visit(visitor);
        key_result
            .use_space_after(&mut self.s0)
            .chain(value_result.use_space_before(&mut self.s1))
    }
}

impl Unparse for Kp<'_> {
    fn serialize_write(&self, formatter: &mut core::fmt::Formatter) -> core::fmt::Result {
        self.key.serialize_write(formatter)?;
        self.s0.serialize_write(formatter)?;
        formatter.write_str(":")?;
        self.s1.serialize_write(formatter)?;
        self.value.serialize_write(formatter)?;
        Ok(())
    }

    fn to_cbor(&self) -> Result<impl Iterator<Item = u8>, InconsistentEdn> {
        Ok([self.key.to_cbor()?, self.value.to_cbor()?]
            .into_iter()
            .flatten())
    }
}

#[derive(Debug, Clone, PartialEq)]
enum Simple<'a> {
    False,
    True,
    Null,
    Undefined,
    // Note that later processing may be upset if the string is not a Number item, but cpa'something' may make sense
    Numeric(Box<StandaloneItem<'a>>),
}

impl Unparse for Simple<'_> {
    fn serialize_write(&self, formatter: &mut core::fmt::Formatter) -> core::fmt::Result {
        match self {
            Simple::False => formatter.write_str("false")?,
            Simple::True => formatter.write_str("true")?,
            Simple::Null => formatter.write_str("null")?,
            Simple::Undefined => formatter.write_str("undefined")?,
            Simple::Numeric(i) => {
                formatter.write_str("simple(")?;
                i.serialize_write(formatter)?;
                formatter.write_str(")")?;
            }
        }
        Ok(())
    }

    fn to_cbor(&self) -> Result<impl Iterator<Item = u8>, InconsistentEdn> {
        let mut result = Vec::new();
        match self {
            Simple::False => result.push(0xf4),
            Simple::True => result.push(0xf5),
            Simple::Null => result.push(0xf6),
            Simple::Undefined => result.push(0xf7),
            Simple::Numeric(i) => {
                let InnerItem::Number(ref number, spec) = i.inner() else {
                    return Err(InconsistentEdn(
                        "Items inside simple() need to be numbers for serialization.",
                    ));
                };
                let NumberValue::Positive(number) = number.value() else {
                    return Err(InconsistentEdn(
                        "Non-positive numbers can not be in a Simple",
                    ));
                };
                if number > 255 {
                    return Err(InconsistentEdn("Spec exceeds valid range of 0..=255"));
                }
                let requested = Spec::encode_argument(spec.as_ref(), Major::FloatSimple, number)?;
                let permissible = Spec::encode_argument(None, Major::FloatSimple, number)?;
                if requested != permissible {
                    return Err(InconsistentEdn(
                        "Encoding indicators on simple value must use the preferred encoding",
                    ));
                }
                result.extend(permissible);
            }
        };
        Ok(result.into_iter())
    }
}

impl<'a> From<Simple<'a>> for Item<'a> {
    fn from(input: Simple<'a>) -> Self {
        InnerItem::Simple(input).into()
    }
}

/// An arbitrary CBOR item
#[derive(Clone, Debug, PartialEq)]
enum InnerItem<'a> {
    Map(SpecMscVec<'a, Kp<'a>>),
    Array(SpecMscVec<'a, Item<'a>>),
    Tagged(u64, Option<Spec>, Box<StandaloneItem<'a>>),
    /// Stored as a string, but we could also explicitly capture the variation:
    /// * is a sign present? (even in an integer negative 0?)
    /// * what is the base?
    /// * how many leading zeros are there?
    /// * is there an explicit power (and if so, does it have an explicit sign, or leading zeros?)
    /// * note that there are no inner spaces or underscors: no "1 000 000" or "1_000_000", the
    ///   latter would conflict with encoding indicators.
    ///
    /// (and it can be arbitrarily long, exceeding a u64)
    Number(Number<'a>, Option<Spec>),
    Simple(Simple<'a>),
    String(CborString<'a>),
    StreamString(MS<'a>, NonemptyMscVec<'a, CborString<'a>>),
}

impl InnerItem<'_> {
    /// Discard any encoding indicators ([Spec]) that may be part of the item
    fn discard_encoding_indicators(&mut self) {
        match self {
            InnerItem::Map(items) => {
                for i in items.iter_mut() {
                    i.key.discard_encoding_indicators();
                    i.value.discard_encoding_indicators();
                }
                items.discard_own_encoding_indicator();
            }
            InnerItem::Array(items) => {
                for i in items.iter_mut() {
                    i.discard_encoding_indicators();
                }
                items.discard_own_encoding_indicator();
            }
            InnerItem::Tagged(_n, spec, item) => {
                *spec = None;
                item.item_mut().discard_encoding_indicators();
            }
            InnerItem::Number(_n, spec) => {
                *spec = None;
            }
            InnerItem::Simple(Simple::Numeric(i)) => i.item_mut().discard_encoding_indicators(),
            InnerItem::Simple(_) => {}
            InnerItem::String(items) => {
                items.discard_encoding_indicators();
            }
            InnerItem::StreamString(_ms, items) => {
                // FIXME: Shouldn't this just become String? (StreamString is kind of an encoding
                // indicator)
                for i in items.iter_mut() {
                    i.discard_encoding_indicators();
                }
            }
        }
    }

    fn set_delimiters(&mut self, policy: DelimiterPolicy) {
        use DelimiterPolicy::*;

        let nested_policy = if let IndentedRegularSpacing {
            base_indent,
            indent_level,
            max_width,
        } = policy
        {
            // Try fitting it in one line; that doesn't do anything that won't be changed by proper
            // indentation later anyway, so we don't need to roll back.
            self.set_delimiters(SingleLineRegularSpacing);
            if self.serialize().len() + base_indent < max_width {
                return;
            }

            IndentedRegularSpacing {
                base_indent: base_indent + indent_level,
                indent_level,
                max_width,
            }
        } else {
            policy
        };

        match self {
            InnerItem::Map(items) => match items {
                SpecMscVec::Absent { s, .. } => s.set_delimiters(nested_policy, false),
                SpecMscVec::Present { s, items, .. } => {
                    s.set_delimiters(nested_policy, true);
                    let set_on_item = |kp: &mut Kp| {
                        kp.key.set_delimiters(nested_policy);
                        kp.value.set_delimiters(nested_policy);
                        kp.s0.set_delimiters(nested_policy, false);
                        // Or true … but that may need an extra case in the top-level
                        // inden`ted-to-single-line logic
                        kp.s1.set_delimiters(nested_policy, false);
                    };
                    set_on_item(&mut items.first);
                    for (msc, item) in items.tail.iter_mut() {
                        set_on_item(item);
                        msc.set_delimiters(nested_policy, true);
                    }
                    items.soc.set_delimiters(policy, true);
                }
            },
            InnerItem::Array(items) => match items {
                SpecMscVec::Absent { s, .. } => s.set_delimiters(nested_policy, false),
                SpecMscVec::Present { s, items, .. } => {
                    s.set_delimiters(nested_policy, true);
                    items.first.set_delimiters(nested_policy);
                    for (msc, item) in items.tail.iter_mut() {
                        item.set_delimiters(nested_policy);
                        msc.set_delimiters(nested_policy, true);
                    }
                    items.soc.set_delimiters(policy, true);
                }
            },
            InnerItem::Tagged(_n, _spec, item) => {
                item.set_delimiters(nested_policy);
            }
            InnerItem::Number(_n, _spec) => {}
            InnerItem::Simple(Simple::Numeric(item)) => {
                // Setting the nested_policy on the item as a whole would lead to unsightly indentation --
                // setting it piecemeal instead.
                item.0.set_delimiters(nested_policy, false);
                item.1.set_delimiters(nested_policy);
                item.2.set_delimiters(nested_policy, false);
            }
            InnerItem::Simple(_) => {}
            InnerItem::String(CborString { items, separators }) => {
                for i in items {
                    i.set_delimiters(nested_policy);
                }
                for (sep_pre, sep_post) in separators {
                    match nested_policy {
                        SingleLineRegularSpacing => {
                            sep_pre.set_delimiters(SingleSpace, true);
                            sep_post.set_delimiters(SingleSpace, false);
                        }
                        _ => {
                            sep_pre.set_delimiters(nested_policy, true);
                            sep_post.set_delimiters(nested_policy, false);
                        }
                    }
                }
            }
            InnerItem::StreamString(ms, NonemptyMscVec { first, tail, soc }) => {
                ms.set_delimiters(nested_policy, true);
                first.set_delimiters(nested_policy);
                for (ms, item) in tail {
                    ms.set_delimiters(nested_policy, true);
                    item.set_delimiters(nested_policy);
                }
                soc.set_delimiters(policy, true);
            }
        }
    }

    fn visit(&mut self, visitor: &mut impl Visitor) {
        match self {
            InnerItem::Map(spec_msc_vec) => {
                spec_msc_vec.visit(visitor);
            }
            InnerItem::Array(spec_msc_vec) => {
                spec_msc_vec.visit(visitor);
            }
            InnerItem::Tagged(_number, _spec, standalone_item) => {
                // This mainly returns no ProcessResult because comments can well be placed inside
                // the item -- but if someone really wants to act on the outside, that could be
                // taken through here.
                standalone_item.visit(visitor);
            }
            InnerItem::Number(_number, _spec) => (),
            InnerItem::Simple(_simple) => (),
            InnerItem::String(_cbor_string) => (),
            InnerItem::StreamString(_ms, _nonempty_msc_vec) => (),
        }
    }
}

impl Unparse for InnerItem<'_> {
    fn serialize_write(&self, formatter: &mut core::fmt::Formatter) -> core::fmt::Result {
        match self {
            InnerItem::Map(items) => {
                write!(formatter, "{{")?;
                items.serialize_write(formatter)?;
                write!(formatter, "}}")?;
                Ok(())
            }
            InnerItem::Array(items) => {
                write!(formatter, "[")?;
                items.serialize_write(formatter)?;
                write!(formatter, "]")?;
                Ok(())
            }
            InnerItem::Tagged(n, spec, item) => {
                write!(formatter, "{}", n)?;
                if let Some(spec) = spec {
                    spec.serialize_write(formatter)?;
                }
                formatter.write_str("(")?;
                item.serialize_write(formatter)?;
                formatter.write_str(")")?;
                Ok(())
            }
            InnerItem::Number(n, spec) => {
                formatter.write_str(&n.0)?;
                if let Some(spec) = spec {
                    spec.serialize_write(formatter)?;
                }
                Ok(())
            }
            InnerItem::Simple(s) => s.serialize_write(formatter),
            InnerItem::String(s) => s.serialize_write(formatter),
            InnerItem::StreamString(ms, nmv) => {
                formatter.write_str("(_")?;
                ms.serialize_write(formatter)?;
                nmv.serialize_write(formatter)?;
                formatter.write_str(")")?;
                Ok(())
            }
        }
    }

    fn to_cbor(&self) -> Result<impl Iterator<Item = u8>, InconsistentEdn> {
        let mut result = vec![];
        match self {
            InnerItem::Map(smv) => {
                let len = smv.len();
                let spec = smv.spec();
                let (head, tail) = Spec::encode_item_count(spec.as_ref(), Major::Map, len)?;
                result.extend(head);
                for i in smv.iter() {
                    result.extend(i.to_cbor()?);
                }
                result.extend(tail);
            }
            InnerItem::Array(smv) => {
                let len = smv.len();
                let spec = smv.spec();
                let (head, tail) = Spec::encode_item_count(spec.as_ref(), Major::Array, len)?;
                result.extend(head);
                for i in smv.iter() {
                    result.extend(i.to_cbor()?);
                }
                result.extend(tail);
            }
            InnerItem::Tagged(n, spec, item) => {
                result.extend(Spec::encode_argument(spec.as_ref(), Major::Tagged, *n)?);
                result.extend(item.to_cbor()?);
            }
            InnerItem::Number(n, spec) => match n.value() {
                NumberValue::Positive(n) => {
                    result.extend(Spec::encode_argument(spec.as_ref(), Major::Unsigned, n)?)
                }
                NumberValue::Negative(n) => {
                    result.extend(Spec::encode_argument(spec.as_ref(), Major::Negative, n)?)
                }
                NumberValue::Float(n) => result.extend(float::encode(n, *spec)?),
                NumberValue::Big(n) => match spec {
                    None => {
                        let (tag, positive) = if n >= num_bigint::BigInt::ZERO {
                            (2, n)
                        } else {
                            (3, -n)
                        };
                        use num_traits::ops::bytes::ToBytes;
                        result.extend(Spec::encode_argument(None, Major::Tagged, tag)?);
                        let bytes = positive.to_be_bytes();
                        result.extend(Spec::encode_argument(
                            None,
                            Major::ByteString,
                            bytes
                                .len()
                                .try_into()
                                .expect("Even on 128-bit systems, EDN does not exceed 64bit sizes"),
                        )?);
                        result.extend(bytes);
                    }
                    _ => {
                        return Err(InconsistentEdn(
                            "Encoding indicators not specified for bignums",
                        ))
                    }
                },
            },
            InnerItem::Simple(s) => result.extend(s.to_cbor()?),
            InnerItem::String(s) => result.extend(s.to_cbor()?),
            InnerItem::StreamString(_ms, NonemptyMscVec { first, tail, .. }) => {
                let major = first.encoded_major_type()?;
                if !matches!(major, Major::TextString | Major::ByteString) {
                    // Syntax can't catch this: Might be an application oriented literal that is
                    // not string-valued
                    return Err(InconsistentEdn(
                        "Item in indefinite length string that is neither bytes nor string",
                    ));
                }
                result.push(((major as u8) << 5) | 31);
                result.extend(first.to_cbor()?);
                for item in tail.iter() {
                    if item.1.encoded_major_type()? != major {
                        return Err(InconsistentEdn("Item in indefinite length string has different encoding than head element"));
                    }
                    result.extend(item.1.to_cbor()?);
                }
                result.push(0xff);
            }
        }
        Ok(result.into_iter())
    }
}

#[derive(PartialEq, Debug, Copy, Clone)]
enum Major {
    Unsigned = 0,
    Negative = 1,
    ByteString = 2,
    TextString = 3,
    Array = 4,
    Map = 5,
    Tagged = 6,
    FloatSimple = 7,
}

impl Major {
    /// Given a byte, return its major type and the additional information.
    fn from_byte(byte: u8) -> (Self, u8) {
        (
            match byte >> 5 {
                0 => Major::Unsigned,
                1 => Major::Negative,
                2 => Major::ByteString,
                3 => Major::TextString,
                4 => Major::Array,
                5 => Major::Map,
                6 => Major::Tagged,
                7 => Major::FloatSimple,
                _ => unreachable!(),
            },
            byte & 0x1f,
        )
    }
}

/// An encoding indicator
///
/// Encoding indicators are typically rendered with an underscore, eg. in `4_1`, `_1` is the
/// encoding indicator `Spec("1")`, and tells that the number 4 was encoded in more bytes than
/// would have been needed.
///
/// While encoding indicators are described as an extensible registry, new values would interfere
/// so deeply with this crate's operation that they would need a code change; consequently, unknown
/// values are rejected at parsing time.
#[derive(Copy, Clone, Debug, PartialEq)]
#[allow(non_camel_case_types)] // reason: underscores are part of what we express here
enum Spec {
    S_,
    S_i,
    S_0,
    S_1,
    S_2,
    S_3,
}

impl Spec {
    /// Given an item count, produce the encoded item count for a given Major type (only makes
    /// sense for an array and map), as well as any terminator that'd be necessary after the list
    /// in case of in indefinite length encoding
    fn encode_item_count(
        self_: Option<&Self>,
        major: Major,
        count: usize,
    ) -> Result<(Vec<u8>, &[u8]), InconsistentEdn> {
        debug_assert!(matches!(major, Major::Map | Major::Array), "Encoding an item count only makes see for maps and arrays; strings work a bit different.");
        Ok((
            Spec::encode_argument(self_, major, count.try_into().expect("Even on 128bit architectures we can't have more than 64bit long counts of items"))?,
            if matches!(self_, Some(Spec::S_)) { [0xff].as_slice() } else { [].as_slice() },
        ))
    }

    fn encode_argument(
        self_: Option<&Self>,
        major: Major,
        argument: u64,
    ) -> Result<Vec<u8>, InconsistentEdn> {
        let full_spec = match (self_, argument) {
            (None, 0..=23) => Self::S_i,
            (None, 0..=U8MAX) => Self::S_0,
            (None, 0..=U16MAX) => Self::S_1,
            (None, 0..=U32MAX) => Self::S_2,
            (None, _) => Self::S_3,
            (Some(s), _) => *s,
        };

        let immediate_value = match full_spec {
            Self::S_ => 31,
            Self::S_i => {
                if argument < 24 {
                    argument as u8
                } else {
                    return Err(InconsistentEdn(
                        "Immediate encoding demanded but value exceeds 23",
                    ));
                }
            }
            Self::S_0 => 24,
            Self::S_1 => 25,
            Self::S_2 => 26,
            Self::S_3 => 27,
        };
        let first = core::iter::once(((major as u8) << 5) | immediate_value);
        Ok(match full_spec {
            Self::S_ | Self::S_i => first.collect(),
            Self::S_0 => first.chain(u8::try_from(argument)?.to_be_bytes()).collect(),
            Self::S_1 => first
                .chain(u16::try_from(argument)?.to_be_bytes())
                .collect(),
            Self::S_2 => first
                .chain(u32::try_from(argument)?.to_be_bytes())
                .collect(),
            Self::S_3 => first.chain(argument.to_be_bytes()).collect(),
        })
    }

    fn serialize_write(&self, formatter: &mut core::fmt::Formatter) -> core::fmt::Result {
        match self {
            Self::S_ => formatter.write_str("_"),
            Self::S_i => formatter.write_str("_i"),
            Self::S_0 => formatter.write_str("_0"),
            Self::S_1 => formatter.write_str("_1"),
            Self::S_2 => formatter.write_str("_2"),
            Self::S_3 => formatter.write_str("_3"),
        }
    }

    /// Return None if the integer argument leads to self being selected in preferred encoding
    /// anyway.
    ///
    /// We can't do this in [process_cbor_major_argument] because floats are not so trivial to
    /// classify.
    fn or_none_if_default_for_arg(self, arg: u64) -> Option<Self> {
        const U8MAXPLUS: u64 = U8MAX + 1;
        const U16MAXPLUS: u64 = U16MAX + 1;
        const U32MAXPLUS: u64 = U32MAX + 1;
        match (self, arg) {
            (Spec::S_i, 0..=23) => None,
            (Spec::S_0, 24..=U8MAX) => None,
            (Spec::S_1, U8MAXPLUS..=U16MAX) => None,
            (Spec::S_2, U16MAXPLUS..=U32MAX) => None,
            (Spec::S_3, U32MAXPLUS..=u64::MAX) => None,
            (s, _) => Some(s),
        }
    }
}

impl core::str::FromStr for Spec {
    type Err = &'static str;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "" => Ok(Self::S_),
            "i" => Ok(Self::S_i),
            "0" => Ok(Self::S_0),
            "1" => Ok(Self::S_1),
            "2" => Ok(Self::S_2),
            "3" => Ok(Self::S_3),
            _ => Err("Unsupported encoding indicator"),
        }
    }
}

/// From a byte string, process the first and subsequent bytes into a major type, an argument, and
/// a spec
///
/// Spec will be S_ iff the [`Option<u64>`] is none.
#[allow(clippy::type_complexity)]
// reason: All items make sense here, and it is an internal function used in situations when you
// would expect those very items.
fn process_cbor_major_argument(
    cbor: &[u8],
) -> Result<(Major, Option<u64>, Spec, &[u8]), CborError> {
    // It would be tempting to use minicbor or another CBOR implementation, but they don't
    // expose which option was chosen for argument, so we are on our own, because we need that
    // information for encoding indicators.
    let head = cbor
        .first()
        .ok_or(CborError("Expected item, out of data"))?;

    let (major, additional) = Major::from_byte(*head);
    let tail = &cbor[1..];

    let (argument, spec, skip): (Option<u64>, _, _) = match additional {
        0..=23 => (Some(additional.into()), Spec::S_i, 0),
        24 => (
            Some(
                tail.first()
                    .copied()
                    .ok_or(CborError("Missing 1 byte"))?
                    .into(),
            ),
            Spec::S_0,
            1,
        ),
        25 => (
            Some(
                u16::from_be_bytes(
                    tail.get(..2)
                        .ok_or(CborError("Missing 2 bytes"))?
                        .try_into()
                        .unwrap(),
                )
                .into(),
            ),
            Spec::S_1,
            2,
        ),
        26 => (
            Some(
                u32::from_be_bytes(
                    tail.get(..4)
                        .ok_or(CborError("Missing 4 bytes"))?
                        .try_into()
                        .unwrap(),
                )
                .into(),
            ),
            Spec::S_2,
            4,
        ),
        27 => (
            Some(u64::from_be_bytes(
                tail.get(..8)
                    .ok_or(CborError("Missing 8 bytes"))?
                    .try_into()
                    .unwrap(),
            )),
            Spec::S_3,
            8,
        ),
        31 => (None, Spec::S_, 0),
        _ => return Err(CborError("Reserved header byte")),
    };

    Ok((major, argument, spec, &tail[skip..]))
}

peg::parser! { grammar cbordiagnostic() for str {

// seq             = S [item *(MSC item) SOC]
    pub rule seq() -> Sequence<'input>
        = s0:S() items:(first:item() tail:(msc:MSC() inner:item() { (msc, inner) })* soc:SOC() { NonemptyMscVec::new_parsing(first, tail, soc) })? {
            Sequence { s0, items }
        }


// one-item        = S item S
    pub rule one_item() -> StandaloneItem<'input>
        = s1:S() i:item() s2:S() { StandaloneItem(s1, i, s2) }

// item            = map / array / tagged
//                 / number / simple
//                 / string / streamstring
    rule item() -> Item<'input>
        = inner:(map() / array() / tagged() /
          number() / simple() /
          string:string() { InnerItem::String(string) } / streamstring()) { inner.into() }

// string1         = (tstr / bstr) spec
    rule string1() -> String1e<'input>
        = value:$(tstr() / bstr()) spec:spec() {?
            Ok(if value.starts_with("<<") {
                // FIXME: How can we propagate the parsing we already did instead of parsing again
                // and having bad error handling?
                String1e::EmbeddedChunk(cbordiagnostic::seq(&value[2..value.len() - 2]).map_err(|_| "Parse error in embedded CBOR")?, spec)
            } else {
                String1e::TextChunk(Cow::Borrowed(value), spec)
            })
        }
// string1e        = string1 / ellipsis
    rule string1e() -> String1e<'input>
        = string1() / ellipsis()
// ellipsis        = 3*"." ; "..." or more dots
    rule ellipsis() -> String1e<'input>
        = dots:$("."*<3,>) { String1e::Ellipsis(dots.len()) }
// string          = string1e *(S "+" S string1e)
    rule string() -> CborString<'input>
        = head:string1e() tail:(separator:S() "+" s1:S() inner:string1e() { (separator, s1, inner) })* {
            CborString {
                items: core::iter::once(head).chain(tail.iter().map(|(_sep_pre, _sep_post, inner)| inner).cloned()).collect(),
                separators: tail.iter().map(|(sep_pre, sep_post, _inner)| (sep_pre.clone(), sep_post.clone())).collect()
            }
        }

// number          = (hexfloat / hexint / octint / binint
//                    / decnumber / nonfin) spec
    rule number() -> InnerItem<'input>
        = num:$((hexfloat() / hexint() / octint() / binint() / decnumber() / nonfin())) spec:spec() {InnerItem::Number(Number(Cow::Borrowed(num)), spec)}

// sign            = "+" / "-"
    rule sign() -> Sign
        = "+" { Sign::Plus } / "-" { Sign::Minus }

// decnumber       = [sign] (1*DIGIT ["." *DIGIT] / "." 1*DIGIT)
//                          ["e" [sign] 1*DIGIT]
    pub rule decnumber() -> NumberParts<'input>
        = sign:sign()? prepost:(predot:$(DIGIT()+) postdot:("." postdot:$(DIGIT()*) { postdot })? { (predot, postdot) } / "." postdot:$(DIGIT()+) { ("", Some(postdot)) })
                         exponent:(['e'|'E'] sign:sign()? exponent:$(DIGIT()+) {(sign, exponent)})?
        {
            let (predot, postdot) = prepost;
            NumberParts {
                base: 10,
                sign,
                predot,
                postdot,
                exponent,
            }
        }
// hexfloat        = [sign] "0x" (1*HEXDIG ["." *HEXDIG] / "." 1*HEXDIG)
//                          "p" [sign] 1*DIGIT
   pub rule hexfloat() -> NumberParts<'input>
       = sign:sign()?
       "0" ['x'|'X']
       prepost:(
           predot:$(HEXDIG()+) postdot:("." postdot:$(HEXDIG()*) { postdot })?
           { (Some(predot), postdot) }
           / "." postdot:$(HEXDIG()+)
           { (None, Some(postdot)) }
       )
       ['p'|'P']
       expsign:sign()?
       exp:$(DIGIT()+)
       {
           NumberParts {
               base: 16,
               sign,
               predot: prepost.0.unwrap_or(""),
               postdot: prepost.1,
               exponent: Some((expsign, exp))
           }
       }
// hexint          = [sign] "0x" 1*HEXDIG
   pub rule hexint() -> NumberParts<'input>
       = sign:sign()? "0" ['x'|'X'] predot:$(HEXDIG()+) { NumberParts {base: 16, sign, predot, postdot: None, exponent: None} }
// octint          = [sign] "0o" 1*ODIGIT
   pub rule octint() -> NumberParts<'input>
       = sign:sign()? "0" ['o'|'O'] predot:$(ODIGIT()+) { NumberParts {base: 8, sign, predot, postdot: None, exponent: None} }
// binint          = [sign] "0b" 1*BDIGIT
   pub rule binint() -> NumberParts<'input>
       = sign:sign()? "0" ['b'|'B'] predot:$(BDIGIT()+) { NumberParts {base: 2, sign, predot, postdot: None, exponent: None} }
// nonfin          = %s"Infinity"
//                 / %s"-Infinity"
//                 / %s"NaN"
    rule nonfin()
        = "Infinity" / "-Infinity" / "NaN"
// simple          = %s"false"
//                 / %s"true"
//                 / %s"null"
//                 / %s"undefined"
//                 / %s"simple(" S item S ")"
    rule simple() -> InnerItem<'input>
        = "false" { InnerItem::Simple(Simple::False) }
                / "true" { InnerItem::Simple(Simple::True) }
                / "null" { InnerItem::Simple(Simple::Null) }
                / "undefined" { InnerItem::Simple(Simple::Undefined) }
                / "simple(" s1:S() i:item() s2:S() ")" {InnerItem::Simple(Simple::Numeric(Box::new(StandaloneItem(s1, i, s2))))}
// uint            = "0" / DIGIT1 *DIGIT
    rule uint() -> u64
        = n:$("0" / DIGIT1() DIGIT()*) {? n.parse().or(Err("Exceeding tag space")) }
// tagged          = uint spec "(" S item S ")"
    rule tagged() -> InnerItem<'input>
        = tag:uint() tagspec:spec() "(" s0:S() value:item() s1:S() ")" { InnerItem::Tagged(tag, tagspec, Box::new(StandaloneItem(s0, value, s1))) }

// app-prefix      = lcalpha *lcalnum ; including h and b64
//                 / ucalpha *ucalnum ; tagged variant, if defined
    pub rule app_prefix() =
        quiet!{lcalpha() lcalnum()* / ucalpha() ucalnum()*} / expected!("application prefix")
// app-string      = app-prefix sqstr
    pub rule app_string() -> (&'input str, String)
        = prefix:$(app_prefix()) data:sqstr() { (prefix, data) }
// sqstr           = SQUOTE *single-quoted SQUOTE
    pub rule sqstr() -> String // Yes it is String: Just because they can contain binary doesn't mean
                               // that the ABNF allows it -- no '\xff'.
        = SQUOTE() sqstr:single_quoted()* SQUOTE() { sqstr.iter().filter_map(|c| *c).collect() }
// bstr            = app-string / sqstr / embedded
//                   ; app-string could be any type
    rule bstr()
        = app_string() / sqstr() / embedded()
// tstr            = DQUOTE *double-quoted DQUOTE
    pub rule tstr() -> String
        = DQUOTE() text:double_quoted()* DQUOTE() { text.iter().filter_map(|c| *c).collect() }

// embedded        = "<<" seq ">>"
    rule embedded()
        = "<<" seq() ">>"

// array           = "[" (specms S item *(MSC item) SOC / spec S) "]"
    rule array() -> InnerItem<'input>
        = "[" array:(
            spec:specms() s:S() first:item() tail:(msc:MSC() inner:item() { (msc, inner) })* soc:SOC()
            { SpecMscVec::Present { spec, s, items: NonemptyMscVec::new_parsing(first, tail, soc) } }
            / spec:spec() s:S()
            { SpecMscVec::Absent { spec, s } }
            ) "]"
        { InnerItem::Array(array) }
// map             = "{" (specms S keyp *(MSC keyp) SOC / spec S) "}"
    rule map() -> InnerItem<'input>
        = "{" map:(
            spec:specms() s:S() first:keyp() tail:(msc:MSC() inner:keyp() { (msc, inner) })* soc:SOC()
            { SpecMscVec::Present { spec, s, items: NonemptyMscVec::new_parsing(first, tail, soc) } }
            / spec:spec() s:S()
            { SpecMscVec::Absent { spec, s } }
            ) "}"
        { InnerItem::Map(map) }
// keyp            = item S ":" S item
    rule keyp() -> Kp<'input>
        = key:item() s0:S() ":" s1:S() value:item() { Kp { key, s0, s1, value } }

// ; We allow %x09 HT in prose, but not in strings
// blank           = %x09 / %x0A / %x0D / %x20
    rule blank() -> ()
        = quiet!{"\x09" / "\x0A" / "\x0D" / "\x20"} / expected!("tabs, spaces or newlines")

// non-slash       = blank / %x21-2e / %x30-D7FF / %xE000-10FFFF
    rule non_slash() -> ()
        = blank() / ['\x21'..='\x2e' | '\x30'..='\u{D7FF}' | '\u{E000}'..='\u{10FFFF}'] {}
// non-lf          = %x09 / %x0D / %x20-D7FF / %xE000-10FFFF
    rule non_lf() -> ()
        = ['\x09' | '\x0D' | '\x20'..='\u{D7FF}' | '\u{E000}'..='\u{10FFFF}'] {}

// comment         = "/" *non-slash "/"
//                 / "#" *non-lf %x0A
    rule comment() -> Comment
        = quiet!{"/" body:$(non_slash()*) "/" { Comment::Slashed } / "#" body:$(non_lf()*) "\x0A" { Comment::Hashed }} / expected!("comment")

// ; optional space
// S               = *blank *(comment *blank)
    // This rule is expressed twice because it is very common to need `s0:S()`, but for comment
    // reshaping we occasionally need the internals
    rule S() -> S<'input>
        = data:S_details() { S(Cow::Borrowed(data.data)) }
    pub(crate) rule S_details() -> SDetails<'input>
        = sliced:with_slice(<blank()* comments:(comment:comment() blank()* { comment })* { comments.last().cloned() }>) { SDetails { data: sliced.1, last_comment_style: sliced.0 } }
// ; mandatory space
// MS              = (blank/comment) S
    rule MS() -> MS<'input>
        = data:$( (blank() / comment() ) S()) { MS(Cow::Borrowed(data)) }
// ; mandatory comma and/or space
// MSC             = ("," S) / (MS ["," S])
    rule MSC() -> MSC<'input>
        = data:$( ("," S()) / (MS() ("," S())?) ) { MSC(Cow::Borrowed(data)) }

// ; optional comma and/or space
// SOC             = S ["," S]
    rule SOC() -> SOC<'input>
        = data:$( SOC_details() ) { SOC(Cow::Borrowed(data)) }
    pub(crate) rule SOC_details() -> (SDetails<'input>, Option<SDetails<'input>>)
        = before:S_details() after:("," after:S_details() { after })? { (before, after) }

// ; check semantically that strings are either all text or all bytes
// ; note that there must be at least one string to distinguish
// streamstring    = "(_" MS string *(MSC string) SOC ")"
    rule streamstring() -> InnerItem<'input>
        = "(_" ms:MS() first:string() tail:(msc:MSC() inner:string() { (msc, inner) })* soc:SOC() ")" {
            InnerItem::StreamString(ms, NonemptyMscVec::new_parsing(first, tail, soc))
        }

// spec            = ["_" *wordchar]
    rule spec() -> Option<Spec>
        = quiet!{("_" spec:$(wordchar()*) {? spec.parse() })? } / expected!(r#"a valid encoding indicator ("_", "_i", "_0", "_1", "_2" or "_3")"#)
// specms          = ["_" *wordchar MS]
    rule specms() -> Option<(Spec, MS<'input>)>
        = quiet!{("_" spec:$(wordchar()*) ms:MS() {? spec.parse().map(|spec| (spec, ms)) })? } / expected!(r#"a valid encoding indicator ("_", "_i", "_0", "_1", "_2" or "_3")"#)

// double-quoted   = unescaped
//                 / SQUOTE
//                 / "\" DQUOTE
//                 / "\" escapable
    rule double_quoted() -> Option<char>
        = unescaped() /
            SQUOTE() { Some('\'') } /
            "\\" DQUOTE() { Some('"') } /
            "\\" e:escapable() { Some(e) }

// single-quoted   = unescaped
//                 / DQUOTE
//                 / "\" SQUOTE
//                 / "\" escapable
    rule single_quoted() -> Option<char>
        = unescaped() / DQUOTE() { Some('"') } / "\\" SQUOTE() { Some('\'') } / "\\" e:escapable() { Some(e) }

// escapable       = %s"b" ; BS backspace U+0008
//                 / %s"f" ; FF form feed U+000C
//                 / %s"n" ; LF line feed U+000A
//                 / %s"r" ; CR carriage return U+000D
//                 / %s"t" ; HT horizontal tab U+0009
//                 / "/"   ; / slash (solidus) U+002F (JSON!)
//                 / "\"   ; \ backslash (reverse solidus) U+005C
//                 / (%s"u" hexchar) ;  uXXXX      U+XXXX
    rule escapable() -> char
        = "b" { '\x08' }
            / "f" { '\x0c' }
            / "n" { '\n' }
            / "r" { '\r' }
            / "t" { '\t' }
            / "/" { '/' }
            / "\\" { '\\' }
            / h:("u" h:hexchar() { h }) { h }

// hexchar         = "{" (1*"0" [ hexscalar ] / hexscalar) "}"
//                 / non-surrogate
//                 / (high-surrogate "\" %s"u" low-surrogate)
    rule hexchar() -> char
        =
            "{" hex:$("0"+ hexscalar()? / hexscalar()) "}"
            {
                char::try_from(
                    u32::from_str_radix(hex, 16)
                        .expect("Syntax ensures this works")
                    )
                    .expect("Syntax rules out surrogate sequences and numbers beyond Unicode specification")
            }
            / hex:$(non_surrogate())
            {
                char::try_from(
                    u32::from(
                        u16::from_str_radix(hex, 16)
                            .expect("Syntax ensures this works")
                        )
                    )
                    .expect("Syntax rules out surrogate sequences and numbers beyond Unicode specification")
            }
            / hl:(h:$(high_surrogate()) "\\" "u" l:$(low_surrogate()) { format!("{h}{l}") /* conveniently, syntax ensures it's always 4 nibbles */ } )
            {
                encoding_rs::UTF_16BE.decode(
                    &u32::from_str_radix(&hl, 16)
                        .expect("Syntax ensures this works")
                        .to_be_bytes()
                        // now it is UTF-16
                    )
                    .0
                    .chars()
                    .next()
                    .expect("Syntax ensures this produces exactly one valid character")
            }
// non-surrogate   = ((DIGIT / "A"/"B"/"C" / "E"/"F") 3HEXDIG)
//                 / ("D" ODIGIT 2HEXDIG )
    rule non_surrogate()
        = ((DIGIT() / "A"/"B"/"C" / "E"/"F" / "a"/"b"/"c" / "e"/"f") HEXDIG()*<3,3>)
                / (("D" / "d") ODIGIT() HEXDIG()*<2,2> )
// high-surrogate  = "D" ("8"/"9"/"A"/"B") 2HEXDIG
    rule high_surrogate()
        = ("D" / "d") ("8"/"9"/"A"/"B"/"a"/"b") HEXDIG()*<2,2>
// low-surrogate   = "D" ("C"/"D"/"E"/"F") 2HEXDIG
    rule low_surrogate()
        = ("D" / "d") ("C"/"D"/"E"/"F" / "c"/"d"/"e"/"f") HEXDIG()*<2,2>
// hexscalar       = "10" 4HEXDIG / HEXDIG1 4HEXDIG
//                 / non-surrogate / 1*3HEXDIG
    rule hexscalar()
        = "10" HEXDIG()*<4,4> / HEXDIG1() HEXDIG()*<4,4> / non_surrogate() / HEXDIG()*<1,3>

// ; Note that no other C0 characters are allowed, including %x09 HT
// unescaped       = %x0A ; new line
//                 / %x0D ; carriage return -- ignored on input
//                 / %x20-21
//                      ; omit 0x22 "
//                 / %x23-26
//                      ; omit 0x27 '
//                 / %x28-5B
//                      ; omit 0x5C \
//                 / %x5D-D7FF ; skip surrogate code points
//                 / %xE000-10FFFF
    // Returning an option to express that the carriage return is ignored
    rule unescaped() -> Option<char> = "\r" { None } / good:[ '\x0a' | '\x0D' | '\x20'..='\x21' | '\x23'..='\x26' | '\x28'..='\x5b' | '\x5d'..='\u{d7ff}' | '\u{e000}'..='\u{10ffff}' ] { Some(good) }

// DQUOTE          = %x22    ; " double quote
    rule DQUOTE() = "\""
// SQUOTE          = "'"     ; ' single quote
    rule SQUOTE() = "'"

// DIGIT           = %x30-39 ; 0-9
// DIGIT1          = %x31-39 ; 1-9
// ODIGIT          = %x30-37 ; 0-7
// BDIGIT          = %x30-31 ; 0-1
// HEXDIG          = DIGIT / "A" / "B" / "C" / "D" / "E" / "F"
// HEXDIG1         = DIGIT1 / "A" / "B" / "C" / "D" / "E" / "F"
    rule DIGIT() = quiet!{['0'..='9']} / expected!("digits")
    rule DIGIT1() = quiet!{['1'..='9']} / expected!("digits excluding 0")
    rule ODIGIT() = ['0'..='7']
    rule BDIGIT() = ['0'..='1']
    rule HEXDIG() -> u8 = n:$(DIGIT() / ['A'..='F' | 'a'..='f']) { u8::from_str_radix(n, 16).expect("Syntax ensures this is OK") }
    rule HEXDIG1() = DIGIT1() / ['A'..='F' | 'a'..='f']

// ; Note: double-quoted strings as in "A" are case-insensitive in ABNF
// lcalpha         = %x61-7A ; a-z
// lcalnum         = lcalpha / DIGIT
// ucalpha         = %x41-5A ; A-Z
// ucalnum         = ucalpha / DIGIT
// wordchar        = "_" / lcalnum / ucalpha ; [_a-z0-9A-Z]
    rule lcalpha() = ['a'..='z']
    rule lcalnum() = ['a'..='z'] / DIGIT()
    rule ucalpha() = ['A'..='Z']
    rule ucalnum() = ['A'..='Z'] / DIGIT()
    rule wordchar() = "_" / lcalnum() / ucalpha()

// Not starting a new grammar for these: their names are unique enough, and they reuse many of the
// other definitions

// app-string-h    = S *(HEXDIG S HEXDIG S / ellipsis S)
//                   ["#" *non-lf]
    pub rule app_string_h() -> Vec<u8> = S() byte:(high:HEXDIG() S() low:HEXDIG() S() { (high << 4) | low } / ellipsis() S() {? Err("Hex string was abbreviated") })*
        ("#" non_lf()*)?
        { byte }

    /// Return both the value and slice matched by the rule.
    ///
    /// This is the canonical workaround to get both a slice and a value, as discussed in
    /// <https://github.com/kevinmehall/rust-peg/issues/377#issuecomment-2158664327>
    rule with_slice<T>(r: rule<T>) -> (T, &'input str)
        = value:&r() input:$(r()) { (value, input) }
}}
