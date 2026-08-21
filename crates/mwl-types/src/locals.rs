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
//! **Known gaps**, beyond the ones `crate` docs already name: `switch` and
//! `try`/`catch` bodies conservatively contribute nothing to definite
//! assignment after them (documented at each site below) — safe, since it
//! can only reject a valid program, never accept an invalid one; a nested
//! class/interface/trait/enum declaration inside a function body is not
//! descended into at all (its own methods go unchecked, same as a closure's
//! body — see `crate::expr`'s docs for the latter).

use mwl_diagnostics::{Diagnostic, Span, code};
use mwl_syntax::ast::{DestructureElement, DestructureTarget, Expr, ExprKind, Stmt, StmtKind};
use rustc_hash::{FxHashMap, FxHashSet};

use crate::expr::{check_expr, check_return, check_unset_target, require_stringable};
use crate::lower::{lower_optional_type, lower_type};
use crate::ty::TypeId;
use crate::{Ctx, Env, span_text, strip_sigil};

/// One local variable's declared type and where it was declared.
#[derive(Debug)]
pub(crate) struct LocalInfo {
    pub ty: TypeId,
    pub declared_span: Span,
}

/// One function/method/closure body's local variables — a single table for
/// the whole body, since declaration is function-scoped (ADR 0007 § 1), not
/// block-scoped. A closure gets a fresh, empty one of its own: ADR 0031's
/// capture is by value, never a shared binding.
#[derive(Debug, Default)]
pub(crate) struct LocalScope {
    pub(crate) by_name: FxHashMap<String, LocalInfo>,
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

/// Walks `stmts` in sequence, checking each one and updating `live`/`scope`
/// in place.
pub(crate) fn check_block(
    stmts: &[Stmt],
    live: &mut FxHashSet<String>,
    scope: &mut LocalScope,
    return_ty: TypeId,
    ctx: &Ctx<'_>,
    env: &mut Env<'_>,
) {
    for stmt in stmts {
        check_stmt(stmt, live, scope, return_ty, ctx, env);
    }
}

#[expect(
    clippy::too_many_lines,
    reason = "one match arm per AST statement variant, each a couple of lines"
)]
fn check_stmt(
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
            check_stmt(then, &mut then_live, scope, return_ty, ctx, env);
            if let Some(else_stmt) = else_ {
                let mut else_live = live.clone();
                check_stmt(else_stmt, &mut else_live, scope, return_ty, ctx, env);
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
            check_stmt(body, &mut body_live, scope, return_ty, ctx, env);
        }
        StmtKind::DoWhile { body, cond } => {
            // The body runs at least once, so its assignments carry forward.
            check_stmt(body, live, scope, return_ty, ctx, env);
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
            check_stmt(body, &mut body_live, scope, return_ty, ctx, env);
            for e in step {
                check_expr(e, None, &mut body_live, scope, ctx, env);
            }
        }
        StmtKind::Foreach {
            subject,
            key,
            value,
            body,
            ..
        } => {
            check_expr(subject, None, live, scope, ctx, env);
            let mut body_live = live.clone();
            if let Some(k) = key {
                let ty = lower_optional_type(k.ty.as_ref(), ctx, env);
                let name = strip_sigil(span_text(env.src, k.name)).to_owned();
                declare_binding(scope, &name, ty, k.name, false, env);
                body_live.insert(name);
            }
            let value_ty = lower_optional_type(value.ty.as_ref(), ctx, env);
            let value_name = strip_sigil(span_text(env.src, value.name)).to_owned();
            declare_binding(scope, &value_name, value_ty, value.name, false, env);
            body_live.insert(value_name);
            check_stmt(body, &mut body_live, scope, return_ty, ctx, env);
        }
        StmtKind::Switch { subject, cases } => {
            check_expr(subject, None, live, scope, ctx, env);
            // Conservatively contributes nothing to definite assignment
            // after the switch — see the module docs' known gaps.
            for case in cases {
                let mut case_live = live.clone();
                if let Some(c) = &case.cond {
                    check_expr(c, None, &mut case_live, scope, ctx, env);
                }
                check_block(&case.body, &mut case_live, scope, return_ty, ctx, env);
            }
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
            let mut body_live = live.clone();
            check_block(&body.stmts, &mut body_live, scope, return_ty, ctx, env);
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
            }
            // `finally` always runs, so its assignments carry forward.
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
                *live = finally_live;
            }
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
            let declared_ty = lower_type(ty, ctx, env);
            let name_str = strip_sigil(span_text(env.src, *name)).to_owned();
            declare_binding(scope, &name_str, declared_ty, *name, true, env);
            if let Some(value) = value {
                check_expr(value, Some(declared_ty), live, scope, ctx, env);
                live.insert(name_str);
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
        | StmtKind::TraitDecl(_)
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
