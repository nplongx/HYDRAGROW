use hexfloat2::parse;

#[track_caller]
fn check_parse_f32(s: &str, expected: f32) {
    assert_eq!(parse::<f32>(s).unwrap(), expected);
}

#[track_caller]
fn check_parse_f64(s: &str, expected: f64) {
    assert_eq!(parse::<f64>(s).unwrap(), expected);
}

#[test]
fn known_values() {
    // 2^53 - 1
    check_parse_f64("0x1fffffffffffff", 9007199254740991.0);
    // 2^54 - 1 will be truncated. The precise value would be 18014398509481983.
    check_parse_f64("0x3fffffffffffff", 18014398509481982.0);

    // f32 will truncate.
    check_parse_f32("0x1.0000000000001p0", 1.0);
    // f64 will not truncate. The decimal representation is imprecise; the
    // real value is 1.0000000000000002220446049250313080847263336181640625
    check_parse_f64("0x1.0000000000001p0", 1.000000000000000222);
}
