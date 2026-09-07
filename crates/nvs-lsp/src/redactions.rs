//! Which bytes of one document the editor conceals, and why.
//!
//! `nvs/redactions` is the one request of Novis's own in
//! `rule:ide/the-request-set-is-closed`'s list, and it exists because the
//! alternative is the client deciding what a secret is
//! (`rule:security/redaction-ranges-come-from-the-server`). A
//! `TextDocumentIdentifier` goes in and a list of `{range, kind}` comes back,
//! `kind` being the open string [`SECRET_LITERAL`] today.
//!
//! It is **not** the semantic-token channel with a modifier on it, even though
//! [`crate::semantic`] already carries `secret` there. That channel degrades by
//! falling back to the underlying token type, and a default whose failure mode
//! is *the value becomes visible* cannot inherit that degradation. Two
//! mechanisms, two contracts.
//!
//! # Decision: this is a projection of the one walk, not a second traversal
//!
//! [`crate::semantic`] matches [`nvs_syntax::ast`] itself, and its module doc
//! names what that costs: those enums are `#[non_exhaustive]`, so a match in
//! this crate needs a wildcard and a production landing later emits nothing and
//! fails no build. It pays that because it needs a *name's* span, which
//! [`nvs_syntax::walk`] deliberately does not model.
//!
//! This walk needs no name. It needs a literal's span, an interpolation slot's
//! span, and which production contains them — exactly what a [`Node`] carries.
//! So it reads `nvs_syntax::walk::of_stmts`, whose match is written beside the
//! grammar and is exhaustive, and a new production is a build error in the file
//! its author is already in rather than a range that quietly stops being
//! concealed. For a security default that is the whole difference: silently
//! covering less is the failure this request exists to prevent.
//!
//! # Decision: a literal is attributed through the binding it is written into
//!
//! `rule:security/redaction-covers-bytes-only` answers a range when the literal
//! token's static type carries `secret`. A literal's own type never does — the
//! qualifier reaches it from the binding it flows into — and mid-edit there is
//! often no type at all, which is exactly when a value must not flash on
//! screen. So the question asked here is ADR 0101 § 1's fail direction, asked
//! everywhere rather than only on failure: **does the binding this value is
//! written into carry `secret`?**
//!
//! - A `LocalDecl` declares a binding, and the checker recorded every binding
//!   of every body with the span it was declared at
//!   ([`nvs_types::LocalBinding`]). A declaration whose statement contains one
//!   of the `secret` ones conceals its initializer. That survives an
//!   initializer that does not check, because the binding's type comes from the
//!   declaration rather than from the value.
//! - An assignment conceals its value when its *target* carries `secret` — a
//!   variable through [`Analysed::local_ty`], a property through the entry the
//!   checker recorded against the access, which is how [`crate::semantic`] asks
//!   the same question.
//!
//! **The value is followed only through the productions that carry it**:
//! parentheses, a ternary's branches, and `.` and `??`, which are the
//! compositions the checker itself spreads the qualifier across. A literal
//! anywhere else under a `secret` declaration — an argument to the call that
//! produced the value, a key in an array being indexed — is a byte of the
//! program rather than a byte of the secret, and concealing it would be the
//! over-redaction ADR 0101 § 1 refuses.
//!
//! An interpolation slot is the other half, and it is answered wherever it is
//! written: `"prefix $key suffix"` conceals `$key` even in a statement that
//! refuses to compile, because a program that misuses a secret is still a
//! program whose secret is on screen.
//!
//! # What it does not reach
//!
//! **A parameter's default and a property declaration's default.** Neither
//! declaration is a binding the checker records a span for, and
//! [`nvs_syntax::walk`] gives a function's default the same shape as its body's
//! statements, so there is nothing here to attribute the literal through. The
//! property half is the same gap [`crate::semantic`]'s module doc names, and
//! closing it is a `nvs-types` change rather than one here.
//!
//! **Everything past the editor's own decorations.**
//! `rule:security/redaction-does-not-reach` is the list, and it is part of the
//! decision rather than a caveat on it.
//!
//! # What it spends
//!
//! One [`Node`] tree over the entry document per request — the same shape
//! [`nvs_syntax::SyntaxIndex`] builds and drops, built a second time here
//! rather than cached, because an analysis rebuilds the tree either way
//! (`rule:ide/a-full-reanalysis-stays-under-a-bound`). Then one [`Span`] per
//! concealed range. Both are O(nodes in the entry document) and both are
//! dropped with the answer. If that walk ever shows up in the bound, the move
//! is the one `nvs_syntax::index`'s own doc names: emit the nodes to a sink the
//! consumers share, not to match the grammar a second time here.

use nvs_diagnostics::{PositionEncoding, Span};
use nvs_syntax::walk::{self, Field, Node};
use nvs_types::ExprInfo;
use nvs_types::TypeId;
use nvs_types::expr::quals::is_secret;

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
/// An open string on the wire (ADR 0101 § 1), so a later qualifier adds a
/// spelling here without either end changing shape.
pub const SECRET_LITERAL: &str = "secretLiteral";

/// Every range of the entry document of `analysed` the client conceals, in
/// `encoding`.
///
/// Sorted by where a range starts, which is the order they were written —
/// stated rather than inherited from the walk, on [`crate::links::for_document`]'s
/// terms: a `.lspt` case freezes this list and it should read as the document
/// does.
///
/// Empty for a document whose analysis reached no statements, which is the
/// answer [`crate::semantic::for_document`] gives for the same reason. Empty
/// means *nothing to conceal here*; a client that is sent no answer at all
/// holds the last one it had, and the two must not be confused
/// (`rule:security/redaction-ranges-come-from-the-server`).
#[must_use]
pub fn for_document(analysed: &Analysed, encoding: PositionEncoding) -> Vec<Redaction> {
    let Some(loaded) = analysed
        .loaded
        .iter()
        .find(|loaded| loaded.id == analysed.entry)
    else {
        return Vec::new();
    };
    let mut concealing = Concealing {
        analysed,
        declared: secret_bindings(analysed),
        spans: Vec::new(),
    };
    for node in &walk::of_stmts(&loaded.stmts) {
        concealing.node(node, false);
    }
    let file = analysed.map.file(analysed.entry);
    let mut spans = concealing.spans;
    spans.sort_by_key(|span| (span.start, span.end));
    spans.dedup();
    spans
        .into_iter()
        .map(|span| Redaction {
            range: range_at(file, span, encoding),
            kind: SECRET_LITERAL.to_owned(),
        })
        .collect()
}

/// Where every `secret` binding the checker recorded was declared.
///
/// The whole analysis's, not the entry's alone: a [`Span`] names its own file,
/// so containment answers which document a binding was declared in without a
/// filter that would have to be kept true.
fn secret_bindings(analysed: &Analysed) -> Vec<Span> {
    analysed
        .exprs
        .local_scopes()
        .flat_map(|(_, locals)| locals.iter())
        .filter(|local| is_secret(local.ty, &analysed.interner))
        .map(|local| local.declared)
        .collect()
}

/// One walk, and what it has decided to conceal so far.
struct Concealing<'a> {
    /// What the checker resolved about this document.
    ///
    /// Not optional, unlike [`crate::semantic`]'s: a walk over a parse alone
    /// knows no qualifier, and a redaction walk that knew none would answer
    /// nothing at all rather than answering less.
    analysed: &'a Analysed,
    /// Where each `secret` binding was declared, from [`secret_bindings`].
    declared: Vec<Span>,
    /// The ranges concealed, in the order the walk found them.
    spans: Vec<Span>,
}

impl Concealing<'_> {
    /// Walks one node, `carrying` saying whether the value it produces is
    /// written into a `secret` binding.
    fn node(&mut self, node: &Node, carrying: bool) {
        match node.kind {
            "Str" => {
                if carrying {
                    self.spans.push(node.span);
                }
            }
            "Interpolated" => {
                if carrying {
                    // The whole literal, its slots included: they are inside
                    // the range already, and answering them again would be two
                    // decorations over the same bytes.
                    self.spans.push(node.span);
                } else {
                    for slot in &node.children {
                        if self.carries_secret(slot) {
                            self.spans.push(slot.span);
                        }
                    }
                }
                self.children(node, false, 0);
            }
            "LocalDecl" => {
                let secret = self.declares_secret(node.span);
                self.children(node, secret, 0);
            }
            "Assign" => {
                let secret = node
                    .children
                    .first()
                    .is_some_and(|target| self.carries_secret(target));
                // The target names a binding rather than producing a value, so
                // a literal inside it — an index, a computed property name — is
                // never the secret being stored.
                self.children(node, secret, 1);
            }
            _ => {
                let (carries, skip) = carrier(node);
                self.children(node, carrying && carries, skip);
            }
        }
    }

    /// Walks `node`'s children, carrying into all but the first `skip` of them.
    fn children(&mut self, node: &Node, carrying: bool, skip: usize) {
        for (at, child) in node.children.iter().enumerate() {
            self.node(child, carrying && at >= skip);
        }
    }

    /// Whether the statement covering `decl` declares a `secret` binding.
    ///
    /// A binding's declared span is the name it wrote, so it is inside the
    /// declaration that wrote it and inside nothing else at that depth — which
    /// is what lets a statement be asked about a binding the walk cannot see.
    fn declares_secret(&self, decl: Span) -> bool {
        self.declared.iter().any(|declared| {
            declared.file == decl.file && decl.start <= declared.start && declared.end <= decl.end
        })
    }

    /// Whether the value `node` reads carries `secret`.
    ///
    /// The two questions [`crate::semantic`] asks for its modifier bitset,
    /// asked here for a boolean: a property's type off the entry the checker
    /// recorded against the access itself, and a variable's off the binding it
    /// reads, which [`Analysed::local_ty`] is the one home of. Both end at
    /// `nvs_types::expr::quals`, so an editor and a compile cannot disagree
    /// about what a value carries.
    fn carries_secret(&self, node: &Node) -> bool {
        let recorded = self
            .analysed
            .exprs
            .lookup(node.span)
            .and_then(|info| match info {
                ExprInfo::Property { ty, .. }
                | ExprInfo::StaticProperty { ty, .. }
                | ExprInfo::HookedProperty { ty, .. }
                | ExprInfo::ShapeProperty { ty, .. } => Some(*ty),
                _ => None,
            });
        recorded
            .or_else(|| self.local_ty(node))
            .is_some_and(|ty| is_secret(ty, &self.analysed.interner))
    }

    /// The declared type of the binding `node` reads, when it reads one.
    fn local_ty(&self, node: &Node) -> Option<TypeId> {
        (node.kind == "Variable")
            .then(|| self.analysed.local_ty(node.span, node.span.start))
            .flatten()
    }
}

/// Whether `node` passes its children's value through, and how many leading
/// children it does not pass it through.
///
/// The set is the checker's own: `rule:security/secret-qualifier` spreads the
/// qualifier across a concatenation and an interpolation, so a literal on
/// either side of one is part of the value being stored. A ternary's condition
/// is the one skipped child — it decides which branch is stored and is not
/// stored itself.
fn carrier(node: &Node) -> (bool, usize) {
    match node.kind {
        "Paren" => (true, 0),
        "Ternary" => (true, 1),
        "Binary" => (matches!(word(node, "op"), Some("Concat" | "Coalesce")), 0),
        _ => (false, 0),
    }
}

/// The word `node`'s production recorded under `name`, if it recorded one.
fn word(node: &Node, name: &str) -> Option<&'static str> {
    node.fields.iter().find_map(|(field, value)| match value {
        Field::Word(word) if *field == name => Some(*word),
        Field::Word(_) | Field::Flag(_) => None,
    })
}
