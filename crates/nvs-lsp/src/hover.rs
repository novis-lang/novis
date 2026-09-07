//! What the declaration under the cursor documents.
//!
//! `textDocument/hover` answers three things in
//! `rule:ide/the-request-set-is-closed`'s list, and this module is the first of
//! them: the `///` run attached to the declaration the name under the cursor
//! resolves to, handed to the client as Markdown. The other two — a declared
//! type, and a `Core` member's registry signature row — are their own slices
//! and will be arms beside this one rather than a second module, because they
//! answer the same request at the same cursor.
//!
//! **The walk from the cursor to the declaration is
//! [`crate::definition`]'s.** Hover and go-to-definition ask the same question
//! about *where* and differ only in what they read once they are there, so the
//! resolution is shared: `crate::definition::site` finds the declaration the
//! cursor's name resolves to, and this module takes its `///` run while
//! `definition` takes its span. A second walk here would be a second answer to
//! "where", and the two would disagree the first time one of them learned
//! something.
//!
//! **The run is already on the node, and nothing here rescans trivia.** The
//! parser attaches it while it parses the declaration
//! (`rule:tooling/doc-comment-attaches-to-the-next-declaration`), which is what
//! makes this reachable at all: [`crate::document::Analysed`] keeps the trivia
//! of the **entry** document only, and the declaration a cursor points at is
//! often in a required file. What is read instead is that file's statements,
//! which the graph walk parsed once and kept.
//!
//! **A declaration with no run answers nothing**, rather than an empty popup.
//! `None` is what LSP has for "there is nothing to say here", and an editor
//! shows it as no popup at all; a `Hover` carrying an empty string is a box
//! that opens onto nothing every time a name is passed over.
//!
//! **A `@see` line is prose here, not a link.** ADR 0099 § 3 wants its target
//! rendered as one, and a link needs a URI: the target is a member, and the
//! `file:` URI of its declaration is what a client would follow. That is
//! [`crate::definition`]'s answer pointed at a name written in a comment rather
//! than in code, which is a resolution of its own and so a slice of its own.
//! Until then the tag renders as what was written, which is readable and not
//! wrong.

use lsp_types::{Hover, HoverContents, MarkupContent, MarkupKind};
use nvs_diagnostics::{BytePos, PositionEncoding};
use nvs_syntax::DOC_MARKER;
use nvs_syntax::ast::DocComment;

use crate::definition::{named_at, site};
use crate::document::Analysed;
use crate::position::range_at;

/// What the name at `offset` in the entry document documents.
///
/// `None` when the cursor is inside no node, when the node it is in resolved to
/// no name this can follow, when the name is declared nowhere the analysis
/// reached, and when the declaration carries no `///` run — a `Core` class is
/// the third of those, since it is declared in Rust and documents itself in the
/// registry instead (`rule:core-api/reference-card`).
#[must_use]
pub fn at(analysed: &Analysed, offset: BytePos, encoding: PositionEncoding) -> Option<Hover> {
    let (target, node) = named_at(analysed, offset)?;
    let declared = site(analysed, &target)?;
    let value = markdown(analysed.map.file(declared.span.file).text(), declared.doc?);
    // A run of bare `///` markers is a run with nothing in it, and it takes the
    // same answer as no run at all rather than opening a popup on whitespace.
    if value.trim().is_empty() {
        return None;
    }
    Some(Hover {
        contents: HoverContents::Markup(MarkupContent {
            kind: MarkupKind::Markdown,
            value,
        }),
        // The node the answer came from, so the editor underlines the
        // expression the reader is on. It is in the entry document, which is
        // the only file a cursor is ever in, and never in the file the run was
        // written in.
        range: Some(range_at(analysed.map.file(analysed.entry), node, encoding)),
    })
}

/// One run's prose, as the Markdown a client renders.
///
/// The lines are spans into `text` and carry their markers, which is
/// `nvs_syntax::ast::DocComment`'s decision: the source is the text, and the
/// consumer is what decides where the prose starts.
fn markdown(text: &str, doc: &DocComment) -> String {
    doc.lines
        .iter()
        .map(|line| prose(text.get(line.range()).unwrap_or_default()))
        .collect::<Vec<_>>()
        .join("\n")
}

/// One line with its marker off, and the one space after it that a writer
/// leaves and does not mean.
///
/// Exactly one space and never a trim: indentation is Markdown's own syntax, so
/// eating the rest would turn the indented example a doc comment is often
/// mostly made of into a paragraph. The *trailing* whitespace does go, which
/// costs the two-space hard break — a line ending a `.lspt` expectation in
/// invisible spaces is one nobody can maintain, and a blank line is the break
/// that survives being read.
fn prose(line: &str) -> &str {
    let body = line.strip_prefix(DOC_MARKER).unwrap_or(line);
    body.strip_prefix(' ').unwrap_or(body).trim_end()
}
