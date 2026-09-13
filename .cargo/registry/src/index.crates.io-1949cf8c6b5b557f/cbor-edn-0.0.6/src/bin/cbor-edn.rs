use clap::{Args, Parser, Subcommand, ValueEnum};
use eyre::Context;

const HEXLOWER_WRAPPED: data_encoding::Encoding = data_encoding_macro::new_encoding! {
    symbols: "0123456789abcdef",
    // Tolerate upper-case input
    translate_from: "ABCDEF",
    translate_to: "abcdef",
    // Not pretty, but better than nothing
    wrap_width: 64,
    wrap_separator: "\n",
};

#[derive(Parser)]
#[command(version, about, long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

/// Subcommands of the cbor-edn command
///
/// CommonArgs are carried inside all variants rather than in the Cli object because otherwise Clap
/// would rearrange them to be positional before the subcommand.
#[derive(Subcommand)]
// Not setting rename_all = "lower" because that would also affect inner attributes; naming them
// individually instead
enum Commands {
    /// Convert diagnostic notation (EDN) to CBOR.
    ///
    /// By default, this converts all known application oriented literals to their respective CBOR
    /// representations, and respects encoding indicators.
    #[command(name = "diag2cbor")]
    Diag2Cbor {
        #[command(flatten)]
        from_diag: FromDiagArgs,
        #[command(flatten)]
        to_cbor: ToCborArgs,
        #[command(flatten)]
        common: CommonArgs,
    },
    /// Convert CBOR to diagnostic notation (EDN).
    ///
    /// By default, this (FIXME: does not yet) applies heuristics to pick sensible representations
    /// of values, applies application oriented literals to known tags, and some indentation.
    #[command(name = "cbor2diag")]
    Cbor2Diag {
        #[command(flatten)]
        from_cbor: FromCborArgs,
        #[command(flatten)]
        to_diag: ToDiagArgs,
        #[command(flatten)]
        common: CommonArgs,
    },
    /// Convert diagnostic notation (EDN) to diagnostic notation.
    ///
    /// By default, this applies some indenting, but leaves the EDN otherwise unmodified. No checks
    /// are performed as to whether CBOR encoding would be possible (as that can only be decided
    /// when transforming application oriented literals).
    #[command(name = "diag2diag")]
    Diag2Diag {
        #[command(flatten)]
        from_diag: FromDiagArgs,
        #[command(flatten)]
        to_diag: ToDiagArgs,
        #[command(flatten)]
        common: CommonArgs,

        /// Preserve comments and indentation (unless another option modifies them)
        #[arg(long)]
        preserve_space: bool,
        /// Remove all comments and spaces from the input data.
        #[arg(long)]
        remove_comments: bool,
    },
}

impl Commands {
    fn common(&mut self) -> &mut CommonArgs {
        use Commands::*;
        match self {
            Diag2Cbor { common, .. } | Cbor2Diag { common, .. } | Diag2Diag { common, .. } => {
                common
            }
        }
    }

    fn args_from_cbor(&self) -> Option<&FromCborArgs> {
        use Commands::*;
        match self {
            Cbor2Diag { ref from_cbor, .. } => Some(from_cbor),
            _ => None,
        }
    }

    fn args_from_diag(&self) -> Option<&FromDiagArgs> {
        use Commands::*;
        match self {
            Diag2Cbor { ref from_diag, .. } | Diag2Diag { ref from_diag, .. } => Some(from_diag),
            _ => None,
        }
    }

    fn args_to_diag(&self) -> Option<&ToDiagArgs> {
        use Commands::*;
        match self {
            Cbor2Diag { ref to_diag, .. } | Diag2Diag { ref to_diag, .. } => Some(to_diag),
            _ => None,
        }
    }

    fn load<'a>(&self, data: &'a [u8]) -> eyre::Result<cbor_edn::Sequence<'a>> {
        use Commands::*;
        Ok(match self {
            Diag2Cbor { .. } | Diag2Diag { .. } => {
                let data = &std::str::from_utf8(data).context("Error processing input file")?;
                cbor_edn::Sequence::parse(data).context("Error parsing input data")?
            }
            Cbor2Diag {
                from_cbor, common, ..
            } => {
                let input_format = from_cbor.input_format.unwrap_or_default();
                let hexdecoded;
                let data = if input_format == BinaryFormat::Hex
                    || (input_format == BinaryFormat::Auto && common.input.is_tty())
                {
                    hexdecoded = HEXLOWER_WRAPPED.decode(data)?;
                    &hexdecoded
                } else {
                    data
                };
                cbor_edn::Sequence::from_cbor(data)?
            }
        })
    }

    fn serialize(&self, data: cbor_edn::Sequence<'_>) -> eyre::Result<Vec<u8>> {
        use Commands::*;
        Ok(match self {
            Diag2Cbor {
                common, to_cbor, ..
            } => {
                // FIXME: If we could inspec tht error to see that there are left-over
                // application-oriented literals, we could recommend doing a 2diag
                // --aol-to-item.
                let output = data.to_cbor()?;
                let output_format = to_cbor.output_format.unwrap_or_default();
                if output_format == BinaryFormat::Hex
                    || (output_format == BinaryFormat::Auto && common.output.is_tty())
                {
                    HEXLOWER_WRAPPED.encode(&output).into()
                } else {
                    output
                }
            }
            Cbor2Diag { .. } | Diag2Diag { .. } => data.serialize().into_bytes(),
        })
    }

    fn transform(&self, data: &mut cbor_edn::Sequence<'_>) -> eyre::Result<()> {
        if self.args_from_diag().is_some_and(|fd| fd.annotate_tags)
            || self.args_from_cbor().is_some_and(|fc| !fc.no_annotate_tags)
        {
            data.visit_tag(&mut cbor_edn::application::ip_tag_to_aol);
            data.visit_tag(&mut cbor_edn::application::dt_tag_to_aol);
            data.visit_tag(&mut cbor_edn::application::comment_lang_tag);
        }
        if matches!(
            self,
            Commands::Diag2Diag {
                remove_comments: true,
                ..
            }
        ) {
            data.set_delimiters(cbor_edn::DelimiterPolicy::DiscardAll);
        }
        if self.args_to_diag().is_none() || self.args_to_diag().is_some_and(|td| td.aol_to_item) {
            data.visit_application_literals(&mut cbor_edn::application::ip_aol_to_item);
            data.visit_application_literals(&mut cbor_edn::application::dt_aol_to_item);
        }
        if let Some(annotator) = self.args_to_diag().and_then(|td| td.annotate) {
            match annotator {
                KnownAnnotation::Ccs => {
                    for item in data.get_items_mut() {
                        // FIXME: How do we best propagate a "that's not a map" error?
                        // For what it's worth, do we even expect that there is more than one or no
                        // CCS item, and if so, what does that mean for errors?
                        let _ = item.visit_map_elements(&mut cbor_edn::application::comment_ccs);
                    }
                }
            }
        }
        if self
            .args_to_diag()
            .is_some_and(|td| !td.no_bignum_from_tags)
        {
            data.visit_tag(&mut cbor_edn::application::tag23_to_edn_integer);
        }
        if self.args_from_diag().is_some_and(|fd| fd.apply_999) {
            data.visit_application_literals(&mut cbor_edn::application::any_aol_to_tag999);
        }
        if !matches!(
            self,
            Commands::Diag2Diag {
                preserve_space: true,
                ..
            }
        ) {
            // We don't need to run it in 2Cbor, but it doesn't really hurt either
            data.set_delimiters(cbor_edn::DelimiterPolicy::indented());
        }
        Ok(())
    }
}

#[derive(Args)]
struct CommonArgs {
    /// File to load from ("-" means stdin).
    #[clap(value_parser, default_value = "-")]
    input: clio::Input,
    /// File to store to ("-" means stdout).
    #[clap(value_parser, default_value = "-")]
    output: clio::Output,
}

#[derive(Copy, Clone, Default, PartialEq, ValueEnum)]
enum BinaryFormat {
    /// Process data as hexadecimal
    Hex,
    /// Process data as binary stream
    Binary,
    /// Process data hex when used from a terminal or as binary when used from a pipe/file
    #[default]
    Auto,
}

#[derive(Args)]
struct ToCborArgs {
    /// Output data format
    #[arg(short, long, value_enum)]
    output_format: Option<BinaryFormat>,
}

#[derive(Args)]
struct FromCborArgs {
    /// Keep tags in numeric form even if there is an application-oriented literal to represent
    /// them, and do not apply other custom formatting (such as comments or alternative string
    /// styles) based on tags.
    #[arg(long)]
    no_annotate_tags: bool,
    /// Input data format
    #[arg(short, long, value_enum)]
    input_format: Option<BinaryFormat>,
}

#[derive(Args)]
struct ToDiagArgs {
    /// For all known application-oriented literals, perform the encoding into the CBOR data mode.
    ///
    /// Unknown literals are left in place, possibly with a comment if convesion failed due to the
    /// item's internal structure.
    #[arg(long)]
    aol_to_item: bool,

    /// Keep tags 2/3 as tags rather than using EDN bignums
    ///
    /// Note that some tags 2/3 (eg. those having encoding indicators) are still left as tags.
    #[arg(long)]
    no_bignum_from_tags: bool,

    /// Set comments (and possibly other EDN specifics such as literal choice) from knowledge of
    /// the item's type
    #[arg(long, value_enum)]
    annotate: Option<KnownAnnotation>,
}

#[derive(Copy, Clone, ValueEnum)]
enum KnownAnnotation {
    /// Single item is a CCS (CWT Claims Set)
    Ccs,
}

#[derive(Args)]
struct FromDiagArgs {
    /// Convert any application oriented literals not covered by other options (or from disabled
    /// options) into CBOR tag 999.
    #[arg(long)]
    apply_999: bool,
    /// When known tags are encountered, convert them to application-oriented literals, or perform
    /// other annotations that do not alter the resulting CBOR.
    #[arg(long)]
    annotate_tags: bool,
}

fn main() -> eyre::Result<()> {
    let mut cli = Cli::parse();

    use std::io::{Read, Write};
    let mut input_data = Vec::new();
    cli.command.common().input.read_to_end(&mut input_data)?;

    let mut parsed = cli.command.load(&input_data)?;

    cli.command.transform(&mut parsed)?;

    let bytes = cli.command.serialize(parsed)?;

    cli.command.common().output.write_all(&bytes)?;

    Ok(())
}
