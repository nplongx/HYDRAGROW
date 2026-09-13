//! Converters for application-oriented literals
//!
//! Implementations in this module do not use any private APIs, could just as well be implemented
//! by users, but are provided for convenience and reference.

// potential custom extensions: big'123', big'123e-2', big'123p-2', uuid'', mac'', arruint8 etc,
// oid''

/// Visitor for [`visit_application_literals`](super::StandaloneItem::visit_application_literals) that
/// converts all known application oriented literals into their respective values.
///
/// This calls all `visit_application_literals` compatible visitors in this module except
/// [`any_aol_to_tag999`], where the tags are designed to be specifically opt-in by the
/// application.
///
/// This calls:
/// * [`dt_aol_to_item()`']
/// * [`ip_aol_to_item()`']
pub fn all_aol_to_item(id: String, value: String, item: &mut super::Item) -> Result<(), String> {
    // Beware that while those might conflict and overwrite each other, or even panic if they
    // access the item under the premise of being an visit_application_literals visitor (asserting
    // it is an AOL), but ours don't.

    // When extending this list, consider updating the bin/ -- it exerts more fine-grained control
    // and thus does notuse this function.
    dt_aol_to_item(id.clone(), value.clone(), item)?;
    ip_aol_to_item(id.clone(), value.clone(), item)?;
    Ok(())
}

/// Visitor for [`visit_tag`](super::StandaloneItem::visit_tag) that processes any known tag into a more
/// understandable form.
///
/// This calls conversions into application orientented literals, annotators (currently none
/// available that are tag driven), and other EDN specifics:
///
/// * [`dt_tag_to_aol`()]
/// * [`ip_tag_to_aol()`]
/// * [`tag23_to_edn_integer()`]
pub fn all_tag_prettify(tag: u64, item: &mut super::Item) -> Result<(), String> {
    // Beware that while those might conflict and overwrite each other, or even panic if they
    // access the item under the premise of being an visit_tag visitor (asserting there is a tagged
    // item), but ours don't.

    // When extending this list, consider updating the bin/ -- it exerts more fine-grained control
    // and thus does notuse this function.
    dt_tag_to_aol(tag, item)?;
    ip_tag_to_aol(tag, item)?;
    tag23_to_edn_integer(tag, item)?;
    Ok(())
}

/// Visitor for [`visit_application_literals`](super::StandaloneItem::visit_application_literals) that
/// converts all application oriented literals into tag 999.
///
/// This is convenient when processing EDN into another tool that can deal with the application
/// oriented literals but has no own EDN processor.
///
/// ```rust
/// # use cbor_edn::*;
/// # use cbor_edn::application::*;
/// let mut with_unknown_literals = StandaloneItem::parse("foo'bar'").unwrap();
/// with_unknown_literals.visit_application_literals(&mut any_aol_to_tag999);
/// assert_eq!(
///     with_unknown_literals.serialize(),
///     r#"999(["foo", "bar"])"#,
/// );
/// ```
pub fn any_aol_to_tag999(id: String, value: String, item: &mut super::Item) -> Result<(), String> {
    if matches!(id.as_str(), "h" | "b32" | "h32" | "b64") {
        // These are built into EDN
        return Ok(());
    }

    let id = super::Item::new_text(&id);
    let value = super::Item::new_text(&value);
    let array = super::Item::new_array([id, value].into_iter());
    let mut array = super::StandaloneItem::from(array);
    array.set_delimiters(super::DelimiterPolicy::SingleLineRegularSpacing);
    *item = array.tagged(999);

    Ok(())
}

/// Visitor for [`visit_application_literals`](super::StandaloneItem::visit_application_literals) that
/// processes `dt''` literals into RFC9164 (tagged) items.
///
/// ```rust
/// # use cbor_edn::*;
/// # use cbor_edn::application::*;
/// let mut with_dt_literals = StandaloneItem::parse("[dt'1970-01-01T00:00:23Z']").unwrap();
/// with_dt_literals.visit_application_literals(&mut dt_aol_to_item);
/// assert_eq!(
///     with_dt_literals.serialize(),
///     "[23]",
/// );
/// ```
///
/// If the timestamp does not conform to the required format, the application oriented literal is
/// left in place. A comment is added around the item to explain the error.
pub fn dt_aol_to_item(id: String, value: String, item: &mut super::Item) -> Result<(), String> {
    let is_tagged = match id.as_str() {
        "dt" => false,
        "DT" => true,
        _ => return Ok(()),
    };

    let Ok(parsed) = chrono::DateTime::parse_from_rfc3339(&value) else {
        return Err("DateTime parsing failed".to_string());
    };
    let nanos = parsed.timestamp_subsec_nanos();
    let mut replacement = if nanos == 0 {
        super::Item::new_integer_decimal(parsed.timestamp())
    } else {
        // Not worrying about integer precision loss: 2**53 seconds is way beyond age-of-universe.
        //
        // FIXME: if nanos exceeds 999999999 in the event of a leap second, is the conversion correct?
        super::Item::new_float_decimal(parsed.timestamp() as f64 + f64::from(nanos) / 1000000000.0)
    };
    if is_tagged {
        replacement = replacement.tagged(1);
    }
    replacement.set_delimiters(super::DelimiterPolicy::SingleLineRegularSpacing);
    *item = replacement;
    Ok(())
}

/// Visitor for [`visit_tag`](super::StandaloneItem::visit_tag) that
/// processes tag 1 into `dt''` literals.
///
/// ```rust
/// # use cbor_edn::*;
/// # use cbor_edn::application::*;
/// let mut with_dt_literals = StandaloneItem::parse("{/now/0: 1(42)}").unwrap();
/// with_dt_literals.visit_tag(&mut dt_tag_to_aol);
/// assert_eq!(
///     with_dt_literals.serialize(),
///     "{/now/0: DT'1970-01-01T00:00:42+00:00'}",
/// );
/// ```
///
/// Note that the precise format of the serialization is not guaranteed; the implementation might
/// return `...:42Z` just as well.
pub fn dt_tag_to_aol(tag: u64, item: &mut super::Item) -> Result<(), String> {
    if tag != 1 {
        return Ok(());
    }

    let tagged = item
        .get_tagged()
        .expect("Function is required to be called on a tagged item")
        .item();

    *item = dt_item_to_aol(tagged, true)?;
    Ok(())
}

/// Build a dt'' application-oriented literal from an existing item.
///
/// On error, this returns text that can might placed in a comment around the original item.
pub fn dt_item_to_aol<'a>(
    item: &super::Item<'a>,
    wrap_in_tag: bool,
) -> Result<super::Item<'a>, &'static str> {
    let int_part;
    let nano_part;
    if let Ok(timestamp) = item.get_integer() {
        let timestamp = i64::try_from(timestamp)
            .map_err(|_| "Value out of range for integer time processing")?;
        int_part = timestamp;
        nano_part = 0;
    } else {
        let timestamp = item
            .get_float()
            .map_err(|_| "Numeric value expected for dt'' literal")?;
        // As with the other direction, inexactness here has not happened since the beginning of
        // the universe.
        int_part = timestamp as i64;
        nano_part = ((timestamp - int_part as f64) * 1000000000.0) as u32;
    };
    let chronotime = chrono::DateTime::<chrono::offset::Utc>::from_timestamp(int_part, nano_part)
        .ok_or("Value out of range for time stamps")?;

    let label = if wrap_in_tag { "DT" } else { "dt" };
    Ok(
        super::Item::new_application_literal(label, &chronotime.to_rfc3339())
            .expect("DT is a valid application identifier"),
    )
}

#[test]
fn test_dt_vectors() {
    let input = "
    dt'1969-07-21T02:56:16Z',
    dt'1969-07-21T02:56:16.5Z',
    DT'1969-07-21T02:56:16Z'
    ";
    let output = "
    -14159024,
    -14159023.5,
    1(-14159024)
    ";
    test_decode_vectors(input, output, dt_aol_to_item);
    // Not attempting a re-encode: Our format is known to be different because chrono produces
    // +00:00 rather than Z.
}

#[test]
fn test_dt_reencode() {
    // While the integer example is covered in the doc test, the float needs testing too.

    let mut with_dt_literals = super::StandaloneItem::parse("{/now/0: 1(1.5)}").unwrap();
    with_dt_literals.visit_tag(&mut dt_tag_to_aol);
    assert_eq!(
        with_dt_literals.serialize(),
        "{/now/0: DT'1970-01-01T00:00:01.500+00:00'}",
    );
}

/// Visitor for [`visit_application_literals`](super::StandaloneItem::visit_application_literals) that
/// processes `ip''` literals into RFC9164 (tagged) items.
///
/// ```rust
/// # use cbor_edn::*;
/// # use cbor_edn::application::*;
/// let mut with_ip_literals = StandaloneItem::parse("[ip'2001:db8::1234', IP'127.0.0.0/8']").unwrap();
/// with_ip_literals.visit_application_literals(&mut ip_aol_to_item);
/// assert_eq!(
///     with_ip_literals.serialize(),
///     "[h'20010db8000000000000000000001234', 52([8, h'7f'])]",
/// );
/// ```
///
/// If addresses do not conform to the required format, the application oriented literal is left in
/// place. A comment is added around the item to explain the error.
pub fn ip_aol_to_item(id: String, value: String, item: &mut super::Item) -> Result<(), String> {
    let is_tagged = match id.as_str() {
        "ip" => false,
        "IP" => true,
        _ => return Ok(()),
    };
    let ip;
    let prefix: Option<u8>;
    if let Some((first, second)) = value.split_once('/') {
        let Ok(second) = second.parse() else {
            return Err("Prefix is not a (short) number".to_string());
        };
        ip = first;
        prefix = Some(second);
    } else {
        ip = &value;
        prefix = None;
    }
    let ip: std::net::IpAddr = ip
        .parse()
        .map_err(|e| format!("No recognized address format: {}", e))?;
    let (address_bits, ip) = match ip {
        std::net::IpAddr::V4(ip) => (32, u128::from_be_bytes(ip.to_ipv6_compatible().octets())),
        std::net::IpAddr::V6(ip) => (128, u128::from_be_bytes(ip.octets())),
    };
    if let Some(prefix) = prefix {
        if !(0..=address_bits).contains(&prefix) {
            return Err("Prefix exceeds address size".to_string());
        }
        let required_zero_bits = address_bits - prefix;
        let allow_nonzero_mask = u128::MAX << required_zero_bits;
        if ip & allow_nonzero_mask != ip {
            // See Section 4.2 (1) of RFC9164 which disallows that.
            return Err("No bits must be set in the address outside the prefix length".to_string());
        }
    }
    let ip = ip.to_be_bytes();
    let mut ip = ip.as_slice();
    ip = &ip[usize::from((128 - address_bits) / 8)..];
    if prefix.is_some() {
        while ip.last() == Some(&0) {
            ip = &ip[..ip.len() - 1];
        }
    }
    let ip = super::Item::new_bytes_hex(ip);
    let mut replacement = if let Some(prefix) = prefix {
        let prefix = super::Item::new_integer_decimal(prefix);
        super::Item::new_array([prefix, ip].into_iter())
    } else {
        ip
    };
    if is_tagged {
        replacement = replacement.tagged(if address_bits == 32 { 52 } else { 54 });
    }
    replacement.set_delimiters(super::DelimiterPolicy::SingleLineRegularSpacing);
    *item = replacement;
    Ok(())
}

/// Visitor for [`visit_tag`](super::StandaloneItem::visit_tag) that
/// processes tag 52/54 into `ip''` literals.
pub fn ip_tag_to_aol(tag: u64, item: &mut super::Item) -> Result<(), String> {
    let Some(version) = IpVersion::from_tag(tag) else {
        return Ok(());
    };

    let tagged = item
        .get_tagged()
        .expect("Function is required to be called on a tagged item")
        .item();

    *item = ip_item_to_aol(tagged, version, true)?;
    Ok(())
}

#[derive(Copy, Clone)]
pub enum IpVersion {
    V4,
    V6,
}

impl IpVersion {
    fn from_tag(tag: u64) -> Option<Self> {
        match tag {
            52 => Some(IpVersion::V4),
            54 => Some(IpVersion::V6),
            _ => None,
        }
    }

    fn tag_number(&self) -> u64 {
        match self {
            IpVersion::V4 => 52,
            IpVersion::V6 => 54,
        }
    }

    fn addr_len(&self) -> usize {
        match self {
            IpVersion::V4 => 4,
            IpVersion::V6 => 16,
        }
    }
}

/// Build an ip'' application-oriented literal from an existing item.
///
/// On error, this returns text that can might placed in a comment around the original item.
pub fn ip_item_to_aol<'a>(
    item: &super::Item<'a>,
    ip_version: IpVersion,
    wrap_in_tag: bool,
) -> Result<super::Item<'a>, &'static str> {
    let (mut addr, prefix) = if let Ok(addr) = item.get_bytes() {
        (addr, None)
    } else {
        let mut items = item
            .get_array_items()
            .map_err(|_| "Unexpected type for this tag")?;
        let elem1 = items.next().ok_or("Array too short for IP tag")?;
        if elem1.get_bytes().is_ok() {
            // This is the address-with-prefix form; we can't offer an application-oriented literal
            // for that.
            //
            // But we can offer a version of the item where the value is replaced in-place:
            let mut updated = item.clone();
            let address = updated
                .get_array_items_mut()
                .ok()
                .and_then(|mut i| i.next())
                .expect("Succeeded in clone template");
            *address = ip_item_to_aol(address, ip_version, false)?;
            if wrap_in_tag {
                updated = updated.tagged(ip_version.tag_number());
            }
            return Ok(updated);
        };
        let prefix = elem1;
        let addr = items.next().ok_or("Array too short for IP tag")?;
        if items.next().is_some() {
            return Err("Address with prefix and zone identifier is inexpressible in IP'' literal");
        }
        let prefix = prefix
            .get_integer()
            .map_err(|_| "Address with prefix is inexpressible in IP'' literal")?;
        let addr = addr
            .get_bytes()
            .map_err(|_| "Second element after an integer must be a byte string")?;
        (addr, Some(prefix))
    };

    use std::fmt::Write;
    let mut formatted = String::new();

    if addr.len() > ip_version.addr_len() {
        return Err("Data too long for address type");
    }
    addr.resize_with(ip_version.addr_len(), || 0);
    match ip_version {
        IpVersion::V4 => {
            let addr: [u8; 4] = addr.as_slice().try_into().expect("Works by construction");
            write!(formatted, "{}", std::net::Ipv4Addr::from(addr))
                .expect("Writing to a String is infallible");
        }
        IpVersion::V6 => {
            let addr: [u8; 16] = addr.as_slice().try_into().expect("Works by construction");
            write!(formatted, "{}", std::net::Ipv6Addr::from(addr))
                .expect("Writing to a String is infallible");
        }
    }
    if let Some(prefix) = prefix {
        write!(formatted, "/{}", prefix).expect("Writing to a String is infallible");
    }

    let label = if wrap_in_tag { "IP" } else { "ip" };
    Ok(super::Item::new_application_literal(label, &formatted)
        .expect("IP is a valid application identifier"))
}

/// Visitor for [`visit_tag`](super::StandaloneItem::visit_tag) that places comments inside a ta 38
/// language tagged string.
///
/// ```
/// # use cbor_edn::*;
/// # use cbor_edn::application::*;
/// let mut texts = StandaloneItem::parse(r#"[38(["en", "Hello"]), 38(["he", "שלום", true])]"#).unwrap();
/// texts.visit_tag(&mut comment_lang_tag);
/// assert_eq!(
///     texts.serialize(),
///     r#"[38(["en", "Hello"]), 38(["he", "שלום", true/ Right-to-left /])]"#,
/// );
/// ```
pub fn comment_lang_tag(tag: u64, item: &mut super::Item) -> Result<(), String> {
    if tag != 38 {
        return Ok(());
    }

    let tagged = item
        .get_tagged_mut()
        .expect("Function is required to be called on a tagged item")
        .item_mut();

    // FIXME: That we need to wrap and even leak the callback this way indicates that our for<'b>
    // lifetimes in the visitors are too generic -- the inner callbacks should only be for all
    // shorter lifetimes than the callback itself.
    let mut index = 0;
    let commenter: Box<dyn for<'b> FnMut(&mut crate::Item<'b>) -> _> = Box::new(move |item| {
        let result = match index {
            0 | 1 => None,
            2 => {
                // FIXME: This shows the need for simple value accessors -- or should this maybe be
                // played via CBOR encoding instead?
                if item == crate::StandaloneItem::parse("true").unwrap().item() {
                    Some("Right-to-left")
                } else if item == crate::StandaloneItem::parse("false").unwrap().item() {
                    Some("Left-to-right")
                } else if item == crate::StandaloneItem::parse("null").unwrap().item() {
                    Some("Automatic direction")
                } else {
                    None
                }
                .map(|s| s.to_string())
            }
            // Might be extended compatibly
            _ => None,
        };
        index += 1;
        Ok(result)
    });

    tagged
        .visit_array_elements(commenter)
        .map_err(|e| format!("{e}"))
}

#[test]
fn test_ip_vectors() {
    let input = "
    ip'192.0.2.42',
    IP'192.0.2.42',
    IP'192.0.2.0/24',
    ip'2001:db8::42',
    IP'2001:db8::42',
    IP'2001:db8::/64'
    52([ip'192.0.2.42',24])
    ";
    let output = "
    h'c000022a',
    52(h'c000022a'),
    52([24,h'c00002']),
    h'20010db8000000000000000000000042',
    54(h'20010db8000000000000000000000042'),
    54([64,h'20010db8'])
    52([h'c000022a',24])
    ";
    test_decode_vectors(input, output, ip_aol_to_item);
    test_encode_vectors(input, output, ip_tag_to_aol);
}

#[cfg(test)]
fn test_decode_vectors(
    input: &str,
    output: &str,
    mut map: impl for<'b> FnMut(String, String, &mut super::Item<'b>) -> Result<(), String>,
) {
    let input = input
        .lines()
        .map(|l| l.trim().trim_end_matches(','))
        .filter(|l| !l.is_empty());
    let output = output
        .lines()
        .map(|l| l.trim().trim_end_matches(','))
        .filter(|l| !l.is_empty());

    for (input, output) in input.zip(output) {
        println!("Converting {:?} to {:?}", input, output);
        let mut input_item =
            super::StandaloneItem::parse(input).expect("Failed to parse test vector");
        input_item.visit_application_literals(&mut map);
        // The converter tries to give nice optical outptu, but the test vectors are compact
        input_item.set_delimiters(super::DelimiterPolicy::DiscardAllButComments);
        assert_eq!(
            input_item.serialize(),
            output,
            "Test vector {input} did not convert exactly"
        );
    }
}

#[cfg(test)]
fn test_encode_vectors(
    aol: &str,
    maybetagged: &str,
    mut map: impl for<'b> FnMut(u64, &mut super::Item<'b>) -> Result<(), String>,
) {
    let aol = aol
        .lines()
        .map(|l| l.trim().trim_end_matches(','))
        .filter(|l| !l.is_empty());
    let maybetagged = maybetagged
        .lines()
        .map(|l| l.trim().trim_end_matches(','))
        .filter(|l| !l.is_empty());

    for (aol, maybetagged) in aol.zip(maybetagged) {
        println!("Converting {:?} to {:?}", maybetagged, aol);
        let mut input_item =
            super::StandaloneItem::parse(maybetagged).expect("Failed to parse test vector");
        if input_item.item().get_tag().is_err() {
            println!("Skipping this item: not tagged, thus not convertible");
            continue;
        }
        input_item.visit_tag(&mut map);
        // The converter tries to give nice optical outptu, but the test vectors are compact
        input_item.set_delimiters(super::DelimiterPolicy::DiscardAllButComments);
        assert_eq!(
            input_item.serialize(),
            aol,
            "Test vector {maybetagged} did not convert exactly"
        );
    }
}

/// Given a map that represents a CWT Claims Set (CCS), annotate the keys with their corresponding
/// JWT Claim Names in a comment, and possibly apply other nested annotations.
///
/// This currently only processes the items of the registry that were placed there by RFCs, but
/// that is subject to change.
pub fn comment_ccs(
    key: &mut super::Item<'_>,
) -> Result<(Option<String>, Option<crate::visitor::MapValueHandler>), String> {
    let Ok(key) = key.get_integer() else {
        // FIXME complain about non-int/tstr keys
        return Ok((None, None));
    };

    let handle_timestamp: crate::visitor::MapValueHandler = Box::new(|item| {
        *item = dt_item_to_aol(item, false)?;
        Ok(None)
    });

    let (comment, processor): (_, Option<crate::visitor::MapValueHandler>) = match key {
        1 => (Some("iss"), None),
        2 => (Some("sub"), None),
        3 => (Some("aud"), None),
        4 => (Some("exp"), Some(handle_timestamp)),
        5 => (Some("nbf"), Some(handle_timestamp)),
        6 => (Some("iat"), Some(handle_timestamp)),
        7 => (Some("jti"), None),
        8 => (
            Some("cnf"),
            Some(Box::new(|item| {
                // Not enforcing a particular cardinality: "At most one of the COSE_Key and
                // Encrypted_COSE_Key […] may be present", but I think a KID to go with it is fine.
                item.visit_map_elements(&mut comment_ccs_cnf)
                    .map(|_| None)
                    .map_err(|e| format!("{e}"))
            })),
        ),
        9 => (Some("scope"), None),
        38 => (Some("ace_profile"), None),
        39 => (Some("cnonce"), None),
        40 => (Some("exi"), None),
        _ => (None, None),
    };

    Ok((comment.map(|c| c.to_string()), processor))
}

fn comment_ccs_cnf(
    key: &mut super::Item<'_>,
) -> Result<(Option<String>, Option<crate::visitor::MapValueHandler>), String> {
    let Ok(key) = key.get_integer() else {
        // It doesn't really say that they are integers in 8747, does it?
        return Ok((None, None));
    };

    // Keys from https://www.iana.org/assignments/cwt/cwt.xhtml#confirmation-methods
    let (comment, processor): (_, Option<crate::visitor::MapValueHandler>) = match key {
        3 => (Some("kid"), None),
        4 => (
            Some("osc"),
            Some(Box::new(|item| {
                item.visit_map_elements(&mut comment_cnf_osc)
                    .map(|_| None)
                    .map_err(|e| format!("{e}"))
            })),
        ),
        _ => (None, None),
    };

    Ok((comment.map(|c| c.to_string()), processor))
}

fn comment_cnf_osc(
    key: &mut super::Item<'_>,
) -> Result<(Option<String>, Option<crate::visitor::MapValueHandler>), String> {
    let key = key.get_integer().map_err(|e| format!("{e}"))?;

    // Keys from https://www.iana.org/assignments/ace/ace.xhtml#oscore-security-context-parameters
    let (comment, processor): (_, Option<crate::visitor::MapValueHandler>) = match key {
        0 => (Some("id"), None),
        1 => (Some("version"), None),
        2 => (Some("ms"), None),
        3 => (Some("hkdf"), Some(Box::new(&comment_cose_algorithm))),
        4 => (Some("alg"), Some(Box::new(&comment_cose_algorithm))),
        5 => (Some("salt"), None),
        6 => (Some("contextId"), None),
        _ => (None, None),
    };

    Ok((comment.map(|c| c.to_string()), processor))
}

fn comment_cose_algorithm(item: &mut super::Item<'_>) -> Result<Option<String>, String> {
    // From https://www.iana.org/assignments/cose/cose.xhtml#algorithms
    // FIXME do more of those
    if let Ok(n) = item.get_integer() {
        Ok(Some(
            match n {
                -18 => "SHAKE128",
                -17 => "SHA-512/256",
                -16 => "SHA-256",
                -15 => "SHA-256/64",
                -14 => "SHA-1",
                -13 => "direct+HKDF-AES-256",
                -12 => "direct+HKDF-AES-128",
                -11 => "direct+HKDF-SHA-512",
                -10 => "direct+HKDF-SHA-256",
                -9 => "Unassigned",
                -8 => "EdDSA",
                -7 => "ES256",
                -6 => "direct",
                -5 => "A256KW",
                -4 => "A192KW",
                -3 => "A128KW",
                0 => return Err("Reserved".into()),
                1 => "A128GCM",
                2 => "A192GCM",
                3 => "A256GCM",
                4 => "HMAC 256/64",
                5 => "HMAC 256/256",
                6 => "HMAC 384/384",
                7 => "HMAC 512/512",
                10 => "AES-CCM-16-64-128",
                11 => "AES-CCM-16-64-256",
                12 => "AES-CCM-64-64-128",
                13 => "AES-CCM-64-64-256",
                14 => "AES-MAC 128/64",
                15 => "AES-MAC 256/64",
                24 => "ChaCha20/Poly1305",
                // Not commenting on values we don't know
                _ => return Ok(None),
            }
            .to_string(),
        ))
    } else {
        // FIXME: Should return Err("COSE algorithms are integer or strings"), but we don't have an
        // accessor for string literals yet.
        Ok(None)
    }
}

#[doc(inline)]
pub use crate::number::tag23_to_edn_integer;
