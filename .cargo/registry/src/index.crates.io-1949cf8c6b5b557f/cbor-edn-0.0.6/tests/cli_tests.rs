#[test]
#[cfg_attr(miri, ignore)]
fn cli_tests() {
    trycmd::TestCases::new()
        .case("tests/cli/*.trycmd")
        .case("tests/cli/*.toml")
        .case("README.md");
}
