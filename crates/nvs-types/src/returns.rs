//! "Does every path through this body leave the frame?" — `rule:types/declaration`'s
//! *nothing is untyped*, applied to the two exits a body can take without
//! writing anything at all.
//!
//! A body checked against a return type other than `void` promises a value of
//! that type at every exit. The path that reaches the closing brace writes
//! none, and there is nothing for it to hand back: `nvs_ir::lower::lower_method`
//! seals a body's fall-through exit with `Terminator::Return(None)`, so the
//! caller of an `int` method reads a slot the callee never wrote. PHP answers
//! `null` there; `rule:types/conversion` has no implicit conversion for that to be, and
//! inventing one would change a binding's type behind its declaration — so the
//! path is refused where it is written (`E0739`). A written `return;` reaches
//! the same terminator by the same promise and is refused as `E0822`, which
//! asks nothing of the walk below: the declared type is the whole of whether
//! one is legal. `crate::check`'s `check_body_exits` reads both, over every
//! block body a declared type is checked against —
//! `rule:statements/a-body-never-falls-off-its-end` is the pair's home.
//!
//! **The analysis is asymmetric on purpose, and the opposite way round from
//! [`crate::locals`]'s `terminates`.** That one answers "may I drop this branch's
//! bindings at a join", where a wrong `true` loses information and a wrong
//! `false` only costs precision. This one decides whether to *report*, so every
//! shape it cannot prove reaches the end is treated as exiting. The bar for a
//! diagnostic is therefore "I walked this body and found a path out of its
//! bottom", never "I did not find a `return`":
//!
//! - a `while (true)` / `for (;;)` with no `break` reaching it never falls out,
//!   whatever its body does, and is the idiomatic spelling of a server loop;
//! - a `switch` exits only with a `default` arm, no `break` of its own, and a
//!   last arm that exits — every earlier arm may fall through into it;
//! - a `try` exits through a `finally` that exits, or through a body *and* every
//!   `catch` exiting — once `rule:statements/no-return-leaves-a-finally` lands (goal `surface`) a
//!   `finally` can leave only by throwing, and this arm and `escapes_block`'s narrow to that;
//! - a `foreach`, and a `while` over anything but a literal `true`, may run zero
//!   times, so neither can be the reason a body exits.
//!
//! A generator is not checked at all: `rule:iteration/one-way-only` leaves a generator's body no
//! return value to produce, and `crate::check` already checks that body against
//! `void` — which is also what makes `return;` its one legal stop.

use nvs_diagnostics::Span;
use nvs_syntax::ast::{Block, Expr, ExprKind, Stmt, StmtKind};

use crate::expr_table::ExprTypeTable;

/// Whether every path through `stmts` leaves the frame before reaching the end
/// of the list. `exprs` is read for the one fact the syntax does not carry:
/// which expression statements the checker typed `never`.
pub(crate) fn block_always_exits(stmts: &[Stmt], exprs: &ExprTypeTable) -> bool {
    stmts.iter().any(|s| always_exits(s, exprs))
}

/// Every `return <expr>;` written in `stmts`, in source order.
pub(crate) fn for_each_return<F: FnMut(&Expr)>(stmts: &[Stmt], f: &mut F) {
    for_each_return_stmt(stmts, &mut |operand, _| {
        if let Some(e) = operand {
            f(e);
        }
    });
}

/// Every `return;` written in `stmts` that carries no value, as its own span.
pub(crate) fn for_each_valueless_return<F: FnMut(Span)>(stmts: &[Stmt], f: &mut F) {
    for_each_return_stmt(stmts, &mut |operand, span| {
        if operand.is_none() {
            f(span);
        }
    });
}

/// Every `return` written in `stmts`, in source order: its operand where it has
/// one, and the statement's own span either way.
///
/// An anonymous function's body is *not* descended into: `fn` is an expression, so
/// its returns belong to its own frame and are checked when
/// `crate::expr::calls::check_anon_fn` checks that body. Nothing else here
/// walks expressions at all, which is what makes that free rather than a case
/// to remember.
fn for_each_return_stmt<F: FnMut(Option<&Expr>, Span)>(stmts: &[Stmt], f: &mut F) {
    for stmt in stmts {
        visit_return(stmt, f);
    }
}

fn visit_return<F: FnMut(Option<&Expr>, Span)>(stmt: &Stmt, f: &mut F) {
    match &stmt.kind {
        StmtKind::Return(operand) => f(operand.as_ref(), stmt.span),
        StmtKind::Block(b) => for_each_return_stmt(&b.stmts, f),
        StmtKind::If { arms, else_ } => {
            for arm in arms {
                visit_return(&arm.then, f);
            }
            if let Some(else_) = else_ {
                visit_return(else_, f);
            }
        }
        StmtKind::While { body, .. }
        | StmtKind::DoWhile { body, .. }
        | StmtKind::For { body, .. }
        | StmtKind::Foreach { body, .. } => visit_return(body, f),
        StmtKind::Switch { cases, .. } => {
            for case in cases {
                for_each_return_stmt(&case.body, f);
            }
        }
        StmtKind::Try {
            body,
            catches,
            finally,
        } => {
            for_each_return_stmt(&body.stmts, f);
            for catch in catches {
                for_each_return_stmt(&catch.body.stmts, f);
            }
            if let Some(finally) = finally {
                for_each_return_stmt(&finally.stmts, f);
            }
        }
        _ => {}
    }
}

/// Whether every path through `stmt` leaves the frame — returns, throws, exits
/// the process, or loops forever.
fn always_exits(stmt: &Stmt, exprs: &ExprTypeTable) -> bool {
    match &stmt.kind {
        StmtKind::Return(_) => true,
        StmtKind::Expr(e) => expr_always_exits(e, exprs),
        StmtKind::Block(b) => block_always_exits(&b.stmts, exprs),
        StmtKind::If {
            arms,
            else_: Some(else_),
        } => arms.iter().all(|arm| always_exits(&arm.then, exprs)) && always_exits(else_, exprs),
        // A loop is the reason a body exits only when nothing can leave it: an
        // `if` inside a `while (true)` that returns is *not* what makes the
        // enclosing body exit, because the `if` may be false forever.
        StmtKind::While { cond, body } => is_literal_true(cond) && !escapes(body),
        StmtKind::For { cond, body, .. } => {
            cond.last().is_none_or(is_literal_true) && !escapes(body)
        }
        // The one loop whose body is guaranteed to run: it exits either because
        // that first iteration always does, or because there is no way out.
        StmtKind::DoWhile { body, cond } => {
            always_exits(body, exprs) || (is_literal_true(cond) && !escapes(body))
        }
        StmtKind::Switch { cases, .. } => {
            cases.iter().any(|c| c.cond.is_none())
                && !cases.iter().any(|c| escapes_block(&c.body))
                && cases
                    .last()
                    .is_some_and(|c| block_always_exits(&c.body, exprs))
        }
        StmtKind::Try {
            body,
            catches,
            finally,
        } => {
            finally
                .as_ref()
                .is_some_and(|f| block_always_exits(&f.stmts, exprs))
                || (block_always_exits(&body.stmts, exprs)
                    && catches
                        .iter()
                        .all(|c| block_always_exits(&c.body.stmts, exprs)))
        }
        _ => false,
    }
}

/// The expression statements that do not come back: `throw`, `exit`, and any
/// statement the checker typed `never` — a call to a `never` method no
/// subtype overrides, or to a `never` function. That last one is sound only
/// because a `never` body that reaches its own end is itself refused
/// (`crate::check::check_body_exits`); `crate::expr::check_expr_stmt` owns
/// why an overridden one is left out.
fn expr_always_exits(e: &Expr, exprs: &ExprTypeTable) -> bool {
    matches!(e.kind, ExprKind::Throw(_) | ExprKind::Exit(_)) || exprs.is_never_stmt(e.span)
}

fn is_literal_true(e: &Expr) -> bool {
    matches!(e.kind, ExprKind::Bool(true))
}

/// Whether a `break` or `continue` written in `stmt` can reach the loop or
/// `switch` that encloses it here — i.e. whether control can leave that
/// construct without leaving the frame.
///
/// Nested loops and `switch`es consume one level, so their own unlabelled
/// `break` is theirs and not ours. A `break 2;` is counted against this level
/// whatever its written level: reading the literal back would take a span
/// lookup, and over-counting only ever makes a construct look escapable, which
/// costs an accepted body rather than a wrong refusal.
fn escapes(stmt: &Stmt) -> bool {
    match &stmt.kind {
        StmtKind::Break(_) => true,
        // A `continue` re-tests the condition, and a `while (true)`'s is still
        // true — so it is not a way out. It is only listed here to say that
        // out loud; the `_ => false` arm below would answer the same.
        StmtKind::Continue(_) => false,
        StmtKind::Block(b) => escapes_block(&b.stmts),
        StmtKind::If { arms, else_ } => {
            arms.iter().any(|arm| escapes(&arm.then)) || else_.as_ref().is_some_and(|e| escapes(e))
        }
        StmtKind::Try {
            body,
            catches,
            finally,
        } => {
            escapes_block(&body.stmts)
                || catches.iter().any(|c| escapes_block(&c.body.stmts))
                || finally.as_ref().is_some_and(|f| escapes_block(&f.stmts))
        }
        // A nested loop or `switch` is what an unlabelled `break` inside it
        // targets, so nothing under one escapes *this* level through it.
        StmtKind::While { .. }
        | StmtKind::DoWhile { .. }
        | StmtKind::For { .. }
        | StmtKind::Foreach { .. }
        | StmtKind::Switch { .. } => false,
        _ => false,
    }
}

fn escapes_block(stmts: &[Stmt]) -> bool {
    stmts.iter().any(escapes)
}

/// The span to point a missing `return` at: the body's closing brace, which is
/// the character the path in question reaches.
pub(crate) fn closing_brace(body: &Block) -> nvs_diagnostics::Span {
    let end = body.span.end;
    nvs_diagnostics::Span::new(body.span.file, end.saturating_sub(1), end)
}
