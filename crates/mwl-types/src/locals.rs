//! Per-function-body local-variable checking: ADR 0007 § 1's declare-once
//! rule ("there is no shadowing") and flow-sensitive definite assignment
//! ("reading a binding on a path that may not have reached its initialiser
//! is a compile error").
//!
//! This is a structural walk of the AST, not a CFG — the milestone has no IR
//! yet, and every other check in this crate already walks the tree directly
//! (mirroring [`mwl_hir::members`]'s own shape). [`check_block`] threads two
//! kinds of state through that walk: [`LocalScope`], one shared table for
//! the *whole* function body (declaration is function-scoped — a name
//! declared inside an `if` is visible, though not necessarily definitely
//! assigned, after it), and a `live: &mut FxHashSet<String>` set that is
//! cloned and merged at every branch point, so it always reflects exactly
//! what is definitely assigned on the path reached so far.
//!
//! **A deliberate, ADR-underspecified judgment call:** a second plain
//! `LocalDecl` for a live name is always a diagnostic (the ADR's own
//! example). A `foreach` key/value binding, a destructuring leaf, or a
//! `catch` variable is *not* held to that same strictness: reusing the same
//! name with the *same* declared type (the ordinary "loop index `$i` used in
//! two separate loops" case) is accepted as reuse rather than a fresh
//! declaration, since the ADR's text doesn't address this directly and
//! rejecting it would make an extremely common pattern a compile error.
//! Reusing a name with a *different* type still conflicts — that really is
//! "a second declared type for one storage location," which ADR 0007 § 1
//! explicitly forbids for a reference and, by the same reasoning, for a
//! plain local too.
//!
//! # Narrowing a nullable local through `!== null`
//!
//! [`narrow`] is the third piece of state this walk threads, and the only
//! one that is *not* cloned at a branch point: [`LocalScope::narrowed`] is a
//! single forward-walked map from a name to the type it provably holds on
//! the path being checked right now. A `!== null`/`=== null` test over a
//! plain variable installs one entry for the branch it proves, and the
//! branch's end restores what was there before — **unless a write already
//! removed it**, in which case the write wins and nothing is put back (see
//! [`Narrowing`], which records what it installed so it can tell the two
//! apart).
//!
//! Two rules keep that sound, and both are load-bearing because `mwl-ir`
//! turns a narrowed receiver into an *unchecked* `mwl_ir::ir::InstKind::Untag`:
//!
//! * **Every path that can change what a local holds calls
//!   [`LocalScope::overwrite`]**, which drops the narrowing and answers the
//!   *declared* type — so `if ($m !== null) { $m = null; $m->x(); }` refuses
//!   the second statement, and `$m = null;` itself is still checked against
//!   `?M` rather than against the narrowed `M`. Adding a new write path to
//!   this crate owes a call to it; that list is `check_assign`,
//!   `check_compound_assign`, `PreIncDec`/`PostIncDec`, `check_by_ref_arg`,
//!   `check_unset_target` and [`declare_binding`] below.
//! * **A loop body drops every narrowing installed outside it**
//!   ([`check_stmt`]'s `While`/`DoWhile`/`For`/`Foreach` arms), because a
//!   write at the *end* of the body invalidates a read at its start on the
//!   next iteration and this walk has no back edge to discover that on. A
//!   `while` condition's own narrowing is installed after that drop and is
//!   sound, since the condition is re-tested before every entry.
//!
//! **Only a `null`-and-one-class union narrows**, deliberately: see
//! [`narrow`] for why a `?int` narrowed here would turn a clean diagnostic
//! into an `mwl-ir` panic.
//!
//! **Known gaps**, beyond the ones `crate` docs already name: a `switch`
//! case that silently falls through to the next one (no explicit `break`/
//! `continue`, and not the last case) contributes nothing to what is live
//! *within* the case it falls into — each case is still checked starting
//! fresh from what was live before the whole `switch`, same as a `case`
//! reached by a direct jump would see (documented at the `Switch` arm below);
//! a nested class/interface/enum declaration inside a function body is not
//! descended into at all (its own methods go unchecked, same as a closure's
//! body — see `crate::expr`'s docs for the latter).

use mwl_diagnostics::{Diagnostic, Span, code};
use mwl_syntax::ast::{DestructureElement, DestructureTarget, Expr, ExprKind, Stmt, StmtKind};
use rustc_hash::{FxHashMap, FxHashSet};

use crate::expr::{check_expr, check_return, check_unset_target, require_stringable};
use crate::lower::{lower_optional_type, lower_type};
use crate::ty::{Ty, TypeId};
use crate::{Ctx, Env, span_text, strip_sigil};

/// One local variable's declared type and where it was declared.
#[derive(Debug)]
pub(crate) struct LocalInfo {
    pub ty: TypeId,
    pub declared_span: Span,
}

/// One function/method/closure body's local variables — a single table for
/// the whole body, since declaration is function-scoped (ADR 0007 § 1), not
/// block-scoped. A closure gets a fresh one of its own: ADR 0031's capture is
/// by value, never a shared binding, so an outer name reaches the body
/// through [`Captures`] rather than through `by_name`.
#[derive(Debug, Default)]
pub(crate) struct LocalScope {
    pub(crate) by_name: FxHashMap<String, LocalInfo>,
    /// Set only for a closure body's own scope — see [`Captures`].
    pub(crate) captures: Option<Captures>,
    /// What a dominating condition proved about a name on the path being
    /// checked right now — see the module docs' narrowing section.
    ///
    /// A [`RefCell`](std::cell::RefCell) for the same reason [`Captures::used`]
    /// is one: the whole expression checker threads a `&LocalScope`, and both
    /// the read that consults a narrowing and the write that must drop it
    /// happen mid-expression.
    narrowed: std::cell::RefCell<FxHashMap<String, TypeId>>,
    /// Narrowings a loop body put out of a read's reach — see [`suspend`].
    /// A write still reaches them ([`LocalScope::overwrite`]), which is what
    /// makes resuming one at the end of the body sound.
    shadowed: std::cell::RefCell<Vec<FxHashMap<String, TypeId>>>,
}

/// The outer bindings a closure body may read, and the ones it actually did.
///
/// [ADR 0031](../../../docs/adr/0031-callable-is-the-only-closure-type.md) § 2
/// captures "exactly the outer variables its body reads," which is a fact
/// about the body rather than about the enclosing scope — so `available`
/// holds every name that *could* be captured, and `used` accumulates the ones
/// a read or a write actually reached, in first-touch order. That order is
/// what `mwl-ir` lays the closure object's fields out in, so it has to be
/// deterministic; a set would not be.
///
/// `used` is a [`RefCell`] because [`crate::expr::check_expr`] takes
/// `&LocalScope` — the whole expression checker threads the scope
/// immutably, and a capture is discovered mid-expression. The alternative,
/// a separate free-variable walk of the body's AST before checking it, would
/// be a second traversal that has to agree with the checker's own notion of
/// what a variable read is; recording at the one lookup site cannot drift
/// from it.
#[derive(Debug, Default)]
pub(crate) struct Captures {
    /// Every binding visible from the enclosing body at the `fn` literal —
    /// its own locals plus, for a nested closure, whatever the enclosing
    /// closure could itself capture.
    pub(crate) available: FxHashMap<String, TypeId>,
    /// The subset of `available` this body touched, in first-touch order.
    pub(crate) used: std::cell::RefCell<Vec<(String, TypeId)>>,
}

impl LocalScope {
    /// An empty scope.
    pub(crate) fn new() -> Self {
        Self::default()
    }

    /// Declares a parameter — always freshly, with no reuse/redeclaration
    /// check: a parameter list's own duplicate names are already diagnosed
    /// at parse time (`E_BAD_PARAM_LIST`), and a parameter is always
    /// definitely assigned, so the caller adds `name` to `live` itself.
    pub(crate) fn declare_param(&mut self, name: String, ty: TypeId, declared_span: Span) {
        self.by_name.insert(name, LocalInfo { ty, declared_span });
    }

    /// The declared type `name` is readable and writable at in this body —
    /// its own local, or, in a closure, an outer binding, which this call
    /// records as captured.
    ///
    /// The one lookup every read and every write goes through, so a name can
    /// never be resolved without the capture being noticed.
    ///
    /// Answers the *narrowed* type where a dominating condition proved one
    /// (module docs). A write must not see that — it is checked against what
    /// the binding was declared as — so every write path goes through
    /// [`Self::overwrite`] instead.
    pub(crate) fn declared_ty(&self, name: &str) -> Option<TypeId> {
        if let Some(narrowed) = self.narrowed.borrow().get(name) {
            return Some(*narrowed);
        }
        if let Some(info) = self.by_name.get(name) {
            return Some(info.ty);
        }
        let captures = self.captures.as_ref()?;
        let ty = *captures.available.get(name)?;
        let mut used = captures.used.borrow_mut();
        if !used.iter().any(|(n, _)| n == name) {
            used.push((name.to_owned(), ty));
        }
        Some(ty)
    }

    /// What `name` was *declared* as, dropping any narrowing on it first —
    /// the call every path that can change what `name` holds must make.
    ///
    /// Both halves matter. Answering the declared type is what lets
    /// `$m = null;` stay legal inside an `if ($m !== null)` block, where the
    /// narrowed type would refuse it; dropping the narrowing is what makes a
    /// later `$m->x()` in that same block a diagnostic again. The module docs
    /// list every caller — a new one that forgets this is the one way a
    /// narrowing can go stale, and `mwl-ir` trusts it unconditionally.
    pub(crate) fn overwrite(&self, name: &str) -> Option<TypeId> {
        self.narrowed.borrow_mut().remove(name);
        for layer in self.shadowed.borrow_mut().iter_mut() {
            layer.remove(name);
        }
        self.declared_ty(name)
    }

    /// Records `name` as captured by this body without reading it — how a
    /// *nested* closure's capture reaches the enclosing one, which has to
    /// capture it too in order to have it to hand on.
    pub(crate) fn note_capture(&self, name: &str) {
        if self.by_name.contains_key(name) {
            return;
        }
        self.declared_ty(name);
    }

    /// Every name this body can see, for seeding a nested closure's
    /// [`Captures::available`].
    pub(crate) fn visible(&self) -> FxHashMap<String, TypeId> {
        let mut out: FxHashMap<String, TypeId> = self
            .captures
            .as_ref()
            .map(|c| c.available.clone())
            .unwrap_or_default();
        for (name, info) in &self.by_name {
            out.insert(name.clone(), info.ty);
        }
        out
    }
}

/// One installed narrowing, and everything needed to take it back off again.
///
/// `installed` is what [`narrow`] put in [`LocalScope::narrowed`]; `previous`
/// is what was there before. [`Narrowing::restore`] puts `previous` back only
/// while the entry still holds `installed` — if a write dropped or replaced
/// it in between, that write is the newer fact and restoring would resurrect
/// a narrowing the program has already invalidated.
#[derive(Debug, Default)]
pub(crate) struct Narrowing(Vec<(String, TypeId, Option<TypeId>)>);

impl Narrowing {
    /// Folds `other`'s entries in, so a run of consecutive guard clauses is
    /// one restore at the end of the enclosing block.
    fn absorb(&mut self, other: Self) {
        self.0.extend(other.0);
    }

    /// Undoes what [`narrow`] installed, innermost first.
    fn restore(self, scope: &LocalScope) {
        let mut map = scope.narrowed.borrow_mut();
        for (name, installed, previous) in self.0.into_iter().rev() {
            if map.get(&name) != Some(&installed) {
                continue;
            }
            match previous {
                Some(ty) => map.insert(name, ty),
                None => map.remove(&name),
            };
        }
    }
}

/// The `$x !== null`/`$x === null` test `cond` is, if it is one at all: the
/// tested variable's name span, and whether the test *holding* means the
/// variable is not `null`.
///
/// `===`/`!==` only, which ADR 0090 turns into a gap rather than a rule:
/// that ADR makes `==` strict and `===` a rejected spelling, so `== null`
/// is the null test and belongs here too. It was excluded because PHP's
/// loose `==` against `null` was a different question (`0 == null` is
/// true), and reading a laundering rule out of the truthy table is exactly
/// the kind of almost-right that ADR 0007 § 4 keeps out of this checker.
/// Widening this to `Eq`/`NotEq` is one arm, and waits only on the lexer
/// half so the two spellings never both exist. A bare
/// `if ($x)` is likewise not a null test: ADR 0035 § 4 makes it one for a
/// nullable object today, but not for a `?string` holding `""`.
fn null_test(cond: &Expr) -> Option<(Span, bool)> {
    match &cond.kind {
        ExprKind::Paren(inner) => null_test(inner),
        ExprKind::Unary {
            op: mwl_syntax::ast::UnaryOp::Not,
            expr: inner,
        } => null_test(inner).map(|(span, non_null)| (span, !non_null)),
        ExprKind::Binary { op, lhs, rhs }
            if matches!(
                op,
                mwl_syntax::ast::BinaryOp::Identical | mwl_syntax::ast::BinaryOp::NotIdentical
            ) =>
        {
            let non_null = *op == mwl_syntax::ast::BinaryOp::NotIdentical;
            match (&lhs.kind, &rhs.kind) {
                (ExprKind::Variable(span), ExprKind::Null)
                | (ExprKind::Null, ExprKind::Variable(span)) => Some((*span, non_null)),
                _ => None,
            }
        }
        _ => None,
    }
}

/// Installs the narrowing `cond` proves on the branch where it evaluates to
/// `when`, and hands back what that branch's end has to restore.
///
/// **Only a union of `null` and exactly one class narrows.** The general
/// rule — drop `null`, keep the rest — is what ADR 0066's body describes and
/// what this would like to do, but `mwl-ir` lowers a `?T` local's slot as
/// [`Ty::Tagged`](mwl_ir::ty::Ty) whatever the checker later proves about it,
/// and today only a *receiver* reads back out of one (an unchecked `Untag`,
/// which `mwl_types` having proved the shape is exactly what licenses).
/// Narrowing `?int` here would make `$n + 1` type-check and then panic in
/// `mwl-ir`'s arithmetic, trading a clean diagnostic for a crash — so the
/// residue is checked and anything else is left alone. Widening this is
/// gated on `mwl-ir` gap 1's tagged arithmetic, not on any decision here.
fn narrow(cond: &Expr, when: bool, scope: &LocalScope, env: &mut Env<'_>) -> Narrowing {
    let Some((name_span, non_null_when_true)) = null_test(cond) else {
        return Narrowing::default();
    };
    if non_null_when_true != when {
        return Narrowing::default();
    }
    let name = strip_sigil(span_text(env.src, name_span)).to_owned();
    let Some(current) = scope.declared_ty(&name) else {
        return Narrowing::default();
    };
    let residue = env.interner.without_null(current);
    if residue == current || !matches!(env.interner.get(residue), Ty::Class(..)) {
        return Narrowing::default();
    }
    let previous = scope.narrowed.borrow_mut().insert(name.clone(), residue);
    Narrowing(vec![(name, residue, previous)])
}

/// A set of narrowings [`suspend`] moved out of reach for a loop body, to be
/// put back by [`Suspended::resume`] when the body is done with.
#[must_use]
pub(crate) struct Suspended;

impl Suspended {
    /// Puts back every suspended narrowing a write inside the body did not
    /// invalidate — [`LocalScope::overwrite`] reaches into the suspended
    /// layer too, so anything the body assigned is simply no longer there.
    fn resume(self, scope: &LocalScope) {
        let layer = scope
            .shadowed
            .borrow_mut()
            .pop()
            .expect("every `suspend` is paired with one `resume`");
        scope.narrowed.borrow_mut().extend(layer);
    }
}

/// Puts every narrowing in force out of a *read*'s reach for a loop body.
///
/// A loop's back edge carries a write at the end of the body to a read at its
/// start, and this walk is a forward one with no back edge to discover that
/// on — so `if ($m === null) { return; } while (…) { $m->x(); $m = f(); }`
/// would otherwise narrow the receiver against a write that reaches it. A
/// narrowing established *inside* the body is unaffected: it is installed
/// after this call, and a `while` condition is re-tested before every entry.
fn suspend(scope: &LocalScope) -> Suspended {
    let taken = std::mem::take(&mut *scope.narrowed.borrow_mut());
    scope.shadowed.borrow_mut().push(taken);
    Suspended
}

/// Declares `name` in `scope`, or checks it against an existing declaration.
///
/// `strict`: a plain `LocalDecl` (`true`) conflicts with *any* prior
/// declaration of the same name, matching or not. A `foreach`/destructuring/
/// `catch` binding (`false`) is accepted as reuse when the existing
/// declaration's type matches exactly — see the module docs' judgment call.
fn declare_binding(
    scope: &mut LocalScope,
    name: &str,
    ty: TypeId,
    span: Span,
    strict: bool,
    env: &mut Env<'_>,
) {
    // A `foreach`/`catch`/destructuring binding writes the name, so it drops
    // whatever a dominating condition proved about it — the module docs' rule
    // that every write path goes through `overwrite`.
    scope.overwrite(name);
    if let Some(existing) = scope.by_name.get(name) {
        if !strict && existing.ty == ty {
            return;
        }
        env.diags.report(
            Diagnostic::error(
                code::E_REDECLARED_LOCAL,
                format!("`${name}` is already declared"),
            )
            .with_primary(span, "second declaration")
            .with_secondary(existing.declared_span, "first declared here"),
        );
        return;
    }
    scope.by_name.insert(
        name.to_owned(),
        LocalInfo {
            ty,
            declared_span: span,
        },
    );
}

/// Whether every path through `stmt` ends in a `return` or `throw` — used at
/// an `if`/`else` join so a branch that never falls through cannot restrict
/// what the *other* branch definitely assigned. Deliberately conservative:
/// `false` for anything not immediately recognisable this way (a `switch` or
/// loop that in fact always returns is not detected), which can only cause a
/// spurious diagnostic later, never a missed one.
fn terminates(stmt: &Stmt) -> bool {
    match &stmt.kind {
        StmtKind::Return(_) => true,
        StmtKind::Expr(Expr {
            kind: ExprKind::Throw(_),
            ..
        }) => true,
        StmtKind::Block(b) => b.stmts.last().is_some_and(terminates),
        StmtKind::If {
            then,
            else_: Some(else_),
            ..
        } => terminates(then) && terminates(else_),
        _ => false,
    }
}

/// Whether `stmt` — drilling into a trailing block exactly like
/// [`terminates`] does — is a bare `break`/`continue`. Used by the `Switch`
/// arm below to tell "this case explicitly exits the switch here" (safe to
/// export its live set to after the switch) from "this case silently falls
/// through to the next one" (must not be treated as an exit). Also used by
/// [`crate::ctor_init`], which needs the same distinction.
pub(crate) fn ends_in_break_or_continue(stmt: &Stmt) -> bool {
    match &stmt.kind {
        StmtKind::Break(_) | StmtKind::Continue(_) => true,
        StmtKind::Block(b) => b.stmts.last().is_some_and(ends_in_break_or_continue),
        _ => false,
    }
}

/// Walks `stmts` in sequence, checking each one and updating `live`/`scope`
/// in place.
///
/// Also where PHP's guard clause narrows: `if ($m === null) { return; }`
/// leaves everything after it on the branch where the condition was *false*,
/// so the narrowing that branch proves holds for the rest of this block —
/// installed here rather than in [`check_stmt`]'s `If` arm, which cannot see
/// what follows it.
pub(crate) fn check_block(
    stmts: &[Stmt],
    live: &mut FxHashSet<String>,
    scope: &mut LocalScope,
    return_ty: TypeId,
    ctx: &Ctx<'_>,
    env: &mut Env<'_>,
) {
    let mut carried = Narrowing::default();
    for stmt in stmts {
        check_stmt(stmt, live, scope, return_ty, ctx, env);
        if let StmtKind::If {
            cond,
            then,
            else_: None,
        } = &stmt.kind
            && terminates(then)
        {
            carried.absorb(narrow(cond, false, scope, env));
        }
    }
    carried.restore(scope);
}

#[expect(
    clippy::too_many_lines,
    reason = "one match arm per AST statement variant, each a couple of lines"
)]
pub(crate) fn check_stmt(
    stmt: &Stmt,
    live: &mut FxHashSet<String>,
    scope: &mut LocalScope,
    return_ty: TypeId,
    ctx: &Ctx<'_>,
    env: &mut Env<'_>,
) {
    match &stmt.kind {
        StmtKind::Expr(e) => {
            check_expr(e, None, live, scope, ctx, env);
        }
        StmtKind::Return(Some(e)) => {
            check_return(e, return_ty, live, scope, ctx, env);
        }
        StmtKind::Return(None) => {
            // Whether a bare `return;` is legal here (only when `return_ty`
            // is `void`) needs reachability analysis this slice doesn't do —
            // see the crate docs' known gaps.
        }
        StmtKind::Block(b) => check_block(&b.stmts, live, scope, return_ty, ctx, env),
        StmtKind::Empty | StmtKind::InlineHtml(_) | StmtKind::Error => {}
        StmtKind::If { cond, then, else_ } => {
            check_expr(cond, None, live, scope, ctx, env);
            let mut then_live = live.clone();
            let narrowed = narrow(cond, true, scope, env);
            check_stmt(then, &mut then_live, scope, return_ty, ctx, env);
            narrowed.restore(scope);
            if let Some(else_stmt) = else_ {
                let mut else_live = live.clone();
                let narrowed = narrow(cond, false, scope, env);
                check_stmt(else_stmt, &mut else_live, scope, return_ty, ctx, env);
                narrowed.restore(scope);
                *live = if terminates(then) {
                    else_live
                } else if terminates(else_stmt) {
                    then_live
                } else {
                    then_live.intersection(&else_live).cloned().collect()
                };
            }
            // No `else`: only the pre-existing `live` carries forward.
        }
        StmtKind::While { cond, body } => {
            check_expr(cond, None, live, scope, ctx, env);
            let mut body_live = live.clone();
            // The condition is re-tested before every entry, so what it
            // proves holds for the whole body — unlike anything proved
            // outside the loop, which `suspend` puts out of reach.
            let suspended = suspend(scope);
            let narrowed = narrow(cond, true, scope, env);
            check_stmt(body, &mut body_live, scope, return_ty, ctx, env);
            narrowed.restore(scope);
            suspended.resume(scope);
        }
        StmtKind::DoWhile { body, cond } => {
            // The body runs at least once, so its assignments carry forward.
            let suspended = suspend(scope);
            check_stmt(body, live, scope, return_ty, ctx, env);
            suspended.resume(scope);
            check_expr(cond, None, live, scope, ctx, env);
        }
        StmtKind::For {
            init,
            cond,
            step,
            body,
        } => {
            for e in init {
                check_expr(e, None, live, scope, ctx, env);
            }
            // `cond` always runs at least once, even if the body never does.
            for e in cond {
                check_expr(e, None, live, scope, ctx, env);
            }
            let mut body_live = live.clone();
            let suspended = suspend(scope);
            check_stmt(body, &mut body_live, scope, return_ty, ctx, env);
            for e in step {
                check_expr(e, None, &mut body_live, scope, ctx, env);
            }
            suspended.resume(scope);
        }
        StmtKind::Foreach {
            subject,
            key,
            value,
            body,
            ..
        } => {
            let subject_ty = check_expr(subject, None, live, scope, ctx, env);
            let source = crate::expr::foreach_source(subject_ty, subject.span, env);
            let mut body_live = live.clone();
            if let Some(k) = key {
                let ty = lower_optional_type(k.ty.as_ref(), ctx, env);
                crate::expr::check_foreach_key(&source, ty, k, env);
                let name = strip_sigil(span_text(env.src, k.name)).to_owned();
                declare_binding(scope, &name, ty, k.name, false, env);
                body_live.insert(name);
            }
            let value_ty = lower_optional_type(value.ty.as_ref(), ctx, env);
            crate::expr::check_foreach_value(&source, value_ty, value, env);
            let value_name = strip_sigil(span_text(env.src, value.name)).to_owned();
            declare_binding(scope, &value_name, value_ty, value.name, false, env);
            body_live.insert(value_name);
            let suspended = suspend(scope);
            check_stmt(body, &mut body_live, scope, return_ty, ctx, env);
            suspended.resume(scope);
        }
        StmtKind::Switch { subject, cases } => {
            check_expr(subject, None, live, scope, ctx, env);
            // Every case starts fresh from the pre-switch `live` — see the
            // module docs' known gaps on why fallthrough isn't modeled for
            // *within*-case reads. What *is* modeled precisely: a case
            // contributes to what's live after the switch only when it
            // definitely exits there — via a trailing `break`/`continue`, by
            // being the last case and falling off the end, or (excluded from
            // the join, exactly like `if`/`else`'s terminating branch) never
            // reaching after the switch at all because it always
            // returns/throws. A case that silently falls through to the next
            // one (no trailing exit, and not the last case) contributes
            // nothing, since its actual exit point is wherever the case it
            // falls into eventually exits. With no `default` arm, "no case
            // matched" is itself a possible path, so the pre-switch `live`
            // joins the other candidates too.
            let last_index = cases.len().saturating_sub(1);
            let mut candidates: Vec<FxHashSet<String>> = Vec::new();
            for (i, case) in cases.iter().enumerate() {
                let mut case_live = live.clone();
                if let Some(c) = &case.cond {
                    check_expr(c, None, &mut case_live, scope, ctx, env);
                }
                check_block(&case.body, &mut case_live, scope, return_ty, ctx, env);
                let exits = case.body.last();
                if exits.is_some_and(terminates) {
                    continue;
                }
                if exits.is_some_and(ends_in_break_or_continue) || i == last_index {
                    candidates.push(case_live);
                }
            }
            if cases.iter().all(|c| c.cond.is_some()) {
                candidates.push(live.clone());
            }
            if let Some(merged) = candidates
                .into_iter()
                .reduce(|a, b| a.intersection(&b).cloned().collect())
            {
                *live = merged;
            }
            // No candidates at all: every case terminates, so nothing after
            // the switch is reachable — `live` stays as-is, unused.
        }
        StmtKind::Break(Some(e)) | StmtKind::Continue(Some(e)) => {
            check_expr(e, None, live, scope, ctx, env);
        }
        StmtKind::Break(None) | StmtKind::Continue(None) => {}
        StmtKind::Try {
            body,
            catches,
            finally,
        } => {
            // `body` and each `catch` start fresh from the pre-`try` `live`
            // (an exception can interrupt `body` before any of its own
            // assignments run, so a `catch` can never assume more than that;
            // `finally` is checked the same way below, for the same reason —
            // it can also be entered by an exception thrown on `body`'s very
            // first statement). What each contributes to what's live *after*
            // the whole construct is the ordinary `if`/`else`-style join
            // across every way it can finish normally: `body` completing
            // with no exception, or any `catch` completing — each excluded
            // from the join when it always returns/throws instead, exactly
            // like a terminating `if`/`else` branch.
            let mut candidates: Vec<FxHashSet<String>> = Vec::new();
            let mut body_live = live.clone();
            check_block(&body.stmts, &mut body_live, scope, return_ty, ctx, env);
            if !body.stmts.last().is_some_and(terminates) {
                candidates.push(body_live);
            }
            for catch in catches {
                let mut catch_live = live.clone();
                if let Some(var) = catch.var {
                    let ty = lower_type(&catch.ty, ctx, env);
                    let name = strip_sigil(span_text(env.src, var)).to_owned();
                    declare_binding(scope, &name, ty, var, false, env);
                    catch_live.insert(name);
                }
                check_block(
                    &catch.body.stmts,
                    &mut catch_live,
                    scope,
                    return_ty,
                    ctx,
                    env,
                );
                if !catch.body.stmts.last().is_some_and(terminates) {
                    candidates.push(catch_live);
                }
            }
            let merged = candidates
                .into_iter()
                .reduce(|a, b| a.intersection(&b).cloned().collect());
            // `finally` always runs, so its assignments carry forward
            // regardless of which candidate above actually happened — union
            // them in rather than discarding the candidates' join.
            if let Some(finally) = finally {
                let mut finally_live = live.clone();
                check_block(
                    &finally.stmts,
                    &mut finally_live,
                    scope,
                    return_ty,
                    ctx,
                    env,
                );
                *live = match merged {
                    Some(m) => m.union(&finally_live).cloned().collect(),
                    None => finally_live,
                };
            } else if let Some(m) = merged {
                *live = m;
            }
            // No `finally` and no candidates: `body` and every `catch`
            // terminate, so nothing after the `try` is reachable — `live`
            // stays as-is, unused.
        }
        StmtKind::Echo(xs) => {
            for x in xs {
                let ty = check_expr(x, None, live, scope, ctx, env);
                require_stringable(ty, x.span, env);
            }
        }
        StmtKind::Unset(xs) => {
            for x in xs {
                check_unset_target(x, live, scope, ctx, env);
            }
        }
        StmtKind::LocalDecl { ty, name, value } => {
            let name_str = strip_sigil(span_text(env.src, *name)).to_owned();
            match ty {
                Some(ty) => {
                    let declared_ty = lower_type(ty, ctx, env);
                    declare_binding(scope, &name_str, declared_ty, *name, true, env);
                    if let Some(value) = value {
                        check_expr(value, Some(declared_ty), live, scope, ctx, env);
                        live.insert(name_str);
                    }
                }
                // ADR 0037: `var` — the parser never produces this without
                // an initializer. Its type is synthesized the same way an
                // `echo` argument's is (`check_expr` with no `expected`),
                // then fixed onto the binding exactly as if it had been
                // written out by hand.
                None => {
                    let value = value
                        .as_ref()
                        .expect("parser guarantees `var`'s initializer");
                    if matches!(value.kind, ExprKind::ArrayLiteral(_)) {
                        env.diags.report(
                            Diagnostic::error(
                                code::E_VAR_ARRAY_LITERAL_NEEDS_TYPE,
                                "`var` cannot infer an array literal's element type",
                            )
                            .with_primary(
                                value.span,
                                "no target type to check this literal against",
                            )
                            .with_help("write the type explicitly: `array<T> $name = [...];`"),
                        );
                    }
                    let inferred_ty = check_expr(value, None, live, scope, ctx, env);
                    declare_binding(scope, &name_str, inferred_ty, *name, true, env);
                    live.insert(name_str);
                }
            }
        }
        StmtKind::Destructure { target, value } => {
            check_expr(value, None, live, scope, ctx, env);
            walk_destructure_target(target, live, scope, ctx, env);
        }
        StmtKind::Global(_) | StmtKind::Goto(_) | StmtKind::StaticLocal { .. } => {
            // Already rejected constructs (ADR 0008 § 5 / "makes the CFG
            // unstructured") — nothing downstream ever acts on them, same as
            // `mwl_hir::members`'s own walk.
        }
        StmtKind::ClassDecl(_)
        | StmtKind::InterfaceDecl(_)
        | StmtKind::EnumDecl(_)
        | StmtKind::NamespaceDecl(_)
        | StmtKind::UseDecl(_)
        | StmtKind::TypeAliasDecl(_)
        | StmtKind::TopLevelFunction(_)
        | StmtKind::TopLevelConst(_) => {
            // A declaration nested inside a function body is not descended
            // into by this slice — see the module docs' known gaps.
        }
        _ => {}
    }
}

fn walk_destructure_target(
    target: &DestructureTarget,
    live: &mut FxHashSet<String>,
    scope: &mut LocalScope,
    ctx: &Ctx<'_>,
    env: &mut Env<'_>,
) {
    for element in &target.elements {
        match element {
            DestructureElement::Skip => {}
            DestructureElement::Leaf { key, ty, name, .. } => {
                if let Some(k) = key {
                    check_expr(k, None, live, scope, ctx, env);
                }
                let declared = lower_optional_type(ty.as_ref(), ctx, env);
                let name_str = strip_sigil(span_text(env.src, *name)).to_owned();
                declare_binding(scope, &name_str, declared, *name, false, env);
                live.insert(name_str);
            }
            DestructureElement::Nested { key, target, .. } => {
                if let Some(k) = key {
                    check_expr(k, None, live, scope, ctx, env);
                }
                walk_destructure_target(target, live, scope, ctx, env);
            }
            _ => {}
        }
    }
}
