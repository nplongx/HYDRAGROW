use super::*;

// Debug printing takes 3 of the 4 minites in a full `cargo miri test` run, and we don't expect
// miri tests to report on *where* something fails, but to report *that* unsafety happened or
// memory was leaked -- therefore, silencing it.
macro_rules! dbg {
    ($val:expr $(,)?) => {
        #[cfg(not(miri))]
        std::dbg!($val);
    };
}

#[derive(Default)]
struct QuirksAllowed {
    /// There are encoding indicators that, if removed, alter the encoded CBOR
    original_has_relevant_encoding_indicators: bool,
    /// There are ellipses in the EDN that need processing.
    encoding_needs_888: bool,
    /// There are unsupported application oriented literals in the EDN that need processing.
    encoding_needs_999: bool,
    /// There are other reasons why encoding would fail even with 888/999 in place
    encoding_impossible: bool,
}

#[track_caller]
fn test_seq(edn: &str, reference_value: Option<Sequence>, quirks_allowed: QuirksAllowed) {
    let parsed = Sequence::parse(edn).unwrap();

    if let Some(reference_value) = reference_value {
        assert_eq!(parsed, reference_value);
    }

    let reencoded = parsed.serialize();
    assert_eq!(edn, reencoded);

    // We don't do 888/999 conversion yet, so we just bail
    if quirks_allowed.encoding_needs_888 || quirks_allowed.encoding_impossible {
        return;
    }

    let mut for_cbor = parsed;

    for_cbor.visit_application_literals(&mut application::all_aol_to_item);
    if quirks_allowed.encoding_needs_999 {
        for_cbor.visit_application_literals(&mut application::any_aol_to_tag999);
    }

    let cbor_encoded: Vec<_> = for_cbor.to_cbor().unwrap();
    let _ = Sequence::from_cbor(&cbor_encoded).unwrap();
}

#[track_caller]
fn test_item(edn: &str, quirks_allowed: QuirksAllowed) {
    let parsed = cbordiagnostic::one_item(edn).unwrap();

    let reencoded = parsed.serialize();
    assert_eq!(edn, reencoded);

    let mut without_indicators = parsed.clone();
    without_indicators.item_mut().discard_encoding_indicators();
    let reencoded_without_indicators = without_indicators.serialize();
    if reencoded_without_indicators != edn {
        dbg!(&edn);
        dbg!(&reencoded_without_indicators);
        assert_eq!(
            cbordiagnostic::one_item(&reencoded_without_indicators).unwrap(),
            without_indicators,
            "Modified form (without encoding indicators) should still round-trip between internal version and diagnostic form"
        );
    }

    let mut without_space = parsed.clone();
    without_space.set_delimiters(DelimiterPolicy::SingleLineRegularSpacing);
    let reencoded_without_space = without_space.serialize();
    if reencoded_without_space != edn {
        dbg!(&edn);
        dbg!(&reencoded_without_space);
        assert_eq!(
            cbordiagnostic::one_item(&reencoded_without_space).unwrap(),
            without_space,
            "Modified form (without space) should still round-trip between internal version and diagnostic form"
        );
    }

    // We don't do 888/999 conversion yet, so we just bail
    if quirks_allowed.encoding_needs_888 || quirks_allowed.encoding_impossible {
        return;
    }

    fn apply_preprocessing<'a>(
        mut item: StandaloneItem<'a>,
        quirks_allowed: &QuirksAllowed,
    ) -> StandaloneItem<'a> {
        item.visit_application_literals(&mut application::all_aol_to_item);
        if quirks_allowed.encoding_needs_999 {
            item.visit_application_literals(&mut application::any_aol_to_tag999);
        }
        item
    }
    let for_cbor = apply_preprocessing(parsed, &quirks_allowed);
    let without_space_for_cbor = apply_preprocessing(without_space, &quirks_allowed);
    let without_indicators_for_cbor = apply_preprocessing(without_indicators, &quirks_allowed);
    dbg!(for_cbor.serialize());

    dbg!(&for_cbor);
    let parsed_cbor: Vec<_> = for_cbor.to_cbor().unwrap();
    dbg!(&parsed_cbor);
    let without_space_cbor: Vec<_> = without_space_for_cbor.to_cbor().unwrap();
    let without_indicators_cbor: Vec<_> = without_indicators_for_cbor.to_cbor().unwrap();

    assert_eq!(
        parsed_cbor, without_space_cbor,
        "Altering delimiters should not alter encoding"
    );
    if !quirks_allowed.original_has_relevant_encoding_indicators {
        assert_eq!(parsed_cbor, without_indicators_cbor, "Original didn't declare that its encoding indicators matter, should encode to CBOR the same");
    }

    let mut parsed_back = StandaloneItem::from_cbor(&parsed_cbor).unwrap();
    parsed_back.visit_tag(&mut application::all_tag_prettify);
    dbg!(parsed_back.serialize());
    dbg!(&parsed_back);
    // We are just comparing the generated CBOR; there is no expectation that the EDN will be
    // identical.
    assert_eq!(
        parsed_back.to_cbor().unwrap(),
        parsed_cbor,
        "Parsed CBOR did not round-trip",
    );
}

#[test]
fn simple_examples() {
    test_seq(
        "1,1,2,3,5,/last one:/ 8_i,/done/",
        Some(Sequence {
            s0: S("".into()),
            items: Some(NonemptyMscVec {
                first: Box::new(InnerItem::Number(Number("1".into()), None).into()),
                tail: vec![
                    (
                        MSC(",".into()),
                        InnerItem::Number(Number("1".into()), None).into(),
                    ),
                    (
                        MSC(",".into()),
                        InnerItem::Number(Number("2".into()), None).into(),
                    ),
                    (
                        MSC(",".into()),
                        InnerItem::Number(Number("3".into()), None).into(),
                    ),
                    (
                        MSC(",".into()),
                        InnerItem::Number(Number("5".into()), None).into(),
                    ),
                    (
                        MSC(",/last one:/ ".into()),
                        InnerItem::Number(Number("8".into()), Some(Spec::S_i)).into(),
                    ),
                ],
                soc: SOC(",/done/".into()),
            }),
        }),
        Default::default(),
    );

    test_item("1234", Default::default());
    test_item("1234.0", Default::default());
    test_item("-1234", Default::default());
    test_item("-1234.5", Default::default());
    test_item("-123456789012345678901234567890", Default::default());
    test_item("Infinity", Default::default());
    test_item("-Infinity", Default::default());
    test_item("simple(4)", Default::default());
    test_item("'hello'", Default::default());
    test_item(
        "foo'hello'",
        QuirksAllowed {
            encoding_needs_999: true,
            ..Default::default()
        },
    );
    test_item("'hello' + ' ' + 'world'", Default::default());
    test_item("(_ 'hello'+' ', 'world', )", Default::default());
    test_item(
        "[_3]",
        QuirksAllowed {
            original_has_relevant_encoding_indicators: true,
            ..Default::default()
        },
    );
    test_item(
        "[_3 3]",
        QuirksAllowed {
            original_has_relevant_encoding_indicators: true,
            ..Default::default()
        },
    );
    test_item(
        "[_ 3]",
        QuirksAllowed {
            original_has_relevant_encoding_indicators: true,
            ..Default::default()
        },
    );
    // This is worth extra testing because no space after _ is only allowed in the empty case.
    test_item(
        "[_]",
        QuirksAllowed {
            original_has_relevant_encoding_indicators: true,
            ..Default::default()
        },
    );
    test_item("[1, 2, \"Risiko\"]", Default::default());
    // This used to be an interesting corner case when removing the optional trailing comma made a
    // difference in where the last comment was stored.
    test_item(
        "{ / x / 1: 4, / y / 2: 3_i, / and diagonal is 5 / }",
        Default::default(),
    );
    test_item(
        "
        98([<< {/alg/ 1: -7 /ECDSA 256/} >>, # == h'a10126'
            ...                              # rest elided here
           ])",
        QuirksAllowed {
            encoding_needs_888: true,
            ..Default::default()
        },
    );

    test_item("ip'2001:db8::1'", Default::default());
    test_item(
        "e'2001:db8::1'",
        QuirksAllowed {
            encoding_needs_999: true,
            ..Default::default()
        },
    ); // pretty hard to convert to CBOR
    test_item(
        "ref'/etc/passwd'",
        QuirksAllowed {
            encoding_needs_999: true,
            ..Default::default()
        },
    ); // hehe ... jk, policy should prevent that

    test_seq(
        "    1 /,/,    'two',3(3)    /,/,/,/ ",
        None,
        Default::default(),
    );

    test_item(".1", Default::default());
    test_item(".1e1", Default::default());
    test_item("0x.1p1", Default::default());
    test_item("0x0.p1", Default::default());
    test_item(
        "99999999999999999999999999999999999999999999999999999999999999999",
        QuirksAllowed {
            encoding_impossible: true,
            ..Default::default()
        },
    );
    test_item(
        "99999999999999999999999999999999999999999999999999999999999999999.0",
        Default::default(),
    );
    test_item(
        "99999999999999999999999999999999999999999999999999999999999999999e100000",
        Default::default(),
    );
    test_item(
        "-1e9999999999999999999999999999999999999999999999999999",
        Default::default(),
    );
    test_item("simple(5)", Default::default());
    test_item(
        "simple(9999)",
        QuirksAllowed {
            encoding_impossible: true,
            ..Default::default()
        },
    );
    test_item(
        "[Infinity, -Infinity, NaN, true, false, null, undefined, simple(0), simple(255), simple(22) / actually null /]",
        // If we ever start linting against known simple values being spelled out, this may need an
        // extra Quirks.
        Default::default(),
    );
    test_item(
        "[h'00', <<1, 2, 3>>, <<<<<<<<>>>>>>/hi/ # linewrap required\n>>]",
        Default::default(),
    );
    // classical base64 encoding
    test_item("b64'/+8='", Default::default());
    // base64url encoding
    test_item("b64'_-8='", Default::default());
    // base64url without padding
    test_item("b64'_-8'", Default::default());
    // and we tolerate mixes and arbitrary padding
    test_item("b64'/-8========='", Default::default());

    // tolerating mixed case input and incomplete padding
    test_item("b32'74Aa=='", Default::default());
    test_item("h32'Vs00='", Default::default());
}

fn recurse_folder(path: &std::path::Path) {
    for entry in std::fs::read_dir(path).unwrap() {
        let entry = entry.unwrap();
        let entry_path = entry.path();
        if entry_path.to_string_lossy().ends_with("~") {
            // Skip editor backup files
            continue;
        }
        if entry_path.is_dir() {
            recurse_folder(&entry_path);
        } else {
            println!("Testing CBOR sequence from {}", entry_path.display());
            let serialized = &std::fs::read_to_string(entry_path).unwrap();
            let mut quirks = QuirksAllowed::default();
            if serialized.contains("h'<cacrts DTLS encrypted re") {
                quirks.encoding_impossible = true;
            }
            if serialized.contains("...") {
                // Criterion might be a bit lax, but there are too many versions in which this can
                // occur to be precise
                quirks.encoding_needs_888 = true;
            }
            test_seq(serialized, None, quirks);
        }
    }
}

#[test]
#[cfg_attr(miri, ignore)]
fn all_examples() {
    let mut dir = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    dir.push("tests/ietf/diag");
    recurse_folder(&dir)
}

#[test]
fn builder() {
    let mut test1 = Sequence::new(
        [
            Item::new_integer_decimal(1234567890)
                .with_comment("Some day in early 2009")
                .tagged(1),
            Item::new_array(
                [
                    Item::new_bytes_hex(b"hello\0\xff"),
                    Item::new_bytes_hex(b"123"),
                ]
                .into_iter(),
            ),
            Item::new_map(core::iter::once((
                Item::new_integer_decimal(1234),
                Item::new_map(core::iter::empty()),
            ))),
        ]
        .into_iter(),
    );
    test1.set_delimiters(DelimiterPolicy::indented());
    println!("{}", test1.serialize());
    println!("{:02x?}", test1.to_cbor().unwrap());
}
