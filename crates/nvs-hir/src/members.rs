//! Member existence resolution for M2 item 2: every `Class::member`
//! reference — a static call, a class constant (an enum case counts as one),
//! `Class::class`, or a static property — must name something actually
//! declared on that class or reached transitively through the
//! [`ClassGraph`] built in [`crate::hierarchy`] (`extends`/`implements`).
//! `rule:classes/no-free-functions-or-constants`
//! gives a callable/constant no bare-name fallback to fall into instead, so
//! there is nothing else a `Class::member` reference could mean.
//!
//! That same ADR's other half is refused here too, and it is the reason
//! [`walk_class_side`] exists: a **bare name in value position** (`PHP_EOL`)
//! is `E0319` and a **bare name called** (`strlen($s)`) is `E0320`, since
//! ADR 0011 §§ 1 and 3 leave no global-function and no global-constant
//! storage row to name — and `self`/`static`/`parent` in value position is
//! `E0321`, each naming a class where a value is expected. These are
//! the *same* [`ExprKind`]s that mean a class on the left of a `::`, so every
//! class-side position skips the value-position walk rather than recursing
//! into it. Reported here, in resolution, rather than in `nvs-types`: the
//! mistake is that the name resolves against nothing, which needs no type,
//! and the `E04xx` band has room for it.
//!
//! Also carries M2 item 5, the property-access counterpart: `$this->name`
//! must name an instance property actually declared on the enclosing class
//! or reached the same way through [`ClassGraph`], per
//! `rule:classes/no-dynamic-properties`'s "no
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
//! Both of a doc comment's tags are checked here too, by [`check_doc`], and
//! for the same reason: each is a name that must resolve against something
//! declared. `rule:tooling/doc-comment-tags-are-see-and-example` keeps the tag
//! set at two by making each buy a check — `@see` resolves against the very
//! [`MemberTable`] a `Class::member` reference does (`E0323`), and `@example`
//! against a file on disk, which must exist (`E0324`) and must sit in a
//! directory the test corpus walks (`E0325`) so an example that stops
//! compiling fails the build. That second one is the only question this module
//! asks of the filesystem; it resolves a written path the way `require` does,
//! relative to the file that wrote it.
//!
//! `rule:tooling/strict-docs`'s lint rides beside that walk rather than growing
//! a second one: [`MemberResolver::check`] takes a `strict_docs` flag, `nvs
//! check --strict-docs` is the only caller that passes it true, and
//! [`check_members`] reports `E_DOC_MISSING` for a member with no `///` above
//! it. **A member is what the flag reports** — a method, a property, a class
//! constant — and public is the absence of `private` and `protected`, since
//! that is the visibility the language gives a member written without one. A
//! method that overrides or implements a documented method of an ancestor
//! inherits that doc comment and is not reported ([`check_documented`]). A
//! class, an interface, an enum, an enum case and a `type` alias are
//! deliberately not reported: the rule names a member, and widening it to every
//! declaration is a decision for the publisher that turns the flag on rather
//! than one taken here.
//!
//! **Four checks this module does not make**, each a narrower version of a gap
//! [`crate::hierarchy`] documents, and each one closed in `nvs-types` or
//! answered by a rule:
//! - `new Foo(...)`/`new self(...)`/etc. does not have its target checked
//!   here — instantiation resolution is a distinct concern from a
//!   callable/constant reference, and `nvs_types::expr::calls` resolves the
//!   target itself, holding a `Core` name to `nvs_stdlib::registry`'s roster
//!   rather than to its spelling.
//! - A member's visibility (`private`/`protected`) is not checked *here* —
//!   only whether it is declared anywhere in the chain. It is closed by
//!   `nvs-types`' `expr::members::check_member_visibility`, which sees the
//!   same `$this->name` access this module does and every other receiver
//!   besides, split across crates the same way the paragraph below splits a
//!   missing property; a method reaches it through
//!   `expr::members::check_method_visibility`, which `new Foo(...)` takes too
//!   so a `private` constructor is the singleton it was written to be. A
//!   promoted constructor parameter is read from like any other declaration:
//!   `nvs_types::signatures` records its visibility beside its type.
//! - A property access on any receiver other than `$this` — a typed local, a
//!   chained call result, `self::factory()`'s return, an explicit
//!   `new Foo()` — is never checked *here*, since this module has no static
//!   type to check it against. That is not left open: `nvs-types`'
//!   `expr::members::check_property_access` closes it once a static type exists,
//!   reporting `E_UNKNOWN_MEMBER` for the same shape of miss this module
//!   reports `E_UNDEFINED_PROPERTY` for on `$this` — split across crates by
//!   which one has the type to check against, not skipped by either.
//! - A property access whose name is not a literal identifier
//!   (`$obj->$name`, `$obj->{expr}`) is a runtime concern per `rule:classes/no-dynamic-properties`,
//!   not a compile-time one, and is silently skipped here regardless of
//!   receiver.

use std::path::{Component, Path};

use nvs_diagnostics::{Diagnostic, Diagnostics, SourceFile, Span, code};
use nvs_syntax::ast::{
    Arg, ArrayItem, Block, CallArgs, ClassMember, ClassMemberKind, DestructureElement,
    DestructureTarget, DocComment, DocTag, DocTagKind, Expr, ExprKind, FnBody, MemberName,
    Modifier, NamespaceDecl, Stmt, StmtKind, StringPart, TestOperand,
};
use rustc_hash::{FxHashMap, FxHashSet};

use crate::hierarchy::{ClassGraph, CoreRoster, resolve_ref};
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
    /// The names in [`Self::methods`] declared with a `///` above them. An
    /// undocumented override of one of these inherits its doc comment, which
    /// is what [`check_documented`] asks of it.
    pub documented_methods: FxHashSet<String>,
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
/// [`ClassMembers`]' sets is checked.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) enum MemberKind {
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
#[derive(Debug)]
pub struct MemberResolver<'a> {
    table: MemberTable,
    /// The `Core` types a fix on an undeclared class side may offer to
    /// import — [`crate::hierarchy::undeclared_name`]'s `core`. A resolver
    /// built with [`Self::new`] holds [`CoreRoster::Trusted`] and offers none.
    core: CoreRoster<'a>,
    /// What PHP's built-in functions became — [`Self::with_php`]. A resolver
    /// built any other way holds `None`.
    php: PhpFunctions,
}

/// What PHP's built-in functions became, as a front end holding the stdlib
/// tells this crate.
///
/// The lookup is given the name a program called as a free function, and
/// returns the clause that finishes "PHP's `name` …", which for `count` is
/// "is `Core\Arr::count` here". It returns `None` for a name PHP does not have
/// and for one the migration table gives no single spelling for. `nvs_stdlib::php_names::became`
/// is that lookup. It arrives from the caller for [`CoreRoster`]'s reason: the
/// table is compiled into the stdlib, which this crate does not depend on.
///
/// `None` is a caller with no stdlib in hand, and every `E0320` then carries
/// its general help.
pub type PhpFunctions = Option<fn(&str) -> Option<String>>;

impl Default for MemberResolver<'_> {
    /// A resolver holding only [`crate::errors`]' members.
    ///
    /// Seeded here rather than at each of the two call sites because a
    /// resolver that had *not* been seeded would silently diagnose
    /// `parent::constructor(…)` in a user exception subclass as an undeclared
    /// member — the shape every migrated PHP exception class has.
    fn default() -> Self {
        let mut table = MemberTable::new();
        let root = table.entry(QName::parse(crate::errors::ROOT));
        root.methods.insert("constructor".to_owned());
        for property in crate::errors::PROPERTIES {
            root.props.insert((*property).to_owned());
        }
        for (name, _) in crate::errors::TREE {
            table.entry(QName::parse(name));
        }
        Self {
            table,
            core: CoreRoster::Trusted,
            php: None,
        }
    }
}

impl<'a> MemberResolver<'a> {
    /// A resolver with nothing collected yet beyond [`crate::errors`]' own
    /// members — see [`Self::default`].
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// [`Self::new`], holding the roster of `Core` types an undeclared class
    /// side's fix may offer to import.
    #[must_use]
    pub fn with_core(core: CoreRoster<'a>) -> Self {
        Self {
            core,
            ..Self::default()
        }
    }

    /// This resolver, holding the lookup `E0320`'s help names a PHP
    /// function's replacement from.
    #[must_use]
    pub fn with_php(self, php: PhpFunctions) -> Self {
        Self { php, ..self }
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
                    let name = src.span_text(m.name).unwrap_or_default().to_owned();
                    // A promoted constructor parameter declares a property,
                    // so `$this->n` inside the class resolves to one — see
                    // `nvs_syntax::ast::Param::is_promoted`, which is the one
                    // home of which parameters those are.
                    if name == "constructor" {
                        for p in m.params.iter().filter(|p| p.is_promoted()) {
                            let text = src.span_text(p.name).unwrap_or_default();
                            entry
                                .props
                                .insert(text.strip_prefix('$').unwrap_or(text).to_owned());
                        }
                    }
                    if member.doc.is_some() {
                        entry.documented_methods.insert(name.clone());
                    }
                    entry.methods.insert(name);
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
    ///
    /// `strict_docs` is `rule:tooling/strict-docs`'s flag, true only under
    /// `nvs check --strict-docs`; the module doc says what it reports.
    pub fn check(
        &self,
        stmts: &[Stmt],
        src: &SourceFile,
        symbols: &SymbolTable,
        graph: &ClassGraph,
        strict_docs: bool,
        diags: &mut Diagnostics,
    ) {
        let mut env = Env {
            symbols,
            graph,
            table: &self.table,
            stmts,
            core: self.core.names(),
            php: self.php,
            refused_toplevel: refused_toplevel_names(stmts, src),
            fn_self: None,
            strict_docs,
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
/// needs, bundled so a recursive call threads one argument rather than one
/// per table.
struct Env<'a> {
    symbols: &'a SymbolTable,
    graph: &'a ClassGraph,
    table: &'a MemberTable,
    /// The file's top-level statements, which is where an undeclared class
    /// side's `use` line would go (`crate::imports::import_site`).
    stmts: &'a [Stmt],
    /// The `Core` types such a fix may offer — [`MemberResolver::with_core`].
    core: &'a [&'a str],
    /// What a PHP function's name became — [`MemberResolver::with_php`].
    php: PhpFunctions,
    /// Every name this file declared as a top-level `function` or `const` —
    /// both already refused by the parser (`E0215`/`E0216`). Calling or
    /// reading one is the same mistake seen from its use site, so `E0320` and
    /// `E0319` skip a name in here rather than reporting the cascade.
    refused_toplevel: FxHashSet<String>,
    /// The self-name of the `fn` literal whose body is being walked
    /// (`rule:types/closure-self-name`), or `None` outside one.
    ///
    /// Set to *this* closure's own name on entering its body and restored
    /// afterwards, so it is `None` again inside a nested literal that declares
    /// no name: § 3's name is visible in one body and not in a closure written
    /// inside it, which is the same reach `nvs_ir::lower::closure`'s `FN_SELF`
    /// receiver has. It exists here for one rule — `fact(...)` inside `fact`'s
    /// own body is not the free function `E0320` refuses.
    fn_self: Option<String>,
    /// `rule:tooling/strict-docs`'s flag: report a public member with no `///`
    /// above it. False everywhere but `nvs check --strict-docs`, which is what
    /// makes the language silent about documentation by default.
    strict_docs: bool,
    diags: &'a mut Diagnostics,
}

/// The names in [`Env::refused_toplevel`], collected before the walk so a use
/// site written *above* its declaration is suppressed too.
fn refused_toplevel_names(stmts: &[Stmt], src: &SourceFile) -> FxHashSet<String> {
    let mut out = FxHashSet::default();
    for stmt in stmts {
        match &stmt.kind {
            StmtKind::TopLevelFunction(method) => {
                out.insert(src.span_text(method.name).unwrap_or_default().to_owned());
            }
            StmtKind::TopLevelConst(consts) => {
                for c in consts {
                    out.insert(src.span_text(c.name).unwrap_or_default().to_owned());
                }
            }
            _ => {}
        }
    }
    out
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
                check_doc(decl.doc.as_ref(), src, &ctx, env);
                check_members(&decl.members, src, &ctx, env);
            }
            StmtKind::InterfaceDecl(decl) => {
                let qname = QName::join(&current_ns, name_text(src, &decl.name));
                let ctx = Ctx {
                    current_class: Some(&qname),
                    namespace: &current_ns,
                    imports: &current_imports,
                };
                check_doc(decl.doc.as_ref(), src, &ctx, env);
                check_members(&decl.members, src, &ctx, env);
            }
            StmtKind::EnumDecl(decl) => {
                let qname = QName::join(&current_ns, name_text(src, &decl.name));
                let ctx = Ctx {
                    current_class: Some(&qname),
                    namespace: &current_ns,
                    imports: &current_imports,
                };
                check_doc(decl.doc.as_ref(), src, &ctx, env);
                for case in &decl.cases {
                    check_doc(case.doc.as_ref(), src, &ctx, env);
                    if let Some(v) = &case.value {
                        walk_expr(v, src, &ctx, env);
                    }
                }
                check_members(&decl.members, src, &ctx, env);
            }
            StmtKind::TypeAliasDecl(decl) => {
                let ctx = Ctx {
                    current_class,
                    namespace: &current_ns,
                    imports: &current_imports,
                };
                check_doc(decl.doc.as_ref(), src, &ctx, env);
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
        check_doc(member.doc.as_ref(), src, ctx, env);
        check_documented(member, src, ctx, env);
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

/// One member's doc comment, or the absence `rule:tooling/strict-docs` reports.
/// Silent unless [`Env::strict_docs`] is set, which is `nvs check
/// --strict-docs` and nothing else.
///
/// **Public is the absence of `private` and `protected`**, since a member
/// written with no visibility at all is public and is exactly the member a
/// reader of the package will reach. `private(set)` is a visibility for writes
/// rather than for the declaration ([`Modifier::SetVisibility`]), so it never
/// hides a member from this. A recovery placeholder
/// ([`ClassMemberKind::Error`]) is skipped: the parser has already reported
/// whatever it stood in for, and a second diagnostic about its documentation
/// would be a cascade.
///
/// **A method inherits a doc comment** from a method of the same name on an
/// ancestor that has one: a parent class, an interface the class implements,
/// or any class or interface those extend or implement in turn. An override
/// or an implementation says what its ancestor already says, so it passes
/// without a `///` of its own. A property and a constant inherit nothing.
///
/// Reported at the member's **name** rather than at [`ClassMember::span`],
/// which reaches back over its attributes and modifiers: the repair is one line
/// written above the declaration, and a primary label covering three lines of
/// `#[...]` says nothing about where.
fn check_documented(member: &ClassMember, src: &SourceFile, ctx: &Ctx<'_>, env: &mut Env<'_>) {
    if !env.strict_docs || member.doc.is_some() {
        return;
    }
    let (modifiers, name) = match &member.kind {
        ClassMemberKind::Method(m) => (&m.modifiers, m.name),
        ClassMemberKind::Property(p) => (&p.modifiers, p.name),
        ClassMemberKind::Const(c) => (&c.modifiers, c.name),
        _ => return,
    };
    if modifiers
        .iter()
        .any(|m| matches!(m, Modifier::Private | Modifier::Protected))
    {
        return;
    }
    let written = src.span_text(name).unwrap_or_default();
    if matches!(member.kind, ClassMemberKind::Method(_))
        && ctx.current_class.is_some_and(|class| {
            documented_on_an_ancestor(class, written, env, &mut FxHashSet::default())
        })
    {
        return;
    }
    let owner = ctx
        .current_class
        .map_or_else(String::new, |class| format!("{class}::"));
    env.diags.report(
        Diagnostic::error(
            code::E_DOC_MISSING,
            format!("`{owner}{written}` is public and has no doc comment"),
        )
        .with_primary(name, "no `///` above this declaration")
        .with_help(
            "write a `///` line above it, or make the member `private` — \
             `--strict-docs` reports only what a reader of this package can reach",
        ),
    );
}

/// Whether something `qname` extends or implements, at any depth, declares the
/// method `name` with a doc comment. `qname` itself is not asked:
/// its own declaration is the one being checked. `seen` stops a cycle, which
/// [`crate::hierarchy`] has already reported.
fn documented_on_an_ancestor(
    qname: &QName,
    name: &str,
    env: &Env<'_>,
    seen: &mut FxHashSet<QName>,
) -> bool {
    let Some(links) = env.graph.get(qname) else {
        return false;
    };
    links
        .extends
        .iter()
        .chain(links.implements.iter())
        .any(|parent| {
            seen.insert(parent.clone())
                && (env
                    .table
                    .get(parent)
                    .is_some_and(|members| members.documented_methods.contains(name))
                    || documented_on_an_ancestor(parent, name, env, seen))
        })
}

/// Both tags of one declaration's doc comment, if it has one. This is the only
/// walk over a `DocTag` in the compiler: the parser has already spanned every
/// tag and refused every spelling but these two, so the checks that keep them
/// honest share one visit rather than each re-deriving where a `doc` hangs.
/// [`check_stmts`] reaches the file-scope declarations — a class, an interface,
/// an enum and its cases, a `type` alias — and [`check_members`] the members
/// inside a body, which between them is every declaration that carries one.
fn check_doc(doc: Option<&DocComment>, src: &SourceFile, ctx: &Ctx<'_>, env: &mut Env<'_>) {
    let Some(doc) = doc else {
        return;
    };
    for tag in &doc.tags {
        match tag.kind {
            DocTagKind::See => check_see(tag, src, ctx, env),
            DocTagKind::Example => check_example(tag, src, env),
        }
    }
}

/// One `@see`'s target, which
/// `rule:tooling/doc-comment-tags-are-see-and-example` requires to resolve.
/// The spelling is a class name, optionally followed by `::member`, and the
/// member half may carry a `$` sigil or a trailing `()` so a page may write a
/// property or a call the way a reader would say it. `self`, `static` and
/// `parent` name a class here exactly as they do in code, since a doc comment
/// sits inside the same class the code below it does.
///
/// A member is looked up against all four [`MemberKind`]s, because `@see` says
/// *what* is named and never which kind it is; `Class::name` resolving as any
/// one of them is the cross-reference the tag promised. A `Core` target is
/// trusted the same way [`member_declared`] trusts one anywhere else.
fn check_see(tag: &DocTag, src: &SourceFile, ctx: &Ctx<'_>, env: &mut Env<'_>) {
    let text = src.span_text(tag.argument).unwrap_or_default().trim();
    if text.is_empty() {
        env.diags.report(
            Diagnostic::error(code::E_DOC_SEE_UNRESOLVED, "`@see` names nothing")
                .with_primary(tag.span, "nothing to resolve")
                .with_help(
                    "name a class, member, enum case or constant — `@see Core\\Money::fromCents`",
                ),
        );
        return;
    }

    let (class_text, member) = match text.split_once("::") {
        Some((class_text, member)) => (class_text.trim(), Some(member.trim())),
        None => (text, None),
    };

    let class = match class_text {
        "self" | "static" => ctx.current_class.cloned(),
        "parent" => ctx
            .current_class
            .and_then(|current| env.graph.get(current))
            .and_then(|links| links.extends.first().cloned()),
        _ => {
            let resolved = resolve_ref(class_text, ctx.namespace, ctx.imports);
            (resolved.is_core() || env.symbols.contains(&resolved)).then_some(resolved)
        }
    };
    let Some(class) = class else {
        env.diags.report(
            Diagnostic::error(
                code::E_DOC_SEE_UNRESOLVED,
                format!("`@see` names `{class_text}`, which resolves to no declaration"),
            )
            .with_primary(tag.argument, "no such class"),
        );
        return;
    };

    let Some(member) = member else {
        return;
    };
    let member = member.trim_end_matches("()").trim_start_matches('$');
    if member.is_empty() {
        env.diags.report(
            Diagnostic::error(
                code::E_DOC_SEE_UNRESOLVED,
                format!("`@see` names no member of `{class}` after the `::`"),
            )
            .with_primary(tag.argument, "nothing to resolve"),
        );
        return;
    }
    let found = [
        MemberKind::Method,
        MemberKind::Const,
        MemberKind::StaticProp,
        MemberKind::Prop,
    ]
    .into_iter()
    .any(|kind| member_declared(&class, member, kind, env.table, env.graph));
    if !found {
        env.diags.report(
            Diagnostic::error(
                code::E_DOC_SEE_UNRESOLVED,
                format!("`{class}` has no member named `{member}` for `@see` to resolve"),
            )
            .with_primary(tag.argument, "no such member"),
        );
    }
}

/// The directories a Novis project's test corpus walks, and so the only ones
/// an `@example` may name a file inside. ADR 0137 § 2 names `examples/`; a
/// project's own cases live under `tests/`, which `nvs test` walks for the
/// same reason. The list is here rather than in configuration because the
/// guarantee the tag buys is that *something* compiles the file, and a
/// directory a project could name in a config file it also controls buys
/// nothing.
const WALKED_DIRECTORIES: [&str; 2] = ["examples", "tests"];

/// One `@example`'s target, which
/// `rule:tooling/doc-comment-tags-are-see-and-example` requires to exist and to
/// sit where the corpus walks it — an example nothing compiles is one that rots
/// while the member it documents moves on.
///
/// The path is relative to the file that wrote it, exactly as a `require`
/// path is (`rule:statements/require-is-the-only-inclusion-construct`), so
/// there is one answer in the language to "what is a written path relative
/// to". A file with no path of its own — a test fixture, an editor buffer that
/// has never been saved — resolves against the process's directory instead,
/// which is the only base such a file has.
///
/// The directory question is asked first, and answers on the written path
/// alone: a path outside every walked directory is refused for being there
/// rather than for a file that would not have helped it.
fn check_example(tag: &DocTag, src: &SourceFile, env: &mut Env<'_>) {
    let text = src.span_text(tag.argument).unwrap_or_default().trim();
    if text.is_empty() {
        env.diags.report(
            Diagnostic::error(code::E_DOC_EXAMPLE_NOT_FOUND, "`@example` names no file")
                .with_primary(tag.span, "nothing to find")
                .with_help("name a file the test corpus walks — `@example examples/charge.nvs`"),
        );
        return;
    }

    let written = Path::new(text);
    let walked = written.components().any(|component| {
        matches!(component, Component::Normal(name)
            if WALKED_DIRECTORIES.iter().any(|dir| name == *dir))
    });
    if !walked {
        env.diags.report(
            Diagnostic::error(
                code::E_DOC_EXAMPLE_NOT_WALKED,
                format!("`{text}` is in no directory the test corpus walks"),
            )
            .with_primary(tag.argument, "nothing compiles this file")
            .with_help(format!(
                "move the example under {}",
                WALKED_DIRECTORIES
                    .map(|dir| format!("`{dir}/`"))
                    .join(" or ")
            )),
        );
        return;
    }

    let base = src.path().and_then(Path::parent).unwrap_or(Path::new(""));
    nvs_footprint::exists(&base.join(written));
    if !base.join(written).is_file() {
        env.diags.report(
            Diagnostic::error(
                code::E_DOC_EXAMPLE_NOT_FOUND,
                format!("`{text}` names no file"),
            )
            .with_primary(tag.argument, "no such file")
            .with_help("the path is relative to the file that wrote it, as a `require` path is"),
        );
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
            // `rule:iteration/for-init-clause`: an init clause may be the same `LocalDecl` that
            // would otherwise sit on the line above the loop, so it walks as a statement.
            if let Some(decl) = init.decl() {
                s!(decl);
            }
            for x in init.exprs().iter().chain(cond).chain(step) {
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

/// The members whose argument at that index is
/// `rule:testing/interaction-after-the-fact`'s method reference — `Mailer::send`
/// written where every other reader of this syntax sees a class constant.
///
/// A roster of names rather than a question asked of the registry, because this
/// crate does not know one: `nvs_types::core_lib::method_ref_param` reads the
/// position back off the row that declared it — the parameter written
/// `nvs_stdlib::registry::CoreTy::MethodRef` — and is the home of *which*
/// positions there are. What this list buys is only that the reference is not
/// reported as an undefined constant before the checker ever sees it, so a
/// member missing from here is refused at the wrong span rather than admitted
/// somewhere it should not be.
const METHOD_REF_ARGS: &[(&str, &str, usize)] = &[
    (r"Core\Test", "assertCalled", 1),
    (r"Core\Test", "assertNeverCalled", 1),
];

/// [`walk_args`] for a static call, with the one argument [`METHOD_REF_ARGS`]
/// names left to `nvs_types` — it is a method reference there and an undefined
/// constant here, and this walk is what would otherwise report it first.
///
/// The class side of the reference is still walked, so an undeclared class in
/// `Mailer::send` is still this pass's `E0303`; only the member half is left
/// alone. A call that writes the argument by name or behind a `...` takes the
/// ordinary path, which is the same spelling `nvs_types` admits.
fn walk_args_admitting_method_ref(
    class: &Expr,
    method: &MemberName,
    args: &CallArgs,
    src: &SourceFile,
    ctx: &Ctx<'_>,
    env: &mut Env<'_>,
) {
    let CallArgs::List(list) = args else {
        return;
    };
    let reference = method_ref_index(class, method, src, ctx)
        .filter(|index| {
            list.iter()
                .take(index + 1)
                .all(|arg| arg.name.is_none() && !arg.spread)
        })
        .filter(|index| {
            matches!(
                list.get(*index).map(|arg| &arg.value.kind),
                Some(ExprKind::ClassConstAccess { .. })
            )
        });
    for (index, Arg { value, .. }) in list.iter().enumerate() {
        match (&value.kind, reference == Some(index)) {
            (ExprKind::ClassConstAccess { class: named, .. }, true) => {
                walk_class_side(named, src, ctx, env);
            }
            _ => walk_expr(value, src, ctx, env),
        }
    }
}

/// Which argument of this static call is a method reference, if any is.
fn method_ref_index(
    class: &Expr,
    method: &MemberName,
    src: &SourceFile,
    ctx: &Ctx<'_>,
) -> Option<usize> {
    let (ExprKind::ConstFetch(class_name), MemberName::Ident(name_span)) = (&class.kind, method)
    else {
        return None;
    };
    let owner = resolve_ref(name_text(src, class_name), ctx.namespace, ctx.imports).to_string();
    let member = src.span_text(*name_span).unwrap_or_default();
    METHOD_REF_ARGS
        .iter()
        .find(|(class, name, _)| *class == owner && *name == member)
        .map(|(_, _, index)| *index)
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
        ExprKind::TypeTest { expr, against } => {
            e!(expr);
            if let TestOperand::Value(operand) = against {
                e!(operand);
            }
        }
        // A bare `strlen($s)` names a storage row
        // `rule:classes/no-free-functions-or-constants` does not have, not a call on a
        // value: the callee is reported here rather than recursed into, so
        // that it names the *function* replacement instead of the constant
        // one `ExprKind::ConstFetch`'s own arm below would give it.
        ExprKind::Call { callee, args } => {
            match &callee.kind {
                ExprKind::ConstFetch(name) => {
                    let text = name_text(src, name);
                    // `rule:types/closure-self-name`: inside `fn fact(...) => … fact(…)`, the
                    // callee is this closure and not a free function. Only in
                    // callee position — the name resolves the way `self::`
                    // does, so it is not a value and a bare `fact` below is
                    // still `E0319`.
                    if env.fn_self.as_deref() == Some(text) {
                        walk_args(args, src, ctx, env);
                        return;
                    }
                    if !env.refused_toplevel.contains(text) {
                        // A name the migration table has one answer for is told
                        // that answer. Any other name gets the general help,
                        // whose `Core\Str::length` is an example of the shape.
                        let help = match env.php.and_then(|became| became(text)) {
                            Some(became) => format!(
                                "PHP's `{}` {became}. `rule:classes/no-free-functions-or-constants`: \
                                 every callable is a method, so there is no free function to \
                                 call, and `docs/spec/02-php-migration.md` has the row of each \
                                 PHP built-in, with its reason and any rewrite",
                                text.trim_start_matches('\\'),
                            ),
                            None => "`rule:classes/no-free-functions-or-constants`: every callable is a method, and the built-ins \
                                     live under the reserved `Core` namespace — \
                                     `Core\\Str::length($s)`, or `use Core\\Str;` and then \
                                     `Str::length($s)`. `docs/spec/02-php-migration.md` maps PHP's \
                                     own name to its `Core` member"
                                .to_owned(),
                        };
                        env.diags.report(
                            Diagnostic::error(
                                code::E_NO_FREE_FUNCTION,
                                format!("`{text}` is not a function that exists"),
                            )
                            .with_primary(callee.span, "no free function has this name")
                            .with_help(help),
                        );
                    }
                }
                _ => e!(callee),
            }
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
            ..
        } => {
            walk_class_side(class, src, ctx, env);
            walk_member_name(method, src, ctx, env);
            walk_args_admitting_method_ref(class, method, args, src, ctx, env);
            if let MemberName::Ident(name_span) = method {
                let name = src.span_text(*name_span).unwrap_or_default();
                // `rule:attributes/structural-retrieval`'s class target — `Foo::constructor(...)` — rests
                // on `rule:classes/definite-property-initialization`'s "every class has one, definitely", so a
                // *reference* to a constructor no class body writes names the
                // synthesized one and is not an undefined member.
                //
                // The exemption is exactly this spelling and no wider. A
                // written `Foo::constructor()` on such a class resolves to no
                // signature in `nvs_types` either, so it would reach `nvs-ir`
                // with no target recorded and panic there; `E0309` is what
                // keeps that a diagnostic about the program, and it is the one
                // code the mistake draws.
                if !(name == "constructor" && matches!(args, CallArgs::FirstClassCallable)) {
                    check_member_ref(class, name, MemberKind::Method, src, ctx, env);
                }
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
            walk_class_side(class, src, ctx, env);
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
            walk_class_side(class, src, ctx, env);
            check_member_ref(
                class,
                src.span_text(*name).unwrap_or_default(),
                MemberKind::Const,
                src,
                ctx,
                env,
            );
        }
        ExprKind::Catch { guarded, arms } => {
            e!(guarded);
            for arm in arms {
                e!(&arm.body);
            }
        }
        ExprKind::ClassNameConst { class } => walk_class_side(class, src, ctx, env),
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
            // `rule:types/closure-self-name`'s self-name covers this body and no other — a
            // default above is outside it, and a nested literal replaces it
            // rather than inheriting it. See `Env::fn_self`.
            let outer = std::mem::replace(
                &mut env.fn_self,
                fn_expr
                    .name
                    .map(|n| src.span_text(n).unwrap_or_default().to_owned()),
            );
            match &fn_expr.body {
                FnBody::Block(block) => walk_block(block, src, ctx, env),
                FnBody::Expr(body) => e!(body),
            }
            env.fn_self = outer;
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
        ExprKind::Await(inner) => e!(inner),
        ExprKind::Require { path } => e!(path),
        // The name-shaped expressions that only ever mean a class are
        // refused here, in value position, because every position where they
        // *do* mean a class goes through `walk_class_side` instead and never
        // reaches this match at all.
        ExprKind::ConstFetch(name) => {
            let text = name_text(src, name);
            if !env.refused_toplevel.contains(text) {
                // `html"…"` is the markup literal written with a string's
                // delimiter, one character from the form that compiles, and a
                // help about class constants reads as "there is no such
                // literal". The bare name followed by a quote is that guess
                // and nothing else, since no constant is ever followed by one.
                let quote_follows = src
                    .text()
                    .get(expr.span.end as usize..)
                    .is_some_and(|after| after.starts_with(['"', '\'']));
                let help = if text == "html" && quote_follows {
                    "the markup literal is written with backticks, `html`<p>{$name}</p>``: its \
                     text is a `Core\\Html\\Markup` and every `{$…}` hole in it is escaped"
                        .to_owned()
                } else {
                    "`rule:statements/storage-that-outlives-a-call`: a constant always belongs to a class, so there is no \
                     global one to fetch — write `Class::NAME`, and for a PHP built-in the \
                     `Core` member `docs/spec/02-php-migration.md` maps it to (`M_PI` is \
                     `Core\\Math::PI`)"
                        .to_owned()
                };
                env.diags.report(
                    Diagnostic::error(
                        code::E_NO_GLOBAL_CONSTANT,
                        format!("`{text}` is not a constant that exists"),
                    )
                    .with_primary(expr.span, "no global constant has this name")
                    .with_help(help),
                );
            }
        }
        ExprKind::SelfExpr | ExprKind::StaticExpr | ExprKind::ParentExpr => {
            let written = match &expr.kind {
                ExprKind::SelfExpr => "self",
                ExprKind::StaticExpr => "static",
                _ => "parent",
            };
            env.diags.report(
                Diagnostic::error(
                    code::E_CLASS_NAME_NOT_A_VALUE,
                    format!("`{written}` names a class, and a class is not a value"),
                )
                .with_primary(expr.span, "used where a value is expected")
                .with_help(format!(
                    "write `{written}::` and the member wanted — a method call, a constant or a \
                     static property. There is no class handle to pass around: `rule:classes/no-free-functions-or-constants` puts \
                     every reflective question on `Core\\Reflect` instead"
                )),
            );
        }
        _ => {}
    }
}

/// Walks the left-hand side of a `::` — a static call, a static property, a
/// class constant, `::class` — and every other position that names a *class*
/// rather than producing a value.
///
/// The four name-shaped [`ExprKind`]s — `self`, `static`, `parent` and a bare
/// name — mean a class here and nothing else, so they are skipped rather than
/// walked: [`walk_expr`]'s own arms for them report `E0319`/`E0321`, which are
/// about *value* position and would fire on every `Foo::bar()` in the program
/// if a class side recursed. A dynamic class side (`$name::foo()`, a
/// parenthesized expression) is an ordinary value and is walked; whether it is
/// an *allowed* class side is `nvs_types`' question, not this pass's.
fn walk_class_side(class: &Expr, src: &SourceFile, ctx: &Ctx<'_>, env: &mut Env<'_>) {
    if matches!(
        class.kind,
        ExprKind::SelfExpr | ExprKind::StaticExpr | ExprKind::ParentExpr | ExprKind::ConstFetch(_)
    ) {
        return;
    }
    walk_expr(class, src, ctx, env);
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
                env.diags.report(crate::hierarchy::undeclared_name(
                    crate::hierarchy::Undeclared {
                        qname: &resolved,
                        text,
                        span: class_name.span,
                        namespace: ctx.namespace,
                        stmts: env.stmts,
                        src,
                    },
                    env.symbols,
                    env.core,
                ));
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
    let mut diag = Diagnostic::error(
        code::E_UNDEFINED_MEMBER,
        format!("`{qname}` has no {} named `{name}`", kind.describe()),
    )
    .with_primary(class_expr.span, "referenced here");
    if let Some(nearest) = nearest_member(&qname, name, kind, env.table, env.graph) {
        diag = diag.with_help(format!("did you mean `{nearest}`?"));
    }
    env.diags.report(diag);
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

pub(crate) fn member_declared(
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

/// The member `name` was probably meant to be, or `None` when nothing
/// declared is close enough to be worth naming.
///
/// The candidates are exactly the names a reference on this class side could
/// have resolved to: the same kind of member, on `qname` or on an ancestor
/// [`member_declared`] would have reached. A member of an unrelated class is
/// never among them, because a suggestion the reader cannot write costs more
/// than the reference already did. Distance is Levenshtein distance in
/// characters, and ties go to the alphabetically first name so the same
/// source always produces the same help line.
fn nearest_member(
    qname: &QName,
    name: &str,
    kind: MemberKind,
    table: &MemberTable,
    graph: &ClassGraph,
) -> Option<String> {
    let mut candidates = Vec::new();
    let mut seen = FxHashSet::default();
    collect_candidates(qname, kind, table, graph, &mut seen, &mut candidates);
    let budget = suggestion_budget(name);
    candidates
        .into_iter()
        .filter_map(|candidate| {
            let distance = edit_distance(name, &candidate);
            (distance <= budget).then_some((distance, candidate))
        })
        .min()
        .map(|(_, candidate)| candidate)
}

/// How many edits a suggestion may sit from what was written: one for a short
/// name, two once the name is long enough for a transposition to cost that
/// much, and never more. A name of one character gets no budget at all, since
/// no edit of it is a typo rather than a different name.
///
/// Two is the ceiling because a third edit is where a prefix stops being a
/// slip and starts being a word: `disconnect` is three edits from `connect`,
/// and an agent that is handed `connect` will write it. A suggestion this
/// diagnostic declines to make costs one more read of the class; a wrong one
/// it makes confidently costs the compile that follows.
fn suggestion_budget(name: &str) -> usize {
    let len = name.chars().count();
    (len / 3).clamp(1, 2).min(len.saturating_sub(1))
}

/// Every `kind` member declared on `qname` or reached from it through
/// `extends`/`implements`, walked the way [`member_declared_rec`] walks it and
/// guarded against a cycle the same way.
fn collect_candidates(
    qname: &QName,
    kind: MemberKind,
    table: &MemberTable,
    graph: &ClassGraph,
    seen: &mut FxHashSet<QName>,
    out: &mut Vec<String>,
) {
    if !seen.insert(qname.clone()) {
        return;
    }
    if let Some(members) = table.get(qname) {
        let names = match kind {
            MemberKind::Method => &members.methods,
            MemberKind::Const => &members.consts,
            MemberKind::StaticProp => &members.static_props,
            MemberKind::Prop => &members.props,
        };
        out.extend(names.iter().cloned());
    }
    let Some(links) = graph.get(qname) else {
        return;
    };
    for parent in links.extends.iter().chain(links.implements.iter()) {
        collect_candidates(parent, kind, table, graph, seen, out);
    }
}

/// Levenshtein distance in characters, two rows wide: the number of single
/// character insertions, deletions and substitutions that turn `a` into `b`.
fn edit_distance(a: &str, b: &str) -> usize {
    let b: Vec<char> = b.chars().collect();
    let mut prev: Vec<usize> = (0..=b.len()).collect();
    let mut row = vec![0; b.len() + 1];
    for (i, from) in a.chars().enumerate() {
        row[0] = i + 1;
        for (j, &to) in b.iter().enumerate() {
            row[j + 1] = (prev[j] + usize::from(from != to))
                .min(prev[j + 1] + 1)
                .min(row[j] + 1);
        }
        std::mem::swap(&mut prev, &mut row);
    }
    prev[b.len()]
}

#[cfg(test)]
mod tests {
    use nvs_diagnostics::SourceMap;
    use nvs_syntax::parse_file;

    use super::*;
    use crate::resolve::resolve_file;

    fn check(src: &str) -> Diagnostics {
        let mut map = SourceMap::new();
        let file = map.add("t.nvs", src);
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
            false,
            &mut diags,
        );
        diags
    }

    #[test]
    fn a_self_call_to_an_own_method_resolves() {
        let diags =
            check("<?nvs\nclass Foo { function a(): void { self::b(); } function b(): void {} }\n");
        assert!(!diags.has_errors(), "{diags:?}");
    }

    #[test]
    fn a_self_call_to_an_undeclared_method_is_diagnosed() {
        let diags = check("<?nvs\nclass Foo { function a(): void { self::missing(); } }\n");
        assert!(
            diags
                .iter()
                .any(|d| d.code == Some(code::E_UNDEFINED_MEMBER))
        );
    }

    /// Every `E_UNDEFINED_MEMBER` a fixture reported. A reference reached from
    /// two walks is reported once per walk, so a suggestion is asserted over
    /// the whole set rather than off one item.
    fn undefined_member(diags: &Diagnostics) -> Vec<&Diagnostic> {
        let reported: Vec<_> = diags
            .iter()
            .filter(|d| d.code == Some(code::E_UNDEFINED_MEMBER))
            .collect();
        assert!(!reported.is_empty(), "{diags:?}");
        reported
    }

    #[test]
    fn an_unknown_member_one_edit_from_a_registered_one_suggests_it() {
        let diags = check(
            "<?nvs\nclass Foo { function a(): void { self::grett(); } function greet(): void {} }\n",
        );
        assert!(
            undefined_member(&diags).iter().all(|d| d
                .notes
                .iter()
                .any(|note| note == "help: did you mean `greet`?")),
            "{diags:?}"
        );
    }

    #[test]
    fn an_unknown_member_far_from_every_registered_one_suggests_nothing() {
        let diags = check(
            "<?nvs\nclass Foo { function a(): void { self::disconnect(); } function greet(): void {} }\n",
        );
        assert!(
            undefined_member(&diags).iter().all(|d| d.notes.is_empty()),
            "{diags:?}"
        );
    }

    #[test]
    fn a_suggestion_never_names_a_member_of_a_different_class() {
        let diags = check(
            "<?nvs\n\
             class Other { function greet(): void {} }\n\
             class Foo { function a(): void { self::grett(); } }\n",
        );
        assert!(
            undefined_member(&diags).iter().all(|d| d.notes.is_empty()),
            "{diags:?}"
        );
    }

    #[test]
    fn a_parent_call_to_an_inherited_method_resolves() {
        let diags = check(
            "<?nvs\n\
             class Base { function greet(): void {} }\n\
             class Sub extends Base { function a(): void { parent::greet(); } }\n",
        );
        assert!(!diags.has_errors(), "{diags:?}");
    }

    #[test]
    fn parent_with_no_parent_class_is_diagnosed() {
        let diags = check("<?nvs\nclass Foo { function a(): void { parent::bar(); } }\n");
        assert!(
            diags
                .iter()
                .any(|d| d.code == Some(code::E_UNDEFINED_CLASS))
        );
    }

    #[test]
    fn an_undeclared_class_const_is_diagnosed() {
        let diags =
            check("<?nvs\nclass Foo {}\nclass Bar { function a(): void { Foo::MISSING; } }\n");
        assert!(
            diags
                .iter()
                .any(|d| d.code == Some(code::E_UNDEFINED_MEMBER))
        );
    }

    #[test]
    fn an_undeclared_class_is_diagnosed() {
        let diags = check("<?nvs\nclass Foo { function a(): void { Missing::bar(); } }\n");
        assert!(
            diags
                .iter()
                .any(|d| d.code == Some(code::E_UNDEFINED_CLASS))
        );
    }

    #[test]
    fn an_enum_case_access_resolves() {
        let diags = check(
            "<?nvs\nenum Suit { Hearts, }\nclass Foo { function a(): void { Suit::Hearts; } }\n",
        );
        assert!(!diags.has_errors(), "{diags:?}");
    }

    #[test]
    fn a_dynamic_class_side_is_not_checked() {
        let diags = check("<?nvs\nclass Foo { function a(): void { $c = 'X'; $c::bar(); } }\n");
        assert!(!diags.has_errors(), "{diags:?}");
    }

    #[test]
    fn a_core_target_is_trusted() {
        let diags = check("<?nvs\nclass Foo { function a(): void { Core\\Str::upper('x'); } }\n");
        assert!(!diags.has_errors(), "{diags:?}");
    }

    #[test]
    fn a_static_property_access_is_checked() {
        let diags = check(
            "<?nvs\nclass Foo { public static int $count = 0; function a(): void { self::$count; } }\n",
        );
        assert!(!diags.has_errors(), "{diags:?}");
    }

    #[test]
    fn an_undeclared_static_property_is_diagnosed() {
        let diags = check("<?nvs\nclass Foo { function a(): void { self::$missing; } }\n");
        assert!(
            diags
                .iter()
                .any(|d| d.code == Some(code::E_UNDEFINED_MEMBER))
        );
    }

    #[test]
    fn a_this_property_access_resolves() {
        let diags =
            check("<?nvs\nclass Foo { public int $count; function a(): void { $this->count; } }\n");
        assert!(!diags.has_errors(), "{diags:?}");
    }

    #[test]
    fn a_this_property_access_resolves_through_an_ancestor() {
        let diags = check(
            "<?nvs\n\
             class Base { public int $count; }\n\
             class Sub extends Base { function a(): void { $this->count; } }\n",
        );
        assert!(!diags.has_errors(), "{diags:?}");
    }

    #[test]
    fn an_undeclared_this_property_is_diagnosed() {
        let diags = check("<?nvs\nclass Foo { function a(): void { $this->missing; } }\n");
        assert!(
            diags
                .iter()
                .any(|d| d.code == Some(code::E_UNDEFINED_PROPERTY))
        );
    }

    #[test]
    fn a_non_this_property_access_is_not_checked() {
        let diags = check(
            "<?nvs\nclass Foo { function a(): void { $other = new Foo(); $other->missing; } }\n",
        );
        assert!(!diags.has_errors(), "{diags:?}");
    }

    #[test]
    fn a_see_target_that_resolves_is_accepted() {
        let diags = check(
            "<?nvs\n\
             class Money { const int ZERO = 0; function cents(): int { return 1; } }\n\
             /// The price, in cents.\n\
             ///\n\
             /// @see Money::cents\n\
             class Price {\n\
             /// @see Money::ZERO\n\
             public int $amount;\n\
             /// @see self::amount\n\
             function amount(): int { return $this->amount; }\n\
             }\n",
        );
        assert!(!diags.has_errors(), "{diags:?}");
    }

    /// The fixture has no path of its own, so an `@example` resolves against
    /// the process's directory — which `cargo test` sets to this package's
    /// root, two levels under the repository the examples live in.
    #[test]
    fn an_example_in_a_walked_directory_that_exists_is_accepted() {
        let diags = check(
            "<?nvs\n\
             /// @example ../../examples/hello.nvs\n\
             class Price {}\n",
        );
        assert!(!diags.has_errors(), "{diags:?}");
    }

    #[test]
    fn an_example_naming_a_missing_file_is_refused() {
        let diags = check(
            "<?nvs\n\
             /// @example examples/nothing-is-here.nvs\n\
             class Price {}\n",
        );
        assert!(
            diags
                .iter()
                .any(|d| d.code == Some(code::E_DOC_EXAMPLE_NOT_FOUND)),
            "{diags:?}"
        );
    }

    #[test]
    fn an_example_outside_every_test_directory_is_refused() {
        let diags = check(
            "<?nvs\n\
             /// @example notes/charge.nvs\n\
             class Price {}\n",
        );
        assert!(
            diags
                .iter()
                .any(|d| d.code == Some(code::E_DOC_EXAMPLE_NOT_WALKED)),
            "{diags:?}"
        );
    }

    /// The help of every `E0320` in `src`, from a resolver told `php`.
    ///
    /// The check reports into a sink of its own: `resolve_file` runs a member
    /// check too, with a resolver told nothing, and its copy of each `E0320`
    /// is not the one under test.
    fn free_function_helps(src: &str, php: PhpFunctions) -> Vec<String> {
        let mut map = SourceMap::new();
        let file = map.add("t.nvs", src);
        let mut diags = Diagnostics::new();
        let stmts = parse_file(map.file(file), &mut diags);
        assert!(!diags.has_errors(), "fixture failed to parse: {diags:?}");
        let module = resolve_file(&stmts, map.file(file), &mut diags);
        let mut members = MemberResolver::new().with_php(php);
        members.collect_members(&stmts, map.file(file));
        let mut checked = Diagnostics::new();
        members.check(
            &stmts,
            map.file(file),
            &module.symbols,
            &module.graph,
            false,
            &mut checked,
        );
        checked
            .iter()
            .filter(|d| d.code == Some(code::E_NO_FREE_FUNCTION))
            .map(|d| d.notes.join("\n"))
            .collect()
    }

    /// A resolver told what PHP's functions became says so for the name that
    /// was called, and says nothing of it for a name the lookup does not know.
    /// A resolver told nothing gives every name the general help.
    #[test]
    fn a_free_function_call_is_told_what_the_lookup_says_its_name_became() {
        fn became(name: &str) -> Option<String> {
            (name == "count").then(|| "is `Core\\Arr::count` here".to_owned())
        }
        let counted = "<?nvs\necho count([1, 2]);\n";
        let tallied = "<?nvs\necho tally([1, 2]);\n";
        let general = "the built-ins live under the reserved `Core` namespace";

        let told = free_function_helps(counted, Some(became));
        assert!(!told.is_empty(), "the call is `E0320`");
        assert!(
            told.iter().all(|help| help
                .starts_with("help: PHP's `count` is `Core\\Arr::count` here. ")
                && !help.contains(general)),
            "{told:?}"
        );

        for (src, php) in [
            (tallied, Some(became as fn(&str) -> Option<String>)),
            (counted, None),
        ] {
            let helps = free_function_helps(src, php);
            assert!(!helps.is_empty(), "the call is `E0320`");
            assert!(
                helps
                    .iter()
                    .all(|help| help.contains(general) && !help.contains("PHP's `")),
                "{helps:?}"
            );
        }
    }

    #[test]
    fn a_see_target_that_names_no_member_is_refused() {
        let diags = check(
            "<?nvs\n\
             class Money {}\n\
             /// @see Money::cents\n\
             class Price {}\n",
        );
        assert!(
            diags
                .iter()
                .any(|d| d.code == Some(code::E_DOC_SEE_UNRESOLVED)),
            "{diags:?}"
        );
    }
}
