//! `rule:errors/record-transformations`'s control-byte and bidi transformations, over one implementation.
//!
//! The table is `rule:tooling/terminal-output-is-a-sink`
//! 's, unchanged, and the bidi rule is
//! `rule:security/bidi-predicate`'s
//! predicate called rather than restated. What `rule:errors/diagnostic-record` adds is *where* they
//! run: on the way into the record, not on the way out of a rendering, so
//! every rendering inherits one answer and none may weaken it.
//!
//! # Why the JSON rendering pays for it too
//!
//! Framing already makes substitution unnecessary for safety in JSON. It is
//! applied anyway because a value must not read differently depending on which
//! rendering someone is looking at, or the renderings stop being views of one
//! record — `rule:errors/record-transformations`'s own paragraph.
//!
//! # What this closes
//!
//! CWE-117 for the plaintext rendering. A human-readable log line is not
//! `"$k=$v"` concatenation here: it renders nodes whose control bytes are
//! already substituted, so a newline inside a tainted value cannot forge an
//! entry. That is the property `rule:errors/log-write` chose JSON Lines to guarantee, preserved as the condition on adding a
//! human-readable target at all.

use std::borrow::Cow;

/// The replacement character an unterminated bidi control and a C1 code point
/// both become — `rule:tooling/terminal-output-is-a-sink`'s table, whose own column says why C1 loses its
/// identity rather than taking a Control Picture.
const REPLACEMENT: char = '\u{FFFD}';

/// `text` with `rule:tooling/terminal-output-is-a-sink`'s substitutions applied: every C0 byte but `LF`
/// and `TAB` as its U+2400-block Control Picture, `DEL` as `␡`, every C1 code
/// point as `�`, and every **unterminated** directional control as `�`.
///
/// A balanced directional control passes through — it is legitimate text, and
/// `rule:security/bidi-predicate`'s whole point is that banning the characters outright breaks
/// Arabic and Hebrew.
///
/// The whole string is one span for `rule:security/bidi-predicate`'s purposes, which is that ADR's
/// *"what a span is, is the caller's decision"*: a record's text node is one
/// value, so a scope it opens has the whole of that value to close in.
///
/// # Why a [`Cow`] rather than a `String`
///
/// `rule:tooling/terminal-output-is-a-sink`'s rule is uniform, so the terminal sink runs this on **every**
/// `echo` — and text with nothing to substitute is nearly all of the output a
/// program writes. Answering the borrow there keeps that path at one scan and
/// no allocation, which is what makes a rule that cannot be switched off
/// affordable on the hot path, where latency is what it would cost. A caller
/// that owns
/// the result regardless — [`crate::Rendered::new`], building a record —
/// takes `.into_owned()` and is exactly where it was.
#[must_use]
pub fn substitute(text: &str) -> Cow<'_, str> {
    // Neither transformation fires on the overwhelmingly common input, and
    // both are a scan. Checking first keeps ordinary text at one pass and no
    // allocation at all.
    let mut unterminated = Vec::new();
    crate::bidi::for_each_unterminated(text, |offset, _| unterminated.push(offset));
    if unterminated.is_empty() && !text.chars().any(needs_substitution) {
        return Cow::Borrowed(text);
    }

    let mut out = String::with_capacity(text.len());
    let mut cuts = unterminated.iter().copied().peekable();
    for (offset, c) in text.char_indices() {
        if cuts.peek() == Some(&offset) {
            cuts.next();
            out.push(REPLACEMENT);
            continue;
        }
        match substituted(c) {
            Some(replacement) => out.push(replacement),
            None => out.push(c),
        }
    }
    Cow::Owned(out)
}

/// Whether `c` is one of the code points [`substituted`] replaces — the
/// cheap half of the scan above, with no allocation behind it.
fn needs_substitution(c: char) -> bool {
    substituted(c).is_some()
}

/// What `c` becomes under `rule:tooling/terminal-output-is-a-sink`'s table, or `None` where it passes
/// through.
///
/// The bidi row is not here: it is a property of the *span*, not of the
/// character, so [`substitute`] applies it from `rule:security/bidi-predicate`'s predicate.
fn substituted(c: char) -> Option<char> {
    match c {
        // Legitimate text layout, and no terminal parses either as an
        // introducer.
        '\n' | '\t' => None,
        // Every other C0, `ESC` and a bare `CR` included — the U+2400 block is
        // one code point per input byte and is laid out to match.
        '\u{0}'..='\u{1F}' => char::from_u32('\u{2400}' as u32 + c as u32),
        '\u{7F}' => Some('\u{2421}'),
        // Several terminals still parse a C1 as a CSI introducer, and the
        // U+2400 block has no glyph for one.
        '\u{80}'..='\u{9F}' => Some(REPLACEMENT),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// `rule:tooling/terminal-output-is-a-sink`'s table, row by row.
    #[test]
    fn every_control_byte_takes_its_row() {
        assert_eq!(substitute("a\u{1B}b"), "a\u{241B}b");
        assert_eq!(substitute("a\rb"), "a\u{240D}b");
        assert_eq!(substitute("a\0b"), "a\u{2400}b");
        assert_eq!(substitute("a\u{7F}b"), "a\u{2421}b");
        assert_eq!(substitute("a\u{9B}b"), "a\u{FFFD}b");
    }

    /// `LF` and `TAB` are the two rows that pass through, so a dump keeps the
    /// layout a value actually has.
    #[test]
    fn newline_and_tab_pass_through() {
        assert_eq!(substitute("a\nb\tc"), "a\nb\tc");
    }

    /// `rule:security/bidi-predicate`'s predicate, not a ban: a balanced scope is legitimate text.
    #[test]
    fn a_balanced_directional_control_passes_through() {
        let text = "\u{2066}user_id\u{2069}";
        assert_eq!(substitute(text), text);
    }

    /// The unterminated one is replaced where it stands, and only it — the
    /// text around it is untouched.
    #[test]
    fn an_unterminated_directional_control_is_replaced() {
        assert_eq!(substitute("a\u{202E}b"), "a\u{FFFD}b");
        assert_eq!(substitute("\u{2066}a\u{202E}b"), "\u{FFFD}a\u{FFFD}b");
    }

    /// Ordinary text is returned unchanged, which is the path a dump takes on
    /// essentially every value.
    #[test]
    fn ordinary_text_is_unchanged() {
        assert_eq!(substitute("hello, world"), "hello, world");
        assert_eq!(substitute("خطأ"), "خطأ");
        assert_eq!(substitute(""), "");
    }

    /// And it is unchanged *without being copied* — the property the terminal
    /// sink depends on, since `rule:tooling/terminal-output-is-a-sink` runs this on every `echo` and
    /// almost every one of them has nothing to substitute. Asserted as the
    /// [`Cow`] variant rather than as a timing, because "no allocation" is
    /// what the borrow means here.
    #[test]
    fn ordinary_text_is_not_copied() {
        assert!(matches!(substitute("hello, world"), Cow::Borrowed(_)));
        assert!(matches!(substitute("a\u{1B}b"), Cow::Owned(_)));
    }
}
