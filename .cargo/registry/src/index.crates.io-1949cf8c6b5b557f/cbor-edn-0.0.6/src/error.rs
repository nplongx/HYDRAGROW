//! Error types

/// Error type for conversions from EDN to CBOR
///
/// The conversion is generally fallible; see [crate level documentation](super).
#[derive(Debug, Clone)]
pub struct InconsistentEdn(pub(super) &'static str);

impl From<core::num::TryFromIntError> for InconsistentEdn {
    fn from(_: core::num::TryFromIntError) -> Self {
        InconsistentEdn("Numeric range exceeded")
    }
}

impl core::fmt::Display for InconsistentEdn {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> Result<(), core::fmt::Error> {
        f.write_str(self.0)
    }
}

impl std::error::Error for InconsistentEdn {}

/// Error type for parsing
///
/// This is not a re-export from `peg` to avoid a versioned dependency; it provides location in the
/// EDN data, and a Display implementation.
#[derive(Debug, Clone)]
pub struct ParseError(pub(super) peg::error::ParseError<peg::str::LineCol>);

impl ParseError {
    /// Line number of the error (starting with 1)
    pub fn line(&self) -> usize {
        self.0.location.line
    }
    /// Column of the error inside the line given by [`.line()`](Self::line) (starting with 1)
    pub fn column(&self) -> usize {
        self.0.location.column
    }
    /// Byte at which the error occurred inside the parsed text (starting with 0)
    pub fn offset(&self) -> usize {
        self.0.location.offset
    }
}

impl core::fmt::Display for ParseError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> Result<(), core::fmt::Error> {
        writeln!(
            f,
            "Invalid EDN in line {} column {} (byte {}). Expected any of:",
            self.line(),
            self.column(),
            self.offset()
        )?;
        for t in self.0.expected.tokens() {
            writeln!(f, "* {}", t)?;
        }
        Ok(())
    }
}

impl std::error::Error for ParseError {}

/// Error of item accessors used when the queried type does not match the item
#[derive(Debug, Clone)]
pub struct TypeMismatch {
    expected: &'static str,
}

impl TypeMismatch {
    pub(crate) fn expecting(expected: &'static str) -> Self {
        TypeMismatch { expected }
    }
}

impl core::fmt::Display for TypeMismatch {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> Result<(), core::fmt::Error> {
        write!(f, "Expected {}", self.expected)
    }
}

impl std::error::Error for TypeMismatch {}

/// Error converting from CBOR
#[derive(Debug, Clone)]
pub struct CborError(pub(crate) &'static str);

impl core::fmt::Display for CborError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> Result<(), core::fmt::Error> {
        f.write_str(self.0)
    }
}

impl std::error::Error for CborError {}
