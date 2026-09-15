//! The HTML sink's transform — `rule:core-classes/html-auto-escape`'s escape,
//! as a table beside [`crate::text::substitute`]'s.
//!
//! The two are peers and are written as peers: each is the whole of what one
//! sink does to text on the way in, each answers a borrow for the input that
//! needs nothing done to it, and each is called rather than restated by
//! everything that writes through that sink. `nvs_runtime`'s `echo` picks
//! between them from the sink in force, and `Core\Html::escape` is a caller of
//! this one rather than a second copy of it — which is what makes the
//! launderer and the sink incapable of disagreeing about what an `&` becomes.
//!
//! **This is not `rule:errors/renderings`' HTML rendering of a [`Record`].**
//! That one draws a diagnostic, a dump or a log record as markup and is this
//! crate's own gap 1; this is five characters and a bidi rule over plain text,
//! and it is what such a rendering would have to write *through*.
//!
//! [`Record`]: crate::Record

use std::borrow::Cow;

/// What each of the five characters is written as.
///
/// A `match` rather than a table because it is the whole of the transformation
/// and the compiler turns it into one: five arms over an ASCII byte.
fn escaped(c: char) -> Option<&'static str> {
    match c {
        // First, and the only one whose escape is not about a delimiter: an
        // unescaped `&` makes every other reference in the output ambiguous.
        '&' => Some("&amp;"),
        '<' => Some("&lt;"),
        '>' => Some("&gt;"),
        '"' => Some("&quot;"),
        // Not `&apos;` — that name is XML's and HTML 4 never defined it.
        '\'' => Some("&#39;"),
        _ => None,
    }
}

/// What an unterminated directional control becomes.
///
/// The same replacement [`crate::text`] uses for it, and deliberately not a
/// character reference: the control is being *removed*, not shown, and
/// `&#8235;` in the output would be an escaped payload rather than a neutral
/// one.
const REPLACEMENT: char = '\u{FFFD}';

/// `text` with `&`, `<`, `>`, `"` and `'` written as character references and
/// every unterminated bidirectional control replaced.
///
/// # Why a [`Cow`] rather than a `String`
///
/// [`crate::text::substitute`]'s reason one sink over, and a stronger one:
/// `rule:core-classes/html-auto-escape` makes this the HTML sink's *only*
/// behaviour, so every non-carrier value written into a response passes
/// through here whether or not it is tainted. Text with nothing to escape is
/// therefore not an edge case but most of a page, and answering the borrow
/// keeps it at one scan and no allocation — which is what makes a rule that
/// cannot be switched off affordable on the request path (`AGENTS.md`'s
/// priority 3).
#[must_use]
pub fn escape(text: &str) -> Cow<'_, str> {
    // Both halves are a scan and neither fires on ordinary text, so they are
    // asked before anything is allocated.
    let mut unterminated = Vec::new();
    crate::bidi::for_each_unterminated(text, |offset, _| unterminated.push(offset));
    if unterminated.is_empty() && !text.chars().any(|c| escaped(c).is_some()) {
        return Cow::Borrowed(text);
    }

    // Every escape is longer than what it replaces, so the input's length is a
    // floor and never a wasted reservation.
    let mut out = String::with_capacity(text.len());
    let mut cuts = unterminated.into_iter().peekable();
    for (offset, c) in text.char_indices() {
        if cuts.peek() == Some(&offset) {
            cuts.next();
            out.push(REPLACEMENT);
            continue;
        }
        match escaped(c) {
            Some(reference) => out.push_str(reference),
            None => out.push(c),
        }
    }
    Cow::Owned(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The five characters, in one string rather than one assertion each, so a
    /// table that lost a row fails here rather than passing five lines that
    /// each ask about something else.
    #[test]
    fn every_delimiter_becomes_a_character_reference() {
        assert_eq!(escape("&<>\"'"), "&amp;&lt;&gt;&quot;&#39;");
    }

    /// The transformation, over the boundary each of the five characters sits
    /// on: a character reference is produced for every one of them and for
    /// nothing else in ASCII.
    ///
    /// Counted rather than read off five lines, so a table that escaped a
    /// sixth character — `/`, which several PHP escapers add — fails here.
    /// Asked through [`escape`] rather than of [`escaped`], so it is the
    /// transform callers reach that is being counted.
    #[test]
    fn exactly_five_ascii_characters_are_escaped() {
        let escaped_set: Vec<char> = (0u8..128)
            .map(char::from)
            .filter(|c| escape(&c.to_string()) != c.to_string())
            .collect();
        assert_eq!(escaped_set, vec!['"', '&', '\'', '<', '>']);
    }

    /// The borrow is the point of the signature, so it is asserted rather than
    /// left to the type: a copy taken on every ordinary write would still pass
    /// every other test in this file.
    #[test]
    fn ordinary_text_is_answered_as_the_borrow_it_arrived_as() {
        assert!(matches!(escape("an ordinary paragraph"), Cow::Borrowed(_)));
        assert!(matches!(escape("a & b"), Cow::Owned(_)));
    }

    /// The bidi half, which is the reason this is not five `replace` calls: an
    /// override with no terminator is written out rather than escaped, because
    /// what it does is reorder everything after it and a character reference
    /// would carry the payload through.
    #[test]
    fn an_unterminated_bidi_control_is_replaced_and_not_referenced() {
        assert_eq!(escape("a\u{202E}b"), "a\u{FFFD}b");
    }

    /// Escaping is not idempotent, and that is the property the carrier exists
    /// to protect: text that already reads as a reference is escaped again,
    /// because the author's data said `&amp;`.
    #[test]
    fn a_reference_in_the_input_is_escaped_again() {
        assert_eq!(escape("&amp;"), "&amp;amp;");
    }
}
