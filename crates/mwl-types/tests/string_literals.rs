//! Escape cooking and heredoc/nowdoc indentation, checked at the literal rather than at run time.
//!
//! Moved out of `mwl_types::check`'s inline `mod tests`; every test keeps its
//! own name and body. See `tests/common/mod.rs` for the shared fixtures.

mod common;

use common::*;
use mwl_diagnostics::code;

/// A `\u{...}` escape naming a codepoint past Unicode's `0x10FFFF` scalar
/// ceiling has no UTF-8 encoding — `crate::string_lit`'s own
/// `E_INVALID_UNICODE_ESCAPE`.
#[test]
fn a_unicode_escape_out_of_range_is_diagnosed() {
    let diags = check_in_method(r#"string $s = "\u{110000}";"#);
    assert!(
        diags
            .iter()
            .any(|d| d.code == Some(code::E_INVALID_UNICODE_ESCAPE)),
        "{diags:?}"
    );
}

/// A `\u{...}` escape naming a UTF-16 surrogate codepoint is the other
/// shape with no UTF-8 encoding.
#[test]
fn a_unicode_escape_naming_a_surrogate_is_diagnosed() {
    let diags = check_in_method(r#"string $s = "\u{D800}";"#);
    assert!(
        diags
            .iter()
            .any(|d| d.code == Some(code::E_INVALID_UNICODE_ESCAPE)),
        "{diags:?}"
    );
}

/// A lone `\xFF` byte escape can never be a valid UTF-8 sequence on its
/// own — `string` is guaranteed-valid UTF-8 (ADR 0009), so this is
/// `E_STRING_LITERAL_INVALID_UTF8`, not silently accepted.
#[test]
fn a_byte_escape_producing_invalid_utf8_is_diagnosed() {
    let diags = check_in_method(r#"string $s = "\xFF";"#);
    assert!(
        diags
            .iter()
            .any(|d| d.code == Some(code::E_STRING_LITERAL_INVALID_UTF8)),
        "{diags:?}"
    );
}

/// Two byte escapes that combine into one valid UTF-8 character
/// (`\xC3\xA9` is `é`) are fine — the check is on the assembled result,
/// not each escape byte in isolation.
#[test]
fn byte_escapes_combining_into_valid_utf8_are_not_diagnosed() {
    let diags = check_in_method(r#"string $s = "\xC3\xA9";"#);
    assert!(!diags.has_errors(), "{diags:?}");
}

/// A single-quoted literal's own two escapes (`\\`/`\'`) can never
/// produce invalid UTF-8, so it gets no cooking-diagnostic pass at all —
/// confirming the quote-kind guard actually gates on the literal's own
/// spelling rather than always running.
#[test]
fn a_single_quoted_literal_is_never_escape_diagnosed() {
    let diags = check_in_method(r"string $s = '\xFF';");
    assert!(!diags.has_errors(), "{diags:?}");
}

/// The same invalid-UTF-8 byte-escape check applies inside an
/// interpolated string's own literal text, not just a plain `Str` —
/// `ExprKind::Interpolated`'s `StringPart::Text` arm.
#[test]
fn a_byte_escape_inside_an_interpolated_string_is_diagnosed() {
    let diags = check_in_method("string $y = \"z\";\nstring $s = \"\\xFF$y\";\n");
    assert!(
        diags
            .iter()
            .any(|d| d.code == Some(code::E_STRING_LITERAL_INVALID_UTF8)),
        "{diags:?}"
    );
}

// --- heredoc/nowdoc flexible-indentation stripping ---------------------

/// A heredoc whose closing marker is indented, and whose body lines
/// carry exactly that much indentation, is fine — the common,
/// well-formed shape PHP 7.3's "flexible heredoc" rule exists for.
#[test]
fn a_consistently_indented_heredoc_is_not_diagnosed() {
    let diags = check_in_method("string $s = <<<EOT\n    hello\n    world\n    EOT;\n");
    assert!(!diags.has_errors(), "{diags:?}");
}

/// The closing marker's own indentation mixing spaces and tabs is
/// `E_HEREDOC_MIXED_INDENT` — PHP requires one or the other so a body
/// line's leading whitespace can be compared byte-for-byte.
#[test]
fn a_heredoc_closing_marker_mixing_tabs_and_spaces_is_diagnosed() {
    let diags = check_in_method("string $s = <<<EOT\n\thello\n\t EOT;\n");
    assert!(
        diags
            .iter()
            .any(|d| d.code == Some(code::E_HEREDOC_MIXED_INDENT)),
        "{diags:?}"
    );
}

/// A body line with less leading whitespace than the closing marker is
/// `E_HEREDOC_INSUFFICIENT_INDENT`.
#[test]
fn a_heredoc_body_line_with_insufficient_indentation_is_diagnosed() {
    let diags = check_in_method("string $s = <<<EOT\n    hello\n  world\n    EOT;\n");
    assert!(
        diags
            .iter()
            .any(|d| d.code == Some(code::E_HEREDOC_INSUFFICIENT_INDENT)),
        "{diags:?}"
    );
}

/// A truly empty body line is exempt from the indentation check, even
/// between two consistently indented lines.
#[test]
fn a_blank_heredoc_body_line_is_not_diagnosed() {
    let diags = check_in_method("string $s = <<<EOT\n    hello\n\n    world\n    EOT;\n");
    assert!(!diags.has_errors(), "{diags:?}");
}

/// A nowdoc applies no escape grammar at all (unlike a heredoc, which
/// still cooks `\n`/`\t`/etc.), but still gets indentation-checked —
/// `\n` here must stay two literal characters, and the check above it
/// runs regardless of whether escapes do.
#[test]
fn a_nowdoc_skips_escape_cooking_but_still_checks_indentation() {
    let diags = check_in_method("string $s = <<<'EOT'\n    raw \\n text\n    EOT;\n");
    assert!(!diags.has_errors(), "{diags:?}");
}

/// An interpolated heredoc's own indentation check applies per body run,
/// not just to a plain `Str`-collapsed one — this line becomes its own
/// `StringPart::Text` run picking up right after `$y`'s interpolation
/// site, so it has to be recognized as a fresh line on its own.
#[test]
fn an_interpolated_heredoc_body_line_after_interpolation_is_still_checked() {
    let diags =
        check_in_method("string $y = \"z\";\nstring $s = <<<EOT\n    pre $y\n  post\n    EOT;\n");
    assert!(
        diags
            .iter()
            .any(|d| d.code == Some(code::E_HEREDOC_INSUFFICIENT_INDENT)),
        "{diags:?}"
    );
}

/// The same interpolated-heredoc shape, consistently indented, is fine —
/// confirms the indentation check doesn't false-positive on a
/// well-formed interpolation site.
#[test]
fn a_consistently_indented_interpolated_heredoc_is_not_diagnosed() {
    let diags =
        check_in_method("string $y = \"z\";\nstring $s = <<<EOT\n    pre $y\n    post\n    EOT;\n");
    assert!(!diags.has_errors(), "{diags:?}");
}
