//! The two things a reader cannot see, written where the source left them out.
//!
//! `rule:ide/every-feature-is-staged-behind-its-dependency` lists inlay hints
//! among what the VS Code client gets, and
//! `rule:ide/the-request-set-is-closed` admits them bounded to two idioms.
//! This module answers **only** those two:
//!
//! | Shape | Hint |
//! |---|---|
//! | `var $x = …` or a `foreach` binding written `var` (`rule:types/var-inference`) | `: T`, after the name |
//! | a call argument written as a bare literal | `name:`, before the literal |
//!
//! There is no third, and a candidate is a decision rather than an addition:
//! everything else an editor could annotate — a `foreach` binding with its
//! type written, a chained call's intermediate type, an anonymous function's return — is
//! either written in the source already or is a claim about an idiom that is
//! still moving.
//!
//! # Decision: a hint is read out of the type phase's table, never re-derived
//!
//! Both shapes are answered from what `nvs_types::check_program` already
//! recorded and from nothing else. A local declaration's type is the binding
//! `crate::Analysed::local_ty` reads out of `ExprTypeTable::local_scopes`, and a
//! `foreach` binding's is `ExprTypeTable::declared_ty` at its `var` keyword; the
//! parameter's name is `nvs_types::ResolvedCall::param_names`, reached through
//! that same call's `arg_slots`, which is the checker's own answer to *which
//! parameter did this written argument fill*. A site the table holds nothing
//! for gets **no hint**, which is `tests/hints.rs`'
//! `no_hint_is_produced_from_anything_the_type_phase_did_not_record` and the
//! whole of this module's failure mode: a hint is text an editor draws as if it
//! were in the file, so one this server guessed at would be a second, unchecked
//! description of the program — the failure
//! `rule:ide/completion-offers-only-what-the-compiler-derived` refuses for
//! completion, and the same answer here.
//!
//! # Decision: the walk matches the AST, on [`crate::semantic`]'s terms
//!
//! [`crate::redactions`] reads `nvs_syntax::walk` because a span and a
//! production's name are all it needs. Both facts this module turns on are
//! **absences** that walk does not model: that a declaration wrote *no* type,
//! and that an argument wrote *no* name. So this matches `nvs_syntax::ast`
//! directly and pays what that crate's module doc names — those enums are
//! `#[non_exhaustive]`, so the match in `crate::walk` carries a wildcard and a production
//! landing later reaches no hint and fails no build. The cost is bounded by
//! what a miss means: a hint that is not drawn, never one that is wrong.
//!
//! # Decision: only a literal argument, and only an anonymous one
//!
//! An argument that is a variable, a property or a call already says what it
//! is at the call site; a literal is the one that does not, which is the same
//! bound every editor that ships this feature converged on. A **named**
//! argument (`greet(name: "hi")`) writes the word itself, and a spread or an
//! `inout` marker writes one too, so all three are skipped rather than
//! annotated twice.
//!
//! A request walks only the statements that reach the range it asked about
//! ([`in_range`]). `textDocument/inlayHint` is asked over the visible region
//! and asked again for every region a reader scrolls onto, so a whole walk
//! would repeat the document per scroll. A statement is skipped only when its
//! span ends before the range or starts after it, and every hint sits inside
//! the statement that wrote it, so the skip loses none. `crate::server`'s
//! `inlay_hints` still narrows the answer to the exact positions.
//!
//! # Decision: a call gets hints where its target names its parameters
//!
//! A method call, a static call, `$cls::f(...)` and a `new` read the names of
//! the target the checker resolved, a constructor for a `new`. Two calls get
//! none, because the program names no parameter there. A `callable` type has
//! no parameter names (`rule:types/callable-signature`), and a call on a
//! `mixed` receiver finds its method only when it runs
//! (`rule:types/erased-member-access`). A name drawn at either would be one
//! this server picked, which the first decision above refuses.

use lsp_types::{InlayHint, InlayHintKind, InlayHintLabel};
use nvs_diagnostics::{BytePos, PositionEncoding, Span};
use nvs_syntax::ast::{
    CallArgs, Expr, ExprKind, ForeachBinding, ForeachBindingTy, Stmt, StmtKind, Type,
};
use nvs_types::expr_table::ArgSlot;
use nvs_types::{ExprInfo, ResolvedCall};

use crate::document::Analysed;
use crate::position::position_at;
use crate::walk::{self, Walk};

/// Every hint the entry document carries, in source order.
///
/// Source order because a client draws them where they are and a case freezes
/// them as a list: the walk reaches a declaration's name before the call on the
/// line under it, but a `foreach` header's subject after the binding it
/// precedes, so the order a walk produces is not the order a reader sees.
#[must_use]
pub fn for_document(analysed: &Analysed, encoding: PositionEncoding) -> Vec<InlayHint> {
    in_range(analysed, encoding, 0, BytePos::MAX)
}

/// [`for_document`], for the hints between byte `lo` and byte `hi`, both
/// inclusive. A statement outside them is not walked, so a request costs
/// what the range holds and not what the document holds.
#[must_use]
pub fn in_range(
    analysed: &Analysed,
    encoding: PositionEncoding,
    lo: BytePos,
    hi: BytePos,
) -> Vec<InlayHint> {
    let Some(loaded) = analysed
        .loaded
        .iter()
        .find(|loaded| loaded.id == analysed.entry)
    else {
        return Vec::new();
    };
    let mut hinting = Hinting {
        analysed,
        encoding,
        lo,
        hi,
        hints: Vec::new(),
    };
    hinting.stmts(&loaded.stmts);
    hinting
        .hints
        .sort_by_key(|hint| (hint.position.line, hint.position.character));
    hinting.hints
}

/// One walk of the entry document, collecting what it finds.
struct Hinting<'a> {
    /// The analysis every hint is read out of.
    analysed: &'a Analysed,
    /// The encoding the client negotiated, which is what a position is in.
    encoding: PositionEncoding,
    /// The first byte a hint may sit at.
    lo: BytePos,
    /// The last byte a hint may sit at.
    hi: BytePos,
    /// What the walk has found, in the order it found it.
    hints: Vec<InlayHint>,
}

impl Walk for Hinting<'_> {
    /// Walks only the statements that reach the asked range.
    fn stmts(&mut self, stmts: &[Stmt]) {
        for stmt in stmts {
            if stmt.span.end >= self.lo && stmt.span.start <= self.hi {
                self.stmt(stmt);
            }
        }
    }

    /// Walks one statement, hinting the declaration it may be.
    fn stmt(&mut self, stmt: &Stmt) {
        match &stmt.kind {
            StmtKind::Foreach { key, value, .. } => {
                for binding in key.iter().chain([value]) {
                    self.foreach_binding(binding);
                }
            }
            StmtKind::LocalDecl { ty, name, .. } => self.declared(ty.as_ref(), *name),
            _ => {}
        }
        walk::stmt_children(self, stmt);
    }

    /// Walks one expression, hinting the call it may be.
    fn expr(&mut self, expr: &Expr) {
        match &expr.kind {
            ExprKind::Call { args, .. }
            | ExprKind::MethodCall { args, .. }
            | ExprKind::StaticCall { args, .. }
            | ExprKind::New { args, .. } => self.args(expr.span, args),
            _ => {}
        }
        walk::expr_children(self, expr);
    }
}

impl Hinting<'_> {
    /// The inferred type of a `var` declaration, and nothing for one that wrote
    /// its type out.
    ///
    /// The binding rather than the initializer's own expression: `var`'s type
    /// is what the checker *fixed onto the name*, which is the same value a
    /// hand-written annotation would have had to spell, and it is the one a
    /// reader is asking for. `crate::Analysed::local_ty` is that lookup's one
    /// home and answers `None` for a body the type phase never reached.
    fn declared(&mut self, ty: Option<&Type>, name: Span) {
        if ty.is_some() {
            return;
        }
        let Some(inferred) = self.analysed.local_ty(name, name.start) else {
            return;
        };
        let label = format!(": {}", self.analysed.interner.describe(inferred));
        self.push(name.end, label, InlayHintKind::TYPE, false);
    }

    /// The type a `foreach` binding written `var` was given, and nothing for
    /// one that wrote its type out.
    ///
    /// Read from where the checker recorded it, under the keyword's span —
    /// the lookup `nvs-ir`'s lowering makes — rather than by the binding's
    /// name, which a later binding of the same name in another loop would
    /// answer for as well.
    fn foreach_binding(&mut self, binding: &ForeachBinding) {
        let ForeachBindingTy::Var(var) = binding.ty else {
            return;
        };
        let Some(inferred) = self.analysed.exprs.declared_ty(var) else {
            return;
        };
        let label = format!(": {}", self.analysed.interner.describe(inferred));
        self.push(binding.name.end, label, InlayHintKind::TYPE, false);
    }

    /// The parameter each bare-literal argument of this call fills.
    ///
    /// `span` is the call expression's own, which is the key the checker
    /// recorded its `nvs_types::ResolvedCall` under. `arg_slots` is what joins
    /// the two sides: it carries one entry per *written* argument, so a call
    /// that filled a later parameter by name or left a defaulted one out is
    /// still read correctly, and an argument the checker could not place
    /// carries a slot this skips rather than a position it would have guessed.
    fn args(&mut self, span: Span, args: &CallArgs) {
        let CallArgs::List(list) = args else {
            return;
        };
        let analysed = self.analysed;
        let Some(call) = analysed.exprs.lookup(span).and_then(resolved_call) else {
            return;
        };
        for (written, arg) in list.iter().enumerate() {
            if arg.name.is_some() || arg.spread || arg.inout || !is_literal(&arg.value) {
                continue;
            }
            let Some(ArgSlot::Param(slot)) = call.arg_slots.get(written).copied() else {
                continue;
            };
            let Some(name) = call.param_names.get(slot).filter(|name| !name.is_empty()) else {
                continue;
            };
            let label = format!("{name}:");
            self.push(arg.value.span.start, label, InlayHintKind::PARAMETER, true);
        }
    }

    /// Records one hint at `at`, padded on the side the text it abuts is on.
    ///
    /// A type hint follows the name it describes and needs no space before it —
    /// `$x: int` is how the annotation would have been written. A parameter
    /// hint precedes the value and takes one after it, so `greet(name: "hi")`
    /// reads as the named argument it stands in for.
    fn push(&mut self, at: BytePos, label: String, kind: InlayHintKind, padded: bool) {
        if at < self.lo || at > self.hi {
            return;
        }
        let file = self.analysed.map.file(self.analysed.entry);
        self.hints.push(InlayHint {
            position: position_at(file, at, self.encoding),
            label: InlayHintLabel::String(label),
            kind: Some(kind),
            text_edits: None,
            tooltip: None,
            padding_left: None,
            padding_right: padded.then_some(true),
            data: None,
        });
    }
}

/// The resolved target whose parameter names a call's arguments fill.
///
/// A method call, a `$cls::f(...)` over a `class<T>` and a `new` with a
/// declared constructor each carry one. A call through a `callable` and a call
/// on a `mixed` receiver carry none, and the module doc's last decision says
/// why that is no gap.
fn resolved_call(info: &ExprInfo) -> Option<&ResolvedCall> {
    match info {
        ExprInfo::Call(call) | ExprInfo::ClassRefCall(call) => Some(call),
        ExprInfo::New { ctor, .. } | ExprInfo::NewDynamic { ctor, .. } => ctor.as_ref(),
        _ => None,
    }
}

/// Whether `expr` is a literal written out at the call site.
///
/// The bound the module doc's third decision names. An interpolated string and
/// an array literal are not one: each holds expressions, and what a reader
/// cannot see there is what those are rather than which parameter they fill.
fn is_literal(expr: &Expr) -> bool {
    matches!(
        expr.kind,
        ExprKind::Null
            | ExprKind::Bool(_)
            | ExprKind::Int(_)
            | ExprKind::Float(_)
            | ExprKind::Duration(_)
            | ExprKind::Str(_)
    )
}
