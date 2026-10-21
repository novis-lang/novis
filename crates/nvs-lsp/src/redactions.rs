//! Which bytes of one document the editor conceals, and why.
//!
//! `nvs/redactions` is the one request of Novis's own in
//! `rule:ide/the-request-set-is-closed`'s list, and it exists because the
//! alternative is the client deciding what a secret is
//! (`rule:ide/redaction-ranges-come-from-the-server`). A
//! `TextDocumentIdentifier` goes in and a list of `{range, kind}` comes back,
//! `kind` being an open string, so a later qualifier adds a spelling.
//!
//! # Decision: two kinds on one list, not a list per kind
//!
//! [`SECRET_LITERAL`] is bytes to conceal and [`TAINTED_DECLARATION`] is a name
//! to mark, and they are not the same instruction: one is drawn by default
//! because a credential on a shared screen is an incident, and the other only
//! where `nvs.taint.mark` asks for it
//! (`rule:ide/tainted-has-no-default-decoration`). They travel on one
//! request because the set is closed and an open `kind` is the room it leaves,
//! and on one *list* because a client that reads a field per kind must be
//! taught a new field for every later qualifier, while a client reading kinds
//! already carries them.
//!
//! What that costs the client is one rule, and it runs in the safe direction: a
//! kind it does not recognise is concealed, and only the spellings it knows to
//! be markers are exempt. An unknown spelling then covers bytes that needed no
//! covering, rather than leaving bytes uncovered that did.
//!
//! It is **not** the semantic-token channel with a modifier on it, even though
//! [`crate::semantic`] already carries `secret` there. That channel degrades by
//! falling back to the underlying token type, and a default whose failure mode
//! is *the value becomes visible* cannot inherit that degradation. Two
//! mechanisms, two contracts.
//!
//! # Decision: the walk is `nvs_types::redaction`'s
//!
//! Which bytes are a `secret` value is [`nvs_types::redaction::secret_literals`],
//! and its module doc holds why a literal is attributed through the binding
//! it is written into. The compile path asks the same walk for the snippets
//! a diagnostic prints, so the editor and the terminal conceal the same bytes.
//!
//! # What it does not reach
//!
//! **A parameter and a property, for the marker.** [`TAINTED_DECLARATION`] is
//! answered off the bindings the checker recorded, which are a body's locals;
//! a parameter's name is written outside the body's scope and a property's is
//! not a binding at all, so neither carries a glyph. That is the same boundary
//! [`crate::semantic`] crosses with `qualifiers_declared` and
//! `qualifiers_recorded`, and closing it here means answering those two
//! questions at a *declaration* rather than at a use.
//!
//! **A sink's argument position.** The third value of `nvs.taint.mark`
//! (`rule:ide/tainted-has-no-default-decoration`) needs
//! `rule:security/sink-predicate`'s classification on the member rows before
//! there is anything to answer, and whether it is built at all is open. Until
//! then `nvs.taint.mark = sink` marks what `declaration` does.
//!
//! **Everything past the editor's own decorations.**
//! `rule:ide/redaction-does-not-reach` is the list, and it is part of the
//! decision rather than a caveat on it.
//!
//! # What it spends
//!
//! One walk tree over the entry document per request — the same shape
//! [`nvs_syntax::SyntaxIndex`] builds and drops, built a second time by
//! [`nvs_types::redaction`] rather than cached, because an analysis rebuilds
//! the tree either way (`rule:ide/a-full-reanalysis-stays-under-a-bound`).
//! Then one [`Span`] per concealed range. Both are O(nodes in the entry
//! document) and both are dropped with the answer. If that walk ever shows up
//! in the bound, the move is the one `nvs_syntax::index`'s own doc names: emit
//! the nodes to a sink the consumers share, not to match the grammar a second
//! time.

use nvs_diagnostics::{PositionEncoding, Span};
use nvs_types::expr::quals::is_tainted;
use nvs_types::redaction::secret_literals;

use crate::document::Analysed;
use crate::position::range_at;
use crate::render::Redaction;

/// The method name this request is asked under.
///
/// Namespaced rather than `textDocument/`-prefixed because it is not LSP's:
/// `rule:ide/the-request-set-is-closed` admits exactly one request of Novis's
/// own, and the prefix is what says at a glance which one an editor is looking
/// at.
pub const METHOD: &str = "nvs/redactions";

/// The one kind answered today.
///
/// An open string on the wire, so a later qualifier adds a
/// spelling here without either end changing shape.
pub const SECRET_LITERAL: &str = "secretLiteral";

/// A declaration whose binding carries `tainted`, marked and never concealed.
///
/// The `declaration` value of `nvs.taint.mark`, answered: the range is the name the
/// binding was declared under, and the client draws a glyph after it where
/// `nvs.taint.mark` asks and nothing at all where it does not
/// (`rule:ide/tainted-has-no-default-decoration`). It is answered whatever
/// that setting says, because the server holds no window's configuration and an
/// unread range costs a client nothing.
///
/// **Concealing one would be a bug**, and a visible one: a name is not a secret
/// and `rule:ide/redaction-covers-bytes-only` refuses covering it. The
/// client's rule is in this module's own decision above.
pub const TAINTED_DECLARATION: &str = "taintedDeclaration";

/// Every range of the entry document of `analysed` the client draws over, in
/// `encoding`.
///
/// Sorted by where a range starts, which is the order they were written —
/// stated rather than inherited from the walk, on [`crate::links::for_document`]'s
/// terms: a `.lspt` case freezes this list and it should read as the document
/// does. Both kinds sort together for that reason: the list reads down the
/// file, and a client that wants one of them filters for it.
///
/// Empty for a document whose analysis reached no statements, which is the
/// answer [`crate::semantic::for_document`] gives for the same reason. Empty
/// means *nothing to conceal here*; a client that is sent no answer at all
/// holds the last one it had, and the two must not be confused
/// (`rule:ide/redaction-ranges-come-from-the-server`).
#[must_use]
pub fn for_document(analysed: &Analysed, encoding: PositionEncoding) -> Vec<Redaction> {
    let Some(loaded) = analysed
        .loaded
        .iter()
        .find(|loaded| loaded.id == analysed.entry)
    else {
        return Vec::new();
    };
    let file = analysed.map.file(analysed.entry);
    let mut answered: Vec<(Span, &'static str)> =
        secret_literals(file, &loaded.stmts, &analysed.exprs, &analysed.interner)
            .into_iter()
            .map(|span| (span, SECRET_LITERAL))
            .chain(
                tainted_declarations(analysed)
                    .into_iter()
                    .map(|span| (span, TAINTED_DECLARATION)),
            )
            .collect();
    answered.sort_by_key(|(span, kind)| (span.start, span.end, *kind));
    answered.dedup();
    answered
        .into_iter()
        .map(|(span, kind)| Redaction {
            range: range_at(file, span, encoding),
            kind: kind.to_owned(),
        })
        .collect()
}

/// The name every `tainted` binding of the entry document was declared under.
///
/// The entry's alone, unlike [`secret_bindings`]: these spans are *answered*
/// rather than asked a containment question, and a range in a file the client
/// did not ask about names bytes its document does not have.
///
/// A binding and not a use, which is the whole of what
/// `rule:ide/tainted-has-no-default-decoration` asks for: a
/// `tainted` value is tainted at every use ([`crate::semantic`] already carries
/// the modifier there), so a glyph per use would mark most of a request handler
/// and teach nothing. The declaration is the one place the qualifier was
/// written down.
fn tainted_declarations(analysed: &Analysed) -> Vec<Span> {
    analysed
        .exprs
        .local_scopes()
        .flat_map(|(_, locals)| locals.iter())
        .filter(|local| {
            local.declared.file == analysed.entry && is_tainted(local.ty, &analysed.interner)
        })
        .map(|local| local.declared)
        .collect()
}
