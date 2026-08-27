//! [ADR 0087](../../../docs/adr/0087-unbalanced-bidi-is-rejected-at-every-boundary.md)'s
//! predicate: a bidirectional control that opens a directional scope and never
//! closes it inside the span that opened it.
//!
//! **One rule, three callers.** The lexer calls it per source span and makes a
//! failure a hard compile error; `Core\Html::escape` (M7) and `Core\Cli`'s
//! output sink (M8) call it and substitute `U+FFFD` for each control it names.
//! None of the three restates the rule — that is the whole reason the predicate
//! lives here rather than beside the lexer's string handling.
//!
//! **What a span is, is the caller's decision**, per that ADR § 2's table: the
//! lexer passes one *line* of a token, so a heredoc cannot open a scope on one
//! line and close it on the next; a sink passes one whole write. Per-line is
//! strictly the stronger reading — a text whose every line balances balances as
//! a whole — so this module needs no mode flag to serve both.
//!
//! The model is deliberately simpler than [UAX #9](https://www.unicode.org/reports/tr9/):
//! two independent scopes, embeddings/overrides and isolates, each a stack that
//! an opener pushes and its own terminator pops (popping nothing when empty, so
//! a stray `PDF`/`PDI` closes nothing and is not an error). The question here is
//! not how a span renders — that needs the real algorithm — but whether it ends
//! with a scope open, which the stacks answer soundly. The simplification can
//! only over-approximate towards rejecting.
//!
//! `LRM`/`RLM` (U+200E/U+200F) are marks rather than scopes and are not covered:
//! they open nothing, so they cannot be unterminated. Neither are zero-width and
//! invisible characters, nor homoglyphs, each for the reason that ADR § 4 gives.

/// Embedding and override openers, closed by `PDF` (U+202C).
const LRE: char = '\u{202A}';
const RLE: char = '\u{202B}';
const PDF: char = '\u{202C}';
const LRO: char = '\u{202D}';
const RLO: char = '\u{202E}';

/// Isolate openers, closed by `PDI` (U+2069).
const LRI: char = '\u{2066}';
const RLI: char = '\u{2067}';
const FSI: char = '\u{2068}';
const PDI: char = '\u{2069}';

/// The Unicode name of one of the seven openers, for a diagnostic that has to
/// name a character the reader cannot see.
#[must_use]
pub fn control_name(c: char) -> &'static str {
    match c {
        LRE => "LEFT-TO-RIGHT EMBEDDING",
        RLE => "RIGHT-TO-LEFT EMBEDDING",
        LRO => "LEFT-TO-RIGHT OVERRIDE",
        RLO => "RIGHT-TO-LEFT OVERRIDE",
        LRI => "LEFT-TO-RIGHT ISOLATE",
        RLI => "RIGHT-TO-LEFT ISOLATE",
        FSI => "FIRST STRONG ISOLATE",
        _ => "BIDIRECTIONAL CONTROL",
    }
}

/// The terminator the caller must write to close `c`, for the same reason.
#[must_use]
pub fn terminator_of(c: char) -> &'static str {
    match c {
        LRI | RLI | FSI => "U+2069 POP DIRECTIONAL ISOLATE",
        _ => "U+202C POP DIRECTIONAL FORMATTING",
    }
}

/// Calls `f` with the byte offset and character of every directional control in
/// `text` that is still open when `text` ends, in source order.
///
/// This is the whole rule. [`first_unterminated`] is the lexer's view of it;
/// a sink that substitutes wants every offset, which is this.
pub fn for_each_unterminated(text: &str, mut f: impl FnMut(usize, char)) {
    let (embeddings, isolates) = scan(text);
    // Both stacks are already in ascending offset order, so one merge walk
    // reports the whole set in source order without sorting.
    let (mut e, mut i) = (
        embeddings.into_iter().peekable(),
        isolates.into_iter().peekable(),
    );
    loop {
        let take_embedding = match (e.peek(), i.peek()) {
            (Some((eo, _)), Some((io, _))) => eo < io,
            (Some(_), None) => true,
            (None, Some(_)) => false,
            (None, None) => return,
        };
        let (offset, c) = if take_embedding {
            e.next().expect("peeked")
        } else {
            i.next().expect("peeked")
        };
        f(offset, c);
    }
}

/// The first control in `text` left open at its end, or `None` when every scope
/// the span opened is closed inside it.
#[must_use]
pub fn first_unterminated(text: &str) -> Option<(usize, char)> {
    let (embeddings, isolates) = scan(text);
    match (embeddings.first(), isolates.first()) {
        (Some(&e), Some(&i)) => Some(if e.0 <= i.0 { e } else { i }),
        (Some(&e), None) => Some(e),
        (None, Some(&i)) => Some(i),
        (None, None) => None,
    }
}

/// The openers of one kind of scope that a span left open, in the order the
/// span wrote them: each entry is a byte offset and the control itself.
type Unclosed = Vec<(usize, char)>;

/// The two scope stacks left over after walking `text`: the embeddings and
/// overrides, and the isolates. Each holds the openers that were never popped,
/// in the order they were opened.
fn scan(text: &str) -> (Unclosed, Unclosed) {
    let mut embeddings = Vec::new();
    let mut isolates = Vec::new();
    // Every one of the nine code points is `U+2000`-block, so its UTF-8 form
    // begins `0xE2`. A span without that byte cannot contain one, which is
    // almost every span this is called on.
    if !text.as_bytes().contains(&0xE2) {
        return (embeddings, isolates);
    }
    for (offset, c) in text.char_indices() {
        match c {
            LRE | RLE | LRO | RLO => embeddings.push((offset, c)),
            PDF => {
                embeddings.pop();
            }
            LRI | RLI | FSI => isolates.push((offset, c)),
            PDI => {
                isolates.pop();
            }
            _ => {}
        }
    }
    (embeddings, isolates)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn all(text: &str) -> Vec<(usize, char)> {
        let mut out = Vec::new();
        for_each_unterminated(text, |offset, c| out.push((offset, c)));
        out
    }

    #[test]
    fn a_balanced_scope_is_accepted() {
        assert_eq!(first_unterminated("plain ascii"), None);
        assert_eq!(first_unterminated("a\u{202A}b\u{202C}c"), None);
        assert_eq!(first_unterminated("a\u{2067}b\u{2069}c"), None);
        // Nested, and the two kinds interleaved but each closed.
        assert_eq!(
            first_unterminated("\u{202B}a\u{2066}b\u{2069}c\u{202C}"),
            None
        );
    }

    #[test]
    fn each_opener_left_open_is_rejected() {
        for opener in [LRE, RLE, LRO, RLO, LRI, RLI, FSI] {
            let text = format!("before{opener}after");
            assert_eq!(
                first_unterminated(&text),
                Some((6, opener)),
                "{opener:?} left open was accepted"
            );
        }
    }

    #[test]
    fn a_terminator_closes_only_its_own_kind() {
        // `PDI` does not close an embedding, and `PDF` does not close an
        // isolate -- the two stacks are independent.
        assert_eq!(first_unterminated("\u{202A}x\u{2069}"), Some((0, LRE)));
        assert_eq!(first_unterminated("\u{2066}x\u{202C}"), Some((0, LRI)));
    }

    #[test]
    fn a_stray_terminator_is_accepted() {
        // It closes nothing and opens nothing, so there is no scope left open.
        assert_eq!(first_unterminated("a\u{202C}b"), None);
        assert_eq!(first_unterminated("a\u{2069}b"), None);
        assert_eq!(first_unterminated("\u{202C}\u{2069}\u{202C}"), None);
    }

    #[test]
    fn a_mark_is_not_a_scope() {
        assert_eq!(first_unterminated("\u{200E}\u{200F}\u{200E}"), None);
    }

    #[test]
    fn every_unterminated_control_is_reported_in_source_order() {
        // An isolate opened before an embedding, neither closed: the callback
        // sees them in the order the text writes them, not stack by stack.
        let text = "\u{2066}a\u{202E}b";
        assert_eq!(all(text), vec![(0, LRI), (4, RLO)]);
        assert_eq!(all("\u{202E}a\u{2066}b"), vec![(0, RLO), (4, LRI)]);
    }

    #[test]
    fn balanced_arabic_survives() {
        // The case that fails if the rule is ever "simplified" to a ban.
        let text = "خطأ: \u{2066}user_id\u{2069} غير موجود";
        assert_eq!(first_unterminated(text), None);
        assert!(all(text).is_empty());
    }
}
