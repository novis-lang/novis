//! Cooks the double-quoted escape grammar shared by a `"…"` string literal
//! and an interpolated-heredoc's own text runs — `mwl_syntax::ast::ExprKind`'s
//! `Str` (double-quoted case only) and each `Text` piece of `Interpolated`'s
//! [`mwl_syntax::ast::StringPart`] vector — into the runtime bytes they name.
//!
//! `mwl-syntax`'s lexer already recognizes *which* backslash sequences exist
//! (`Lexer::lex_escape_in_place`) and rejects a syntactically malformed
//! `\u{...}` (missing hex digits, or no closing `}`) at lex time via
//! `code::E_INVALID_ESCAPE`. It does not evaluate what a `\xHH`, octal, or
//! `\u{...}` escape actually *produces* — cooking needs the whole literal
//! assembled first, the same reason `mwl_types::expr::infer`'s `ExprKind::Int`
//! arm (not the lexer) is where an integer literal's *magnitude* gets
//! checked. This module is that arm's sibling for string content: it cooks
//! the escapes and reports the two ways cooking can still fail — an
//! out-of-range/surrogate `\u{...}` codepoint, or a `\xHH`/octal byte escape
//! sequence that does not decode as valid UTF-8 (ADR 0009's "`string` is
//! guaranteed-valid UTF-8" invariant) — before `mwl-ir` ever lowers the
//! literal.
//!
//! `pub`, not `pub(crate)`, and reused directly by `mwl-ir` (which already
//! depends on this crate for [`crate::ty::Ty`]/[`crate::expr_table`]) rather
//! than duplicated the way `mwl_types::expr`'s own `int_literal_digits` and
//! `mwl_ir::lower`'s `int_literal_digits` independently are: that duplicate is
//! a ~15-line digit-splitting helper neither crate depends on the other
//! for reaching, where this is a much larger, subtler cooking routine whose
//! checker-reported diagnosis and `mwl-ir`'s actual lowered value must never
//! silently diverge — sharing one implementation is what guarantees that,
//! not just a style preference.
//!
//! Only the double-quoted escape grammar lives here. A single-quoted literal
//! (`\\`/`\'` only) can never produce invalid UTF-8 — it copies source
//! characters through unchanged aside from those two escapes, both already
//! ASCII — so it has no cooking routine of its own to share; `mwl-ir`'s
//! `cook_str_literal` keeps that tiny case inline. A heredoc/nowdoc-sourced
//! `ExprKind::Str` is a separate, still-open gap (PHP's "flexible heredoc"
//! indentation stripping) — see `mwl-ir`'s own crate docs.

use mwl_diagnostics::{SourceFile, Span};

/// One way [`cook_double_quoted_text`] failed to fully cook its input.
/// Cooking still returns a best-effort `String` alongside these (lossy UTF-8
/// substitution for [`CookIssue::InvalidUtf8`], the escape dropped entirely
/// for [`CookIssue::InvalidUnicodeEscape`]) so a caller that reports and
/// keeps going — every checker diagnostic in this crate does — has something
/// to keep checking against.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CookIssue {
    /// A `\u{...}` escape named a value outside Unicode's valid scalar range
    /// (`> 0x10FFFF`) or inside the UTF-16 surrogate range
    /// (`0xD800..=0xDFFF`) — neither has a UTF-8 encoding. The span covers
    /// just that one escape.
    InvalidUnicodeEscape(Span),
    /// One or more `\xHH`/octal byte escapes combined into a sequence that is
    /// not valid UTF-8 once assembled with the rest of the text. Attributed
    /// to the whole cooked span rather than one escape, since a UTF-8
    /// continuation byte is only invalid in the context of the bytes around
    /// it.
    InvalidUtf8(Span),
}

/// Cooks `span` — the text strictly between a double-quoted literal's own
/// quote characters, or one [`mwl_syntax::ast::StringPart::Text`] run inside
/// an `Interpolated` literal (which never includes a quote character at all)
/// — into the `string` value it denotes.
///
/// Handles every escape `mwl-syntax`'s lexer recognizes when interpolation is
/// active (`Lexer::lex_quoted_body`'s `interpolation` branch, shared
/// verbatim between a double-quoted literal and a heredoc opened without
/// `'quotes'`, which is exactly why this same routine cooks both): `\\`,
/// `\"`, `\$`, `\n`, `\t`, `\r`, `\v`, `\f`, `\e`, a 1-3-digit octal escape
/// (`\0`-`\377`, silently wrapping mod 256 the same way PHP's does), a
/// 1-2-digit hex escape (`\xHH`), and a `\u{...}` Unicode codepoint escape
/// (encoded to its UTF-8 bytes). Any other backslash sequence — including a
/// syntactically malformed `\u{...}` the lexer already flagged separately —
/// is passed through literally, matching PHP and avoiding a second report of
/// something `mwl-syntax` already diagnosed.
#[must_use]
pub fn cook_double_quoted_text(src: &SourceFile, span: Span) -> (String, Vec<CookIssue>) {
    let text = src.span_text(span).unwrap_or_default();
    let chars: Vec<(usize, char)> = text.char_indices().collect();
    let n = chars.len();
    let mut bytes: Vec<u8> = Vec::with_capacity(text.len());
    let mut issues = Vec::new();
    let mut buf = [0u8; 4];

    let mut i = 0usize;
    while i < n {
        let (_, c) = chars[i];
        if c != '\\' {
            bytes.extend_from_slice(c.encode_utf8(&mut buf).as_bytes());
            i += 1;
            continue;
        }
        // `c` is the backslash itself; a lone trailing `\` (only possible at
        // the very end of a `StringPart::Text` run immediately followed by
        // an interpolation site, e.g. `"\` right before `{$x}`) has nothing
        // to escape — keep it literal rather than indexing past the end.
        let Some(&(_, next)) = chars.get(i + 1) else {
            bytes.push(b'\\');
            i += 1;
            continue;
        };
        match next {
            '\\' | '"' | '$' => {
                bytes.push(u8::try_from(next).expect("\\, \", $ are all ASCII"));
                i += 2;
            }
            'n' => {
                bytes.push(b'\n');
                i += 2;
            }
            't' => {
                bytes.push(b'\t');
                i += 2;
            }
            'r' => {
                bytes.push(b'\r');
                i += 2;
            }
            'v' => {
                bytes.push(0x0B);
                i += 2;
            }
            'f' => {
                bytes.push(0x0C);
                i += 2;
            }
            'e' => {
                bytes.push(0x1B);
                i += 2;
            }
            '0'..='7' => {
                let mut val: u32 = 0;
                let mut j = i + 1;
                let mut consumed = 0;
                while consumed < 3
                    && let Some(&(_, d)) = chars.get(j)
                    && ('0'..='7').contains(&d)
                {
                    val = val * 8 + d.to_digit(8).expect("guarded to '0'..='7' above");
                    j += 1;
                    consumed += 1;
                }
                bytes.push(u8::try_from(val % 256).expect("reduced mod 256 above"));
                i = j;
            }
            'x' => {
                let mut val: u32 = 0;
                let mut j = i + 2;
                let mut consumed = 0;
                while consumed < 2
                    && let Some(&(_, d)) = chars.get(j)
                    && let Some(hex) = d.to_digit(16)
                {
                    val = val * 16 + hex;
                    j += 1;
                    consumed += 1;
                }
                if consumed == 0 {
                    // PHP itself leaves a bare `\x` (no hex digit follows)
                    // literal rather than erroring — the lexer never flags
                    // this shape either, so mirror that instead of inventing
                    // a diagnostic for it here.
                    bytes.extend_from_slice(b"\\x");
                    i += 2;
                } else {
                    bytes.push(u8::try_from(val).expect("at most 2 hex digits always fits a u8"));
                    i = j;
                }
            }
            'u' if chars.get(i + 2).map(|&(_, c)| c) == Some('{') => {
                let hex_start = i + 3;
                let mut j = hex_start;
                while chars.get(j).is_some_and(|&(_, d)| d.is_ascii_hexdigit()) {
                    j += 1;
                }
                if j > hex_start && chars.get(j).map(|&(_, c)| c) == Some('}') {
                    let hex: String = chars[hex_start..j].iter().map(|&(_, c)| c).collect();
                    let value = u32::from_str_radix(&hex, 16).unwrap_or(u32::MAX);
                    let escape_end = chars
                        .get(j + 1)
                        .map_or(span.end, |&(off, _)| span.start + off_as_u32(off));
                    match char::from_u32(value) {
                        Some(ch) => bytes.extend_from_slice(ch.encode_utf8(&mut buf).as_bytes()),
                        None => issues.push(CookIssue::InvalidUnicodeEscape(Span::new(
                            span.file,
                            span.start + off_as_u32(chars[i].0),
                            escape_end,
                        ))),
                    }
                    i = j + 1;
                } else {
                    // A syntactically malformed `\u{...}` (no digits, or no
                    // closing `}`) — `mwl-syntax` already reported
                    // `E_INVALID_ESCAPE` for this at lex time. Pass the two
                    // characters through literally rather than double-report.
                    bytes.extend_from_slice(b"\\u");
                    i += 2;
                }
            }
            other => {
                bytes.push(b'\\');
                bytes.extend_from_slice(other.encode_utf8(&mut buf).as_bytes());
                i += 2;
            }
        }
    }

    match String::from_utf8(bytes) {
        Ok(s) => (s, issues),
        Err(e) => {
            issues.push(CookIssue::InvalidUtf8(span));
            (String::from_utf8_lossy(e.as_bytes()).into_owned(), issues)
        }
    }
}

/// A source file caps out at 4 GiB (`mwl_diagnostics::span::BytePos` is
/// `u32`), so a byte offset within one always fits back into a `u32`.
fn off_as_u32(off: usize) -> u32 {
    u32::try_from(off).expect("a byte offset within one source file fits u32 (BytePos's own type)")
}

#[cfg(test)]
mod tests {
    use mwl_diagnostics::{SourceMap, Span};

    use super::*;

    fn cook(src_text: &str) -> (String, Vec<CookIssue>) {
        let mut map = SourceMap::new();
        let file = map.add("t.mwl", src_text);
        let span = Span::new(file, 0, u32::try_from(src_text.len()).unwrap());
        cook_double_quoted_text(map.file(file), span)
    }

    #[test]
    fn named_escapes_cook_to_their_control_characters() {
        let (s, issues) = cook(r#"a\nb\tc\rd\ve\ff\eg\\h\$i\"j"#);
        assert!(issues.is_empty());
        assert_eq!(s, "a\nb\tc\rd\x0Be\x0Cf\x1Bg\\h$i\"j");
    }

    #[test]
    fn plain_text_passes_through_unchanged() {
        let (s, issues) = cook("hello, world");
        assert!(issues.is_empty());
        assert_eq!(s, "hello, world");
    }

    #[test]
    fn octal_escape_cooks_and_wraps_mod_256() {
        let (s, issues) = cook(r"\101\102");
        assert!(issues.is_empty());
        assert_eq!(s, "AB");

        // \400 is 256 decimal, which wraps to 0 mod 256 -- PHP's own
        // documented overflow behavior for this escape.
        let (s, issues) = cook(r"\400");
        assert!(issues.is_empty());
        assert_eq!(s, "\0");
    }

    #[test]
    fn octal_escape_stops_at_three_digits() {
        let (s, issues) = cook(r"\1019");
        assert!(issues.is_empty());
        // \101 is 'A'; the trailing '9' is not part of the escape.
        assert_eq!(s, "A9");
    }

    #[test]
    fn hex_escape_cooks_one_or_two_digits() {
        let (s, issues) = cook(r"\x41\x4");
        assert!(issues.is_empty());
        assert_eq!(s, "A\x04");
    }

    #[test]
    fn bare_hex_escape_with_no_digit_is_left_literal() {
        let (s, issues) = cook(r"\xZ");
        assert!(issues.is_empty());
        assert_eq!(s, "\\xZ");
    }

    #[test]
    fn unicode_escape_cooks_to_utf8() {
        let (s, issues) = cook(r"\u{48}\u{65}\u{1F600}");
        assert!(issues.is_empty());
        assert_eq!(s, "He\u{1F600}");
    }

    #[test]
    fn unicode_escape_out_of_range_is_diagnosed() {
        let (s, issues) = cook(r"a\u{110000}b");
        assert_eq!(s, "ab");
        assert_eq!(issues.len(), 1);
        assert!(matches!(issues[0], CookIssue::InvalidUnicodeEscape(_)));
    }

    #[test]
    fn unicode_escape_surrogate_is_diagnosed() {
        let (s, issues) = cook(r"a\u{D800}b");
        assert_eq!(s, "ab");
        assert_eq!(issues.len(), 1);
        assert!(matches!(issues[0], CookIssue::InvalidUnicodeEscape(_)));
    }

    #[test]
    fn malformed_unicode_escape_passes_through_without_a_second_diagnostic() {
        // No digits at all -- mwl-syntax's lexer already reported
        // `E_INVALID_ESCAPE` for this at lex time; cooking must not panic or
        // double-report.
        let (s, issues) = cook(r"\u{}");
        assert!(issues.is_empty());
        assert_eq!(s, "\\u{}");
    }

    #[test]
    fn byte_escapes_combining_into_invalid_utf8_are_diagnosed() {
        // \xFF alone is never a valid UTF-8 byte on its own.
        let (s, issues) = cook(r"\xFF");
        assert_eq!(issues.len(), 1);
        assert!(matches!(issues[0], CookIssue::InvalidUtf8(_)));
        assert_eq!(s, "\u{FFFD}");
    }

    #[test]
    fn byte_escapes_combining_into_valid_utf8_cook_cleanly() {
        // \xC3\xA9 is 'é' (U+00E9) encoded as UTF-8.
        let (s, issues) = cook(r"\xC3\xA9");
        assert!(issues.is_empty());
        assert_eq!(s, "\u{E9}");
    }

    #[test]
    fn unrecognized_escape_is_left_literal() {
        let (s, issues) = cook(r"\q");
        assert!(issues.is_empty());
        assert_eq!(s, "\\q");
    }
}
