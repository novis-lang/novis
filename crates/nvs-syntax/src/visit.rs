//! The expressions an expression evaluates: one match over
//! [`crate::ast::ExprKind`]'s productions, yielding the [`crate::ast::Expr`]
//! nodes themselves.
//!
//! # Decision: the match lives beside the grammar, for the reason [`crate::walk`] gives
//!
//! [`crate::ast::ExprKind`], [`crate::ast::MemberName`] and
//! [`crate::ast::NewTarget`] are `#[non_exhaustive]`, so a `match` written in
//! any other crate needs a wildcard arm and a production added later walks as
//! a leaf there — a checking pass looking for an assignment or a read would
//! stop finding one inside the new form, and nothing would fail to build.
//! Written here the compiler checks it, and a new variant is an error in the
//! file its author is already in.
//!
//! This is not [`crate::walk::of_stmts`] under another name. That walk answers
//! `rule:core-classes/ast-is-inert`'s question — what a *running program* may
//! see of a parse — and deliberately owns nothing of the source, so what
//! crosses its boundary is a [`crate::walk::Node`] and never an
//! [`crate::ast::Expr`]. A compiler pass needs the node: it reads the receiver
//! of a `->`, the class of a `::`, the operator of a `Binary`. Both are the
//! grammar matched once for one consumer's question.
//!
//! # Decision: a child is what this expression evaluates, not what it contains
//!
//! A closure's body and an anonymous class's members are written inside an
//! expression and run when they are *called*, which is not where they are
//! written and need not be ever. So [`each_child_expr`] stops at
//! [`crate::ast::ExprKind::Fn`] and at
//! [`crate::ast::NewTarget::AnonClass`], and a pass asking what an expression
//! does reads neither as this expression's doing. A parameter's default is the
//! same case and stops there with the body holding it. What a `new` *does*
//! evaluate — the class expression of a `new $cls()`, and every argument — is
//! yielded like any other child.
//!
//! **Short-circuiting and branching are not filtered out here.** The right
//! operand of `&&`, `||` and `??`, both arms of a ternary, and every arm of a
//! `match` or an expression-level `catch` are children like any other, because
//! this walk describes shape and a caller's question decides what an unrun
//! operand is worth: a pass collecting reads wants them all, and one proving
//! definite assignment joins them ([`crate::ast::BinaryOp::short_circuits`] is
//! what that second kind asks). Filtering here would make the walk answer one
//! of those two questions and quietly mislead the other.
//!
//! **What it spends:** nothing. The callback is handed borrowed nodes in
//! source order and the walk allocates nothing at all.

use crate::ast::{CallArgs, Expr, ExprKind, MemberName, NewTarget, StringPart, TestOperand};

/// Calls `f` on every expression `e` evaluates as part of evaluating itself,
/// in source order — one level deep, so a caller that wants the whole subtree
/// recurses through `f`.
///
/// See the module docs for what a child is: a closure's body and an anonymous
/// class's members are not children of the expression that writes them, and a
/// branch that may not run is.
pub fn each_child_expr(e: &Expr, f: &mut dyn FnMut(&Expr)) {
    match &e.kind {
        ExprKind::Null
        | ExprKind::Bool(_)
        | ExprKind::Int(_)
        | ExprKind::Float(_)
        | ExprKind::Duration(_)
        | ExprKind::Str(_)
        | ExprKind::Variable(_)
        | ExprKind::ConstFetch(_)
        | ExprKind::SelfExpr
        | ExprKind::StaticExpr
        | ExprKind::ParentExpr
        | ExprKind::Error(_) => {}
        // A body of its own, run when it is called: see the module docs.
        ExprKind::Fn(_) => {}
        ExprKind::Interpolated(parts) | ExprKind::Markup(parts) => {
            for part in parts {
                if let StringPart::Expr(hole) = part {
                    f(hole);
                }
            }
        }
        ExprKind::ArrayLiteral(items) => {
            for item in items {
                if let Some(key) = &item.key {
                    f(key);
                }
                f(&item.value);
            }
        }
        ExprKind::Unary { expr, .. }
        | ExprKind::PreIncDec { expr, .. }
        | ExprKind::PostIncDec { expr, .. }
        | ExprKind::Conversion { expr, .. } => f(expr),
        ExprKind::TypeTest { expr, against } => {
            f(expr);
            if let TestOperand::Value(operand) = against {
                f(operand);
            }
        }
        ExprKind::Clone(operand)
        | ExprKind::YieldFrom(operand)
        | ExprKind::Print(operand)
        | ExprKind::Throw(operand)
        | ExprKind::Empty(operand)
        | ExprKind::Await(operand)
        | ExprKind::Paren(operand) => f(operand),
        ExprKind::Binary { lhs, rhs, .. } => {
            f(lhs);
            f(rhs);
        }
        ExprKind::Assign { target, value, .. } => {
            f(target);
            f(value);
        }
        ExprKind::Ternary { cond, then, else_ } => {
            f(cond);
            if let Some(then) = then {
                f(then);
            }
            f(else_);
        }
        ExprKind::Call { callee, args } => {
            f(callee);
            each_arg(args, f);
        }
        ExprKind::MethodCall {
            object,
            method,
            args,
            ..
        } => {
            f(object);
            each_member_name_expr(method, f);
            each_arg(args, f);
        }
        ExprKind::StaticCall {
            class,
            method,
            args,
            ..
        } => {
            f(class);
            each_member_name_expr(method, f);
            each_arg(args, f);
        }
        ExprKind::PropertyAccess {
            object, property, ..
        } => {
            f(object);
            each_member_name_expr(property, f);
        }
        ExprKind::StaticPropertyAccess { class, .. }
        | ExprKind::ClassConstAccess { class, .. }
        | ExprKind::ClassNameConst { class } => f(class),
        ExprKind::Index { base, index } => {
            f(base);
            if let Some(index) = index {
                f(index);
            }
        }
        ExprKind::New { target, args, .. } => {
            match target {
                NewTarget::Expr(class) => f(class),
                // The members of an anonymous class are bodies of their own,
                // exactly as a closure's is.
                NewTarget::Name(_)
                | NewTarget::SelfTy
                | NewTarget::StaticTy
                | NewTarget::ParentTy
                | NewTarget::AnonClass(_) => {}
            }
            each_arg(args, f);
        }
        ExprKind::Match { subject, arms } => {
            f(subject);
            for arm in arms {
                for cond in arm.conditions.iter().flatten() {
                    f(cond);
                }
                f(&arm.body);
            }
        }
        ExprKind::Catch { guarded, arms } => {
            f(guarded);
            for arm in arms {
                f(&arm.body);
            }
        }
        ExprKind::Yield { key, value } => {
            if let Some(key) = key {
                f(key);
            }
            if let Some(value) = value {
                f(value);
            }
        }
        ExprKind::Isset(targets) => {
            for target in targets {
                f(target);
            }
        }
        ExprKind::Exit(status) => {
            if let Some(status) = status {
                f(status);
            }
        }
        ExprKind::SpawnScript { path, options } => {
            f(path);
            for option in options {
                f(&option.value);
            }
        }
        ExprKind::Require { path } => f(path),
        ExprKind::ObjectLiteral(fields) => {
            for field in fields {
                f(&field.value);
            }
        }
    }
}

/// The expressions an argument list evaluates. `(...)` — the first-class
/// callable marker — names a callable rather than calling one, so it
/// evaluates nothing.
fn each_arg(args: &CallArgs, f: &mut dyn FnMut(&Expr)) {
    if let CallArgs::List(list) = args {
        for arg in list {
            f(&arg.value);
        }
    }
}

/// The expression a dynamic member name evaluates — `->$name` and `->{expr}`
/// compute the name at run time, and a written or invented identifier
/// computes nothing.
fn each_member_name_expr(name: &MemberName, f: &mut dyn FnMut(&Expr)) {
    match name {
        MemberName::Ident(_) | MemberName::Missing(_) => {}
        MemberName::Variable(computed) | MemberName::Expr(computed) => f(computed),
    }
}

#[cfg(test)]
mod tests {
    use nvs_diagnostics::{Diagnostics, SourceMap};

    use super::each_child_expr;
    use crate::ast::{Expr, ExprKind, StmtKind};
    use crate::parse_file;

    /// The one expression of a one-statement file.
    fn expr_of(src: &str) -> Expr {
        let mut map = SourceMap::new();
        let file = map.add("t.nvs", src);
        let mut diags = Diagnostics::new();
        let stmts = parse_file(map.file(file), &mut diags);
        assert!(!diags.has_errors(), "fixture failed to parse: {diags:?}");
        let [stmt] = &stmts[..] else {
            panic!("expected one statement, got {}", stmts.len())
        };
        match &stmt.kind {
            StmtKind::Expr(e) => e.clone(),
            other => panic!("expected an expression statement, got {other:?}"),
        }
    }

    /// The kind name of each child, through [`crate::walk`]'s own spelling of
    /// a production, so a failure names the form rather than a span.
    fn child_kinds(src: &str) -> Vec<&'static str> {
        let e = expr_of(src);
        let mut kinds = Vec::new();
        each_child_expr(&e, &mut |child| {
            kinds.push(match &child.kind {
                ExprKind::Assign { .. } => "Assign",
                ExprKind::Variable(_) => "Variable",
                ExprKind::Int(_) => "Int",
                ExprKind::Fn(_) => "Fn",
                ExprKind::PropertyAccess { .. } => "PropertyAccess",
                _ => "other",
            });
        });
        kinds
    }

    /// Every arm of a `match` is a child: the pass proving definite
    /// assignment needs to see what each one assigns, and joining them is its
    /// question rather than this walk's.
    #[test]
    fn every_match_arm_body_is_a_child() {
        assert_eq!(
            child_kinds("<?nvs\nmatch ($x) { 1 => $this->a = 1, default => $this->b = 2 };\n"),
            ["Variable", "Int", "Assign", "Assign"]
        );
    }

    /// A closure's body runs when the closure is called, so the assignment
    /// inside this one is not a child of the expression that writes it — only
    /// the closure itself is.
    #[test]
    fn a_closure_body_is_not_walked_into() {
        assert_eq!(
            child_kinds("<?nvs\n$f = fn () => $this->a = 1;\n"),
            ["Variable", "Fn"]
        );
    }

    /// A parenthesized expression carries no meaning beyond its span, and a
    /// walk that stopped at one would lose whatever was written inside it.
    #[test]
    fn parentheses_yield_what_they_wrap() {
        assert_eq!(child_kinds("<?nvs\n($this->a = 1);\n"), ["Assign"]);
    }
}
