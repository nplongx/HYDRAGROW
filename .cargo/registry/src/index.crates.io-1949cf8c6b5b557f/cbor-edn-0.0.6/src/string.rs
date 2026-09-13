use std::borrow::Cow;

use super::{
    cbordiagnostic, space::S, DelimiterPolicy, InconsistentEdn, Major, Sequence, Spec, Unparse,
};

pub(super) const ENGINE_B32IGNORECASE: data_encoding::Encoding = data_encoding_macro::new_encoding! {
    symbols: "ABCDEFGHIJKLMNOPQRSTUVWXYZ234567",
    // Be case insensitive
    translate_from: "abcdefghijklmnopqrstuvwxyz",
    translate_to: "ABCDEFGHIJKLMNOPQRSTUVWXYZ",
    // Rather than make it padding, we ignore it, so we tolerate input with or without padding
    // (as well as input with = signs in arbitrary locations, which is not great but tolerable
    // for this rather exotic set)
    ignore: "=",
};
pub(super) const ENGINE_H32IGNORECASE: data_encoding::Encoding = data_encoding_macro::new_encoding! {
    symbols: "0123456789ABCDEFGHIJKLMNOPQRSTUV",
    translate_from: "abcdefghijklmnopqrstuv",
    translate_to: "ABCDEFGHIJKLMNOPQRSTUV",
    ignore: "=",
};
pub(super) const ENGINE_B64: data_encoding::Encoding = data_encoding_macro::new_encoding! {
    symbols: "ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/",
    // This is case sensitive by necessity, but we can use the translation feature to accept base64
    // and base64url in a single go
    translate_from: "-_",
    translate_to: "+/",
    ignore: "=",
};

/// A CBOR string
///
/// This mostly contains one item; if it contains more, they are semantically concatenated in EDN,
/// and indistinguishable from a single string in CBOR.
///
/// This is untyped w/rt binary vs. text strings; it's the type of the first element that guides
/// the type.
#[derive(Clone, Debug, PartialEq)]
pub(super) struct CborString<'a> {
    /// Actual string elents; at least 1
    pub(super) items: Vec<String1e<'a>>,
    /// Separators between the items; always length of items minus 1
    pub(super) separators: Vec<(S<'a>, S<'a>)>,
}

impl CborString<'_> {
    pub(super) fn new_bytes_hex_with_spec(value: &[u8], spec: Option<Spec>) -> Self {
        CborString {
            items: vec![String1e::TextChunk(
                format!("h'{}'", hex::encode(value)).into(),
                spec,
            )],
            separators: vec![],
        }
    }

    pub(super) fn new_text_with_spec(value: &str, spec: Option<Spec>) -> Self {
        use core::fmt::Write;
        let mut escaped = String::new();
        escaped.push('"');
        for c in value.chars() {
            match c {
                // This is essentially `unescaped`, but without the gap for surrogate pairs because
                // they don't exist in char
                '\x0a' | '\x20'..='\x21' | '\x23'..='\x26' | '\x28'..='\x5b' | '\x5d'.. => {
                    escaped.push(c)
                }
                '\r' => escaped.push_str("\\r"),
                '\t' => escaped.push_str("\\t"),
                '\x08' => escaped.push_str("\\b"),
                '\x0c' => escaped.push_str("\\f"),
                '\\' => escaped.push_str("\\\\"),
                '"' => escaped.push_str("\\\""),
                _ => write!(escaped, "\\u{{{:x}}}", u32::from(c))
                    .expect("Writing to a str is infallible"),
            }
        }
        escaped.push('"');
        CborString {
            items: vec![String1e::TextChunk(escaped.into(), spec)],
            separators: vec![],
        }
    }

    /// Create a new single quoted or application literal.
    ///
    /// Note that it is up to the caller to verify that the identifier is valid, or whether the
    /// spec is meaningful.
    pub(super) fn new_application_literal(
        identifier: &str,
        value: &str,
        spec: Option<Spec>,
    ) -> Self {
        // This is very similar to new_text_with_spec, but differs in escaping details
        use core::fmt::Write;
        let mut escaped = String::new();
        escaped.push_str(identifier);
        escaped.push('\'');
        for c in value.chars() {
            match c {
                // This is essentially `unescaped`, but without the gap for surrogate pairs because
                // they don't exist in char
                '\x0a' | '\x20'..='\x21' | '\x23'..='\x26' | '\x28'..='\x5b' | '\x5d'.. => {
                    escaped.push(c)
                }
                '\r' => escaped.push_str("\\r"),
                '\t' => escaped.push_str("\\t"),
                '\x08' => escaped.push_str("\\b"),
                '\x0c' => escaped.push_str("\\f"),
                '\\' => escaped.push_str("\\\\"),
                '\'' => escaped.push_str("\\'"),
                _ => write!(escaped, "\\u{{{:x}}}", u32::from(c))
                    .expect("Writing to a str is infallible"),
            }
        }
        escaped.push('\'');
        CborString {
            items: vec![String1e::TextChunk(escaped.into(), spec)],
            separators: vec![],
        }
    }

    pub(super) fn discard_encoding_indicators(&mut self) {
        for item in &mut self.items {
            item.discard_encoding_indicators();
        }
    }

    pub(super) fn set_delimiters(&mut self, policy: DelimiterPolicy) {
        for i in &mut self.items {
            i.set_delimiters(policy);
        }
        for (s_pre, s_post) in &mut self.separators {
            s_pre.set_delimiters(policy, true);
            s_post.set_delimiters(policy, false);
        }
    }

    pub(super) fn encoded_major_type(&self) -> Result<Major, InconsistentEdn> {
        self.items[0].encoded_major_type()
    }
}

impl Unparse for CborString<'_> {
    fn serialize_write(&self, formatter: &mut core::fmt::Formatter) -> core::fmt::Result {
        assert!(self.items.len() == self.separators.len() + 1);
        let mut separators = self.separators.iter();
        for item in &self.items {
            item.serialize_write(formatter)?;
            if let Some((sep_pre, sep_post)) = separators.next() {
                sep_pre.serialize_write(formatter)?;
                formatter.write_str("+")?;
                sep_post.serialize_write(formatter)?;
            }
        }
        Ok(())
    }

    fn to_cbor(&self) -> Result<impl Iterator<Item = u8>, InconsistentEdn> {
        let major = self.encoded_major_type()?;
        match major {
            Major::ByteString | Major::TextString => {
                let single_spec = if let &[String1e::TextChunk(_, spec)
                | String1e::EmbeddedChunk(_, spec)] = &self.items.as_slice()
                {
                    *spec
                } else {
                    if self.items.iter().any(|i| match i {
                        String1e::Ellipsis(_) => false,
                        String1e::TextChunk(_, None) => false,
                        String1e::TextChunk(_, Some(_)) => true,
                        String1e::EmbeddedChunk(_, None) => false,
                        String1e::EmbeddedChunk(_, Some(_)) => true,
                    }) {
                        return Err(InconsistentEdn("Encoding indicators present on string expressed in multiple EDN chunks"));
                    }
                    None
                };
                let data = self
                    .items
                    .iter()
                    .map(|i| i.bytes_value())
                    .collect::<Result<Vec<_>, _>>()?;
                let mut data: Vec<u8> = data.into_iter().flat_map(|i| i.into_iter()).collect();
                if major == Major::TextString {
                    let _ = core::str::from_utf8(&data).map_err(|_| {
                        InconsistentEdn("Concatenated text string is not valid UTF-8")
                    })?;
                };

                if matches!(single_spec, Some(Spec::S_)) {
                    if !data.is_empty() {
                        return Err(InconsistentEdn(
                            "Indefinite-length encoding used with single non-empty string item.",
                        ));
                    }
                    // an early return with anything different would cause type trouble, but we
                    // know the string is empty so let's just roll with it
                    data.push(0xff);
                }

                Ok(
                    Spec::encode_argument(single_spec.as_ref(),
                        major,
                        data
                            .len()
                            .try_into()
                            .expect("Even on 128bit architectures we can't have more than 64bit long counts of items")
                        )?
                        .into_iter()
                        .chain(data)
                )
            }
            _ => {
                // We could distringuish between "is it a single item or multiple" and err
                // differently in the latter case, but in the end, both need to be preprocessed.
                Err(InconsistentEdn(
                    "Non-string EDN items need preprocessing to encode in CBOR",
                ))
            }
        }
    }
}

#[derive(Debug)]
pub(super) enum PreprocessedStringComponent {
    /// `...`
    Ellipsis,
    /// `"text"` -> "text"
    TStr(String),
    /// `'bytes'` -> "bytes"
    SQStr(String),
    /// `h'001122'` -> "h", "001122"
    AppString(String, String),
    /// `<< 1, 2, 3 >>` -> [0x01, 0x02, 0x03]
    Embedded(Vec<u8>),
}

/// A component of a chained string
///
/// Note that having a Spec is meaningless in some contexts (eg. when there is not exactly one
/// text chunk in a String)
#[derive(Clone, Debug, PartialEq)]
pub(super) enum String1e<'a> {
    Ellipsis(usize),
    TextChunk(Cow<'a, str>, Option<Spec>),
    EmbeddedChunk(Sequence<'a>, Option<Spec>),
}

impl String1e<'_> {
    pub(super) fn discard_encoding_indicators(&mut self) {
        match self {
            String1e::Ellipsis(_) => (),
            String1e::TextChunk(_value, spec) => {
                *spec = None;
            }
            String1e::EmbeddedChunk(_value, spec) => {
                // Explicitly *not* affecting the value, as CBOR encoded into a byte string is
                // usually just kept in that style in order to avoid that kind of tampering. There
                // could be a variant of discarding that reaches into it, but that should know
                // where to reach in (possibly in some kind of visitor or CBORPath direction)
                *spec = None;
            }
        }
    }

    pub(super) fn set_delimiters(&mut self, _: DelimiterPolicy) {
        // We don't need to do anything right now, but may want to do more once we look deeper into
        // the structures, eg. inside hex strings
    }

    pub(super) fn preprocess(&self) -> Result<PreprocessedStringComponent, InconsistentEdn> {
        Ok(match self {
            String1e::Ellipsis(_) => PreprocessedStringComponent::Ellipsis,
            String1e::TextChunk(value, _) => match &value[0..1] {
                "\"" => PreprocessedStringComponent::TStr(
                    cbordiagnostic::tstr(value).expect("Text was parsed to match string1"),
                ),
                "'" => PreprocessedStringComponent::SQStr(
                    cbordiagnostic::sqstr(value).expect("Text was parsed to match string1"),
                ),
                _ => {
                    let (app_str, sqstr) = cbordiagnostic::app_string(value)
                        .expect("Text was parsed to match string1");
                    PreprocessedStringComponent::AppString(app_str.to_owned(), sqstr)
                }
            },
            String1e::EmbeddedChunk(value, _) => {
                PreprocessedStringComponent::Embedded(value.to_cbor()?)
            }
        })
    }

    pub(super) fn encoded_major_type(&self) -> Result<Major, InconsistentEdn> {
        match &self.preprocess()? {
            PreprocessedStringComponent::Ellipsis => Err(InconsistentEdn(
                "Attempted to serialize EDN with unknown application oriented literal or ellipsisi present",
            )),
            PreprocessedStringComponent::TStr(_) => Ok(Major::TextString),
            PreprocessedStringComponent::SQStr(_) => Ok(Major::ByteString),
            PreprocessedStringComponent::AppString(t, _) if matches!(t.as_str(), "h" | "b32" | "h32" | "b64") => Ok(Major::ByteString),
            PreprocessedStringComponent::Embedded(_) => Ok(Major::ByteString),
            _ => Err(InconsistentEdn("Unsupported application oriented literal")),
        }
    }

    pub(super) fn bytes_value(&self) -> Result<Vec<u8>, InconsistentEdn> {
        Ok(match self.preprocess()? {
            PreprocessedStringComponent::TStr(s) => s.into(),
            PreprocessedStringComponent::SQStr(s) => s.into(),
            PreprocessedStringComponent::AppString(a, s) if a == "h" => {
                cbordiagnostic::app_string_h(&s)
                    // FIXME: More beautiful error propagation
                    .map_err(|_| InconsistentEdn("Ellipsis or other error in hex string"))?
            }
            PreprocessedStringComponent::AppString(a, s) if a == "b64" => ENGINE_B64
                .decode(s.as_bytes())
                .map_err(|_| InconsistentEdn("b64 input is neither base64 nor base64url"))?,
            PreprocessedStringComponent::AppString(a, s) if a == "b32" => ENGINE_B32IGNORECASE
                .decode(s.as_bytes())
                .map_err(|_| InconsistentEdn("b32 input is not base32"))?,
            PreprocessedStringComponent::AppString(a, s) if a == "h32" => ENGINE_H32IGNORECASE
                .decode(s.as_bytes())
                .map_err(|_| InconsistentEdn("h32 input is not base32hex"))?,
            PreprocessedStringComponent::Embedded(data) => data,
            _ => {
                return Err(InconsistentEdn(
                    "Unknown application oriented literal style",
                ))
            }
        })
    }

    fn serialize_write(&self, formatter: &mut core::fmt::Formatter) -> core::fmt::Result {
        match self {
            String1e::Ellipsis(n) => {
                for _ in 0..*n {
                    formatter.write_str(".")?;
                }
                Ok(())
            }
            String1e::TextChunk(s, spec) => {
                formatter.write_str(s)?;
                if let Some(spec) = spec {
                    spec.serialize_write(formatter)?;
                }
                Ok(())
            }
            String1e::EmbeddedChunk(item, spec) => {
                formatter.write_str("<<")?;
                item.serialize_write(formatter)?;
                formatter.write_str(">>")?;
                if let Some(spec) = spec {
                    spec.serialize_write(formatter)?;
                }
                Ok(())
            }
        }
    }
}
