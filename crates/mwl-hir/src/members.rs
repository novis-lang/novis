//! Member existence resolution for M2 item 2: every `Class::member`
//! reference — a static call, a class constant (an enum case counts as one),
//! `Class::class`, or a static property — must name something actually
//! declared on that class or reached transitively through the
//! [`ClassGraph`] built in [`crate::hierarchy`] (`extends`/`implements`).
//! [ADR 0011](../../../docs/adr/0011-functions-and-constants-are-class-members.md)
//! gives a callable/constant no bare-name fallback to fall into instead, so
//! there is nothing else a `Class::member` reference could mean.
//!
//! Also carries M2 item 5, the property-access counterpart: `$this->name`
//! must name an instance property actually declared on the enclosing class
//! or reached the same way through [`ClassGraph`], per
//! [ADR 0014](../../../docs/adr/0014-property-observer.md) § 5's "no
//! `__get`/`__set` fallback." `$this` is the only property-access receiver
//! whose class is knowable without a type checker — see the known gaps
//! below for every other receiver shape.
//!
//! [`MemberResolver::collect_members`] walks a file's declarations, same
//! shape as [`crate::hierarchy::HierarchyResolver::collect_links`], recording
//! each class/interface/enum's own directly-declared method, constant
//! (enum cases included), instance-property and static-property names into a
//! [`MemberTable`]. [`MemberResolver::check`] then walks the same file's
//! statements a second time — this time descending into every method body,
//! property default, constant value and parameter default it finds —
//! looking for a `self::`/`static::`/`parent::`/explicit-class-name member
//! reference or a `$this->name` property access, and checking either against
//! the table, walking ancestors the same way [`crate::hierarchy::detect_cycles`]
//! does. A dynamic class side (`$var::method()`, `(expr)::CONST`) or a
//! non-`$this` property receiver is not statically resolvable and is
//! silently skipped, same as everywhere else this milestone only reports
//! what it can be sure of.
//!
//! **Known gaps**, narrower versions of gaps [`crate::hierarchy`] already
//! documents:
//! - `new Foo(...)`/`new self(...)`/etc. does not have its target checked
//!   here — instantiation resolution is a distinct concern from a
//!   callable/constant reference and is left for later.
//! - A member's visibility (`private`/`protected`) is not checked — only
//!   whether it is declared anywhere in the chain.
//! - A property access on any receiver other than `$this` — a typed local, a
//!   chained call result, `self::factory()`'s return, an explicit
//!   `new Foo()` — is never checked *here*, since this module has no static
//!   type to check it against. That is not left open: `mwl-types`'
//!   `expr::check_property_access` closes it once a static type exists,
//!   reporting `E_UNKNOWN_MEMBER` for the same shape of miss this module
//!   reports `E_UNDEFINED_PROPERTY` for on `$this` — split across crates by
//!   which one has the type to check against, not skipped by either.
//! - A property access whose name is not a literal identifier
//!   (`$obj->$name`, `$obj->{expr}`) is a runtime concern per ADR 0014 § 5,
//!   not a compile-time one, and is silently skipped here regardless of
//!   receiver.

use mwl_diagnostics::{Diagnostic, Diagnostics, SourceFile, Span, code};
use mwl_syntax::ast::{
    Arg, ArrayItem, Block, CallArgs, ClassMember, ClassMemberKind, DestructureElement,
    DestructureTarget, Expr, ExprKind, FnBody, MemberName, Modifier, NamespaceDecl, Stmt, StmtKind,
    StringPart,
};
use rustc_hash::{FxHashMap, FxHashSet};

use crate::hierarchy::{ClassGraph, resolve_ref};
use crate::qname::QName;
use crate::resolve::{name_text, qname_segments};
use crate::symbol::SymbolTable;

/// One class/interface/trait/enum's own directly-declared members — never
/// includes anything pulled in via `extends`/`implements`/trait-use; walking
/// those is [`ClassGraph`]'s job, done at check time.
#[derive(Clone, Debug, Default)]
pub struct ClassMembers {
    /// Method names.
    pub methods: FxHashSet<String>,
    /// Constant names, enum cases included.
    pub consts: FxHashSet<String>,
    /// Static property names, `$` sigil stripped.
    pub static_props: FxHashSet<String>,
    /// Instance property names, `$` sigil stripped.
    pub props: FxHashSet<String>,
}

/// Every declaration's own [`ClassMembers`], keyed by its [`QName`].
#[derive(Debug, Default)]
pub struct MemberTable {
    by_class: FxHashMap<QName, ClassMembers>,
}

impl MemberTable {
    /// An empty table.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// One declaration's own members, if any were collected under this name.
    #[must_use]
    pub fn get(&self, qname: &QName) -> Option<&ClassMembers> {
        self.by_class.get(qname)
    }

    fn entry(&mut self, qname: QName) -> &mut ClassMembers {
        self.by_class.entry(qname).or_default()
    }
}

/// Which kind of member a `Class::member` reference names — decides which of
/// [`ClassMembers`]' three sets is checked.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum MemberKind {
    Method,
    Const,
    StaticProp,
    Prop,
}

impl MemberKind {
    const fn describe(self) -> &'static str {
        match self {
            Self::Method => "method",
            Self::Const => "constant",
            Self::StaticProp => "static property",
            Self::Prop => "property",
        }
    }
}

/// Collects a [`MemberTable`] from one or more files ([`Self::collect_members`]),
/// then checks every `Class::member` reference in those same files against it
/// and the already-resolved [`ClassGraph`] ([`Self::check`]). Two calls, same
/// two-pass shape as [`crate::hierarchy::HierarchyResolver`], for the same
/// reason: a reference may name a member declared in a file collected later.
#[derive(Debug, Default)]
pub struct MemberResolver {
    table: MemberTable,
}

impl MemberResolver {
    /// A resolver with nothing collected yet.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Consumes the resolver, returning the [`MemberTable`] it collected.
    #[must_use]
    pub fn into_table(self) -> MemberTable {
        self.table
    }

    /// Walks `stmts`, recording every class/interface/trait/enum's own
    /// directly-declared members into this resolver's [`MemberTable`].
    pub fn collect_members(&mut self, stmts: &[Stmt], src: &SourceFile) {
        self.collect_in(stmts, src, &[]);
    }

    fn collect_in(&mut self, stmts: &[Stmt], src: &SourceFile, namespace: &[String]) {
        let mut current_ns: Vec<String> = namespace.to_vec();

        for stmt in stmts {
            match &stmt.kind {
                StmtKind::NamespaceDecl(NamespaceDecl { name, body, .. }) => {
                    let new_ns = name
                        .as_ref()
                        .map_or_else(Vec::new, |n| qname_segments(src, n));
                    match body {
                        Some(block) => self.collect_in(&block.stmts, src, &new_ns),
                        None => current_ns = new_ns,
                    }
                }
                StmtKind::ClassDecl(decl) => {
                    let qname = QName::join(&current_ns, name_text(src, &decl.name));
                    self.collect_class_members(&decl.members, src, qname);
                }
                StmtKind::InterfaceDecl(decl) => {
                    let qname = QName::join(&current_ns, name_text(src, &decl.name));
                    self.collect_class_members(&decl.members, src, qname);
                }
                StmtKind::EnumDecl(decl) => {
                    let qname = QName::join(&current_ns, name_text(src, &decl.name));
                    {
                        let members = self.table.entry(qname.clone());
                        for case in &decl.cases {
                            members.consts.insert(name_text(src, &case.name).to_owned());
                        }
                    }
                    self.collect_class_members(&decl.members, src, qname);
                }
                _ => {}
            }
        }
    }

    fn collect_class_members(&mut self, members: &[ClassMember], src: &SourceFile, qname: QName) {
        let entry = self.table.entry(qname);
        for member in members {
            match &member.kind {
                ClassMemberKind::Method(m) => {
                    entry
                        .methods
                        .insert(src.span_text(m.name).unwrap_or_default().to_owned());
                }
                ClassMemberKind::Const(c) => {
                    entry
                        .consts
                        .insert(src.span_text(c.name).unwrap_or_default().to_owned());
                }
                ClassMemberKind::Property(p) => {
                    let text = src.span_text(p.name).unwrap_or_default();
                    let name = text.strip_prefix('$').unwrap_or(text).to_owned();
                    if p.modifiers.contains(&Modifier::Static) {
                        entry.static_props.insert(name);
                    } else {
                        entry.props.insert(name);
                    }
                }
                ClassMemberKind::Error => {}
                _ => {}
            }
        }
    }

    /// Checks every `Class::member` reference found while walking `stmts`
    /// against `symbols`, `graph` and this resolver's [`MemberTable`],
    /// reporting `E_UNDEFINED_CLASS` for a class side that resolves to
    /// nothing declared and `E_UNDEFINED_MEMBER` for a class side that
    /// resolves but the named member is not declared anywhere in its
    /// `extends`/`implements`/trait-use chain. Call once, after every file
    /// sharing this table and its `SymbolTable`/`ClassGraph` has run
    /// [`Self::collect_members`].
    pub fn check(
        &self,
        stmts: &[Stmt],
        src: &SourceFile,
        symbols: &SymbolTable,
        graph: &ClassGraph,
        diags: &mut Diagnostics,
    ) {
        let mut env = Env {
            symbols,
            graph,
            table: &self.table,
            diags,
        };
        check_stmts(stmts, src, None, &[], &FxHashMap::default(), &mut env);
    }
}

/// Everything a `Class::member` reference needs to resolve at the point it
/// was written: the class it sits inside (if any), and the namespace/`use`
/// scope active there.
struct Ctx<'a> {
    current_class: Option<&'a QName>,
    namespace: &'a [String],
    imports: &'a FxHashMap<String, QName>,
}

/// The read-only tables and the diagnostics sink every walking function
/// needs, bundled so a recursive call threads one argument instead of four.
struct Env<'a> {
    symbols: &'a SymbolTable,
    graph: &'a ClassGraph,
    table: &'a MemberTable,
    diags: &'a mut Diagnostics,
}

fn check_stmts(
    stmts: &[Stmt],
    src: &SourceFile,
    current_class: Option<&QName>,
    namespace: &[String],
    imports: &FxHashMap<String, QName>,
    env: &mut Env<'_>,
) {
    let mut current_ns: Vec<String> = namespace.to_vec();
    let mut current_imports: FxHashMap<String, QName> = imports.clone();

    for stmt in stmts {
        match &stmt.kind {
            StmtKind::NamespaceDecl(NamespaceDecl { name, body, .. }) => {
                let new_ns = name
                    .as_ref()
                    .map_or_else(Vec::new, |n| qname_segments(src, n));
                match body {
                    Some(block) => check_stmts(
                        &block.stmts,
                        src,
                        current_class,
                        &new_ns,
                        &FxHashMap::default(),
                        env,
                    ),
                    None => {
                        current_ns = new_ns;
                        current_imports.clear();
                    }
                }
            }
            StmtKind::UseDecl(use_decl) => {
                let target = QName::parse(name_text(src, &use_decl.path));
                current_imports.insert(target.short_name().to_owned(), target);
            }
            StmtKind::ClassDecl(decl) => {
                let qname = QName::join(&current_ns, name_text(src, &decl.name));
                let ctx = Ctx {
                    current_class: Some(&qname),
                    namespace: &current_ns,
                    imports: &current_imports,
                };
                check_members(&decl.members, src, &ctx, env);
            }
            StmtKind::InterfaceDecl(decl) => {
                let qname = QName::join(&current_ns, name_text(src, &decl.name));
                let ctx = Ctx {
                    current_class: Some(&qname),
                    namespace: &current_ns,
                    imports: &current_imports,
                };
                check_members(&decl.members, src, &ctx, env);
            }
            StmtKind::EnumDecl(decl) => {
                let qname = QName::join(&current_ns, name_text(src, &decl.name));
                let ctx = Ctx {
                    current_class: Some(&qname),
                    namespace: &current_ns,
                    imports: &current_imports,
                };
                for case in &decl.cases {
                    if let Some(v) = &case.value {
                        walk_expr(v, src, &ctx, env);
                    }
                }
                check_members(&decl.members, src, &ctx, env);
            }
            _ => {
                let ctx = Ctx {
                    current_class,
                    namespace: &current_ns,
                    imports: &current_imports,
                };
                walk_stmt(stmt, src, &ctx, env);
            }
        }
    }
}

fn check_members(members: &[ClassMember], src: &SourceFile, ctx: &Ctx<'_>, env: &mut Env<'_>) {
    for member in members {
        match &member.kind {
            ClassMemberKind::Method(m) => {
                for param in &m.params {
                    if let Some(default) = &param.default {
                        walk_expr(default, src, ctx, env);
                    }
                }
                if let Some(body) = &m.body {
                    check_stmts(
                        &body.stmts,
                        src,
                        ctx.current_class,
                        ctx.namespace,
                        ctx.imports,
                        env,
                    );
                }
            }
            ClassMemberKind::Const(c) => walk_expr(&c.value, src, ctx, env),
            ClassMemberKind::Property(p) => {
                if let Some(default) = &p.default {
                    walk_expr(default, src, ctx, env);
                }
            }
            ClassMemberKind::Error => {}
            _ => {}
        }
    }
}

fn walk_block(block: &Block, src: &SourceFile, ctx: &Ctx<'_>, env: &mut Env<'_>) {
    check_stmts(
        &block.stmts,
        src,
        ctx.current_class,
        ctx.namespace,
        ctx.imports,
        env,
    );
}

fn walk_stmt(stmt: &Stmt, src: &SourceFile, ctx: &Ctx<'_>, env: &mut Env<'_>) {
    macro_rules! e {
        ($expr:expr) => {
            walk_expr($expr, src, ctx, env)
        };
    }
    macro_rules! s {
        ($stmt:expr) => {
            walk_stmt($stmt, src, ctx, env)
        };
    }

    match &stmt.kind {
        StmtKind::Expr(x) => e!(x),
        StmtKind::Return(Some(x)) | StmtKind::Break(Some(x)) | StmtKind::Continue(Some(x)) => {
            e!(x);
        }
        StmtKind::Block(b) => walk_block(b, src, ctx, env),
        StmtKind::If { cond, then, else_ } => {
            e!(cond);
            s!(then);
            if let Some(else_) = else_ {
                s!(else_);
            }
        }
        StmtKind::While { cond, body } => {
            e!(cond);
            s!(body);
        }
        StmtKind::DoWhile { body, cond } => {
            s!(body);
            e!(cond);
        }
        StmtKind::For {
            init,
            cond,
            step,
            body,
        } => {
            for x in init.iter().chain(cond).chain(step) {
                e!(x);
            }
            s!(body);
        }
        StmtKind::Foreach { subject, body, .. } => {
            e!(subject);
            s!(body);
        }
        StmtKind::Switch { subject, cases } => {
            e!(subject);
            for case in cases {
                if let Some(cond) = &case.cond {
                    e!(cond);
                }
                check_stmts(
                    &case.body,
                    src,
                    ctx.current_class,
                    ctx.namespace,
                    ctx.imports,
                    env,
                );
            }
        }
        StmtKind::Try {
            body,
            catches,
            finally,
        } => {
            walk_block(body, src, ctx, env);
            for catch in catches {
                walk_block(&catch.body, src, ctx, env);
            }
            if let Some(finally) = finally {
                walk_block(finally, src, ctx, env);
            }
        }
        StmtKind::Echo(xs) | StmtKind::Unset(xs) => {
            for x in xs {
                e!(x);
            }
        }
        StmtKind::LocalDecl {
            value: Some(value), ..
        } => e!(value),
        StmtKind::Destructure { target, value } => {
            walk_destructure_target(target, src, ctx, env);
            e!(value);
        }
        StmtKind::StaticLocal { vars, .. } => {
            for var in vars {
                if let Some(default) = &var.default {
                    e!(default);
                }
            }
        }
        // Declarations reachable inside a body are handled by `check_stmts`
        // itself, not here — this arm (and the trailing wildcard) cover
        // everything else, including `Return`/`Break`/`Continue` with no
        // expression and `LocalDecl` with no initializer.
        _ => {}
    }
}

fn walk_destructure_target(
    target: &DestructureTarget,
    src: &SourceFile,
    ctx: &Ctx<'_>,
    env: &mut Env<'_>,
) {
    for element in &target.elements {
        match element {
            DestructureElement::Leaf { key: Some(key), .. } => walk_expr(key, src, ctx, env),
            DestructureElement::Nested { key, target, .. } => {
                if let Some(key) = key {
                    walk_expr(key, src, ctx, env);
                }
                walk_destructure_target(target, src, ctx, env);
            }
            _ => {}
        }
    }
}

fn walk_member_name(member: &MemberName, src: &SourceFile, ctx: &Ctx<'_>, env: &mut Env<'_>) {
    if let MemberName::Variable(e) | MemberName::Expr(e) = member {
        walk_expr(e, src, ctx, env);
    }
}

fn walk_args(args: &CallArgs, src: &SourceFile, ctx: &Ctx<'_>, env: &mut Env<'_>) {
    let CallArgs::List(list) = args else {
        return;
    };
    for Arg { value, .. } in list {
        walk_expr(value, src, ctx, env);
    }
}

#[expect(
    clippy::too_many_lines,
    reason = "one match arm per AST expression variant, each a couple of lines"
)]
fn walk_expr(expr: &Expr, src: &SourceFile, ctx: &Ctx<'_>, env: &mut Env<'_>) {
    macro_rules! e {
        ($expr:expr) => {
            walk_expr($expr, src, ctx, env)
        };
    }

    match &expr.kind {
        ExprKind::Interpolated(parts) => {
            for part in parts {
                if let StringPart::Expr(x) = part {
                    e!(x);
                }
            }
        }
        ExprKind::ArrayLiteral(items) => {
            for ArrayItem { key, value, .. } in items {
                if let Some(key) = key {
                    e!(key);
                }
                e!(value);
            }
        }
        ExprKind::Unary { expr, .. }
        | ExprKind::PreIncDec { expr, .. }
        | ExprKind::PostIncDec { expr, .. }
        | ExprKind::Clone(expr)
        | ExprKind::YieldFrom(expr)
        | ExprKind::Print(expr)
        | ExprKind::Throw(expr)
        | ExprKind::Empty(expr)
        | ExprKind::Paren(expr) => e!(expr),
        ExprKind::Binary { lhs, rhs, .. } => {
            e!(lhs);
            e!(rhs);
        }
        ExprKind::Assign { target, value, .. } => {
            e!(target);
            e!(value);
        }
        ExprKind::Ternary { cond, then, else_ } => {
            e!(cond);
            if let Some(then) = then {
                e!(then);
            }
            e!(else_);
        }
        ExprKind::Conversion { expr, .. } => e!(expr),
        ExprKind::InstanceOf { expr, class } => {
            e!(expr);
            e!(class);
        }
        ExprKind::Call { callee, args } => {
            e!(callee);
            walk_args(args, src, ctx, env);
        }
        ExprKind::MethodCall {
            object,
            method,
            args,
            ..
        } => {
            e!(object);
            walk_member_name(method, src, ctx, env);
            walk_args(args, src, ctx, env);
        }
        ExprKind::StaticCall {
            class,
            method,
            args,
        } => {
            e!(class);
            walk_member_name(method, src, ctx, env);
            walk_args(args, src, ctx, env);
            if let MemberName::Ident(name_span) = method {
                check_member_ref(
                    class,
                    src.span_text(*name_span).unwrap_or_default(),
                    MemberKind::Method,
                    src,
                    ctx,
                    env,
                );
            }
        }
        ExprKind::PropertyAccess {
            object, property, ..
        } => {
            e!(object);
            walk_member_name(property, src, ctx, env);
            if let (ExprKind::Variable(var_span), MemberName::Ident(name_span)) =
                (&object.kind, property)
                && src.span_text(*var_span) == Some("$this")
            {
                check_property_ref(
                    src.span_text(*name_span).unwrap_or_default(),
                    object.span,
                    ctx,
                    env,
                );
            }
        }
        ExprKind::StaticPropertyAccess { class, name } => {
            e!(class);
            let text = src.span_text(*name).unwrap_or_default();
            check_member_ref(
                class,
                text.strip_prefix('$').unwrap_or(text),
                MemberKind::StaticProp,
                src,
                ctx,
                env,
            );
        }
        ExprKind::ClassConstAccess { class, name } => {
            e!(class);
            check_member_ref(
                class,
                src.span_text(*name).unwrap_or_default(),
                MemberKind::Const,
                src,
                ctx,
                env,
            );
        }
        ExprKind::ClassNameConst { class } => e!(class),
        ExprKind::Index { base, index } => {
            e!(base);
            if let Some(index) = index {
                e!(index);
            }
        }
        ExprKind::New { args, .. } => walk_args(args, src, ctx, env),
        ExprKind::Fn(fn_expr) => {
            for param in &fn_expr.params {
                if let Some(default) = &param.default {
                    e!(default);
                }
            }
            match &fn_expr.body {
                FnBody::Block(block) => walk_block(block, src, ctx, env),
                FnBody::Expr(body) => e!(body),
            }
        }
        ExprKind::Match { subject, arms } => {
            e!(subject);
            for arm in arms {
                if let Some(conds) = &arm.conditions {
                    for cond in conds {
                        e!(cond);
                    }
                }
                e!(&arm.body);
            }
        }
        ExprKind::Yield { key, value } => {
            if let Some(key) = key {
                e!(key);
            }
            if let Some(value) = value {
                e!(value);
            }
        }
        ExprKind::Exit(Some(x)) => e!(x),
        ExprKind::Isset(xs) => {
            for x in xs {
                e!(x);
            }
        }
        ExprKind::SpawnScript { path, options } => {
            e!(path);
            for option in options {
                e!(&option.value);
            }
        }
        ExprKind::Require { path } => e!(path),
        _ => {}
    }
}

/// Resolves `class_expr` to a real class-side [`QName`] the same way
/// [`crate::hierarchy`] resolves `extends`/`implements` — `self`/`static`
/// against the enclosing class, `parent` against its first `extends` entry,
/// an explicit name via `use`-import/namespace lookup — and, once resolved,
/// checks `name` against it and its ancestors. A dynamic class side (a
/// variable, a parenthesized expression, ...) is not statically resolvable
/// and is silently skipped, same as `self`/`static`/`parent` used with no
/// enclosing class (a diagnostic for that belongs to a different check).
fn check_member_ref(
    class_expr: &Expr,
    name: &str,
    kind: MemberKind,
    src: &SourceFile,
    ctx: &Ctx<'_>,
    env: &mut Env<'_>,
) {
    let qname = match &class_expr.kind {
        ExprKind::SelfExpr | ExprKind::StaticExpr => ctx.current_class.cloned(),
        ExprKind::ParentExpr => {
            let Some(current) = ctx.current_class else {
                return;
            };
            match env
                .graph
                .get(current)
                .and_then(|links| links.extends.first())
            {
                Some(parent) => Some(parent.clone()),
                None => {
                    env.diags.report(
                        Diagnostic::error(
                            code::E_UNDEFINED_CLASS,
                            format!("`{current}` has no parent class for `parent::` to name"),
                        )
                        .with_primary(class_expr.span, "no parent class"),
                    );
                    return;
                }
            }
        }
        ExprKind::ConstFetch(class_name) => {
            let text = name_text(src, class_name);
            let resolved = resolve_ref(text, ctx.namespace, ctx.imports);
            if resolved.is_core() || env.symbols.contains(&resolved) {
                Some(resolved)
            } else {
                env.diags.report(
                    Diagnostic::error(
                        code::E_UNDEFINED_CLASS,
                        format!("`{resolved}` is not declared"),
                    )
                    .with_primary(class_name.span, "no matching declaration"),
                );
                None
            }
        }
        _ => None,
    };

    let Some(qname) = qname else {
        return;
    };
    if member_declared(&qname, name, kind, env.table, env.graph) {
        return;
    }
    env.diags.report(
        Diagnostic::error(
            code::E_UNDEFINED_MEMBER,
            format!("`{qname}` has no {} named `{name}`", kind.describe()),
        )
        .with_primary(class_expr.span, "referenced here"),
    );
}

/// Checks `$this->name` against the enclosing class and its
/// `extends`/`implements`/trait-use ancestors, the same table
/// [`check_member_ref`] checks a `Class::member` reference against. `$this`
/// with no enclosing class in scope — top-level script code, outside any
/// method — has no class to check against and is silently skipped, same as
/// `self`/`static` used the same way in [`check_member_ref`].
fn check_property_ref(name: &str, span: Span, ctx: &Ctx<'_>, env: &mut Env<'_>) {
    let Some(current) = ctx.current_class else {
        return;
    };
    if member_declared(current, name, MemberKind::Prop, env.table, env.graph) {
        return;
    }
    env.diags.report(
        Diagnostic::error(
            code::E_UNDEFINED_PROPERTY,
            format!("`{current}` has no property named `{name}`"),
        )
        .with_primary(span, "referenced here"),
    );
}

fn member_declared(
    qname: &QName,
    name: &str,
    kind: MemberKind,
    table: &MemberTable,
    graph: &ClassGraph,
) -> bool {
    let mut seen = FxHashSet::default();
    member_declared_rec(qname, name, kind, table, graph, &mut seen)
}

fn member_declared_rec(
    qname: &QName,
    name: &str,
    kind: MemberKind,
    table: &MemberTable,
    graph: &ClassGraph,
    seen: &mut FxHashSet<QName>,
) -> bool {
    if qname.is_core() {
        return true;
    }
    if !seen.insert(qname.clone()) {
        return false;
    }
    if let Some(members) = table.get(qname) {
        let found = match kind {
            MemberKind::Method => members.methods.contains(name),
            MemberKind::Const => members.consts.contains(name),
            MemberKind::StaticProp => members.static_props.contains(name),
            MemberKind::Prop => members.props.contains(name),
        };
        if found {
            return true;
        }
    }
    let Some(links) = graph.get(qname) else {
        return false;
    };
    links
        .extends
        .iter()
        .chain(links.implements.iter())
        .any(|parent| member_declared_rec(parent, name, kind, table, graph, seen))
}

#[cfg(test)]
mod tests {
    use mwl_diagnostics::SourceMap;
    use mwl_syntax::parse_file;

    use super::*;
    use crate::resolve::resolve_file;

    fn check(src: &str) -> Diagnostics {
        let mut map = SourceMap::new();
        let file = map.add("t.mwl", src);
        let mut diags = Diagnostics::new();
        let stmts = parse_file(map.file(file), &mut diags);
        assert!(!diags.has_errors(), "fixture failed to parse: {diags:?}");
        let module = resolve_file(&stmts, map.file(file), &mut diags);
        let mut members = MemberResolver::new();
        members.collect_members(&stmts, map.file(file));
        members.check(
            &stmts,
            map.file(file),
            &module.symbols,
            &module.graph,
            &mut diags,
        );
        diags
    }

    #[test]
    fn a_self_call_to_an_own_method_resolves() {
        let diags =
            check("<?mwl\nclass Foo { function a(): void { self::b(); } function b(): void {} }\n");
        assert!(!diags.has_errors(), "{diags:?}");
    }

    #[test]
    fn a_self_call_to_an_undeclared_method_is_diagnosed() {
        let diags = check("<?mwl\nclass Foo { function a(): void { self::missing(); } }\n");
        assert!(
            diags
                .iter()
                .any(|d| d.code == Some(code::E_UNDEFINED_MEMBER))
        );
    }

    #[test]
    fn a_parent_call_to_an_inherited_method_resolves() {
        let diags = check(
            "<?mwl\n\
             class Base { function greet(): void {} }\n\
             class Sub extends Base { function a(): void { parent::greet(); } }\n",
        );
        assert!(!diags.has_errors(), "{diags:?}");
    }

    #[test]
    fn parent_with_no_parent_class_is_diagnosed() {
        let diags = check("<?mwl\nclass Foo { function a(): void { parent::bar(); } }\n");
        assert!(
            diags
                .iter()
                .any(|d| d.code == Some(code::E_UNDEFINED_CLASS))
        );
    }

    #[test]
    fn an_undeclared_class_const_is_diagnosed() {
        let diags =
            check("<?mwl\nclass Foo {}\nclass Bar { function a(): void { Foo::MISSING; } }\n");
        assert!(
            diags
                .iter()
                .any(|d| d.code == Some(code::E_UNDEFINED_MEMBER))
        );
    }

    #[test]
    fn an_undeclared_class_is_diagnosed() {
        let diags = check("<?mwl\nclass Foo { function a(): void { Missing::bar(); } }\n");
        assert!(
            diags
                .iter()
                .any(|d| d.code == Some(code::E_UNDEFINED_CLASS))
        );
    }

    #[test]
    fn an_enum_case_access_resolves() {
        let diags = check(
            "<?mwl\nenum Suit { Hearts, }\nclass Foo { function a(): void { Suit::Hearts; } }\n",
        );
        assert!(!diags.has_errors(), "{diags:?}");
    }

    #[test]
    fn a_dynamic_class_side_is_not_checked() {
        let diags = check("<?mwl\nclass Foo { function a(): void { $c = 'X'; $c::bar(); } }\n");
        assert!(!diags.has_errors(), "{diags:?}");
    }

    #[test]
    fn a_core_target_is_trusted() {
        let diags = check("<?mwl\nclass Foo { function a(): void { Core\\Str::upper('x'); } }\n");
        assert!(!diags.has_errors(), "{diags:?}");
    }

    #[test]
    fn a_static_property_access_is_checked() {
        let diags = check(
            "<?mwl\nclass Foo { public static int $count = 0; function a(): void { self::$count; } }\n",
        );
        assert!(!diags.has_errors(), "{diags:?}");
    }

    #[test]
    fn an_undeclared_static_property_is_diagnosed() {
        let diags = check("<?mwl\nclass Foo { function a(): void { self::$missing; } }\n");
        assert!(
            diags
                .iter()
                .any(|d| d.code == Some(code::E_UNDEFINED_MEMBER))
        );
    }

    #[test]
    fn a_this_property_access_resolves() {
        let diags =
            check("<?mwl\nclass Foo { public int $count; function a(): void { $this->count; } }\n");
        assert!(!diags.has_errors(), "{diags:?}");
    }

    #[test]
    fn a_this_property_access_resolves_through_an_ancestor() {
        let diags = check(
            "<?mwl\n\
             class Base { public int $count; }\n\
             class Sub extends Base { function a(): void { $this->count; } }\n",
        );
        assert!(!diags.has_errors(), "{diags:?}");
    }

    #[test]
    fn an_undeclared_this_property_is_diagnosed() {
        let diags = check("<?mwl\nclass Foo { function a(): void { $this->missing; } }\n");
        assert!(
            diags
                .iter()
                .any(|d| d.code == Some(code::E_UNDEFINED_PROPERTY))
        );
    }

    #[test]
    fn a_non_this_property_access_is_not_checked() {
        let diags = check(
            "<?mwl\nclass Foo { function a(): void { $other = new Foo(); $other->missing; } }\n",
        );
        assert!(!diags.has_errors(), "{diags:?}");
    }
}
