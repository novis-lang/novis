//! Where a `.nvs` file stops being Novis, so the editor's own services can
//! answer inside it.
//!
//! `nvs/regions` is the second request of Novis's own
//! (`rule:ide/the-request-set-is-closed`), beside [`crate::redactions`], and it
//! exists for the reason that one does: the alternative is the client deciding
//! the answer. A `TextDocumentIdentifier` goes in and a list of
//! `{range, language}` comes back, `language` naming which of the editor's
//! built-in services owns those bytes.
//!
//! # Decision: the lexer is asked, never a grammar in the client
//!
//! `rule:ide/a-template-region-gets-services-but-no-second-formatter` puts the
//! list on the server because the lexer already knows where a mode ends:
//! [`TokenKind::InlineHtml`] is one run of text outside any `<?…` tag and its
//! span is exactly the bytes an HTML service may have. A client deriving the
//! same boundaries from a TextMate grammar would be a second implementation of
//! the lexer, disagreeing with the first on every file where the difference
//! mattered — `rule:ide/redaction-ranges-come-from-the-server`'s reasoning,
//! applied to a boundary rather than to a range.
//!
//! # Decision: a lex, and not an analysis
//!
//! [`for_source`] takes a [`SourceFile`] and nothing else. A region is a
//! lexical fact — no name is resolved and no type is asked for — and a client
//! asks this question again every time an edit could have moved a boundary, so
//! answering it out of a full analysis would buy a `require` graph and a type
//! check to report where a `?>` is. [`for_document`] is the same walk over an
//! analysis already in hand, which is what the `.lspt` suite holds.
//!
//! A lexical diagnostic raised on the way is dropped rather than answered:
//! `textDocument/publishDiagnostics` is how a file's problems reach the editor,
//! and a region list that emptied on a broken file would take the services away
//! at exactly the keystroke a half-written tag needs them.
//!
//! # Decision: one region per run, and the client joins them
//!
//! `<?= $name ?>` in the middle of a paragraph splits the markup around it into
//! two runs, and both are answered as two regions rather than one range
//! covering the hole. The hole is Novis and an HTML service asked about it
//! would answer about Novis bytes; joining the runs into one virtual document
//! is the client's job, and it is the client that knows what it wants to put in
//! the gap.
//!
//! # What is not a region yet
//!
//! **A markup literal's body.** ``html`…` `` (`rule:core-classes/html-literal`)
//! is markup written in expression position, and the rule names it a region on
//! the same terms — but the lexer has no token for one, so there is nothing
//! here to report until it does. That lands as one more arm of the filter
//! below rather than as a second mechanism.
//!
//! **Anything at all about formatting.** The regions are what the embedded
//! services are registered over, and they are registered for everything except
//! formatting: `nvs fmt` is the only formatter a `.nvs` file has, and a second
//! one inside it is what that rule's load-bearing half refuses. No answer here
//! can say otherwise — a [`Region`] is a range and a language, which
//! `tests/regions.rs` holds it to, and [`crate::capabilities`] declares no
//! formatting provider for the file around it.
//!
//! # What it spends
//!
//! One token vector over the one document per request, dropped with the answer,
//! and one range per region. Both are O(bytes in the document), and neither
//! outlives the reply.

use nvs_diagnostics::{Diagnostics, PositionEncoding, SourceFile};
use nvs_syntax::{TokenKind, tokenize};

use crate::document::Analysed;
use crate::position::range_at;
use crate::render::Region;

/// The method name this request is asked under.
///
/// Namespaced rather than `textDocument/`-prefixed for
/// [`crate::redactions::METHOD`]'s reason: it is not LSP's, and the prefix is
/// what says which one an editor is looking at.
pub const METHOD: &str = "nvs/regions";

/// The one language answered today.
///
/// An open string on the wire, the way a redaction's kind is: the client maps a
/// spelling to one of the editor's services, and a second markup language adds
/// one here without either end changing shape. A spelling the client does not
/// know forwards nothing, which is the safe direction — the bytes stay Novis's,
/// and Novis already answers for them.
pub const HTML: &str = "html";

/// Every region of `file`, in the order they are written, in `encoding`.
///
/// Empty for a document that is Novis from its first byte, which is most of
/// them, and empty is an answer: a client that is sent nothing holds its last
/// list and would keep forwarding into a region that has since been deleted.
#[must_use]
pub fn for_source(file: &SourceFile, encoding: PositionEncoding) -> Vec<Region> {
    let mut diags = Diagnostics::new();
    tokenize(file, &mut diags)
        .into_iter()
        .filter(|token| token.kind == TokenKind::InlineHtml && token.span.start < token.span.end)
        .map(|token| Region {
            range: range_at(file, token.span, encoding),
            language: HTML.to_owned(),
        })
        .collect()
}

/// The same list for the entry document of an analysis already in hand.
#[must_use]
pub fn for_document(analysed: &Analysed, encoding: PositionEncoding) -> Vec<Region> {
    for_source(analysed.map.file(analysed.entry), encoding)
}

/// One region as the client reads it: a range, a language, and nothing else.
///
/// The wire shape is written here rather than at the call site so it has one
/// home, which is what lets
/// `a_region_answer_carries_a_span_and_a_language_and_nothing_else` read the
/// object the server actually sends. A field added to a [`Region`] and put on
/// the wire is a field that test sees; one that never reaches this function
/// never reaches an editor.
#[must_use]
pub fn wire(region: &Region) -> serde_json::Value {
    serde_json::json!({ "range": region.range, "language": region.language })
}
