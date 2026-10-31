//! Text into XML: the escapes every part needs, and the one Excel adds for characters XML 1.0
//! cannot carry.
//!
//! [`text`] writes a cell's or a part's text. `&`, `<` and `>` are entities. A character XML 1.0
//! forbids (a C0 control other than tab and line feed, and U+FFFE and U+FFFF) is Excel's
//! `_xHHHH_` escape, and so is a carriage return, which an XML parser would fold into a line feed.
//! An underscore that would begin such an escape is written `_x005F_`, so text that holds
//! `_x0041_` reads back as itself. [`attr`] writes an attribute value, which adds `"`.

use std::fmt::Write as _;

/// Whether `text` needs `xml:space="preserve"` to keep its leading or trailing whitespace.
pub fn needs_preserve(text: &str) -> bool {
    let edge = |c: char| matches!(c, ' ' | '\t' | '\n' | '\r');
    text.starts_with(edge) || text.ends_with(edge)
}

/// Appends `value` as element text.
pub fn text(out: &mut String, value: &str) {
    escape(out, value, false);
}

/// Appends `value` as an attribute's text, inside double quotes the caller writes.
pub fn attr(out: &mut String, value: &str) {
    escape(out, value, true);
}

fn escape(out: &mut String, value: &str, quote: bool) {
    let bytes = value.as_bytes();
    let mut start = 0;
    for (at, c) in value.char_indices() {
        let replace: Option<&str> = match c {
            '&' => Some("&amp;"),
            '<' => Some("&lt;"),
            '>' => Some("&gt;"),
            '"' if quote => Some("&quot;"),
            '_' if starts_escape(&bytes[at..]) => Some("_x005F_"),
            _ => None,
        };
        if let Some(entity) = replace {
            out.push_str(&value[start..at]);
            out.push_str(entity);
            start = at + c.len_utf8();
        } else if forbidden(c) {
            out.push_str(&value[start..at]);
            let _ = write!(out, "_x{:04X}_", u32::from(c));
            start = at + c.len_utf8();
        }
    }
    out.push_str(&value[start..]);
}

/// Appends `value` as the text of an ods element, or of an attribute when `quote` is set. ODF
/// has no escape for a character XML 1.0 forbids, so such a character, and a carriage return,
/// is written as U+FFFD, the replacement character.
pub fn odf(out: &mut String, value: &str, quote: bool) {
    let mut start = 0;
    for (at, c) in value.char_indices() {
        let replace = match c {
            '&' => "&amp;",
            '<' => "&lt;",
            '>' => "&gt;",
            '"' if quote => "&quot;",
            c if forbidden(c) => "\u{fffd}",
            _ => continue,
        };
        out.push_str(&value[start..at]);
        out.push_str(replace);
        start = at + c.len_utf8();
    }
    out.push_str(&value[start..]);
}

/// A character XML 1.0 cannot carry, or a carriage return.
fn forbidden(c: char) -> bool {
    matches!(c, '\u{0}'..='\u{8}' | '\u{b}' | '\u{c}' | '\r' | '\u{e}'..='\u{1f}' | '\u{fffe}' | '\u{ffff}')
}

/// Whether `rest`, which begins with `_`, is an `_xHHHH_` escape.
fn starts_escape(rest: &[u8]) -> bool {
    rest.len() >= 7
        && rest[1] == b'x'
        && rest[2..6].iter().all(u8::is_ascii_hexdigit)
        && rest[6] == b'_'
}

#[cfg(test)]
mod tests {
    use super::*;

    fn escaped(value: &str) -> String {
        let mut out = String::new();
        text(&mut out, value);
        out
    }

    #[test]
    fn markup_controls_and_excel_escapes_are_escaped() {
        assert_eq!(escaped("a<b & c>"), "a&lt;b &amp; c&gt;");
        assert_eq!(escaped("bell\u{7}"), "bell_x0007_");
        assert_eq!(escaped("a\r\nb\tc"), "a_x000D_\nb\tc");
        assert_eq!(escaped("_x0041_ and _x"), "_x005F_x0041_ and _x");
        assert_eq!(escaped("Grüße"), "Grüße");
    }

    #[test]
    fn whitespace_at_either_end_is_preserved() {
        assert!(needs_preserve(" a"));
        assert!(needs_preserve("a\n"));
        assert!(!needs_preserve("a b"));
    }
}
