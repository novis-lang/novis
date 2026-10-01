//! Per-function-body local-variable checking: `rule:types/declaration`'s declare-once
//! rule ("there is no shadowing") and flow-sensitive definite assignment
//! ("reading a binding on a path that may not have reached its initialiser
//! is a compile error").
//!
//! This is a structural walk of the AST, not a CFG — the milestone has no IR
//! yet, and every other check in this crate already walks the tree directly
//! (mirroring [`nvs_hir::members`]'s own shape). [`check_block`] threads two
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
//! "a second declared type for one storage location," which `rule:types/declaration`
//! explicitly forbids for a reference and, by the same reasoning, for a
//! plain local too.
//!
//! # Narrowing a local through `is`, `!= null` or a literal
//!
//! [`narrow`] is the third piece of state this walk threads, and the only
//! one that is *not* cloned at a branch point: [`LocalScope::narrowed`] is a
//! single forward-walked map from a name to the type it provably holds on
//! the path being checked right now. A `!= null`/`== null` test over a
//! plain variable, an `is` one over the same, or a comparison of one
//! against a written literal, installs one entry for the branch it proves —
//! the latter two on the edge where the test holds alone, `rule:types/narrowing`'s
//! true-edge rule and `rule:types/literal-types`'s guard row — and the branch's end
//! restores what was there before — **unless a write already
//! removed it**, in which case the write wins and nothing is put back (see
//! [`Narrowing`], which records what it installed so it can tell the two
//! apart).
//!
//! Two rules keep that sound, and both are load-bearing because `nvs-ir`
//! turns a narrowed receiver into an *unchecked* `nvs_ir::ir::InstKind::Untag`:
//!
//! * **Every path that can change what a local holds calls
//!   [`LocalScope::overwrite`]**, which drops the narrowing and answers the
//!   *declared* type — so `if ($m !== null) { $m = null; $m->x(); }` refuses
//!   the second statement, and `$m = null;` itself is still checked against
//!   `?M` rather than against the narrowed `M`. Adding a new write path to
//!   this crate owes a call to it; that list is `check_assign`,
//!   `check_compound_assign`, `PreIncDec`/`PostIncDec`, `check_inout_arg` and
//!   `check_unset_target`. A path that *declares* the name instead —
//!   [`declare_binding`] and [`bind_catch_arm`] below — calls
//!   [`LocalScope::drop_narrowing`], which drops the same narrowing and
//!   resolves nothing, because a declaration is not a write of an enclosing
//!   binding that shares the spelling.
//! * **A loop body drops every narrowing installed outside it**
//!   ([`check_stmt`]'s `While`/`DoWhile`/`For`/`Foreach` arms), because a
//!   write at the *end* of the body invalidates a read at its start on the
//!   next iteration and this walk has no back edge to discover that on. A
//!   `while` condition's own narrowing is installed after that drop and is
//!   sound, since the condition is re-tested before every entry.
//!
//! **Every `?T` narrows, to whatever dropping `null` leaves** — see
//! [`narrow`] for the restriction that used to sit here, and for what lifted
//! it: the narrowing is recorded on the variable read's own span
//! ([`crate::expr_table::ExprInfo::NarrowedRead`]) and `nvs-ir` discharges it
//! once, where the value is produced, rather than at each consumer. **An `is`
//! narrows to the type it names**, which is that same discharge with no
//! restriction on the type at all — a `mixed` or a union subject is one
//! `Ty::Tagged` slot, and the proved type is exactly what the `Untag` relabels
//! it to. `rule:types/narrowing` makes `is` the general spelling, the one test
//! that reaches a class as readily as a scalar, and [`type_test_residue`] owns
//! which edge proves it. **A literal
//! comparison narrows to the literal's own type**, an enum case included,
//! which costs nothing below the checker at all: `rule:types/literal-types` gives a literal
//! type and an enum-case type their base's representation exactly, so the read
//! is the same one either way.
//!
//! **A `match (true)`/`switch (true)` label is a condition**, so each arm body
//! is checked under whatever the tests above prove for its own label —
//! [`is_true_literal`] owns which subject qualifies, and why a `default` arm
//! and a comma-separated run of labels are given nothing.
//!
//! **Every `switch` case is checked from what was live before the whole
//! `switch`**, whether it is reached by its own label or by falling out of the
//! case above it. That is the join of those two entries rather than an
//! approximation of it: a case is always reachable by its own label, so a name
//! the case above assigned is not live on every way in, and intersecting the
//! entries leaves exactly the pre-`switch` set. What it costs is one program PHP
//! runs: a read whose only assignment is the case above it is refused, which is
//! `rule:types/declaration`'s *definite assignment is checked* over the entry a
//! fall-through does not remove, and the fix is a binding made before the
//! `switch`. The `Switch` arm below documents what the same walk models
//! precisely, which is what each case contributes to what is live *after* it.
//!
//! **A type declaration reaching this walk is refused, not descended into.**
//! [`crate::check::check_stmts`] matches `class`/`interface`/`enum` at file
//! scope itself and never forwards one here, so arriving is proof of nesting
//! — inside a method body, a property hook, a closure body, or a block at
//! file scope — and [`nested_declaration`] reports `E0233` on the spot.
//! The decision is `docs/adr/README.md` § *Decisions taken at project start*:
//! a name whose existence depends on control flow has no reading the static
//! class table can give it. Nothing below the checker ever sees one, which
//! is why `nvs_ir::lower::stmt`'s dispatch does not carry an arm for it.

use nvs_diagnostics::{Diagnostic, Span, code};
use nvs_syntax::ast::{DestructureElement, DestructureTarget, Expr, ExprKind, Stmt, StmtKind};
use rustc_hash::{FxHashMap, FxHashSet};

use crate::expr::{
    check_array_key_type, check_condition, check_expr, check_expr_stmt, check_return,
    check_unset_target, int_literal_digits, is_assignable, reject_finish_marker_arm,
    reject_secret_output, report_mismatch, require_stringable,
};
use crate::expr_table::ExprInfo;
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
/// the whole body, since declaration is function-scoped (`rule:types/declaration`), not
/// block-scoped. A closure gets a fresh one of its own: `rule:types/closure-literal`'s capture is
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
    /// The bindings an expression-level `catch` arm has in scope right now —
    /// `rule:expressions/catch-expression`'s `catch (T $e) => …`, whose `$e` is a local of the
    /// enclosing function exactly as a clause's is.
    ///
    /// A [`RefCell`](std::cell::RefCell) for the reason `narrowed` is one, and
    /// a side table rather than an entry in `by_name` for one more: an arm is
    /// the only place in the language where an *expression* introduces a
    /// binding, so there is no statement boundary afterwards at which the name
    /// could be taken back out. [`ArmBinding`] is that boundary.
    arm_bound: std::cell::RefCell<FxHashMap<String, LocalInfo>>,
}

/// The outer bindings a closure body may read, and the ones it actually did.
///
/// `rule:types/implicit-capture`
/// captures "exactly the outer variables its body reads," which is a fact
/// about the body rather than about the enclosing scope — so `available`
/// holds every name that *could* be captured, and `used` accumulates the ones
/// a read or a write actually reached, in first-touch order. That order is
/// what `nvs-ir` lays the closure object's fields out in, so it has to be
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
        if let Some(info) = self.arm_bound.borrow().get(name) {
            return Some(info.ty);
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

    /// Whether a receiver is in scope at all — `$this` declared by this body
    /// (`crate::check` seeds it for a non-`static` method and for a property
    /// hook) or reachable from an enclosing one.
    ///
    /// Deliberately *not* [`Self::declared_ty`], which records a capture as a
    /// side effect: this question is asked by
    /// `crate::expr::calls::infer_static_call` about a call that never
    /// mentions `$this`, and answering it must not make a closure capture one
    /// it does not use (`rule:statements/a-closure-binds-this-only-where-it-uses-it`).
    pub(crate) fn holds_receiver(&self) -> bool {
        self.by_name.contains_key("this")
            || self
                .captures
                .as_ref()
                .is_some_and(|c| c.available.contains_key("this"))
    }

    /// The narrowed type a dominating `!= null` test proved for `name`, or
    /// `None` where nothing narrowed it.
    ///
    /// [`Self::declared_ty`] deliberately answers the narrowed type without
    /// saying that it *is* one, which is right for every check it feeds. The
    /// one caller that needs to know the difference is `crate::expr`'s
    /// variable-read arm, because the fact has to reach `nvs-ir` on the read's
    /// own span — see [`crate::expr_table::ExprInfo::NarrowedRead`].
    pub(crate) fn narrowed_ty(&self, name: &str) -> Option<TypeId> {
        self.narrowed.borrow().get(name).copied()
    }

    /// What `name` was *declared* as, dropping any narrowing on it first —
    /// the call every path that can change what `name` holds must make.
    ///
    /// Both halves matter. Answering the declared type is what lets
    /// `$m = null;` stay legal inside an `if ($m !== null)` block, where the
    /// narrowed type would refuse it; dropping the narrowing is what makes a
    /// later `$m->x()` in that same block a diagnostic again. The module docs
    /// list every caller — a new one that forgets this is the one way a
    /// narrowing can go stale, and `nvs-ir` trusts it unconditionally.
    pub(crate) fn overwrite(&self, name: &str) -> Option<TypeId> {
        self.narrowed.borrow_mut().remove(name);
        for layer in self.shadowed.borrow_mut().iter_mut() {
            layer.remove(name);
        }
        self.declared_ty(name)
    }

    /// Drops whatever a dominating condition proved about `name`, resolving
    /// nothing.
    ///
    /// A `foreach`, `catch` or destructuring binding *declares* `name` in this
    /// body, so it is not a write of an enclosing binding that happens to share
    /// the spelling. [`Self::overwrite`] is the wrong call for one: it resolves
    /// the name, which records a capture as a side effect (see
    /// [`Self::declared_ty`]), and `nvs-ir` then panics on a capture the
    /// enclosing frame has no binding for. The narrowing still has to go — from
    /// here on the name means the new binding.
    pub(crate) fn drop_narrowing(&self, name: &str) {
        self.narrowed.borrow_mut().remove(name);
        for layer in self.shadowed.borrow_mut().iter_mut() {
            layer.remove(name);
        }
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
        // A closure written inside a `catch` arm may capture the arm's own
        // binding, which is a local of this body like any other for as long as
        // the arm lasts.
        for (name, info) in self.arm_bound.borrow().iter() {
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
    pub(crate) fn restore(self, scope: &LocalScope) {
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

/// The `$x != null`/`$x == null` test `cond` is, if it is one at all: the
/// tested variable's name span, and whether the test *holding* means the
/// variable is not `null`.
///
/// `==`/`!=` are the whole of it, because
/// `rule:expressions/one-equality-operator` leaves one spelling and its § 3 makes it a tag test rather than PHP's
/// truthy-table question — `0 == null` was *true* in PHP, which is why this
/// read only `===`/`!==` while both spellings existed. A bare `if ($x)` is
/// still not a null test: `rule:enums/truthiness` makes it one for a nullable object,
/// but not for a `?string` holding `""`.
fn null_test(cond: &Expr) -> Option<(Span, bool)> {
    match &cond.kind {
        ExprKind::Paren(inner) => null_test(inner),
        ExprKind::Unary {
            op: nvs_syntax::ast::UnaryOp::Not,
            expr: inner,
        } => null_test(inner).map(|(span, non_null)| (span, !non_null)),
        ExprKind::Binary { op, lhs, rhs }
            if matches!(
                op,
                nvs_syntax::ast::BinaryOp::Eq | nvs_syntax::ast::BinaryOp::NotEq
            ) =>
        {
            let non_null = *op == nvs_syntax::ast::BinaryOp::NotEq;
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
/// Each test below installs one, and they are tried in this order because no
/// two of them match one condition. **A `!= null` test drops `null` and keeps
/// the rest** — `rule:expressions/nullable-conversion`'s body's own rule, and every residue takes it: a
/// class, an `array<T>`, a scalar, or a union of them. **An `is` test proves
/// the type it names, on its true edge only** — `rule:types/narrowing`'s
/// general spelling, the one that reaches a class as well as every other type
/// a value can inhabit; see [`type_test_residue`] for why the false edge
/// proves nothing. **A
/// comparison against a written literal proves that literal's own type** —
/// `rule:types/literal-types`'s guard row, and [`literal_residue`] owns which spellings
/// reach it.
///
/// This used to be restricted to a single-class residue, because `nvs-ir`
/// lowers a `?T` local's slot as [`Ty::Tagged`](nvs_ir::ty::Ty) whatever the
/// checker later proves about it, and the only *consumer* that narrowed back
/// out of one was a call receiver. What lifted the restriction is that the
/// narrowing is now recorded on the variable read's own span
/// ([`crate::expr_table::ExprInfo::NarrowedRead`]) and discharged once, where
/// `nvs-ir` produces the value, rather than at each consumer that wants it —
/// so a subscript base, a `foreach` subject, an array-write root and an
/// argument all see the narrow representation with no site left to forget.
///
/// These tests are what a *condition* proves, so every site that writes
/// one reaches this: the `if`/`while` arms below, the guard clause
/// [`check_block`] carries, a ternary's two arms and the right operand of `&&`
/// and `||` (`crate::expr::infer`), and — through [`is_true_literal`] — each
/// label of a `match (true)`/`switch (true)`, which is `rule:types/narrowing`'s
/// fourth spelling and is a label only in where it is written.
pub(crate) fn narrow(cond: &Expr, when: bool, scope: &LocalScope, env: &mut Env<'_>) -> Narrowing {
    let residue = match null_residue(cond, when, scope, env) {
        Some(found) => Some(found),
        None => match type_test_residue(cond, when, scope, env) {
            Some(found) => Some(found),
            None => literal_residue(cond, when, scope, env),
        },
    };
    let Some((name, residue)) = residue else {
        return Narrowing::default();
    };
    let previous = scope.narrowed.borrow_mut().insert(name.clone(), residue);
    Narrowing(vec![(name, residue, previous)])
}

/// The local a `!= null` test narrows on the branch where it evaluates to
/// `when`, and what it leaves.
fn null_residue(
    cond: &Expr,
    when: bool,
    scope: &LocalScope,
    env: &mut Env<'_>,
) -> Option<(String, TypeId)> {
    let (name_span, non_null_when_true) = null_test(cond)?;
    if non_null_when_true != when {
        return None;
    }
    let name = strip_sigil(span_text(env.src, name_span)).to_owned();
    let current = scope.declared_ty(&name)?;
    let residue = env.interner.without_null(current);
    (residue != current).then_some((name, residue))
}

/// The local an `is` test narrows on the branch where it evaluates to `when`,
/// and the type it proves.
///
/// The general spelling of narrowing (`rule:types/narrowing`), and the only one
/// that reaches a class: it proves any type a value can inhabit — a scalar,
/// `null`, an `array<T>`, a shape, a literal, an enum case, a class or a union
/// of them.
///
/// **Only the edge where the test holds proves anything**: `$x is Foo` being
/// *false* leaves the declared type untouched, since every other type it could
/// hold is still in it. That is the asymmetry with [`null_residue`], where both
/// edges name a type. A `!` flips which branch that is rather than removing it,
/// so the guard clause a ported program writes —
/// `if (!($x is Foo)) { return; }` — narrows everything after it, exactly as
/// the `== null` spelling already did.
///
/// **Nothing about the subject is checked here.** `is` refuses no left-hand
/// side at all (ADR 0150 § 6), so a declared `int` narrowing to a literal `1`
/// is an ordinary case, and a subject that can hold no object has already
/// folded rather than recorded anything for the value arm.
///
/// The type comes from `crate::expr_table::ExprInfo::TypeTest`, or the base
/// from `ExprInfo::ClassRefTest` for the value arm, recorded by
/// [`crate::expr::type_test::infer_type_test`] when the condition was checked a
/// moment earlier: a written type is placed by the namespace and the imports of
/// the site that wrote it and interned there, and this walk carries neither.
/// **Which variant is on the span is the guard**: the checker records these two
/// only for a test whose answer is a run-time `bool`, so a type that already
/// covers the declared one folded to `true`, carries a
/// `crate::expr_table::ExprInfo::SettledTypeTest` instead, and never arrives
/// here to widen a binding.
///
/// A `class<T>` operand narrows the subject to **`T`**, which is sound because
/// the reference holds a `T` or an implementor of one
/// (`rule:types/class-reference-sites`).
fn type_test_residue(
    cond: &Expr,
    when: bool,
    scope: &LocalScope,
    env: &mut Env<'_>,
) -> Option<(String, TypeId)> {
    let (name_span, test_span, proved_when) = type_test(cond)?;
    if proved_when != when {
        return None;
    }
    let name = strip_sigil(span_text(env.src, name_span)).to_owned();
    let current = scope.declared_ty(&name)?;
    let residue = match env.exprs.lookup(test_span) {
        Some(&ExprInfo::TypeTest { tested }) => tested,
        Some(&ExprInfo::ClassRefTest { base }) => base,
        _ => return None,
    };
    (residue != current).then_some((name, residue))
}

/// The `$x is Type` test `cond` is, if it is one at all: the tested variable's
/// name span, the whole test's own span — which is the key
/// `crate::expr_table::ExprInfo::TypeTest` was recorded under — and whether the
/// type is proved when the condition *holds*, which a `!` inverts.
fn type_test(cond: &Expr) -> Option<(Span, Span, bool)> {
    match &cond.kind {
        ExprKind::Paren(inner) => type_test(inner),
        ExprKind::Unary {
            op: nvs_syntax::ast::UnaryOp::Not,
            expr: inner,
        } => type_test(inner).map(|(name, test, proved)| (name, test, !proved)),
        ExprKind::TypeTest { expr, .. } => match &expr.kind {
            ExprKind::Variable(span) => Some((*span, cond.span, true)),
            _ => None,
        },
        _ => None,
    }
}

/// The local a comparison against a written literal narrows on the branch
/// where it evaluates to `when`, and the literal type it proves.
///
/// `rule:types/literal-types`'s own
/// row: a wider literal union reaches a narrower one through a guard, and `==`
/// is that guard's simplest spelling. `==` proves the literal where it holds
/// and `!=` where it does not, which is the same edge written two ways.
///
/// A **string or integer literal's** type is built from its own **text**
/// rather than from what the condition inferred a moment ago, because an
/// operand's inferred type is not recorded anywhere and re-inferring one would
/// report its escape-grammar diagnostics a second time. Those two cook to a
/// value with no context at all, which is what makes reading the text sound
/// for them and unsound for the third spelling.
///
/// An **enum case** (`$m == Mode::Read`) is that third one, and it is read
/// back off [`crate::expr_table::ExprInfo::EnumCase`] instead, for
/// [`type_test_residue`]'s reason: which case a written name means is a
/// question about the namespace and the imports of the site that wrote it, and
/// this walk carries neither. The residue is `rule:types/enum-case-type`'s `Ty::EnumCase` and
/// not the whole enum — that is the point of the guard — and it costs nothing
/// below the checker for § 5's reason, a case being its backing integer in
/// every representation. The roster is closed at those three: anything else
/// records no [`ExprInfo`] this can read and narrows nothing.
///
/// The residue has to be a **subtype of what the local was declared**, so a
/// comparison the declared type does not admit narrows nothing — it is a guard
/// reaching one member of a union, never a re-declaration.
pub(crate) fn literal_residue(
    cond: &Expr,
    when: bool,
    scope: &LocalScope,
    env: &mut Env<'_>,
) -> Option<(String, TypeId)> {
    let (name_span, literal, proved_when) = literal_test(cond)?;
    if proved_when != when {
        return None;
    }
    let name = strip_sigil(span_text(env.src, name_span)).to_owned();
    let current = scope.declared_ty(&name)?;
    let residue = match &literal.kind {
        ExprKind::Str(span) => {
            let value = crate::string_lit::cook_string_literal(env.src, *span);
            env.interner.string_literal(value)
        }
        ExprKind::Int(span) => {
            let (radix, digits) = int_literal_digits(env.src, *span);
            env.interner
                .int_literal(i64::from_str_radix(&digits, radix).ok()?)
        }
        // `rule:types/literal-types`'s guard row over an enum: `$m == Mode::Read` proves the
        // case's own type, which is `Ty::EnumCase` rather than the enum. The
        // enum and the case are read back off
        // [`crate::expr_table::ExprInfo::EnumCase`], recorded when the operand
        // was checked a moment ago, for [`type_test_residue`]'s reason
        // exactly — the name is placed by the writing site's namespace and
        // imports, and this walk carries neither.
        _ => {
            let Some(ExprInfo::EnumCase { enum_, case, .. }) = env.exprs.lookup(literal.span)
            else {
                return None;
            };
            let (enum_, case) = (enum_.clone(), case.clone());
            let backing = env.enums.backing_of(&enum_);
            env.interner.enum_case(enum_, backing, case)
        }
    };
    if residue == current {
        return None;
    }
    is_assignable(residue, current, env.interner, env.graph, env.signatures)
        .then_some((name, residue))
}

/// The `$x == <literal>` test `cond` is, if it is one at all: the tested
/// variable's name span, the literal operand, and whether the literal is
/// proved when the condition *holds* — which `!=` inverts, and a `!` inverts
/// again.
///
/// Either operand may be the variable: `rule:expressions/one-equality-operator` leaves one equality
/// operator and it is symmetric, so `"read" == $mode` is the same test.
fn literal_test(cond: &Expr) -> Option<(Span, &Expr, bool)> {
    match &cond.kind {
        ExprKind::Paren(inner) => literal_test(inner),
        ExprKind::Unary {
            op: nvs_syntax::ast::UnaryOp::Not,
            expr: inner,
        } => literal_test(inner).map(|(name, lit, proved)| (name, lit, !proved)),
        ExprKind::Binary { op, lhs, rhs }
            if matches!(
                op,
                nvs_syntax::ast::BinaryOp::Eq | nvs_syntax::ast::BinaryOp::NotEq
            ) =>
        {
            let proved = *op == nvs_syntax::ast::BinaryOp::Eq;
            match (&lhs.kind, &rhs.kind) {
                (ExprKind::Variable(span), _) => Some((*span, rhs, proved)),
                (_, ExprKind::Variable(span)) => Some((*span, lhs, proved)),
                _ => None,
            }
        }
        _ => None,
    }
}

/// Whether `subject` is the written literal `true` — the subject of `rule:types/narrowing`'s `match (true)` spelling, and of the `switch (true)` one beside it.
///
/// Under that subject a label is not a value the subject is compared against
/// but a **condition** in its own right, so an arm is reached exactly where
/// its label held and the body may be checked under everything [`narrow`]
/// would install for it. Only the written literal counts, and deliberately not
/// a `bool` local that happens to hold `true`: what makes the spelling narrow
/// is that the label's own truth is what selected the arm, and a variable
/// subject says only that the two agree — `$flag == ($x is Foo)`
/// proves the class on neither edge.
///
/// A label proves nothing for any *other* arm, so nothing is installed for a
/// `default` arm or for one of a comma-separated run: `match (true)` evaluates
/// labels in order and the arm taken is the first that held, which says the
/// earlier ones did not — a residue this pass has no way to subtract, `rule:types/narrowing`'s narrowings each naming a type rather than removing one.
pub(crate) fn is_true_literal(subject: &Expr) -> bool {
    match &subject.kind {
        ExprKind::Paren(inner) => is_true_literal(inner),
        ExprKind::Bool(value) => *value,
        _ => false,
    }
}

/// A set of narrowings [`suspend`] moved out of reach for a loop body, to be
/// put back by [`Suspended::resume`] when the body is done with.
#[must_use]
pub(crate) struct Suspended;

impl Suspended {
    /// Puts back every suspended narrowing a write inside the body did not
    /// invalidate — dropping a narrowing reaches into the suspended layer too,
    /// through either [`LocalScope::overwrite`] or
    /// [`LocalScope::drop_narrowing`], so anything the body assigned or
    /// redeclared is simply no longer there.
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
    // A `foreach`/`catch`/destructuring binding declares the name rather than
    // writing an enclosing one of the same spelling, so it drops what a
    // dominating condition proved about it without resolving it — see
    // [`LocalScope::drop_narrowing`].
    scope.drop_narrowing(name);
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

/// Declares an expression-level `catch` arm's `$e` for the length of that arm
/// — `rule:expressions/catch-expression`'s "a local of the enclosing function under the block form's
/// rule", written against the `&LocalScope` the expression checker threads.
///
/// The rule is [`declare_binding`]'s non-strict path: reuse when an existing
/// declaration's type matches exactly, and `E0406` otherwise. What differs is
/// only where the binding lives and how it ends — see
/// [`LocalScope::arm_bound`] and [`ArmBinding`].
pub(crate) fn bind_catch_arm(
    scope: &LocalScope,
    name: &str,
    ty: TypeId,
    span: Span,
    env: &mut Env<'_>,
) -> ArmBinding {
    // The arm declares the name, so it drops whatever a dominating condition
    // proved about it without resolving it — [`declare_binding`] takes the
    // same path, and [`LocalScope::drop_narrowing`] owns why.
    scope.drop_narrowing(name);
    let existing = {
        let arms = scope.arm_bound.borrow();
        arms.get(name)
            .or_else(|| scope.by_name.get(name))
            .map(|info| (info.ty, info.declared_span))
    };
    if let Some((existing_ty, first)) = existing {
        if existing_ty != ty {
            env.diags.report(
                Diagnostic::error(
                    code::E_REDECLARED_LOCAL,
                    format!("`${name}` is already declared"),
                )
                .with_primary(span, "second declaration")
                .with_secondary(first, "first declared here"),
            );
        }
        return ArmBinding(None);
    }
    scope.arm_bound.borrow_mut().insert(
        name.to_owned(),
        LocalInfo {
            ty,
            declared_span: span,
        },
    );
    ArmBinding(Some(name.to_owned()))
}

/// One arm binding, and what it takes to end it — see [`bind_catch_arm`].
///
/// Ending it is not tidiness: only the caught value ever assigns the name and
/// no path out of the guarded expression carries one, so a binding left behind
/// would reach `nvs-ir` as a readable local instead of the definite-assignment
/// error a later `$e` is. The block form's `StmtKind::Try` arm removes the
/// same name from `live` for the same reason.
#[derive(Debug)]
pub(crate) struct ArmBinding(Option<String>);

impl ArmBinding {
    /// Ends the binding. An arm that reused an existing declaration holds
    /// nothing and this is a no-op, exactly as a clause's own reuse path is.
    pub(crate) fn release(self, scope: &LocalScope) {
        if let Some(name) = self.0 {
            scope.arm_bound.borrow_mut().remove(&name);
        }
    }
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
///
/// A guard that ends in `break` or `continue` carries the same narrowing, and
/// for the same reason: neither reaches the statement after the `if`, so the
/// rest of this block is only ever walked on the edge where the condition was
/// false. [`terminates`] itself stays narrower than that, because it also
/// decides what an `if`/`else` join may assume about definite assignment.
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
            && (terminates(then) || ends_in_break_or_continue(then))
        {
            carried.absorb(narrow(cond, false, scope, env));
        }
    }
    carried.restore(scope);
}

/// Enters a loop body — one more `break`/`continue` target, and a loop.
///
/// A `switch` pushes a `false` inline in its own arm: it is a `break` target
/// without being a loop, and PHP's `break N`/`continue N` count it all the
/// same, which is why [`Env::exit_targets`] is a stack of kinds rather than a
/// count.
fn enter_loop(env: &mut Env<'_>) {
    env.exit_targets.push(true);
}

/// Leaves a loop body, undoing [`enter_loop`].
fn leave_loop(env: &mut Env<'_>) {
    env.exit_targets.pop();
}

/// Whether `continue N` has a loop to reach: the frame it lands on, or any
/// frame outside that one — a level landing on a `switch` walks outward. See
/// [`check_exit_level`] for why that walk is the same rule a bare `continue`
/// inside a `switch` already follows.
fn reaches_a_loop(targets: &[bool], n: usize) -> bool {
    n <= targets.len() && targets[..=targets.len() - n].iter().any(|&is_loop| is_loop)
}

/// `break`/`continue`, with or without PHP's optional level operand.
///
/// The level names the `N`-th enclosing target *statically*, counting every
/// loop and every `switch` exactly as PHP does, so everything this refuses is
/// refused because there is no target to name and not because lowering is
/// missing: a level computed at run time (which PHP stopped accepting in
/// 5.4), a `0`, a level counting past the enclosing depth — which subsumes
/// the bare `break;` written outside every loop — and, for `continue` alone,
/// a level with no loop at or outside the frame it lands on. One code,
/// [`code::E_BREAK_LEVEL`], for all of them.
///
/// That last rule is where `continue` differs, and it is not a second
/// decision: a level landing on a `switch` walks outward to the enclosing
/// loop, which is Novis's already-settled reading of a bare `continue` inside a
/// `switch` (`nvs_ir::lower::Lowering::lower_switch`'s doc comment is its
/// home) generalized to a written level. It makes `continue 2` inside a
/// `switch` — PHP's own idiomatic spelling — mean in Novis what it means in
/// PHP.
fn check_exit_level(
    level: Option<&Expr>,
    stmt_span: Span,
    keyword: &str,
    live: &mut FxHashSet<String>,
    scope: &mut LocalScope,
    ctx: &Ctx<'_>,
    env: &mut Env<'_>,
) {
    let depth = env.exit_targets.len();
    let Some(level) = level else {
        let (ok, enclosing) = if keyword == "break" {
            (depth >= 1, "loop or `switch`")
        } else {
            (reaches_a_loop(&env.exit_targets, 1), "loop")
        };
        if !ok {
            env.diags.report(
                Diagnostic::error(
                    code::E_BREAK_LEVEL,
                    format!("`{keyword}` is not inside a {enclosing}"),
                )
                .with_primary(stmt_span, format!("no enclosing {enclosing} to leave")),
            );
        }
        return;
    };
    // Checked before the shape test so a level written as `$n` still records
    // the read — the binding is a real one either way, and a second
    // diagnostic about an unassigned local is more use than silence.
    check_expr(level, None, live, scope, ctx, env);
    let ExprKind::Int(span) = &level.kind else {
        env.diags.report(
            Diagnostic::error(
                code::E_BREAK_LEVEL,
                format!("a `{keyword}` level must be an integer literal"),
            )
            .with_primary(level.span, "this is computed at run time")
            .with_help(format!(
                "`{keyword} N;` names the N-th enclosing loop or `switch` at compile time, so N \
                 has to be written out"
            )),
        );
        return;
    };
    let (radix, digits) = crate::expr::int_literal_digits(env.src, *span);
    let n = usize::from_str_radix(&digits, radix).unwrap_or(usize::MAX);
    if n == 0 {
        env.diags.report(
            Diagnostic::error(code::E_BREAK_LEVEL, format!("`{keyword} 0` leaves nothing"))
                .with_primary(level.span, "the innermost enclosing statement is level 1"),
        );
        return;
    }
    if n > depth {
        env.diags.report(
            Diagnostic::error(
                code::E_BREAK_LEVEL,
                format!("`{keyword} {n}` names more levels than enclose it"),
            )
            .with_primary(
                level.span,
                format!("{depth} enclosing loop/`switch` here, so the highest level is {depth}"),
            ),
        );
        return;
    }
    if keyword == "continue" && !reaches_a_loop(&env.exit_targets, n) {
        env.diags.report(
            Diagnostic::error(
                code::E_BREAK_LEVEL,
                format!("`continue {n}` names no enclosing loop"),
            )
            .with_primary(
                level.span,
                "every statement at or outside this level is a `switch`",
            )
            .with_help(
                "`continue` needs a loop to start the next iteration of; a `switch` has none, so \
                 a level landing on one looks further out and finds nothing here",
            ),
        );
    }
}

/// `rule:types/grammar`: `void` and `never` are return-only, so neither names a
/// binding either — a written one, or the type a `var` takes from a call that
/// hands nothing back.
///
/// Unrefused, the written form declares a name for a value that does not exist
/// and the inferred form defines that name with nothing at all, which
/// `nvs-ir` reports as an operand used before it is defined — an internal error
/// for a program the checker accepted. Recovers as `mixed` so one refused type
/// costs one diagnostic rather than a second at every use of the name.
fn reject_void_or_never_binding(
    ty: TypeId,
    span: Span,
    inferred: bool,
    env: &mut Env<'_>,
) -> TypeId {
    let atom = match env.interner.get(ty) {
        Ty::Void => "void",
        Ty::Never => "never",
        _ => return ty,
    };
    let (label, help) = if inferred {
        (
            format!("this hands back `{atom}`"),
            format!(
                "a call returning `{atom}` gives nothing to name — call it as a statement of its \
                 own, on a line with no `var`"
            ),
        )
    } else {
        (
            format!("declared `{atom}` here"),
            format!(
                "`{atom}` says what a call hands back, not what a name holds — write the type the \
                 value actually has, or drop the binding"
            ),
        )
    };
    env.diags.report(
        Diagnostic::error(
            code::E_VOID_OR_NEVER_OUTSIDE_RETURN,
            format!("`{atom}` is a return type only, and this is a binding"),
        )
        .with_primary(span, label)
        .with_help(help),
    );
    env.interner.mixed()
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
        // `check_expr_stmt`, not `check_expr`: `yield $v;` and `require '…';`
        // are the two shapes that mean something *only* as a statement, and
        // that function is the one home for the split. See its doc comment.
        StmtKind::Expr(e) => {
            check_expr_stmt(e, live, scope, ctx, env);
        }
        StmtKind::Return(Some(e)) => {
            check_return(e, return_ty, live, scope, ctx, env);
        }
        StmtKind::Return(None) => {
            // Nothing to check about the value there is not: whether a bare
            // `return;` is legal is the declared type alone, and
            // `crate::check`'s `check_body_exits` reads it once per body
            // rather than here, where the declaration is not in reach.
        }
        StmtKind::Block(b) => check_block(&b.stmts, live, scope, return_ty, ctx, env),
        StmtKind::Empty | StmtKind::InlineHtml(_) | StmtKind::Error => {}
        StmtKind::If { cond, then, else_ } => {
            check_condition(cond, live, scope, ctx, env);
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
            check_condition(cond, live, scope, ctx, env);
            let mut body_live = live.clone();
            // The condition is re-tested before every entry, so what it
            // proves holds for the whole body — unlike anything proved
            // outside the loop, which `suspend` puts out of reach.
            let suspended = suspend(scope);
            let narrowed = narrow(cond, true, scope, env);
            enter_loop(env);
            check_stmt(body, &mut body_live, scope, return_ty, ctx, env);
            leave_loop(env);
            narrowed.restore(scope);
            suspended.resume(scope);
        }
        StmtKind::DoWhile { body, cond } => {
            // The body runs at least once, so its assignments carry forward.
            let suspended = suspend(scope);
            enter_loop(env);
            check_stmt(body, live, scope, return_ty, ctx, env);
            leave_loop(env);
            suspended.resume(scope);
            check_condition(cond, live, scope, ctx, env);
        }
        StmtKind::For {
            init,
            cond,
            step,
            body,
        } => {
            // `rule:iteration/for-init-clause` and `rule:iteration/for-counter-scope`: the counter is declared here and is
            // function-scoped, so § 1's declare-once rule applies to it
            // through the same arm — a second `for (int $i = 0; …)` below is
            // the re-declaration diagnostic that arm already reports, and the
            // binding stays live after the loop.
            if let Some(decl) = init.decl() {
                check_stmt(decl, live, scope, return_ty, ctx, env);
            }
            for e in init.exprs() {
                check_expr(e, None, live, scope, ctx, env);
            }
            // `cond` always runs at least once, even if the body never does.
            for e in cond {
                check_condition(e, live, scope, ctx, env);
            }
            let mut body_live = live.clone();
            let suspended = suspend(scope);
            enter_loop(env);
            check_stmt(body, &mut body_live, scope, return_ty, ctx, env);
            leave_loop(env);
            for e in step {
                check_expr(e, None, &mut body_live, scope, ctx, env);
            }
            suspended.resume(scope);
        }
        StmtKind::Foreach {
            subject,
            key,
            value,
            value_inout,
            body,
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
            if *value_inout {
                crate::expr::check_foreach_inout(&source, subject, value_ty, value, env);
            }
            let value_name = strip_sigil(span_text(env.src, value.name)).to_owned();
            declare_binding(scope, &value_name, value_ty, value.name, false, env);
            body_live.insert(value_name);
            let suspended = suspend(scope);
            enter_loop(env);
            check_stmt(body, &mut body_live, scope, return_ty, ctx, env);
            leave_loop(env);
            suspended.resume(scope);
        }
        StmtKind::Switch { subject, cases } => {
            let subject_ty = check_expr(subject, None, live, scope, ctx, env);
            // Every case starts fresh from the pre-switch `live`, which is the
            // join of its two entries rather than a stand-in for one — the
            // module docs own why. What is modeled precisely here: a case
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
            // A `switch` is a `break` target without being a loop — PHP counts
            // it as a level for both keywords, and `continue` then walks out
            // of it to the loop (see `check_exit_level`).
            env.exit_targets.push(false);
            // `rule:types/unions-and-mixed`'s `switch (true)`: every label is a condition, so
            // each case body is checked under what its own label proves.
            let labels_are_conditions = is_true_literal(subject);
            for (i, case) in cases.iter().enumerate() {
                let mut case_live = live.clone();
                let mut narrowed = Narrowing::default();
                if let Some(c) = &case.cond {
                    // `rule:expressions/switch-match-equality`: a `case` label is compared against the
                    // subject by the one equality rule, so a label whose type
                    // is disjoint from the subject's is § 2's refusal written
                    // without the operator.
                    let label_ty = check_expr(c, None, &mut case_live, scope, ctx, env);
                    crate::expr::reject_disjoint_equality(subject_ty, label_ty, c.span, env);
                    if labels_are_conditions {
                        narrowed = narrow(c, true, scope, env);
                    }
                }
                check_block(&case.body, &mut case_live, scope, return_ty, ctx, env);
                narrowed.restore(scope);
                let exits = case.body.last();
                if exits.is_some_and(terminates) {
                    continue;
                }
                if exits.is_some_and(ends_in_break_or_continue) || i == last_index {
                    candidates.push(case_live);
                }
            }
            env.exit_targets.pop();
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
        StmtKind::Break(level) => {
            check_exit_level(level.as_ref(), stmt.span, "break", live, scope, ctx, env);
        }
        StmtKind::Continue(level) => {
            check_exit_level(level.as_ref(), stmt.span, "continue", live, scope, ctx, env);
        }
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
                let mut bound = None;
                // Lowered whether or not the clause binds, because what the
                // refusal below reads is the class the clause *names* and a
                // clause that binds nothing names one just as loudly.
                let ty = lower_type(&catch.ty, ctx, env);
                reject_finish_marker_arm(ty, catch.ty.span, env);
                if let Some(var) = catch.var {
                    let name = strip_sigil(span_text(env.src, var)).to_owned();
                    declare_binding(scope, &name, ty, var, false, env);
                    catch_live.insert(name.clone());
                    // Unless the name was already assigned before the `try`,
                    // in which case the clause reused an existing binding
                    // (`declare_binding`'s non-strict path) and what was live
                    // going in is still live coming out.
                    bound = (!live.contains(&name)).then_some(name);
                }
                check_block(
                    &catch.body.stmts,
                    &mut catch_live,
                    scope,
                    return_ty,
                    ctx,
                    env,
                );
                // The binding ends with its clause. Only the thrown value ever
                // assigns it, and no path out of the `try` carries one, so it
                // must not join `live` even when the clause is the only way
                // the construct finishes normally — that leak is what made a
                // later read reach the lowerer as an undeclared local instead
                // of the definite-assignment error it is.
                if let Some(name) = bound {
                    catch_live.remove(&name);
                }
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
            // `rule:security/response-body-is-one-typed-member`'s sixth row: a response body written by `echo` and
            // by a typed member both. The statement's own span rather than an
            // operand's, because here the *writer* is what a reader has to
            // find — see `crate::response`.
            crate::response::note_echo(stmt.span, env);
            for x in xs {
                let ty = check_expr(x, None, live, scope, ctx, env);
                // `rule:security/secret-sinks-refuse`'s terminal-output sink, which this statement
                // reaches with no member in between — see
                // [`crate::expr::reject_secret_output`], including for why a
                // refused operand is not then asked the stringable question.
                if !reject_secret_output(ty, x.span, "echo", env) {
                    require_stringable(ty, x.span, env);
                }
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
                    let written = lower_type(ty, ctx, env);
                    let declared_ty = reject_void_or_never_binding(written, ty.span, false, env);
                    declare_binding(scope, &name_str, declared_ty, *name, true, env);
                    if let Some(value) = value {
                        check_expr(value, Some(declared_ty), live, scope, ctx, env);
                        live.insert(name_str);
                    }
                }
                // `rule:types/var-inference`: `var` — the parser never produces this without
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
                    let synthesized = check_expr(value, None, live, scope, ctx, env);
                    let inferred_ty =
                        reject_void_or_never_binding(synthesized, value.span, true, env);
                    declare_binding(scope, &name_str, inferred_ty, *name, true, env);
                    live.insert(name_str);
                }
            }
        }
        StmtKind::Destructure { target, value } => {
            let subject = check_expr(value, None, live, scope, ctx, env);
            walk_destructure_target(target, Some(subject), value.span, live, scope, ctx, env);
        }
        StmtKind::Global(_) | StmtKind::Goto(_) | StmtKind::StaticLocal { .. } => {
            // Already rejected constructs (`rule:statements/no-function-static-and-no-global` / "makes the CFG
            // unstructured") — nothing downstream ever acts on them, same as
            // `nvs_hir::members`'s own walk.
        }
        // A declaration only ever reaches this walk from *inside* a body:
        // `crate::check::check_stmts` matches every one of these at file scope
        // and never forwards one here. So the refusals below need no "am I
        // nested" test — arriving is the test. See this module's own doc
        // comment.
        StmtKind::ClassDecl(decl) => nested_declaration("class", decl.name.span, true, env),
        StmtKind::InterfaceDecl(decl) => {
            nested_declaration("interface", decl.name.span, true, env);
        }
        StmtKind::EnumDecl(decl) => nested_declaration("enum", decl.name.span, true, env),
        StmtKind::TypeAliasDecl(decl) => nested_declaration("type", decl.name.span, true, env),
        // The three that introduce no name of their own: they change what the
        // names around them *mean*, for the whole file, which is the same
        // thing control flow has no reading to give.
        StmtKind::NamespaceDecl(decl) => {
            let at = decl.name.as_ref().map_or(stmt.span, |n| n.span);
            nested_declaration("namespace", at, false, env);
        }
        StmtKind::UseDecl(decl) => nested_declaration("use", decl.path.span, false, env),
        StmtKind::AutoloadDecl(decl) => nested_declaration("autoload", decl.span, false, env),
        StmtKind::TopLevelFunction(_) | StmtKind::TopLevelConst(_) => {
            // `E0215`/`E0216` from `nvs_hir::members` already refuse these at
            // every scope, by the rule they actually break (`rule:classes/no-free-functions-or-constants`: a
            // function is a method, a constant belongs to a class), so a
            // second diagnostic here would only say it worse.
        }
        _ => {}
    }
}

/// `E0233` — a declaration written anywhere but file scope, named for what it
/// is rather than left to panic below the checker.
///
/// All seven spellings share one code because they break one rule: every
/// declaration in Novis is in effect before any code runs, so "which statements
/// have run by now" is a question none of the tables below can answer. `kind`
/// is the keyword as written, so the message reads back the source.
///
/// `introduces_a_name` picks between the two halves of that rule, and `at` is
/// the span each half wants underlined:
///
/// * `class`, `interface`, `enum` and `type` **introduce** a name — `at` is
///   that name, the shortest thing to point at and the one part of the
///   declaration a reader has to move.
/// * `namespace`, `use` and `autoload` introduce none; they change what the
///   names around them resolve to. `at` is the shortest span that identifies
///   which one — the namespace's name, the import's path, the whole `autoload`
///   — since there is no declared name to quote back.
pub(crate) fn nested_declaration(kind: &str, at: Span, introduces_a_name: bool, env: &mut Env<'_>) {
    let title = format!("a nested `{kind}` declaration is not supported");
    let diag = if introduces_a_name {
        let name_text = span_text(env.src, at).to_owned();
        Diagnostic::error(code::E_NESTED_TYPE_DECLARATION_UNSUPPORTED, title)
            .with_primary(
                at,
                format!("`{name_text}` would only exist once control reached this statement"),
            )
            .with_help(format!(
                "move `{name_text}` out to file scope — every type in Novis is declared before any \
                 code runs, so a name's existence never depends on control flow"
            ))
    } else {
        Diagnostic::error(code::E_NESTED_TYPE_DECLARATION_UNSUPPORTED, title)
            .with_primary(
                at,
                format!("this `{kind}` would only take effect once control reached this statement"),
            )
            .with_help(format!(
                "move the `{kind}` out to file scope — it applies to the whole file, so what a \
                 name resolves to never depends on control flow"
            ))
    };
    env.diags.report(diag);
}

/// `rule:types/grammar`.3's destructuring target, against the type of the value being
/// taken apart.
///
/// **Every element is an element read.** `[int $a, int $b] = $pair` is
/// `$pair[0]` and `$pair[1]`, keyed exactly as `rule:types/arrays` normalizes them,
/// so this asks a subscript's own three questions at a statement that writes
/// no subscript:
///
/// * the value must be an `array<T>` — a `mixed`, a scalar or an untested
///   `?array<T>` names no element type, which is [`code::E_SUBSCRIPT_ON_NON_ARRAY`]'s
///   rule verbatim and gets its code;
/// * the element type must be assignable to what the leaf declares
///   ([`code::E_TYPE_MISMATCH`]), the same direction and the same covariance a
///   `foreach` value binding gets from [`crate::expr::check_foreach_value`];
/// * a leaf may not bind by reference ([`code::E_ARRAY_ELEMENT_BY_REFERENCE`]) —
///   an aliasing element has no owner under `rule:types/implicit-capture` and `rule:classes/two-copy-depths`, which
///   is that code's rule for an array *literal*'s element and is unchanged
///   here, the leaf being the same element from the other side.
///
/// `subject` is `None` once an enclosing level has already refused: the walk
/// still declares every binding below it — a name that exists is what keeps
/// the rest of the body from reporting a second wave of undeclared-variable
/// errors — but asks nothing more about types it no longer knows.
///
/// It also **records each leaf's element type in the expression table under
/// the leaf's own span**, the same [`ExprInfo::Index`] entry `crate::expr`
/// records for a subscript, because `nvs_ir::lower` lowers the read at the
/// element's representation and coerces to the declared one, and a
/// destructuring statement writes no expression it could hang that on.
fn walk_destructure_target(
    target: &DestructureTarget,
    subject: Option<TypeId>,
    subject_span: Span,
    live: &mut FxHashSet<String>,
    scope: &mut LocalScope,
    ctx: &Ctx<'_>,
    env: &mut Env<'_>,
) {
    let elem_ty = subject.and_then(|subject| match env.interner.get(subject) {
        Ty::Array(elem) => Some(*elem),
        _ => {
            let found = env.interner.describe(subject);
            env.diags.report(
                Diagnostic::error(
                    code::E_SUBSCRIPT_ON_NON_ARRAY,
                    format!("cannot destructure `{found}`"),
                )
                .with_primary(subject_span, format!("this is `{found}`"))
                .with_help(
                    "destructuring reads elements out of an `array<T>` — declare one, or read \
                     the value out by hand",
                ),
            );
            None
        }
    });
    for element in &target.elements {
        match element {
            DestructureElement::Skip => {}
            DestructureElement::Leaf {
                key,
                ty,
                inout,
                name,
                span,
            } => {
                check_destructure_key(key.as_ref(), live, scope, ctx, env);
                if *inout {
                    report_inout_leaf(*span, env);
                }
                let declared = lower_optional_type(ty.as_ref(), ctx, env);
                if let Some(elem_ty) = elem_ty {
                    if is_assignable(elem_ty, declared, env.interner, env.graph, env.signatures) {
                        env.exprs.record(
                            *span,
                            ExprInfo::Index {
                                elem_ty,
                                guarded: false,
                            },
                        );
                        crate::expr::note_float_widening_at(*span, elem_ty, declared, env);
                    } else {
                        report_mismatch(*span, declared, elem_ty, env);
                    }
                }
                let name_str = strip_sigil(span_text(env.src, *name)).to_owned();
                declare_binding(scope, &name_str, declared, *name, false, env);
                live.insert(name_str);
            }
            DestructureElement::Nested { key, target, span } => {
                check_destructure_key(key.as_ref(), live, scope, ctx, env);
                walk_destructure_target(target, elem_ty, *span, live, scope, ctx, env);
            }
            _ => {}
        }
    }
}

/// A destructuring element's explicit `key =>`, checked as the array key it
/// is — the same [`code::E_ARRAY_KEY_INVALID_TYPE`] question a subscript and an
/// array literal's explicit key both already answer, asked here because
/// `nvs_ir::lower::Lowering::lower_array_key` trusts all three call sites
/// alike and panics on a key it cannot render.
fn check_destructure_key(
    key: Option<&Expr>,
    live: &mut FxHashSet<String>,
    scope: &mut LocalScope,
    ctx: &Ctx<'_>,
    env: &mut Env<'_>,
) {
    let Some(key) = key else {
        return;
    };
    let key_ty = check_expr(key, None, live, scope, ctx, env);
    check_array_key_type(key_ty, key.span, env);
}

/// `[inout int $x] = $pair;` — refused, with [`code::E_ARRAY_ELEMENT_BY_REFERENCE`]'s
/// own rule and its own code, because it is that rule's other side: PHP's
/// leaf aliases the element it came from, and `rule:types/implicit-capture` leaves no binding
/// that aliases another for it to be.
fn report_inout_leaf(span: Span, env: &mut Env<'_>) {
    env.diags.report(
        Diagnostic::error(
            code::E_ARRAY_ELEMENT_BY_REFERENCE,
            "a destructuring leaf cannot bind by reference",
        )
        .with_primary(span, "this would alias the element it was read from")
        .with_help(
            "drop the `inout` — the leaf is a copy, exactly as an array literal's element is; \
             to share one mutable cell, put it in an object (`rule:types/implicit-capture`)",
        ),
    );
}
