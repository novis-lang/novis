//! Which bytes of one file are a `secret` value written into the program.
//!
//! [`secret_literals`] is the one walk that answers it, and two readers ask:
//! the language server's `nvs/redactions`, which conceals the ranges in an
//! editor (`rule:ide/redaction-ranges-come-from-the-server`), and the compile
//! path, which hands them to the source map so a diagnostic's snippet prints
//! none of them (`rule:security/redaction-reaches-the-tools-own-renderings`).
//! One walk is what keeps the editor and the terminal from disagreeing about
//! which bytes are secret.
//!
//! # Decision: this is a projection of the one walk, not a second traversal
//!
//! It reads `nvs_syntax::walk::of_stmts`, whose match is written beside the
//! grammar and is exhaustive, rather than matching [`nvs_syntax::ast`] itself:
//! those enums are `#[non_exhaustive]`, so a match outside `nvs-syntax` needs
//! a wildcard, and a production landing later would conceal nothing and fail
//! no build. A [`Node`] carries everything this walk needs: a literal's span,
//! an interpolation slot's span, and which production contains them. For a
//! security default that is the whole difference: silently covering less is
//! the failure this walk exists to prevent.
//!
//! # Decision: a literal is attributed through the binding it is written into
//!
//! `rule:ide/redaction-covers-bytes-only` answers a range when the literal
//! token's static type carries `secret`. A literal's own type never does — the
//! qualifier reaches it from the binding it flows into — and mid-edit there is
//! often no type at all, which is exactly when a value must not flash on
//! screen. So the question asked here is the fail direction
//! `rule:ide/redaction-ranges-come-from-the-server` names, asked
//! everywhere rather than only on failure: **does the binding this value is
//! written into carry `secret`?**
//!
//! - A `LocalDecl` declares a binding, and the checker recorded every binding
//!   of every body with the span it was declared at ([`crate::LocalBinding`]).
//!   A declaration whose statement contains one of the `secret` ones conceals
//!   its initializer. That survives an initializer that does not check,
//!   because the binding's type comes from the declaration rather than from
//!   the value.
//! - An assignment conceals its value when its *target* carries `secret` — a
//!   variable through [`ExprTypeTable::local_ty`], a property through the
//!   entry the checker recorded against the access.
//!
//! **The value is followed only through the productions that carry it**:
//! parentheses, a ternary's branches, and `.` and `??`, which are the
//! compositions the checker itself spreads the qualifier across. A literal
//! anywhere else under a `secret` declaration — an argument to the call that
//! produced the value, a key in an array being indexed — is a byte of the
//! program rather than a byte of the secret, and concealing it would be the
//! over-redaction `rule:ide/redaction-covers-bytes-only` refuses.
//!
//! An interpolation slot is the other half, and it is answered wherever it is
//! written: `"prefix $key suffix"` conceals `$key` even in a statement that
//! refuses to compile, because a program that misuses a secret is still a
//! program whose secret is on screen.
//!
//! # What it does not reach
//!
//! **A parameter's default.** A parameter declaration is not a binding the
//! checker records a span for, and [`nvs_syntax::walk`] gives a function's
//! default the same shape as its body's statements, so there is nothing here
//! to attribute the literal through. A property initializer is reached:
//! [`ExprTypeTable::property_default_ty`] carries its declared type across
//! from the signature pass.
//!
//! **A file the checker never reached.** A parse that fails records no
//! binding, so a snippet under a syntax error is printed as written.
//!
//! # What it spends
//!
//! One [`Node`] tree over the file per call, and one [`Span`] per concealed
//! range, both O(nodes in the file). The tree is dropped with the call. The
//! compile path calls it once per loaded file and only when there is a
//! diagnostic to render, so a program that compiles pays nothing.

use nvs_diagnostics::{SourceFile, Span};
use nvs_syntax::ast::Stmt;
use nvs_syntax::walk::{self, Field, Node};

use crate::expr::quals;
use crate::{ExprInfo, ExprTypeTable, TypeId, TypeInterner};

/// Every range of `file` that is a `secret` value written into the program,
/// `stmts` being its parsed statements and `exprs` what the checker recorded
/// over the whole program.
///
/// In the order the walk found them, which is source order except that a
/// declaration's literals come before the literals inside them.
#[must_use]
pub fn secret_literals(
    file: &SourceFile,
    stmts: &[Stmt],
    exprs: &ExprTypeTable,
    interner: &TypeInterner,
) -> Vec<Span> {
    let mut concealing = Concealing {
        file,
        exprs,
        interner,
        declared: secret_bindings(exprs, interner),
        spans: Vec::new(),
    };
    for node in &walk::of_stmts(stmts) {
        concealing.node(node, false);
    }
    concealing.spans
}

/// Where every `secret` binding the checker recorded was declared.
///
/// The whole analysis's, not one file's: a [`Span`] names its own file, so
/// containment answers which file a binding was declared in without a filter
/// that would have to be kept true.
fn secret_bindings(exprs: &ExprTypeTable, interner: &TypeInterner) -> Vec<Span> {
    exprs
        .local_scopes()
        .flat_map(|(_, locals)| locals.iter())
        .filter(|local| quals::carries_secret(local.ty, interner))
        .map(|local| local.declared)
        .collect()
}

/// One walk, and what it has decided to conceal so far.
struct Concealing<'a> {
    /// The file being walked, whose text names the variable a node reads.
    file: &'a SourceFile,
    /// What the checker resolved about the program.
    exprs: &'a ExprTypeTable,
    /// The types `exprs` names.
    interner: &'a TypeInterner,
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
    /// `nvs_syntax::walk` gives this production one `PropertyHook` node per
    /// hook, and the one other child that is neither an attribute nor a shape
    /// type is the initializer. A hook is a body rather than a value written
    /// into the property, so a literal inside one is a byte of the program —
    /// concealing it would be the over-redaction
    /// `rule:ide/redaction-covers-bytes-only` refuses.
    fn property(&mut self, node: &Node) {
        for child in &node.children {
            match child.kind {
                // Written by the declaration, and no value stored in the
                // property.
                "Attribute" | "ShapeType" => {}
                "PropertyHook" => self.node(child, false),
                _ => {
                    let secret = self
                        .exprs
                        .property_default_ty(child.span)
                        .is_some_and(|ty| quals::carries_secret(ty, self.interner));
                    self.node(child, secret);
                }
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
    /// A property's type off the entry the checker recorded against the access
    /// itself, and a variable's off the binding it reads. Both end at
    /// [`quals::carries_secret`], so an editor and a compile cannot disagree
    /// about what a value carries, a `?secret string` included.
    fn carries_secret(&self, node: &Node) -> bool {
        let recorded = self.exprs.lookup(node.span).and_then(|info| match info {
            ExprInfo::Property { ty, .. }
            | ExprInfo::StaticProperty { ty, .. }
            | ExprInfo::HookedProperty { ty, .. }
            | ExprInfo::ShapeProperty { ty, .. } => Some(*ty),
            _ => None,
        });
        recorded
            .or_else(|| self.local_ty(node))
            .is_some_and(|ty| quals::carries_secret(ty, self.interner))
    }

    /// The declared type of the binding `node` reads, when it reads one.
    fn local_ty(&self, node: &Node) -> Option<TypeId> {
        if node.kind != "Variable" {
            return None;
        }
        let name = self.file.text().get(node.span.range())?.strip_prefix('$')?;
        self.exprs.local_ty(node.span.file, name, node.span.start)
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
