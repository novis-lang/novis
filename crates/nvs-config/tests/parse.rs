//! `rule:config/a-duplicate-key-is-an-error-and-so-is-an-unknown-one`'s per-file refusals, and that a refusal points at the line rather than at a byte.

use nvs_diagnostics::{Severity, SourceMap, code};

/// The whole of `tests/config/duplicate-key.toml`, inline, because a case that reads a fixture
/// tests the fixture's path as much as the refusal. The file on disk is the same three lines and is
/// what the `nvs config check` acceptance runs against.
const DUPLICATE: &str = "[limits]\nmemory = \"128M\"\nmemory = \"256M\"\n";

/// `rule:config/a-duplicate-key-is-an-error-and-so-is-an-unknown-one`: in a root-owned file where one table grants capabilities, an assignment silently
/// shadowed by a later copy of itself is a security-relevant failure, so a duplicate key inside one
/// file is refused. Across an `[[include]]` the same key is an override instead (`rule:config/later-wins-and-every-override-is-recorded`), and
/// nothing here should be read as answering about that.
#[test]
fn a_duplicate_key_in_one_file_is_refused() {
    let mut sources = SourceMap::new();
    let (_, parsed) = nvs_config::file::parse::<toml::Table>(&mut sources, "nvs.toml", DUPLICATE);

    let diagnostic = parsed.expect_err("a key set twice in one file is refused");
    assert_eq!(diagnostic.severity, Severity::Error);
    assert_eq!(diagnostic.code, Some(code::E_DUPLICATE_DIRECTIVE));
    assert!(
        diagnostic.message.contains("duplicate"),
        "the message should say what was wrong, not merely that something was: {:?}",
        diagnostic.message,
    );

    // The refusal names the *second* assignment, which is the line an operator has to delete.
    let span = diagnostic
        .primary_span()
        .expect("a refusal carries the line that caused it");
    let second = DUPLICATE
        .rfind("memory")
        .expect("the fixture sets `memory` twice");
    assert!(
        span.start as usize >= second,
        "the span points at the first assignment, not the duplicate one",
    );
}

/// The success path registers the file too, because resolving an override across an include has to
/// be able to name the origin of the value it kept (`rule:config/later-wins-and-every-override-is-recorded`).
#[test]
fn a_well_formed_file_parses_and_is_registered_for_reporting() {
    let mut sources = SourceMap::new();
    let text = "[limits]\nmemory = \"128M\"\n";
    let (id, parsed) = nvs_config::file::parse::<toml::Table>(&mut sources, "nvs.toml", text);

    let table = parsed.expect("a well-formed file parses");
    assert_eq!(table["limits"]["memory"].as_str(), Some("128M"));
    assert_eq!(
        id.index(),
        0,
        "the file is in the map the caller will render through"
    );
}

/// A syntax error is not a duplicate, and the code is what a caller switches on.
#[test]
fn a_malformed_file_is_refused_under_the_bad_directive_code() {
    let mut sources = SourceMap::new();
    let (_, parsed) = nvs_config::file::parse::<toml::Table>(&mut sources, "nvs.toml", "[limits\n");

    let diagnostic = parsed.expect_err("an unclosed table header is refused");
    assert_eq!(diagnostic.code, Some(code::E_BAD_DIRECTIVE));
}
