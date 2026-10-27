//! Every type a file writes, each as the whole type written at its position.
//!
//! Two consumers read it: the editor, which asks what a cursor or a copied
//! selection holds, and `nvs fmt`, which spaces the punctuation inside a
//! written type as type punctuation rather than as an operator
//! (`rule:tooling/fmt-one-run-between-two-tokens`). It lives beside the grammar
//! for the reason [`crate::visit`] gives.
//!
//! The positions walked are a parameter's, a local's, a property's, a class
//! constant's, a `catch`'s, a return's, an alias's, a `foreach` binding's, an
//! `as` conversion's, an `is` test's, an anonymous function's signature, a
//! `catch` arm's and a call's `<Type>` argument. A top-level `function` or
//! `const` and a `static` local are deliberately absent for
//! `rule:ide/rejected-syntax-gets-no-colour`'s reason: the construct is
//! refused, so nothing is answered about the names written inside it.
//!
//! **What it spends:** one vector of borrowed types per call.

use nvs_diagnostics::{BytePos, Span};

use crate::ast::{
    ClassMember, ClassMemberKind, Expr, ExprKind, FnBody, ForInit, MethodMember, NewTarget, Param,
    PropertyHook, PropertyHookBody, Stmt, StmtKind, TestOperand, Type,
};
use crate::visit::each_child_expr;

/// Every type written in `stmts` whose span meets `[start, end]`, in source
/// order, each as the whole type written at that position.
///
/// The span test on each statement and each expression is what makes this
/// one path down the file rather than a walk of all of it: a node the range
/// does not meet holds no type the range could. Asked with `start == end` it
/// is the path to the one type under a cursor; asked over
/// `0..=BytePos::MAX` it is every type the file writes.
pub fn written_types_in(stmts: &[Stmt], start: BytePos, end: BytePos) -> Vec<&Type> {
    let mut found = Vec::new();
    stmts_types(stmts, start, end, &mut found);
    found
}

/// Whether `span` and `[start, end]` share at least one offset, both ends
/// included.
const fn meets(span: Span, start: BytePos, end: BytePos) -> bool {
    span.start <= end && start <= span.end
}

/// [`written_types_in`] over one statement sequence, appending to `found`.
fn stmts_types<'a>(stmts: &'a [Stmt], start: BytePos, end: BytePos, found: &mut Vec<&'a Type>) {
    for stmt in stmts {
        stmt_types(stmt, start, end, found);
    }
}

/// The annotations written in one statement that meet the range, and nothing
/// for a statement the range does not meet.
fn stmt_types<'a>(stmt: &'a Stmt, start: BytePos, end: BytePos, found: &mut Vec<&'a Type>) {
    if !meets(stmt.span, start, end) {
        return;
    }
    match &stmt.kind {
        StmtKind::TypeAliasDecl(decl) => root(&decl.ty, start, end, found),
        StmtKind::ClassDecl(decl) => members_types(&decl.members, start, end, found),
        StmtKind::InterfaceDecl(decl) => members_types(&decl.members, start, end, found),
        StmtKind::EnumDecl(decl) => members_types(&decl.members, start, end, found),
        StmtKind::NamespaceDecl(decl) => {
            if let Some(block) = &decl.body {
                stmts_types(&block.stmts, start, end, found);
            }
        }
        StmtKind::LocalDecl { ty, value, .. } => {
            if let Some(ty) = ty {
                root(ty, start, end, found);
            }
            if let Some(value) = value {
                expr_types(value, start, end, found);
            }
        }
        StmtKind::Expr(expr) | StmtKind::Destructure { value: expr, .. } => {
            expr_types(expr, start, end, found);
        }
        StmtKind::Return(value) | StmtKind::Break(value) | StmtKind::Continue(value) => {
            if let Some(expr) = value {
                expr_types(expr, start, end, found);
            }
        }
        StmtKind::Echo(exprs) | StmtKind::Unset(exprs) => exprs_types(exprs, start, end, found),
        StmtKind::Block(block) => stmts_types(&block.stmts, start, end, found),
        StmtKind::If { arms, else_ } => {
            for arm in arms {
                expr_types(&arm.cond, start, end, found);
                stmt_types(&arm.then, start, end, found);
            }
            if let Some(else_) = else_ {
                stmt_types(else_, start, end, found);
            }
        }
        StmtKind::While { cond, body } | StmtKind::DoWhile { cond, body } => {
            expr_types(cond, start, end, found);
            stmt_types(body, start, end, found);
        }
        StmtKind::For {
            init,
            cond,
            step,
            body,
        } => {
            match init {
                ForInit::Decl(decl) => stmt_types(decl, start, end, found),
                ForInit::Exprs(exprs) => exprs_types(exprs, start, end, found),
            }
            exprs_types(cond, start, end, found);
            exprs_types(step, start, end, found);
            stmt_types(body, start, end, found);
        }
        StmtKind::Foreach {
            subject,
            key,
            value,
            body,
            ..
        } => {
            expr_types(subject, start, end, found);
            if let Some(ty) = key.as_ref().and_then(|binding| binding.written_ty()) {
                root(ty, start, end, found);
            }
            if let Some(ty) = value.written_ty() {
                root(ty, start, end, found);
            }
            stmt_types(body, start, end, found);
        }
        StmtKind::Switch { subject, cases } => {
            expr_types(subject, start, end, found);
            for arm in cases {
                if let Some(cond) = &arm.cond {
                    expr_types(cond, start, end, found);
                }
                stmts_types(&arm.body, start, end, found);
            }
        }
        StmtKind::Try {
            body,
            catches,
            finally,
        } => {
            stmts_types(&body.stmts, start, end, found);
            for catch in catches {
                root(&catch.ty, start, end, found);
                stmts_types(&catch.body.stmts, start, end, found);
            }
            if let Some(finally) = finally {
                stmts_types(&finally.stmts, start, end, found);
            }
        }
        _ => {}
    }
}

/// [`expr_types`] over one expression list, appending to `found`.
fn exprs_types<'a>(exprs: &'a [Expr], start: BytePos, end: BytePos, found: &mut Vec<&'a Type>) {
    for expr in exprs {
        expr_types(expr, start, end, found);
    }
}

/// The types written in one expression that meet the range, and nothing for
/// an expression the range does not meet.
///
/// The five places an expression writes a type — an `as` conversion's target,
/// an `is` test's, an anonymous function's signature, a `catch` arm's class and
/// the `<Type>` arguments of a call or a `new` — and then every expression
/// this one evaluates ([`each_child_expr`]). That walk stops at an anonymous
/// function's body and at an anonymous class's members on purpose, so both
/// are stepped into here: the body is a statement sequence or an expression,
/// and the members are a class body like any other.
fn expr_types<'a>(expr: &'a Expr, start: BytePos, end: BytePos, found: &mut Vec<&'a Type>) {
    if !meets(expr.span, start, end) {
        return;
    }
    match &expr.kind {
        ExprKind::Conversion { ty, .. } => root(ty, start, end, found),
        ExprKind::TypeTest {
            against: TestOperand::Type(ty),
            ..
        } => root(ty, start, end, found),
        ExprKind::MethodCall { type_args, .. } | ExprKind::StaticCall { type_args, .. } => {
            for ty in type_args {
                root(ty, start, end, found);
            }
        }
        ExprKind::New {
            target, type_args, ..
        } => {
            for ty in type_args {
                root(ty, start, end, found);
            }
            if let NewTarget::AnonClass(decl) = target {
                members_types(&decl.members, start, end, found);
            }
        }
        ExprKind::Fn(function) => {
            params_types(&function.params, start, end, found);
            if let Some(ty) = &function.return_type {
                root(ty, start, end, found);
            }
            match &function.body {
                FnBody::Expr(body) => expr_types(body, start, end, found),
                FnBody::Block(block) => stmts_types(&block.stmts, start, end, found),
            }
        }
        ExprKind::Catch { arms, .. } => {
            for arm in arms {
                root(&arm.ty, start, end, found);
            }
        }
        _ => {}
    }
    each_child_expr(expr, &mut |child| expr_types(child, start, end, found));
}

/// One annotation, kept where the range meets it.
fn root<'a>(ty: &'a Type, start: BytePos, end: BytePos, found: &mut Vec<&'a Type>) {
    if meets(ty.span, start, end) {
        found.push(ty);
    }
}

/// The annotations among one body's members that meet the range.
fn members_types<'a>(
    members: &'a [ClassMember],
    start: BytePos,
    end: BytePos,
    found: &mut Vec<&'a Type>,
) {
    for member in members {
        match &member.kind {
            ClassMemberKind::Property(property) => {
                root(&property.ty, start, end, found);
                if let Some(hooks) = property.hooks.as_deref() {
                    hooks_types(hooks, start, end, found);
                }
            }
            ClassMemberKind::Const(constant) => {
                if let Some(ty) = &constant.ty {
                    root(ty, start, end, found);
                }
            }
            ClassMemberKind::TypeAlias(alias) => root(&alias.ty, start, end, found),
            ClassMemberKind::Method(method) => method_types(method, start, end, found),
            _ => {}
        }
    }
}

/// The annotations in one method's signature and its body that meet the range.
fn method_types<'a>(
    method: &'a MethodMember,
    start: BytePos,
    end: BytePos,
    found: &mut Vec<&'a Type>,
) {
    params_types(&method.params, start, end, found);
    if let Some(ty) = &method.return_type {
        root(ty, start, end, found);
    }
    if let Some(body) = &method.body {
        stmts_types(&body.stmts, start, end, found);
    }
}

/// The annotations in one parameter list that meet the range.
fn params_types<'a>(params: &'a [Param], start: BytePos, end: BytePos, found: &mut Vec<&'a Type>) {
    for param in params {
        if let Some(ty) = &param.ty {
            root(ty, start, end, found);
        }
    }
}

/// The annotations in one property's hook block that meet the range.
///
/// A hook is a method in the two ways that matter here: `set(Order::Meta $m)`
/// writes a parameter type, and a block-bodied hook writes statements. The
/// short `=> expr;` form writes an expression and so writes no type this walk
/// reaches.
fn hooks_types<'a>(
    hooks: &'a [PropertyHook],
    start: BytePos,
    end: BytePos,
    found: &mut Vec<&'a Type>,
) {
    for hook in hooks {
        if let Some(ty) = hook.param.as_ref().and_then(|param| param.ty.as_ref()) {
            root(ty, start, end, found);
        }
        if let Some(PropertyHookBody::Block(block)) = hook.body.as_ref() {
            stmts_types(&block.stmts, start, end, found);
        }
    }
}
