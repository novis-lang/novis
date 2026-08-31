//! The parse tree as a walkable shape: one entry per node, its production's
//! name and the nodes it contains — what
//! [ADR 0019](../../../docs/adr/0019-reflection-and-ast-parsing-are-core-features.md)
//! § 3's `Core\Ast::parse` hands a running program.
//!
//! # Decision: the walk lives beside the grammar, not beside the `Core` class
//!
//! `Core\Ast` is `nvs_stdlib`'s, and the obvious place for this match was
//! there. It cannot be: [`crate::ast::ExprKind`] and [`crate::ast::StmtKind`]
//! are `#[non_exhaustive]`, so a `match` written in any other crate needs a
//! wildcard arm and a production added later would silently walk as a leaf —
//! the tree would go quietly wrong rather than failing to build. Written here
//! the compiler checks it, and a new variant is a build error in this file,
//! which is the file the variant's author is already in.
//!
//! What crosses the crate boundary is therefore [`Node`], a rose tree of
//! `&'static str` kinds owning nothing of the source. `nvs_stdlib` turns that
//! into `Core\Ast\Node` instances and never sees an AST type — which is also
//! what keeps ADR 0019 § 3's inertness structural rather than promised: there
//! is no [`crate::ast::Expr`] on the other side of this function to reach a
//! lowering with.
//!
//! # Decision: a node is a statement, an expression, or a member of a
//! declaration
//!
//! Those are the productions ADR 0019 § 3's own examples name
//! (`Core\Ast\ClassDecl`, `Core\Ast\MethodDecl`). Everything else the grammar
//! carries — a type, a name, a modifier, an attribute, a parameter, a match
//! arm, a catch clause, a `foreach` binding — is a *property* of the node it
//! belongs to rather than a node of its own, and its nested expressions are
//! children of that node. So a parameter's default is a child of the function
//! it belongs to, and no node stands for the parameter itself.
//!
//! The alternative was one node per struct in [`crate::ast`], which is the
//! shape ADR 0019's typed roster eventually wants — but that roster is one
//! *class* per production, and until those classes exist a node with no type
//! of its own is just a kind string with a longer name. Drawing the line at
//! the three productions the ADR names keeps every expression in the tree,
//! which is what a walk is for, and leaves the redraw to the roster.
//!
//! **What it spends:** one [`Node`] per statement, expression and member —
//! two words for the kind plus a `Vec` header, allocated per parse and dropped
//! when the caller is done with it. Nothing here is cached: a parse is a call,
//! not a compilation unit.

use nvs_diagnostics::{Diagnostics, MAX_SOURCE_LEN, SourceMap};

use crate::ast::{
    AnonClassDecl, Block, CallArgs, ClassMember, ClassMemberKind, DestructureElement,
    DestructureTarget, EnumCase, Expr, ExprKind, FnBody, FnExpr, ForInit, MemberName, MethodMember,
    NewTarget, Param, PropertyHook, PropertyHookBody, Stmt, StmtKind, StringPart,
};
use crate::parse_file;

/// One node of a parsed tree: which production it is, and what it contains.
///
/// The kind is the [`crate::ast`] variant's own spelling — `"Binary"`,
/// `"Echo"`, `"Method"` — because the grammar is the thing being described and
/// a second vocabulary for it would be a second thing to keep true.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Node {
    /// The production this node is.
    pub kind: &'static str,
    /// The nodes this one contains, in source order.
    pub children: Vec<Node>,
}

impl Node {
    /// This node's whole subtree, itself excluded, in source order.
    ///
    /// The transitive closure of [`Self::children`], and excluding the
    /// receiver for the same reason `children` does: both answer *what this
    /// node contains*, and a node does not contain itself.
    #[must_use]
    pub fn descendants(&self) -> Vec<&Self> {
        let mut out = Vec::new();
        self.collect(&mut out);
        out
    }

    fn collect<'a>(&'a self, out: &mut Vec<&'a Self>) {
        for child in &self.children {
            out.push(child);
            child.collect(out);
        }
    }
}

/// Parses `source` exactly as the compiler parses a file of that name, and
/// answers the walk over it.
///
/// One grammar, ADR 0019 § 3: this calls [`parse_file`], so a construct that
/// compiles parses here and a construct the parser refuses is refused here.
/// The root node is the file, whose children are its top-level statements.
///
/// # Errors
///
/// The first error diagnostic, as `line N, column M: message` — a parse that
/// reported an error is a failure here rather than a tree with
/// [`StmtKind::Error`] in it, because a caller asking for a description of
/// source it cannot compile is asking about source it should be told about.
pub fn of_source(name: &str, source: &str) -> Result<Node, String> {
    if source.len() > MAX_SOURCE_LEN {
        return Err("source exceeds 4 GiB".to_owned());
    }
    let mut map = SourceMap::new();
    let id = map.add(name, source);
    let file = map.file(id);
    let mut diags = Diagnostics::new();
    let stmts = parse_file(file, &mut diags);
    if let Some(first) = diags.iter().find(|d| d.is_error()) {
        let where_ = first.primary_span().map_or_else(String::new, |span| {
            let (line, col) = file.line_col(span.start);
            format!("line {}, column {}: ", line + 1, col + 1)
        });
        return Err(format!("{where_}{}", first.message));
    }
    Ok(Node {
        kind: "File",
        children: stmts.iter().map(stmt).collect(),
    })
}

/// One statement, and the nodes under it.
fn stmt(s: &Stmt) -> Node {
    let mut kids = Vec::new();
    let kind = match &s.kind {
        StmtKind::Expr(e) => {
            kids.push(expr(e));
            "Expr"
        }
        StmtKind::Return(value) => {
            push_opt(&mut kids, value.as_ref());
            "Return"
        }
        StmtKind::Block(b) => {
            push_block(&mut kids, b);
            "Block"
        }
        StmtKind::Empty => "Empty",
        StmtKind::InlineHtml(_) => "InlineHtml",
        StmtKind::If { cond, then, else_ } => {
            kids.push(expr(cond));
            kids.push(stmt(then));
            if let Some(otherwise) = else_ {
                kids.push(stmt(otherwise));
            }
            "If"
        }
        StmtKind::While { cond, body } => {
            kids.push(expr(cond));
            kids.push(stmt(body));
            "While"
        }
        StmtKind::DoWhile { body, cond } => {
            kids.push(stmt(body));
            kids.push(expr(cond));
            "DoWhile"
        }
        StmtKind::For {
            init,
            cond,
            step,
            body,
        } => {
            match init {
                ForInit::Decl(decl) => kids.push(stmt(decl)),
                ForInit::Exprs(exprs) => push_exprs(&mut kids, exprs),
            }
            push_exprs(&mut kids, cond);
            push_exprs(&mut kids, step);
            kids.push(stmt(body));
            "For"
        }
        StmtKind::Foreach { subject, body, .. } => {
            // The bindings are not nodes — the module doc's second decision.
            kids.push(expr(subject));
            kids.push(stmt(body));
            "Foreach"
        }
        StmtKind::Switch { subject, cases } => {
            kids.push(expr(subject));
            for case in cases {
                push_opt(&mut kids, case.cond.as_ref());
                kids.extend(case.body.iter().map(stmt));
            }
            "Switch"
        }
        StmtKind::Break(depth) => {
            push_opt(&mut kids, depth.as_ref());
            "Break"
        }
        StmtKind::Continue(depth) => {
            push_opt(&mut kids, depth.as_ref());
            "Continue"
        }
        StmtKind::Try {
            body,
            catches,
            finally,
        } => {
            push_block(&mut kids, body);
            for catch in catches {
                push_block(&mut kids, &catch.body);
            }
            if let Some(finally) = finally {
                push_block(&mut kids, finally);
            }
            "Try"
        }
        StmtKind::Echo(values) => {
            push_exprs(&mut kids, values);
            "Echo"
        }
        StmtKind::Unset(targets) => {
            push_exprs(&mut kids, targets);
            "Unset"
        }
        StmtKind::LocalDecl { value, .. } => {
            push_opt(&mut kids, value.as_ref());
            "LocalDecl"
        }
        StmtKind::Destructure { target, value } => {
            push_destructure(&mut kids, target);
            kids.push(expr(value));
            "Destructure"
        }
        StmtKind::Global(_) => "Global",
        StmtKind::Goto(_) => "Goto",
        StmtKind::StaticLocal { vars, .. } => {
            for var in vars {
                push_opt(&mut kids, var.default.as_ref());
            }
            "StaticLocal"
        }
        StmtKind::ClassDecl(decl) => {
            kids.extend(decl.members.iter().map(member));
            "ClassDecl"
        }
        StmtKind::InterfaceDecl(decl) => {
            kids.extend(decl.members.iter().map(member));
            "InterfaceDecl"
        }
        StmtKind::EnumDecl(decl) => {
            kids.extend(decl.cases.iter().map(enum_case));
            kids.extend(decl.members.iter().map(member));
            "EnumDecl"
        }
        StmtKind::NamespaceDecl(decl) => {
            if let Some(body) = &decl.body {
                push_block(&mut kids, body);
            }
            "NamespaceDecl"
        }
        StmtKind::UseDecl(_) => "UseDecl",
        StmtKind::AutoloadDecl(_) => "AutoloadDecl",
        StmtKind::TypeAliasDecl(_) => "TypeAliasDecl",
        StmtKind::TopLevelFunction(f) => {
            push_method(&mut kids, f);
            "Function"
        }
        StmtKind::TopLevelConst(consts) => {
            for c in consts {
                kids.push(expr(&c.value));
            }
            "Const"
        }
        StmtKind::Error => "Error",
    };
    Node {
        kind,
        children: kids,
    }
}

/// One expression, and the nodes under it.
fn expr(e: &Expr) -> Node {
    let mut kids = Vec::new();
    let kind = match &e.kind {
        ExprKind::Null => "Null",
        ExprKind::Bool(_) => "Bool",
        ExprKind::Int(_) => "Int",
        ExprKind::Float(_) => "Float",
        ExprKind::Duration(_) => "Duration",
        ExprKind::Str(_) => "Str",
        ExprKind::Interpolated(parts) => {
            for part in parts {
                match part {
                    StringPart::Text(_) => {}
                    StringPart::Expr(e) => kids.push(expr(e)),
                }
            }
            "Interpolated"
        }
        ExprKind::Variable(_) => "Variable",
        ExprKind::ConstFetch(_) => "ConstFetch",
        ExprKind::SelfExpr => "SelfExpr",
        ExprKind::StaticExpr => "StaticExpr",
        ExprKind::ParentExpr => "ParentExpr",
        ExprKind::ArrayLiteral(items) => {
            for item in items {
                push_opt(&mut kids, item.key.as_ref());
                kids.push(expr(&item.value));
            }
            "ArrayLiteral"
        }
        ExprKind::Unary { expr: operand, .. } => {
            kids.push(expr(operand));
            "Unary"
        }
        ExprKind::PreIncDec { expr: operand, .. } => {
            kids.push(expr(operand));
            "PreIncDec"
        }
        ExprKind::PostIncDec { expr: operand, .. } => {
            kids.push(expr(operand));
            "PostIncDec"
        }
        ExprKind::Binary { lhs, rhs, .. } => {
            kids.push(expr(lhs));
            kids.push(expr(rhs));
            "Binary"
        }
        ExprKind::Assign { target, value, .. } => {
            kids.push(expr(target));
            kids.push(expr(value));
            "Assign"
        }
        ExprKind::Ternary { cond, then, else_ } => {
            kids.push(expr(cond));
            if let Some(then) = then {
                kids.push(expr(then));
            }
            kids.push(expr(else_));
            "Ternary"
        }
        ExprKind::Conversion { expr: operand, .. } => {
            kids.push(expr(operand));
            "Conversion"
        }
        ExprKind::InstanceOf {
            expr: operand,
            class,
        } => {
            kids.push(expr(operand));
            kids.push(expr(class));
            "InstanceOf"
        }
        ExprKind::Call { callee, args } => {
            kids.push(expr(callee));
            push_args(&mut kids, args);
            "Call"
        }
        ExprKind::MethodCall {
            object,
            method,
            args,
            ..
        } => {
            kids.push(expr(object));
            push_member_name(&mut kids, method);
            push_args(&mut kids, args);
            "MethodCall"
        }
        ExprKind::StaticCall {
            class,
            method,
            args,
            ..
        } => {
            kids.push(expr(class));
            push_member_name(&mut kids, method);
            push_args(&mut kids, args);
            "StaticCall"
        }
        ExprKind::PropertyAccess {
            object, property, ..
        } => {
            kids.push(expr(object));
            push_member_name(&mut kids, property);
            "PropertyAccess"
        }
        ExprKind::StaticPropertyAccess { class, .. } => {
            kids.push(expr(class));
            "StaticPropertyAccess"
        }
        ExprKind::ClassConstAccess { class, .. } => {
            kids.push(expr(class));
            "ClassConstAccess"
        }
        ExprKind::ClassNameConst { class } => {
            kids.push(expr(class));
            "ClassNameConst"
        }
        ExprKind::Index { base, index } => {
            kids.push(expr(base));
            push_opt(&mut kids, index.as_deref());
            "Index"
        }
        ExprKind::New { target, args, .. } => {
            match target {
                NewTarget::Name(_)
                | NewTarget::SelfTy
                | NewTarget::StaticTy
                | NewTarget::ParentTy => {}
                NewTarget::Expr(e) => kids.push(expr(e)),
                NewTarget::AnonClass(decl) => push_anon_class(&mut kids, decl),
            }
            push_args(&mut kids, args);
            "New"
        }
        ExprKind::Clone(operand) => {
            kids.push(expr(operand));
            "Clone"
        }
        ExprKind::Fn(f) => {
            push_fn(&mut kids, f);
            "Fn"
        }
        ExprKind::Match { subject, arms } => {
            kids.push(expr(subject));
            for arm in arms {
                if let Some(conditions) = &arm.conditions {
                    push_exprs(&mut kids, conditions);
                }
                kids.push(expr(&arm.body));
            }
            "Match"
        }
        ExprKind::Catch { guarded, arms } => {
            kids.push(expr(guarded));
            for arm in arms {
                kids.push(expr(&arm.body));
            }
            "Catch"
        }
        ExprKind::Yield { key, value } => {
            push_opt(&mut kids, key.as_deref());
            push_opt(&mut kids, value.as_deref());
            "Yield"
        }
        ExprKind::YieldFrom(operand) => {
            kids.push(expr(operand));
            "YieldFrom"
        }
        ExprKind::Print(operand) => {
            kids.push(expr(operand));
            "Print"
        }
        ExprKind::Throw(operand) => {
            kids.push(expr(operand));
            "Throw"
        }
        ExprKind::Isset(targets) => {
            push_exprs(&mut kids, targets);
            "Isset"
        }
        ExprKind::Empty(operand) => {
            kids.push(expr(operand));
            "Empty"
        }
        ExprKind::Exit(status) => {
            push_opt(&mut kids, status.as_deref());
            "Exit"
        }
        ExprKind::SpawnScript { path, options } => {
            kids.push(expr(path));
            for option in options {
                kids.push(expr(&option.value));
            }
            "SpawnScript"
        }
        ExprKind::Await(operand) => {
            kids.push(expr(operand));
            "Await"
        }
        ExprKind::Require { path } => {
            kids.push(expr(path));
            "Require"
        }
        ExprKind::Paren(inner) => {
            kids.push(expr(inner));
            "Paren"
        }
        ExprKind::ObjectLiteral(fields) => {
            for field in fields {
                kids.push(expr(&field.value));
            }
            "ObjectLiteral"
        }
        ExprKind::Error => "Error",
    };
    Node {
        kind,
        children: kids,
    }
}

/// One member of a class, interface or enum body.
fn member(m: &ClassMember) -> Node {
    let mut kids = Vec::new();
    let kind = match &m.kind {
        ClassMemberKind::Property(p) => {
            push_opt(&mut kids, p.default.as_ref());
            for hook in p.hooks.iter().flatten() {
                push_hook(&mut kids, hook);
            }
            "Property"
        }
        ClassMemberKind::Const(c) => {
            kids.push(expr(&c.value));
            "Const"
        }
        ClassMemberKind::Method(f) => {
            push_method(&mut kids, f);
            "Method"
        }
        ClassMemberKind::Error => "Error",
    };
    Node {
        kind,
        children: kids,
    }
}

/// One `case` of an enum declaration.
fn enum_case(c: &EnumCase) -> Node {
    let mut kids = Vec::new();
    push_opt(&mut kids, c.value.as_ref());
    Node {
        kind: "EnumCase",
        children: kids,
    }
}

fn push_opt(kids: &mut Vec<Node>, value: Option<&Expr>) {
    if let Some(value) = value {
        kids.push(expr(value));
    }
}

fn push_exprs(kids: &mut Vec<Node>, values: &[Expr]) {
    kids.extend(values.iter().map(expr));
}

/// A block that is not itself a statement contributes its statements — a
/// `try` body's braces are the `try`'s, not a node between them.
fn push_block(kids: &mut Vec<Node>, block: &Block) {
    kids.extend(block.stmts.iter().map(stmt));
}

fn push_args(kids: &mut Vec<Node>, args: &CallArgs) {
    match args {
        CallArgs::List(list) => kids.extend(list.iter().map(|arg| expr(&arg.value))),
        CallArgs::FirstClassCallable => {}
    }
}

/// A member name computed from an expression carries that expression; a
/// written one is a name, and the module doc's second decision says a name is
/// not a node.
fn push_member_name(kids: &mut Vec<Node>, name: &MemberName) {
    match name {
        MemberName::Ident(_) => {}
        MemberName::Variable(e) | MemberName::Expr(e) => kids.push(expr(e)),
    }
}

fn push_params(kids: &mut Vec<Node>, params: &[Param]) {
    for param in params {
        push_opt(kids, param.default.as_ref());
    }
}

fn push_method(kids: &mut Vec<Node>, f: &MethodMember) {
    push_params(kids, &f.params);
    if let Some(body) = &f.body {
        push_block(kids, body);
    }
}

fn push_fn(kids: &mut Vec<Node>, f: &FnExpr) {
    push_params(kids, &f.params);
    match &f.body {
        FnBody::Expr(e) => kids.push(expr(e)),
        FnBody::Block(b) => push_block(kids, b),
    }
}

fn push_hook(kids: &mut Vec<Node>, hook: &PropertyHook) {
    if let Some(param) = &hook.param {
        push_opt(kids, param.default.as_ref());
    }
    match &hook.body {
        None => {}
        Some(PropertyHookBody::Expr(e)) => kids.push(expr(e)),
        Some(PropertyHookBody::Block(b)) => push_block(kids, b),
    }
}

fn push_anon_class(kids: &mut Vec<Node>, decl: &AnonClassDecl) {
    kids.extend(decl.members.iter().map(member));
}

/// A destructuring target's *keys* are expressions; its leaves are names.
fn push_destructure(kids: &mut Vec<Node>, target: &DestructureTarget) {
    for element in &target.elements {
        match element {
            DestructureElement::Skip => {}
            DestructureElement::Leaf { key, .. } => push_opt(kids, key.as_ref()),
            DestructureElement::Nested { key, target, .. } => {
                push_opt(kids, key.as_ref());
                push_destructure(kids, target);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::of_source;

    #[test]
    fn a_files_walk_is_every_statement_and_expression_under_it() {
        let tree = of_source("<test>", "<?nvs echo 1 + 2;").expect("parses");
        assert_eq!(tree.kind, "File");
        let kinds: Vec<&str> = tree.descendants().iter().map(|n| n.kind).collect();
        assert_eq!(kinds, ["Echo", "Binary", "Int", "Int"]);
    }

    #[test]
    fn a_method_body_is_reached_through_its_class_and_its_member() {
        let tree = of_source(
            "<test>",
            "<?nvs class C { public function m(): void { echo 1; } }",
        )
        .expect("parses");
        let kinds: Vec<&str> = tree.descendants().iter().map(|n| n.kind).collect();
        assert_eq!(kinds, ["ClassDecl", "Method", "Echo", "Int"]);
    }

    #[test]
    fn source_the_compiler_refuses_is_refused_here_with_its_position() {
        let error = of_source("<test>", "<?nvs echo ;").expect_err("does not parse");
        assert!(error.starts_with("line 1, column "), "{error}");
    }
}
