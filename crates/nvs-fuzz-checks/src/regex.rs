use nvs_stdlib::regex::validate;

/// Compiling a `Core\Regex` pattern must not panic or hang on any text, and
/// the engine a pattern lands on is the same every time it is compiled
/// (`rule:core-classes/regex-two-tiers`).
pub fn regex(data: &[u8]) {
    let Ok(pattern) = std::str::from_utf8(data) else {
        return;
    };
    let once = validate(pattern);
    assert_eq!(once, validate(pattern), "a pattern's tier is fixed");
}
