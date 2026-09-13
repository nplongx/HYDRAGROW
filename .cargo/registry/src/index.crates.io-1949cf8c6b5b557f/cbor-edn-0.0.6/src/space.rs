use std::borrow::Cow;

use super::{cbordiagnostic, DelimiterPolicy, InconsistentEdn, Unparse};

/// Parsed comment (used during refromatting inside S)
// If we ever needed access to the comments and not just their types, this would grow a lifetime
// and some &'a str or Cow<'a, str> in the variants.
#[derive(Copy, Clone)]
pub(super) enum Comment {
    Slashed,
    Hashed,
}

pub(super) struct SDetails<'input> {
    pub(super) data: &'input str,
    pub(super) last_comment_style: Option<Comment>,
}

/// Optional space (including comments)
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct S<'a>(pub(crate) Cow<'a, str>);

impl Default for S<'_> {
    fn default() -> Self {
        S(Cow::Borrowed(""))
    }
}

/// Mandatory space (including comments)
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct MS<'a>(pub(crate) Cow<'a, str>);

impl Default for MS<'_> {
    fn default() -> Self {
        MS(Cow::Borrowed(" "))
    }
}

/// Mandatory space (including comments), possibly including a single comma
#[derive(Debug, Clone, PartialEq)]
#[allow(clippy::upper_case_acronyms)] // with feature(lint_reasons): reason = "Space type names here match ABNF rules"
pub(crate) struct MSC<'a>(pub(crate) Cow<'a, str>);

impl Default for MSC<'_> {
    fn default() -> Self {
        MSC(Cow::Borrowed(","))
    }
}

/// Optional space (including comments), possibly including a single comma
#[derive(Debug, Clone, PartialEq)]
#[allow(clippy::upper_case_acronyms)] // with feature(lint_reasons): reason = "Space type names here match ABNF rules"
pub(crate) struct SOC<'a>(pub(crate) Cow<'a, str>);

impl Default for SOC<'_> {
    fn default() -> Self {
        SOC(Cow::Borrowed(""))
    }
}

// Right now this contains *just* those that we need trait'ified for the
// visitor::ProcessResult::to_space_ functions, but maybe we should use more?
pub(crate) trait Spaceish {
    fn prepend_comment(&mut self, comment: &str);
}

macro_rules! spaceish {
    ($t:ident, $may_have_comma:expr, $is_ms:expr, $is_soc:expr) => {
        impl Unparse for $t<'_> {
            fn serialize_write(&self, formatter: &mut core::fmt::Formatter) -> core::fmt::Result {
                formatter.write_str(&self.0)
            }

            fn to_cbor(&self) -> Result<impl Iterator<Item = u8>, InconsistentEdn> {
                Ok(core::iter::empty())
            }
        }
        impl<'a> $t<'a> {
            pub(crate) fn set_delimiters(
                &mut self,
                policy: DelimiterPolicy,
                potential_start_of_line: bool,
            ) {
                use DelimiterPolicy::*;
                match policy {
                    DiscardAll => {
                        *self = Default::default();
                    }
                    DiscardAllButComments
                    | SingleLineRegularSpacing
                    | IndentedRegularSpacing { .. } => {
                        /// Remove blank around the comment … which is not exactly trivial b/c we can't
                        /// remove the \n after a # if no later // comment is there.
                        fn trim_one(parsed: SDetails<'_>) -> &str {
                            if let Some(Comment::Hashed) = parsed.last_comment_style {
                                // Trimming carefully because we want to preserve a trailing \n. Not just
                                // trimming everything and adding back a \n because that can't be done on a
                                // slice. The indexing ensures we also strip down `# foo \n  \n` down to
                                // the first \n.
                                let b = parsed.data.trim_start_matches([' ', '\n', '\t', '\x09']);
                                let last_hash_index =
                                    b.rfind('#').expect("Parsing ensured this is present");
                                let from_last_hash = &b[last_hash_index..];
                                let first_newline = last_hash_index
                                    + from_last_hash
                                        .find('\n')
                                        .expect("parsing ensured this is present");
                                &b[..first_newline + 1]
                            } else {
                                parsed.data.trim_matches([' ', '\n', '\t', '\x09'])
                            }
                        }
                        // We could take some measures to preserve self.0 as a Cow, especially if
                        // there is no comma in it -- trim_one is prepared for some match self.0 /
                        // Owned => Owned(.trim().to_string(), Borrowed => Borrowed(.trim)
                        // shenanigans, but updating to -14 this became too hard to maintain for
                        // just the benefit of some fewer allocations.
                        let input = &self.0;
                        let (before, after) = cbordiagnostic::SOC_details(input.as_ref())
                            .expect("Comments are always well-formed");
                        let mut owned = String::new();

                        let add_space = |owned: &mut String| match policy {
                            DiscardAllButComments => (),
                            IndentedRegularSpacing { base_indent, .. } => {
                                if potential_start_of_line {
                                    owned.push_str("\n");
                                    for _ in 0..base_indent {
                                        owned.push_str(" ");
                                    }
                                }
                            }
                            _ => {
                                if $may_have_comma && !$is_soc {
                                    owned.push_str(" ");
                                }
                            }
                        };

                        // ie. if is MSC, because commas can only be in MSC and SOC
                        if $may_have_comma && !$is_soc {
                            owned.push_str(trim_one(before));
                            // We always set the comma in these policies, even if None is in after.
                            owned.push_str(",");
                            add_space(&mut owned);
                        } else {
                            add_space(&mut owned);
                            owned.push_str(trim_one(before));
                        }

                        if let Some(after) = after {
                            owned.push_str(trim_one(after));
                        }
                        self.0 = Cow::Owned(owned);

                        if $is_ms && self.0 == "" {
                            *self = Default::default()
                        }
                    }
                    SingleSpace => {
                        self.set_delimiters(DiscardAllButComments, false);
                        self.prefix(Cow::Borrowed(" "));
                    }
                }
            }

            /// Prepend the prefix string to the existing content.
            ///
            /// The prefix string must match the `S` rule.
            // FIXME: Should we check that with type state?
            pub(crate) fn prefix<'longer: 'a>(&mut self, prefix: Cow<'longer, str>) {
                if prefix == "" {
                    return;
                }
                if self.0 == "" {
                    self.0 = prefix
                } else {
                    self.0 = [prefix.as_ref(), self.0.as_ref()].concat().into();
                }
            }
        }

        impl Spaceish for $t<'_> {
            fn prepend_comment(&mut self, comment: &str) {
                self.0 = if comment.contains('/') {
                    format!("# {}\n{}", comment.replace('\n', "\n# "), self.0)
                } else {
                    format!("/ {} /{}", comment, self.0)
                }
                .into();
            }
        }
    };
}

spaceish!(S, false, false, false);
spaceish!(MS, false, true, false);
spaceish!(MSC, true, false, false);
spaceish!(SOC, true, false, true);
