//! The statement and expression walk [`crate::hints`] and [`crate::semantic`]
//! share.
//!
//! [`Walk`]'s default methods reach every statement, member, parameter and
//! expression the two passes read, and nothing else: a string's holes, a class
//! or member written as a name, and the target of a `new` are left to the pass
//! that reads them. A pass overrides a method, does its own work on the node,
//! and then calls the matching free function here for the node's children.
//! Both passes sort what they collect, so the order of the walk is free.
//!
//! The `nvs_syntax::ast` enums are `#[non_exhaustive]`, so every match carries a
//! wildcard, and a production added later is walked by neither pass until it
//! is added here.

use nvs_syntax::ast::{
    Block, CallArgs, ClassMember, ClassMemberKind, EnumDecl, Expr, ExprKind, FnBody, FnExpr,
    ForInit, Param, PropertyHookBody, Stmt, StmtKind, TestOperand,
};

/// One pass over a parsed document. Each overridable method defaults to
/// walking the node's children.
pub(crate) trait Walk {
    /// Walks a run of statements.
    fn stmts(&mut self, stmts: &[Stmt]) {
        for stmt in stmts {
            self.stmt(stmt);
        }
    }

    /// Walks one statement.
    fn stmt(&mut self, stmt: &Stmt) {
        stmt_children(self, stmt);
    }

    /// Walks an enum's case values and members, at file scope or in a body.
    fn enum_decl(&mut self, decl: &EnumDecl) {
        for case in &decl.cases {
            self.opt_expr(case.value.as_ref());
        }
        self.members(&decl.members);
    }

    /// Walks one member of a class, interface or enum body.
    fn class_member(&mut self, member: &ClassMember) {
        member_children(self, member);
    }

    /// Walks one parameter's default, which is where a call can hide.
    fn param(&mut self, param: &Param) {
        self.opt_expr(param.default.as_ref());
    }

    /// Walks one expression.
    fn expr(&mut self, expr: &Expr) {
        expr_children(self, expr);
    }

    /// Walks a block's statements.
    fn block(&mut self, block: &Block) {
        self.stmts(&block.stmts);
    }

    /// Walks a class, interface or enum body.
    fn members(&mut self, members: &[ClassMember]) {
        for member in members {
            self.class_member(member);
        }
    }

    /// Walks a parameter list.
    fn params(&mut self, params: &[Param]) {
        for param in params {
            self.param(param);
        }
    }

    /// Walks a run of expressions.
    fn exprs(&mut self, exprs: &[Expr]) {
        for expr in exprs {
            self.expr(expr);
        }
    }

    /// Walks an expression a production may have omitted.
    fn opt_expr(&mut self, expr: Option<&Expr>) {
        if let Some(expr) = expr {
            self.expr(expr);
        }
    }

    /// Walks an anonymous function's parameters and body.
    fn anon_fn(&mut self, anon_fn: &FnExpr) {
        self.params(&anon_fn.params);
        match &anon_fn.body {
            FnBody::Expr(expr) => self.expr(expr),
            FnBody::Block(block) => self.block(block),
        }
    }
}

/// The statements and expressions inside `stmt`.
pub(crate) fn stmt_children<W: Walk + ?Sized>(walk: &mut W, stmt: &Stmt) {
    match &stmt.kind {
        StmtKind::Expr(expr) => walk.expr(expr),
        StmtKind::Return(value) => walk.opt_expr(value.as_ref()),
        StmtKind::Block(block) => walk.block(block),
        StmtKind::If { arms, else_ } => {
            for arm in arms {
                walk.expr(&arm.cond);
                walk.stmt(&arm.then);
            }
            if let Some(otherwise) = else_ {
                walk.stmt(otherwise);
            }
        }
        StmtKind::While { cond, body } | StmtKind::DoWhile { body, cond } => {
            walk.expr(cond);
            walk.stmt(body);
        }
        StmtKind::For {
            init,
            cond,
            step,
            body,
        } => {
            match init {
                ForInit::Decl(decl) => walk.stmt(decl),
                ForInit::Exprs(exprs) => walk.exprs(exprs),
            }
            walk.exprs(cond);
            walk.exprs(step);
            walk.stmt(body);
        }
        StmtKind::Foreach { subject, body, .. } => {
            walk.expr(subject);
            walk.stmt(body);
        }
        StmtKind::Switch { subject, cases } => {
            walk.expr(subject);
            for case in cases {
                walk.opt_expr(case.cond.as_ref());
                walk.stmts(&case.body);
            }
        }
        StmtKind::Break(level) | StmtKind::Continue(level) => walk.opt_expr(level.as_ref()),
        StmtKind::Try {
            body,
            catches,
            finally,
        } => {
            walk.block(body);
            for catch in catches {
                walk.block(&catch.body);
            }
            if let Some(finally) = finally {
                walk.block(finally);
            }
        }
        StmtKind::Echo(exprs) | StmtKind::Unset(exprs) => walk.exprs(exprs),
        StmtKind::LocalDecl { value, .. } => walk.opt_expr(value.as_ref()),
        StmtKind::Destructure { value, .. } => walk.expr(value),
        StmtKind::StaticLocal { vars, .. } => {
            for var in vars {
                walk.opt_expr(var.default.as_ref());
            }
        }
        StmtKind::ClassDecl(decl) => walk.members(&decl.members),
        StmtKind::InterfaceDecl(decl) => walk.members(&decl.members),
        StmtKind::EnumDecl(decl) => walk.enum_decl(decl),
        StmtKind::NamespaceDecl(decl) => {
            if let Some(body) = &decl.body {
                walk.block(body);
            }
        }
        // A free function or constant does not compile
        // (`rule:classes/no-free-functions-or-constants`). Its body is still
        // walked: a document that does not compile is the one an editor shows
        // most.
        StmtKind::TopLevelFunction(function) => {
            walk.params(&function.params);
            if let Some(body) = &function.body {
                walk.block(body);
            }
        }
        StmtKind::TopLevelConst(declared) => {
            for constant in declared {
                walk.expr(&constant.value);
            }
        }
        _ => {}
    }
}

/// The expressions and statements inside one member of a body.
pub(crate) fn member_children<W: Walk + ?Sized>(walk: &mut W, member: &ClassMember) {
    match &member.kind {
        ClassMemberKind::Property(property) => {
            walk.opt_expr(property.default.as_ref());
            for hook in property.hooks.iter().flatten() {
                if let Some(param) = &hook.param {
                    walk.param(param);
                }
                match &hook.body {
                    Some(PropertyHookBody::Expr(expr)) => walk.expr(expr),
                    Some(PropertyHookBody::Block(block)) => walk.block(block),
                    _ => {}
                }
            }
        }
        ClassMemberKind::Const(declared) => walk.expr(&declared.value),
        ClassMemberKind::Enum(decl) => walk.enum_decl(decl),
        ClassMemberKind::Method(method) => {
            walk.params(&method.params);
            if let Some(body) = &method.body {
                walk.block(body);
            }
        }
        _ => {}
    }
}

/// The expressions inside `expr`, leaving out a string's holes, a name in a
/// class or member position, and what a `new` was written against.
pub(crate) fn expr_children<W: Walk + ?Sized>(walk: &mut W, expr: &Expr) {
    match &expr.kind {
        ExprKind::ArrayLiteral(items) => {
            for item in items {
                walk.opt_expr(item.key.as_ref());
                walk.expr(&item.value);
            }
        }
        ExprKind::AnonObject(fields) => {
            for field in fields {
                walk.expr(&field.value);
            }
        }
        ExprKind::Unary { expr, .. }
        | ExprKind::PreIncDec { expr, .. }
        | ExprKind::PostIncDec { expr, .. }
        | ExprKind::Conversion { expr, .. }
        | ExprKind::PropertyAccess { object: expr, .. } => walk.expr(expr),
        ExprKind::TypeTest { expr, against } => {
            walk.expr(expr);
            if let TestOperand::Value(operand) = against {
                walk.expr(operand);
            }
        }
        ExprKind::Binary { lhs, rhs, .. }
        | ExprKind::Assign {
            target: lhs,
            value: rhs,
            ..
        } => {
            walk.expr(lhs);
            walk.expr(rhs);
        }
        ExprKind::Ternary { cond, then, else_ } => {
            walk.expr(cond);
            walk.opt_expr(then.as_deref());
            walk.expr(else_);
        }
        ExprKind::Call {
            callee: object,
            args,
        }
        | ExprKind::MethodCall { object, args, .. } => {
            walk.expr(object);
            arg_values(walk, args);
        }
        ExprKind::StaticCall { args, .. } | ExprKind::New { args, .. } => arg_values(walk, args),
        ExprKind::Index { base, index } => {
            walk.expr(base);
            walk.opt_expr(index.as_deref());
        }
        ExprKind::Match { subject, arms } => {
            walk.expr(subject);
            for arm in arms {
                if let Some(conditions) = &arm.conditions {
                    walk.exprs(conditions);
                }
                walk.expr(&arm.body);
            }
        }
        ExprKind::Catch { guarded, arms } => {
            walk.expr(guarded);
            for arm in arms {
                walk.expr(&arm.body);
            }
        }
        ExprKind::Yield { key, value } => {
            walk.opt_expr(key.as_deref());
            walk.opt_expr(value.as_deref());
        }
        ExprKind::Clone(inner)
        | ExprKind::YieldFrom(inner)
        | ExprKind::Print(inner)
        | ExprKind::Throw(inner)
        | ExprKind::Empty(inner)
        | ExprKind::Await(inner)
        | ExprKind::Paren(inner)
        | ExprKind::Require { path: inner, .. } => walk.expr(inner),
        ExprKind::Isset(exprs) => walk.exprs(exprs),
        ExprKind::Exit(status) => walk.opt_expr(status.as_deref()),
        ExprKind::Fn(anon_fn) => walk.anon_fn(anon_fn),
        ExprKind::SpawnScript { path, options } => {
            walk.expr(path);
            for option in options {
                walk.expr(&option.value);
            }
        }
        _ => {}
    }
}

/// The value of each argument in a written argument list.
fn arg_values<W: Walk + ?Sized>(walk: &mut W, args: &CallArgs) {
    if let CallArgs::List(list) = args {
        for arg in list {
            walk.expr(&arg.value);
        }
    }
}
