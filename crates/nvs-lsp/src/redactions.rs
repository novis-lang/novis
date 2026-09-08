//! Which bytes of one document the editor conceals, and why.
//!
//! `nvs/redactions` is the one request of Novis's own in
//! `rule:ide/the-request-set-is-closed`'s list, and it exists because the
//! alternative is the client deciding what a secret is
//! (`rule:ide/redaction-ranges-come-from-the-server`). A
//! `TextDocumentIdentifier` goes in and a list of `{range, kind}` comes back,
//! `kind` being the open string ADR 0101 § 1 left open.
//!
//! # Decision: two kinds on one list, not a list per kind
//!
//! [`SECRET_LITERAL`] is bytes to conceal and [`TAINTED_DECLARATION`] is a name
//! to mark, and they are not the same instruction: one is drawn by default
//! because a credential on a shared screen is an incident, and the other only
//! where `nvs.taint.mark` asks for it
//! (`rule:ide/tainted-has-no-default-decoration`). They travel on one
//! request because the set is closed and `kind` is the room ADR 0101 § 1 left,
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
//! `rule:ide/redaction-covers-bytes-only` answers a range when the literal
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
//! **A parameter and a property, for the marker.** [`TAINTED_DECLARATION`] is
//! answered off the bindings the checker recorded, which are a body's locals;
//! a parameter's name is written outside the body's scope and a property's is
//! not a binding at all, so neither carries a glyph. That is the same boundary
//! [`crate::semantic`] crosses with `qualifiers_declared` and
//! `qualifiers_recorded`, and closing it here means answering those two
//! questions at a *declaration* rather than at a use.
//!
//! **A sink's argument position.** ADR 0101 § 4's third setting value needs
//! `rule:security/sink-predicate`'s classification on the member rows before
//! there is anything to answer, and the record leaves whether it is built at
//! all open. Until then `nvs.taint.mark = sink` marks what `declaration` does.
//!
//! **A parameter's default.** A parameter declaration is not a binding the
//! checker records a span for, and [`nvs_syntax::walk`] gives a function's
//! default the same shape as its body's statements, so there is nothing here
//! to attribute the literal through. The property half of this gap is closed:
//! [`nvs_types::ExprTypeTable::property_default_ty`] carries a property
//! initializer's declared type across from the signature pass, which is where
//! the type was always known and where it used to be discarded.
//!
//! **Everything past the editor's own decorations.**
//! `rule:ide/redaction-does-not-reach` is the list, and it is part of the
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
use nvs_types::expr::quals::{is_secret, is_tainted};

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

/// A declaration whose binding carries `tainted`, marked and never concealed.
///
/// ADR 0101 § 4's `declaration` setting, answered: the range is the name the
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
    let mut concealing = Concealing {
        analysed,
        declared: secret_bindings(analysed),
        spans: Vec::new(),
    };
    for node in &walk::of_stmts(&loaded.stmts) {
        concealing.node(node, false);
    }
    let file = analysed.map.file(analysed.entry);
    let mut answered: Vec<(Span, &'static str)> = concealing
        .spans
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
/// A binding and not a use, which is the whole of what ADR 0101 § 4 asks for: a
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
            "Property" => self.property(node),
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

    /// Walks a property declaration: its initializer carries `secret` when the
    /// property does, and its hooks never do.
    ///
    /// The one declaration site with no binding and no access behind it, so
    /// the type comes from neither of [`Self::carries_secret`]'s two sources
    /// but from [`ExprTypeTable::property_default_ty`], which the checker fills
    /// from the signature pass for exactly this.
    ///
    /// `nvs_syntax::walk` gives this production its initializer as the first
    /// child and its hooks after it, and a property that declared no
    /// initializer has hooks in that position instead. Asking the initializer's
    /// own span is what tells those apart without counting: a hook body's span
    /// was never recorded, so it answers `None` and carries nothing. A hook is
    /// a body rather than a value written into the property, so a literal
    /// inside one is a byte of the program — concealing it would be the
    /// over-redaction ADR 0101 § 1 refuses.
    fn property(&mut self, node: &Node) {
        let mut children = node.children.iter();
        if let Some(first) = children.next() {
            let secret = self
                .analysed
                .exprs
                .property_default_ty(first.span)
                .is_some_and(|ty| is_secret(ty, &self.analysed.interner));
            self.node(first, secret);
        }
        for hook in children {
            self.node(hook, false);
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
