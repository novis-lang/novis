//! Where a `.nvs` file stops being Novis, so the editor's own services can
//! answer inside it.
//!
//! `nvs/regions` is the second request of Novis's own
//! (`rule:ide/the-request-set-is-closed`), beside [`crate::redactions`], and it
//! exists for the reason that one does: the alternative is the client deciding
//! the answer. A [`Params`] goes in — a document, and optionally the text to
//! answer about — and a list of `{range, language}` comes back, `language`
//! naming which of the editor's built-in services owns those bytes.
//!
//! # Decision: the lexer is asked, never a grammar in the client
//!
//! `rule:ide/a-template-region-gets-the-editors-services-and-formatter` puts the
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
//! # Decision: the text comes with the question, when the client has one
//!
//! [`Params`] carries an optional `text`, and given one the answer is that
//! text's regions rather than the open buffer's; without one it is the
//! buffer's. Format-on-save asks where the markup is in the output of
//! `nvs fmt`, which is a text no buffer holds yet, and ADR 0173 § 4 refuses
//! both ways of answering it elsewhere: a client lexing that text is the second
//! lexer this request exists to prevent, and `nvs fmt` printing regions is a
//! flag that is not an I/O mode (`rule:tooling/fmt-is-one-canonical-style`).
//! The answer's shape is the same either way, and its ranges are positions in
//! whichever text was lexed — the URI names the document they belong to, not a
//! text to read.
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
//! # Decision: a half-written open tag is a hole too
//!
//! `<?` with nothing after it lexes as markup, because it is not yet one of
//! [`nvs_syntax::OPEN_TAGS`] — and it is the one moment in a run of markup
//! where the developer is writing Novis. [`crate::completion`] offers the tags
//! there, so the HTML service must not answer beside it, and the only way to
//! say so on this wire is a range: the run is cut around the half-written tag,
//! exactly as it is cut around a whole `<?= … ?>`.
//!
//! The hole is the half-written tag **and the one character after it**. A
//! client reads a region as half-open, so a cursor at the end of a hole is on
//! the first byte of the run that follows and would be forwarded; the cursor
//! that matters here is exactly that one, the one still typing the tag. The
//! extra character is almost always the end of the line, and where it is not,
//! the service loses one byte of a document that is mid-edit anyway.
//! [`half_written_tag`] is the one reading of "half-written", and the
//! completion arm asks it too.
//!
//! # Formatting does not cross this wire
//!
//! A client's format pass reads these
//! regions to find the markup it hands its own HTML formatter (ADR 0173 § 1),
//! but nothing about a layout crosses this wire: `nvs fmt` is the only
//! formatter of Novis there is, and no answer here can say otherwise — a
//! [`Region`] is a range and a language, which `tests/regions.rs` holds it to,
//! and [`crate::capabilities`] declares no formatting provider, so the editor
//! reaches the formatter as a process and never as a request.
//!
//! # What it spends
//!
//! One token vector over the one document per request, dropped with the answer,
//! and one range per region. Both are O(bytes in the document), and neither
//! outlives the reply. A request carrying a `text` carries that document once
//! more on the wire and lexes it in place of the buffer, so it costs the one
//! copy and no second walk.
//!
//! # Known gaps
//!
//! 1. **A markup literal's body is not a region.** ``html`…` ``
//!    (`rule:core-classes/html-literal`) is markup written in expression
//!    position, and the rule names it a region on the same terms. The lexer
//!    frames one — [`TokenKind::MarkupOpen`] to [`TokenKind::MarkupClose`] — so
//!    what is left is one more arm of the filter below, which reports
//!    [`TokenKind::InlineHtml`] alone, rather than a second mechanism, with
//!    the literal's two kinds of hole cut out of the region the way a
//!    `<?nvs` block is: [`TokenKind::ComplexInterpOpen`] to
//!    [`TokenKind::ComplexInterpClose`] and [`TokenKind::MarkupEchoOpen`] to
//!    [`TokenKind::MarkupEchoClose`].
//!    — owner: M10

use lsp_types::TextDocumentIdentifier;
use nvs_diagnostics::{BytePos, Diagnostics, PositionEncoding, SourceFile, SourceMap, Span};
use nvs_syntax::{OPEN_TAGS, TokenKind, tokenize};

use crate::document::{Analysed, Documents};
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

/// What one request carries: the document, and the text to answer about when
/// the client holds one the buffer does not.
///
/// `text` is optional and absent means the buffer, so a client that only wants
/// the open document's regions sends what it always sent. The shape is written
/// here, beside the walk it drives, because this is the request's own
/// vocabulary rather than LSP's — the same reason [`METHOD`] is spelled here.
#[derive(Debug, Clone)]
pub struct Params {
    /// The document the ranges are reported against.
    pub text_document: TextDocumentIdentifier,
    /// The text to lex, when it is not what the buffer holds.
    pub text: Option<String>,
}

impl Params {
    /// Read one request's params off the wire.
    ///
    /// Read field by field rather than derived, because deriving it is the
    /// crate's first `serde` derive and this is two fields: a `text` that is
    /// absent, `null` or a string, and the identifier LSP already spells. A
    /// params object missing `textDocument` fails here rather than answering
    /// about a document nobody named.
    ///
    /// # Errors
    ///
    /// The `serde_json` error for params that are not an object, whose
    /// `textDocument` is missing or malformed, or whose `text` is neither a
    /// string nor `null`. The server turns one into `InvalidParams`.
    pub fn from_value(value: serde_json::Value) -> Result<Self, serde_json::Error> {
        let mut fields: serde_json::Map<String, serde_json::Value> = serde_json::from_value(value)?;
        let text: Option<String> = match fields.remove("text") {
            Some(text) => serde_json::from_value(text)?,
            None => None,
        };
        let text_document = serde_json::from_value(
            fields
                .remove("textDocument")
                .unwrap_or(serde_json::Value::Null),
        )?;
        Ok(Self {
            text_document,
            text,
        })
    }
}

/// The regions one request asks for: `params.text`'s if it carries one, and
/// otherwise the open buffer's.
///
/// A URI nothing is open for and no `text` answers nothing, because then there
/// is no text to lex. The lex is over a [`SourceMap`] of this answer's own: a
/// boundary is a lexical fact about the bytes in hand, and the map exists only
/// to give them a [`SourceFile`] to be positions in.
#[must_use]
pub fn for_request(
    documents: &Documents,
    encoding: PositionEncoding,
    params: &Params,
) -> Vec<Region> {
    let uri = &params.text_document.uri;
    let text = match params.text.as_deref() {
        Some(text) => text,
        None => match documents.get(uri) {
            Some(document) => document.text(),
            None => return Vec::new(),
        },
    };
    let mut map = SourceMap::new();
    let id = map.add(uri.as_str(), text);
    for_source(map.file(id), encoding)
}

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
        .filter(|token| token.kind == TokenKind::InlineHtml)
        .flat_map(|token| around_half_written_tags(file, token.span))
        .map(|span| Region {
            range: range_at(file, span, encoding),
            language: HTML.to_owned(),
        })
        .collect()
}

/// The length of the half-written open tag `rest` starts with, or `None` where
/// it starts with none.
///
/// Half-written is `<?` and as much of the code tag's name as has been typed,
/// followed by nothing that could still be part of a tag: `<?`, `<?n` and
/// `<?nv`. `<?=` is a whole tag and the lexer's, and `<?xml` is markup's own
/// processing instruction and stays with the HTML service.
pub(crate) fn half_written_tag(rest: &str) -> Option<usize> {
    let tag = OPEN_TAGS[0];
    (2..tag.len()).rev().find(|&len| {
        rest.starts_with(&tag[..len])
            && !rest[len..]
                .chars()
                .next()
                .is_some_and(|next| next.is_alphanumeric() || next == '_' || next == '=')
    })
}

/// `run`, which is one token of markup, as the pieces of it an HTML service may
/// have: everything but each half-written open tag and the character after it.
///
/// The module doc's *a half-written open tag is a hole too* is the reasoning.
fn around_half_written_tags(file: &SourceFile, run: Span) -> Vec<Span> {
    let text = &file.text()[run.range()];
    let mut pieces = Vec::new();
    let mut from = 0;
    let mut search = 0;
    while let Some(found) = text[search..].find("<?") {
        let at = search + found;
        let Some(len) = half_written_tag(&text[at..]) else {
            search = at + 2;
            continue;
        };
        pieces.push(from..at);
        let after = text[at + len..].chars().next().map_or(0, char::len_utf8);
        from = at + len + after;
        search = from;
    }
    pieces.push(from..text.len());
    pieces
        .into_iter()
        .filter(|piece| piece.start < piece.end)
        .map(|piece| Span {
            start: run.start + BytePos::try_from(piece.start).expect("inside the run"),
            end: run.start + BytePos::try_from(piece.end).expect("inside the run"),
            ..run
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
