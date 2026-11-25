//! Member existence resolution: every `Class::member`
//! reference — a static call, a class constant (an enum case counts as one),
//! `Class::class`, or a static property — must name something actually
//! declared on that class or reached transitively through the
//! [`ClassGraph`] built in [`crate::hierarchy`] (`extends`/`implements`).
//! `rule:classes/no-free-functions-or-constants`
//! gives a callable/constant no bare-name fallback to fall into instead, so
//! there is nothing else a `Class::member` reference could mean.
//!
//! That same rule's other half is refused here too, and it is the reason
//! [`walk_class_side`] exists: a **bare name in value position** (`PHP_EOL`)
//! is `E0319` and a **bare name called** (`strlen($s)`) is `E0320`, since
//! `rule:classes/no-free-functions-or-constants` leaves no global-function and no global-constant
//! storage row to name — and `self`/`static`/`parent` in value position is
//! `E0321`, each naming a class where a value is expected. These are
//! the *same* [`ExprKind`]s that mean a class on the left of a `::`, so every
//! class-side position skips the value-position walk rather than recursing
//! into it. Reported here, in resolution, rather than in `nvs-types`: the
//! mistake is that the name resolves against nothing, which needs no type,
//! and the `E04xx` band has room for it.
//!
//! Also carries the property-access counterpart: `$this->name`
//! must name an instance property actually declared on the enclosing class
//! or reached the same way through [`ClassGraph`], per
//! `rule:classes/no-dynamic-properties`'s "no
//! `__get`/`__set` fallback." `$this` is the only property-access receiver
//! whose class is knowable without a type checker — see the checks this
//! module does not make, below, for every other receiver shape.
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
//! silently skipped, since this module reports only what it can be sure of.
//!
//! A doc comment's five tags are checked here too, by [`check_doc`], and for
//! the same reason: each names something that must exist on the declaration
//! or in the program. `rule:tooling/doc-comment-tags-are-closed-and-checked`
//! keeps the tag set closed by making each tag buy a check — `@see` resolves
//! against the very [`MemberTable`] a `Class::member` reference does
//! (`E0323`), or, written with a `/` or a `.`, against a file inside the
//! project and a heading of it when it is Markdown ([`check_see_file`]).
//! `@example` resolves against a file on disk, which must exist
//! (`E0324`) and must sit in a directory the test corpus walks (`E0325`) so an
//! example that stops compiling fails the build. Those two tags are the only
//! questions this module asks of the filesystem; each resolves a written path
//! the way `require` does, relative to the file that wrote it. What each
//! `@see`, `@example` and `@throws` resolved to is kept for the language server
//! ([`crate::doc_targets`]). `@param`, `@return`
//! and `@throws` are held to the method they sit on: a parameter it has, named
//! once (`E0335`, `E0331`), no type repeated from the signature (`E0332`), a
//! declaration that takes the tag (`E0333`), and an error class (`E0334`).
//! `W1030` offers a parameter its line where the comment documents the
//! others. Each of those carries the fix the editor applies.
//!
//! `rule:tooling/strict-docs`'s lint rides beside that walk rather than growing
//! a second one: [`MemberResolver::check`] takes a `strict_docs` flag, `nvs
//! check --strict-docs` is the only caller that passes it true, and
//! [`check_members`] reports `E_DOC_MISSING` for a member with no `///` above
//! it. **A member is what the flag reports** — a method, a property, a class
//! constant — and public is the absence of `private` and `protected`, since
//! that is the visibility the language gives a member written without one. A
//! member that overrides, implements or redeclares a documented member of the
//! same kind on an ancestor inherits that doc comment and is not reported
//! ([`check_documented`]). A
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
    MethodMember, Modifier, NamespaceDecl, NewTarget, Param, Stmt, StmtKind, StringPart,
    TestOperand, Type, TypeKind,
};
use rustc_hash::{FxHashMap, FxHashSet};

use crate::doc_targets::{DocTarget, Resolved};
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
    /// is what [`check_documented`] asks of it. The three `documented_*` sets
    /// below do the same for their kinds.
    pub documented_methods: FxHashSet<String>,
    /// Constant names, enum cases included.
    pub consts: FxHashSet<String>,
    /// The names in [`Self::consts`] declared with a `///` above them. An
    /// enum case is never among them: no case is a member `--strict-docs`
    /// reports, so none is inherited from.
    pub documented_consts: FxHashSet<String>,
    /// Static property names, `$` sigil stripped.
    pub static_props: FxHashSet<String>,
    /// The names in [`Self::static_props`] declared with a `///` above them.
    pub documented_static_props: FxHashSet<String>,
    /// Instance property names, `$` sigil stripped.
    pub props: FxHashSet<String>,
    /// The names in [`Self::props`] declared with a `///` above them. A
    /// promoted constructor parameter is never among them: a parameter carries
    /// no doc comment of its own.
    pub documented_props: FxHashSet<String>,
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
pub enum MemberKind {
    /// A method, static or not.
    Method,
    /// A class constant or an enum case.
    Const,
    /// A static property.
    StaticProp,
    /// An instance property.
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

    /// The names of this kind that `members` declares with a `///` above
    /// them, which an undocumented redeclaration inherits from.
    const fn documented(self, members: &ClassMembers) -> &FxHashSet<String> {
        match self {
            Self::Method => &members.documented_methods,
            Self::Const => &members.documented_consts,
            Self::StaticProp => &members.documented_static_props,
            Self::Prop => &members.documented_props,
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
    /// The classes a link names under an `autoload links` root
    /// ([`crate::link_targets`]). Empty until [`Self::link_to`].
    links: crate::link_targets::LinkTargets,
}

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
            links: crate::link_targets::LinkTargets::default(),
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

    /// Records the members of a class a loaded extension declares, which has
    /// no source for [`Self::collect_members`] to walk.
    pub fn declare_extension(&mut self, class: &crate::symbol::ExtensionClass) {
        let entry = self.table.entry(class.name.clone());
        entry.methods.extend(class.methods.iter().cloned());
        entry.consts.extend(class.consts.iter().cloned());
    }

    /// Gives [`Self::check`] the classes a link may name under an `autoload
    /// links` root. Their route arguments are checked against `targets`, and
    /// left to `nvs_types::links` for the route itself.
    pub fn link_to(&mut self, targets: &crate::link_targets::LinkTargets) {
        self.links = targets.clone();
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
        // The `use` map in force, which an `extend` block's target is read
        // through.
        let mut imports: FxHashMap<String, QName> = FxHashMap::default();

        for stmt in stmts {
            match &stmt.kind {
                StmtKind::NamespaceDecl(NamespaceDecl { name, body, .. }) => {
                    let new_ns = name
                        .as_ref()
                        .map_or_else(Vec::new, |n| qname_segments(src, n));
                    match body {
                        Some(block) => self.collect_in(&block.stmts, src, &new_ns),
                        None => {
                            current_ns = new_ns;
                            imports.clear();
                        }
                    }
                }
                StmtKind::UseDecl(use_decl) => {
                    let target = QName::parse(name_text(src, &use_decl.path));
                    imports.insert(target.short_name().to_owned(), target);
                }
                StmtKind::ClassDecl(decl) => {
                    let qname = QName::join(&current_ns, name_text(src, &decl.name));
                    self.collect_owned_enums(&decl.members, src, &qname);
                    self.collect_class_members(&decl.members, src, qname);
                }
                StmtKind::InterfaceDecl(decl) => {
                    let qname = QName::join(&current_ns, name_text(src, &decl.name));
                    self.collect_owned_enums(&decl.members, src, &qname);
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
                // An `extend` block's members, under the block's own name. Its
                // nested enums are the extended type's, `<Type>::<Enum>`.
                StmtKind::ExtendDecl(decl) => {
                    let qname = QName::join(&current_ns, name_text(src, &decl.name));
                    let target =
                        crate::resolve_ref(name_text(src, &decl.target), &current_ns, &imports);
                    self.collect_owned_enums(&decl.members, src, &target);
                    self.collect_class_members(&decl.members, src, qname);
                }
                _ => {}
            }
        }
    }

    /// Each enum a class or interface body declares is an entry of its own,
    /// under [`QName::member`]'s key, with its cases as its constants — the
    /// entry a file-scope enum gets (`rule:enums/class-scoped-enum`).
    fn collect_owned_enums(&mut self, members: &[ClassMember], src: &SourceFile, owner: &QName) {
        for member in members {
            if let ClassMemberKind::Enum(decl) = &member.kind {
                let qname = QName::member(owner, name_text(src, &decl.name));
                {
                    let entry = self.table.entry(qname.clone());
                    for case in &decl.cases {
                        entry.consts.insert(name_text(src, &case.name).to_owned());
                    }
                }
                self.collect_class_members(&decl.members, src, qname);
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
                    let name = src.span_text(c.name).unwrap_or_default().to_owned();
                    if member.doc.is_some() {
                        entry.documented_consts.insert(name.clone());
                    }
                    entry.consts.insert(name);
                }
                ClassMemberKind::Property(p) => {
                    let text = src.span_text(p.name).unwrap_or_default();
                    let name = text.strip_prefix('$').unwrap_or(text).to_owned();
                    let (names, documented) = if p.modifiers.contains(&Modifier::Static) {
                        (&mut entry.static_props, &mut entry.documented_static_props)
                    } else {
                        (&mut entry.props, &mut entry.documented_props)
                    };
                    if member.doc.is_some() {
                        documented.insert(name.clone());
                    }
                    names.insert(name);
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
        self.check_recording(
            stmts,
            src,
            (symbols, graph),
            strict_docs,
            (
                &mut crate::uses::Uses::off(),
                &mut crate::doc_targets::DocTargets::off(),
            ),
            diags,
        );
    }

    /// [`Self::check`], recording into `uses` each class, member and doc
    /// tag target the file names, and each one it failed to find
    /// (`crate::uses`), and into `targets` what each doc tag that names
    /// something resolved to (`crate::doc_targets`).
    pub fn check_recording(
        &self,
        stmts: &[Stmt],
        src: &SourceFile,
        (symbols, graph): (&SymbolTable, &ClassGraph),
        strict_docs: bool,
        (uses, targets): (&mut crate::uses::Uses, &mut crate::doc_targets::DocTargets),
        diags: &mut Diagnostics,
    ) {
        let mut env = Env {
            symbols,
            graph,
            table: &self.table,
            stmts,
            core: self.core.names(),
            refused_toplevel: refused_toplevel_names(stmts, src),
            fn_self: None,
            strict_docs,
            in_block: false,
            links: &self.links,
            uses,
            targets,
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
    /// Every name this file declared as a top-level `function` or `const` —
    /// both already refused by the parser (`E0215`/`E0216`). Calling or
    /// reading one is the same mistake seen from its use site, so `E0320` and
    /// `E0319` skip a name in here rather than reporting the cascade.
    refused_toplevel: FxHashSet<String>,
    /// [`MemberResolver::link_to`]'s classes.
    links: &'a crate::link_targets::LinkTargets,
    /// The self-name of the `fn` literal whose body is being walked
    /// (`rule:types/anonymous-function-self-name`), or `None` outside one.
    ///
    /// Set to *this* anonymous function's own name on entering its body and
    /// restored afterwards, so it is `None` again inside a nested one that
    /// declares no name: the self-name is visible in one body and not in an
    /// anonymous function written inside it, which is the same reach
    /// `nvs_ir::lower::anon_fn`'s `FN_SELF`
    /// receiver has. It exists here for one rule — `fact(...)` inside `fact`'s
    /// own body is not the free function `E0320` refuses.
    fn_self: Option<String>,
    /// `rule:tooling/strict-docs`'s flag: report a public member with no `///`
    /// above it. False everywhere but `nvs check --strict-docs`, which is what
    /// makes the language silent about documentation by default.
    strict_docs: bool,
    /// Whether the walk is inside an `extend` block's body, where `static::`
    /// and `parent::` are refused: the block's `self` is the extended type,
    /// and an added static method has no called class to bind.
    in_block: bool,
    /// What the file found and failed to find (`crate::uses`).
    uses: &'a mut crate::uses::Uses,
    /// What each doc tag that names something resolved to
    /// (`crate::doc_targets`).
    targets: &'a mut crate::doc_targets::DocTargets,
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
                check_doc(
                    decl.doc.as_ref(),
                    DocOwner::Other("a class"),
                    src,
                    &ctx,
                    env,
                );
                check_members(&decl.members, src, &ctx, env);
            }
            StmtKind::InterfaceDecl(decl) => {
                let qname = QName::join(&current_ns, name_text(src, &decl.name));
                let ctx = Ctx {
                    current_class: Some(&qname),
                    namespace: &current_ns,
                    imports: &current_imports,
                };
                check_doc(
                    decl.doc.as_ref(),
                    DocOwner::Other("an interface"),
                    src,
                    &ctx,
                    env,
                );
                check_members(&decl.members, src, &ctx, env);
            }
            StmtKind::EnumDecl(decl) => {
                let qname = QName::join(&current_ns, name_text(src, &decl.name));
                let ctx = Ctx {
                    current_class: Some(&qname),
                    namespace: &current_ns,
                    imports: &current_imports,
                };
                check_doc(
                    decl.doc.as_ref(),
                    DocOwner::Other("an enum"),
                    src,
                    &ctx,
                    env,
                );
                for case in &decl.cases {
                    check_doc(
                        case.doc.as_ref(),
                        DocOwner::Other("an enum case"),
                        src,
                        &ctx,
                        env,
                    );
                    if let Some(v) = &case.value {
                        walk_expr(v, src, &ctx, env);
                    }
                }
                check_members(&decl.members, src, &ctx, env);
            }
            // An `extend` block's body sees the extended type as `self` and
            // `$this`, and its own members without a `use`.
            StmtKind::ExtendDecl(decl) => {
                let block = QName::join(&current_ns, name_text(src, &decl.name));
                let Some(target) = env.graph.block_target(&block).cloned() else {
                    continue;
                };
                let mut imports = current_imports.clone();
                imports.insert(block.short_name().to_owned(), block.clone());
                let ctx = Ctx {
                    current_class: Some(&target),
                    namespace: &current_ns,
                    imports: &imports,
                };
                check_doc(
                    decl.doc.as_ref(),
                    DocOwner::Other("an `extend` block"),
                    src,
                    &ctx,
                    env,
                );
                let outer = std::mem::replace(&mut env.in_block, true);
                check_members(&decl.members, src, &ctx, env);
                env.in_block = outer;
            }
            StmtKind::TypeAliasDecl(decl) => {
                let ctx = Ctx {
                    current_class,
                    namespace: &current_ns,
                    imports: &current_imports,
                };
                check_doc(
                    decl.doc.as_ref(),
                    DocOwner::Other("a type alias"),
                    src,
                    &ctx,
                    env,
                );
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
        check_doc(
            member.doc.as_ref(),
            DocOwner::of(&member.kind),
            src,
            ctx,
            env,
        );
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
            ClassMemberKind::Enum(decl) => {
                for case in &decl.cases {
                    check_doc(
                        case.doc.as_ref(),
                        DocOwner::Other("an enum case"),
                        src,
                        ctx,
                        env,
                    );
                    if let Some(v) = &case.value {
                        walk_expr(v, src, ctx, env);
                    }
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
/// **A member inherits a doc comment** from a member of the same kind and name
/// on an ancestor that has one: a parent class, an interface the class
/// implements, or any class or interface those extend or implement in turn. An
/// override, an implementation or a redeclared constant or property means what
/// its ancestor's declaration means, so it passes without a `///` of its own.
/// The kind is the table's: a method, a constant, a static property and an
/// instance property each look only in their own `documented_*` set, since a
/// static and an instance property of one name do not override each other.
///
/// Reported at the member's **name** rather than at [`ClassMember::span`],
/// which reaches back over its attributes and modifiers: the repair is one line
/// written above the declaration, and a primary label covering three lines of
/// `#[...]` says nothing about where.
fn check_documented(member: &ClassMember, src: &SourceFile, ctx: &Ctx<'_>, env: &mut Env<'_>) {
    if !env.strict_docs || member.doc.is_some() {
        return;
    }
    let (modifiers, name, kind) = match &member.kind {
        ClassMemberKind::Method(m) => (&m.modifiers, m.name, MemberKind::Method),
        ClassMemberKind::Property(p) if p.modifiers.contains(&Modifier::Static) => {
            (&p.modifiers, p.name, MemberKind::StaticProp)
        }
        ClassMemberKind::Property(p) => (&p.modifiers, p.name, MemberKind::Prop),
        ClassMemberKind::Const(c) => (&c.modifiers, c.name, MemberKind::Const),
        _ => return,
    };
    if modifiers
        .iter()
        .any(|m| matches!(m, Modifier::Private | Modifier::Protected))
    {
        return;
    }
    let written = src.span_text(name).unwrap_or_default();
    let bare = written.strip_prefix('$').unwrap_or(written);
    if ctx.current_class.is_some_and(|class| {
        documented_on_an_ancestor(class, bare, kind, env, &mut FxHashSet::default())
    }) {
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

/// Whether something `qname` extends or implements, at any depth, declares a
/// member of `kind` called `name` with a doc comment. `qname` itself is not
/// asked: its own declaration is the one being checked. `seen` stops a cycle,
/// which [`crate::hierarchy`] has already reported.
fn documented_on_an_ancestor(
    qname: &QName,
    name: &str,
    kind: MemberKind,
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
                    .is_some_and(|members| kind.documented(members).contains(name))
                    || documented_on_an_ancestor(parent, name, kind, env, seen))
        })
}

/// The declaration a doc comment is attached to, as far as its tags need to
/// know. `@param`, `@return` and `@throws` go on a method alone, since a
/// program declares no free function and an anonymous function carries no
/// doc comment.
#[derive(Clone, Copy)]
enum DocOwner<'a> {
    /// A method, a constructor and an abstract or interface method included.
    Method(&'a MethodMember),
    /// Any other declaration, by the words a diagnostic names it with.
    Other(&'static str),
}

impl<'a> DocOwner<'a> {
    fn of(kind: &'a ClassMemberKind) -> Self {
        match kind {
            ClassMemberKind::Method(m) => Self::Method(m),
            ClassMemberKind::Property(_) => Self::Other("a property"),
            ClassMemberKind::Const(_) => Self::Other("a constant"),
            ClassMemberKind::TypeAlias(_) => Self::Other("a type alias"),
            ClassMemberKind::Enum(_) => Self::Other("an enum"),
            _ => Self::Other("this declaration"),
        }
    }
}

/// What one walk over a doc comment's tags has seen so far, so a second
/// line about the same thing is reported as a repeat.
#[derive(Default)]
struct DocSeen {
    params: FxHashSet<String>,
    returned: bool,
    thrown: FxHashSet<QName>,
}

/// Every tag of one declaration's doc comment, if it has one. This is the only
/// walk over a `DocTag` in the compiler: the parser has already spanned every
/// tag and refused every spelling outside the five, so the checks that keep
/// them honest share one visit rather than each re-deriving where a `doc`
/// hangs. [`check_stmts`] reaches the file-scope declarations — a class, an
/// interface, an enum and its cases, a `type` alias — and [`check_members`]
/// the members inside a body, which between them is every declaration that
/// carries one.
///
/// Every diagnostic about `@param`, `@return` and `@throws` carries its
/// `Suggestion`, so the editor's quick fix is a translation
/// (`rule:ide/a-code-action-ships-only-a-fix-a-diagnostic-already-knows`).
fn check_doc(
    doc: Option<&DocComment>,
    owner: DocOwner<'_>,
    src: &SourceFile,
    ctx: &Ctx<'_>,
    env: &mut Env<'_>,
) {
    let Some(doc) = doc else {
        return;
    };
    let mut seen = DocSeen::default();
    for tag in &doc.tags {
        match tag.kind {
            DocTagKind::See => check_see(tag, src, ctx, env),
            DocTagKind::Example => check_example(tag, src, env),
            DocTagKind::Param => check_param(tag, doc, owner, &mut seen, src, env),
            DocTagKind::Return => check_return(tag, owner, &mut seen, src, env),
            DocTagKind::Throws => check_throws(tag, owner, &mut seen, src, ctx, env),
        }
    }
    if let DocOwner::Method(method) = owner {
        check_missing_params(doc, method, src, env);
    }
}

/// The whole of a tag's lines, its continuation lines and their line breaks
/// included: the span a fix deletes.
fn tag_lines(tag: &DocTag, src: &SourceFile) -> Span {
    let text = src.text();
    let start = line_start_at(text, tag.span.start as usize);
    let end = line_end_after(text, tag.span.end.max(tag.text.end) as usize);
    span_of(tag.span, start, end)
}

/// The byte offset of the start of the line `pos` is on.
fn line_start_at(text: &str, pos: usize) -> usize {
    text.get(..pos)
        .and_then(|before| before.rfind('\n'))
        .map_or(0, |at| at + 1)
}

/// The byte offset just past the line break that ends the line `pos` is on,
/// or the end of the text where that line has none.
fn line_end_after(text: &str, pos: usize) -> usize {
    text.get(pos..)
        .and_then(|after| after.find('\n'))
        .map_or(text.len(), |at| pos + at + 1)
}

/// A span in `like`'s file from `start` to `end`.
fn span_of(like: Span, start: usize, end: usize) -> Span {
    Span::new(
        like.file,
        u32::try_from(start).unwrap_or(u32::MAX),
        u32::try_from(end).unwrap_or(u32::MAX),
    )
}

/// The error for a tag on a declaration that does not take it, with the fix
/// that deletes its lines.
fn report_misplaced(tag: &DocTag, what: &str, src: &SourceFile, env: &mut Env<'_>) {
    let name = tag.kind.name();
    env.diags.report(
        Diagnostic::error(
            code::E_DOC_TAG_MISPLACED,
            format!("`@{name}` does not go on {what}"),
        )
        .with_primary(tag.span, format!("{what} takes no `@{name}`"))
        .with_fix(
            tag_lines(tag, src),
            "",
            format!("delete the `@{name}` line"),
        ),
    );
}

/// The parameters of `method` that no `@param` of `doc` names, in signature
/// order.
fn undocumented<'m>(
    doc: &DocComment,
    method: &'m MethodMember,
    src: &SourceFile,
) -> Vec<&'m Param> {
    let named: FxHashSet<&str> = doc
        .tags
        .iter()
        .filter(|tag| tag.kind == DocTagKind::Param)
        .filter_map(|tag| tag.target.and_then(|target| src.span_text(target)))
        .collect();
    method
        .params
        .iter()
        .filter(|param| {
            src.span_text(param.name)
                .is_some_and(|name| !named.contains(name))
        })
        .collect()
}

/// One `@param $name`: it goes on a method, names one of its parameters, names
/// it once, and its sentence does not start with the parameter's type.
fn check_param(
    tag: &DocTag,
    doc: &DocComment,
    owner: DocOwner<'_>,
    seen: &mut DocSeen,
    src: &SourceFile,
    env: &mut Env<'_>,
) {
    let method = match owner {
        DocOwner::Method(method) => method,
        DocOwner::Other(what) => return report_misplaced(tag, what, src, env),
    };
    // An `@param` with no `$name` is the parser's `E0138` already.
    let Some(target) = tag.target else {
        return;
    };
    let name = src.span_text(target).unwrap_or_default();
    let Some(param) = method
        .params
        .iter()
        .find(|param| src.span_text(param.name) == Some(name))
    else {
        let diag = Diagnostic::error(
            code::E_DOC_PARAM_UNKNOWN,
            format!("`@param` names `{name}`, which is not a parameter here"),
        )
        .with_primary(target, "no such parameter");
        let missing = undocumented(doc, method, src);
        let diag = match missing.as_slice() {
            [only] => {
                let only = src.span_text(only.name).unwrap_or_default();
                diag.with_fix(target, only, "rename the tag to the undocumented parameter")
            }
            _ => diag.with_fix(tag_lines(tag, src), "", "delete the `@param` line"),
        };
        env.diags.report(diag);
        return;
    };
    if !seen.params.insert(name.to_owned()) {
        env.diags.report(
            Diagnostic::error(
                code::E_DOC_TAG_REPEATED,
                format!("`{name}` is documented twice"),
            )
            .with_primary(tag.span, "a second `@param` for the same parameter")
            .with_fix(tag_lines(tag, src), "", "delete this `@param` line"),
        );
        return;
    }
    if let Some(ty) = &param.ty {
        check_repeated_type(tag, ty, src, env);
    }
}

/// One `@return`: it goes on a method that returns a value, once, and its
/// sentence does not start with the return type.
fn check_return(
    tag: &DocTag,
    owner: DocOwner<'_>,
    seen: &mut DocSeen,
    src: &SourceFile,
    env: &mut Env<'_>,
) {
    let method = match owner {
        DocOwner::Method(method) => method,
        DocOwner::Other(what) => return report_misplaced(tag, what, src, env),
    };
    if src.span_text(method.name) == Some("constructor") {
        return report_misplaced(tag, "a constructor", src, env);
    }
    let returns = method
        .return_type
        .as_ref()
        .and_then(|ty| src.span_text(ty.span))
        .map(str::trim);
    if let Some(nothing @ ("void" | "never")) = returns {
        return report_misplaced(tag, &format!("a method that returns `{nothing}`"), src, env);
    }
    if std::mem::replace(&mut seen.returned, true) {
        env.diags.report(
            Diagnostic::error(code::E_DOC_TAG_REPEATED, "`@return` is written twice")
                .with_primary(tag.span, "a second `@return`")
                .with_fix(tag_lines(tag, src), "", "delete this `@return` line"),
        );
        return;
    }
    if let Some(ty) = &method.return_type {
        check_repeated_type(tag, ty, src, env);
    }
}

/// One `@throws Name`: it goes on a method, `Name` resolves like a type name
/// in this file to a class that is a `Throwable`, and names each class once.
/// The body is not checked against it: Novis has no checked exceptions, and
/// the tag says what the author intends.
///
/// A global error class such as `RuntimeError` is found from inside a
/// namespace too, as it is everywhere else a type is written. A `Core` class
/// outside the error tree has no links here to walk, so it is trusted, as
/// [`check_see`] trusts one.
fn check_throws(
    tag: &DocTag,
    owner: DocOwner<'_>,
    seen: &mut DocSeen,
    src: &SourceFile,
    ctx: &Ctx<'_>,
    env: &mut Env<'_>,
) {
    if let DocOwner::Other(what) = owner {
        return report_misplaced(tag, what, src, env);
    }
    // An `@throws` with no name is the parser's `E0139` already.
    let Some(target) = tag.target else {
        return;
    };
    let written = src.span_text(target).unwrap_or_default();
    let resolved = resolve_ref(written, ctx.namespace, ctx.imports);
    let global = QName::parse(written);
    let class = if env.graph.get(&resolved).is_some() || env.symbols.contains(&resolved) {
        Some(resolved)
    } else if global.is_reserved_global_class() {
        Some(global)
    } else if resolved.is_core() {
        Some(resolved)
    } else {
        env.uses.missed_class(&resolved);
        None
    };
    if let Some(class) = &class {
        if env.symbols.contains(class) {
            env.uses.class(class);
        }
        env.targets.push(DocTarget {
            span: target,
            class_span: None,
            resolved: Resolved::Class(class.clone()),
        });
    }
    let throwable = QName::parse(crate::errors::ROOT);
    let is_throwable = class.as_ref().is_some_and(|class| {
        env.graph.get(class).is_none() && class.is_core()
            || crate::hierarchy::implements_interface(class, &throwable, env.graph)
    });
    let Some(class) = class.filter(|_| is_throwable) else {
        env.diags.report(
            Diagnostic::error(
                code::E_DOC_THROWS_NOT_THROWABLE,
                format!("`@throws` names `{written}`, which is not an error class"),
            )
            .with_primary(target, "not a class that extends `Throwable`")
            .with_fix(tag_lines(tag, src), "", "delete the `@throws` line"),
        );
        return;
    };
    if !seen.thrown.insert(class.clone()) {
        env.diags.report(
            Diagnostic::error(
                code::E_DOC_TAG_REPEATED,
                format!("`{class}` is documented twice"),
            )
            .with_primary(tag.span, "a second `@throws` for the same class")
            .with_fix(tag_lines(tag, src), "", "delete this `@throws` line"),
        );
    }
}

/// The written forms a sentence's first word must not be: the whole type, and
/// each member of a union or a nullable type. `null` is never one, because
/// "null when the card is declined" is a normal sentence.
fn type_words<'s>(ty: &Type, src: &'s SourceFile, words: &mut Vec<&'s str>) {
    if let Some(text) = src.span_text(ty.span) {
        words.push(text.trim());
    }
    match &ty.kind {
        TypeKind::Nullable(inner) | TypeKind::Paren(inner) => type_words(inner, src, words),
        TypeKind::Union(members) => {
            for member in members {
                type_words(member, src, words);
            }
        }
        _ => {}
    }
}

/// The error for a sentence that starts with the declared type, with the fix
/// that deletes the word and the space after it. A word that only looks like
/// some other type is never reported: the check compares with `ty` alone.
fn check_repeated_type(tag: &DocTag, ty: &Type, src: &SourceFile, env: &mut Env<'_>) {
    let Some(text) = src.span_text(tag.text) else {
        return;
    };
    let word_len = text.find(char::is_whitespace).unwrap_or(text.len());
    let written = &text[..word_len];
    let word = written
        .trim_end_matches([',', ':', ';', '.'])
        .trim_matches('`');
    if word.is_empty() || word == "null" {
        return;
    }
    let mut words = Vec::new();
    type_words(ty, src, &mut words);
    if !words.contains(&word) {
        return;
    }
    let rest = &text[word_len..];
    let space = rest.len() - rest.trim_start_matches([' ', '\t']).len();
    let start = tag.text.start as usize;
    let deleted = span_of(tag.text, start, start + word_len + space);
    env.diags.report(
        Diagnostic::error(
            code::E_DOC_TAG_REPEATS_TYPE,
            format!("`@{}` repeats the type `{word}`", tag.kind.name()),
        )
        .with_primary(
            span_of(tag.text, start, start + word_len),
            "the type is in the signature",
        )
        .with_fix(deleted, "", format!("delete `{written}`")),
    );
}

/// The warning for each parameter with no `@param`, raised only where the same
/// comment already has an `@param`: nothing requires per-parameter
/// documentation (`rule:tooling/strict-docs`), but an author who documented
/// the others is offered the line a new parameter needs. The fix inserts
/// `/// @param $name ` after the `@param` of the nearest parameter before it
/// in signature order, or before the first `@param` line when none is. It
/// writes no sentence, so the new line shows `E0139` until the author does.
fn check_missing_params(
    doc: &DocComment,
    method: &MethodMember,
    src: &SourceFile,
    env: &mut Env<'_>,
) {
    let params: Vec<&DocTag> = doc
        .tags
        .iter()
        .filter(|tag| tag.kind == DocTagKind::Param && tag.target.is_some())
        .collect();
    let Some(first) = params.first() else {
        return;
    };
    let text = src.text();
    let tag_of = |name: &str| {
        params
            .iter()
            .find(|tag| tag.target.and_then(|target| src.span_text(target)) == Some(name))
    };
    // An `@param` that names no parameter, beside exactly one undocumented
    // parameter, is `E0335` with a rename to that parameter. The warning would
    // report the same mistake a second time, so it is not raised.
    let stale = params.iter().any(|tag| {
        let named = tag.target.and_then(|target| src.span_text(target));
        !method
            .params
            .iter()
            .any(|param| src.span_text(param.name) == named)
    });
    if stale && undocumented(doc, method, src).len() == 1 {
        return;
    }
    for (index, param) in method.params.iter().enumerate() {
        let Some(name) = src.span_text(param.name) else {
            continue;
        };
        if tag_of(name).is_some() {
            continue;
        }
        let before = method.params[..index]
            .iter()
            .rev()
            .find_map(|earlier| src.span_text(earlier.name).and_then(tag_of));
        let at = match before {
            Some(tag) => tag_lines(tag, src).end as usize,
            None => line_start_at(text, first.span.start as usize),
        };
        let anchor = line_start_at(text, first.span.start as usize);
        let indent = &text[anchor..first.span.start as usize];
        let indent = &indent[..indent.find(nvs_syntax::DOC_MARKER).unwrap_or(0)];
        let first_end = line_end_after(text, first.span.start as usize);
        let newline = if text[..first_end].ends_with("\r\n") {
            "\r\n"
        } else {
            "\n"
        };
        env.diags.report(
            Diagnostic::warning(
                code::W_DOC_PARAM_MISSING,
                format!("`{name}` has no `@param` line"),
            )
            .with_primary(param.name, "the other parameters have one")
            .with_fix(
                span_of(param.name, at, at),
                format!("{indent}/// @param {name} {newline}"),
                format!("add an `@param {name}` line"),
            ),
        );
    }
}

/// One `@see`'s target, which
/// `rule:tooling/doc-comment-tags-are-closed-and-checked` requires to resolve.
/// [`nvs_syntax::doc_target::DocTarget::of`] says which of two shapes it has.
/// A target with a `/` or a `.` is a file, and [`check_see_file`] checks it.
///
/// Every other target is a class name, optionally followed by `::member`, and
/// the member half may carry a `$` sigil or a trailing `()` so a page may write
/// a property or a call the way a reader would say it. `self`, `static` and
/// `parent` name a class here exactly as they do in code, since a doc comment
/// sits inside the same class the code below it does.
///
/// A member is looked up against all four [`MemberKind`]s, because `@see` says
/// *what* is named and never which kind it is; `Class::name` resolving as any
/// one of them is the cross-reference the tag promised. A `Core` target is
/// trusted the same way [`member_declared`] trusts one anywhere else. A member
/// that is not found gets the closest member name of the class as its fix
/// ([`closest_name`]).
///
/// Every failure of either shape is `E0323`. What a target resolved to, a
/// class, a member with its owner and kind, or a file with its heading's line,
/// goes into [`crate::doc_targets::DocTargets`] keyed by the argument's span,
/// and the language server reads it from there.
fn check_see(tag: &DocTag, src: &SourceFile, ctx: &Ctx<'_>, env: &mut Env<'_>) {
    let raw = src.span_text(tag.argument).unwrap_or_default();
    let text = raw.trim();
    if text.is_empty() {
        env.diags.report(
            Diagnostic::error(code::E_DOC_SEE_UNRESOLVED, "`@see` names nothing")
                .with_primary(tag.span, "nothing to resolve")
                .with_help(
                    "name a class or a member, `@see Core\\Money::fromCents`, or a file, \
                     `@see docs/guide.md#usage`",
                ),
        );
        return;
    }
    // Where `text` starts in the file, so a fix can replace one part of it.
    let start = tag.argument.start as usize + (raw.len() - raw.trim_start().len());
    if let nvs_syntax::doc_target::DocTarget::File { path, anchor } =
        nvs_syntax::doc_target::DocTarget::of(text)
    {
        let at = span_of(tag.argument, start, start + text.len());
        return check_see_file(at, path, anchor, src, env);
    }

    let (class_text, member) = match text.split_once("::") {
        Some((class_text, member)) => (class_text.trim(), Some(member)),
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
            if env.symbols.contains(&resolved) {
                env.uses.class(&resolved);
            } else if !resolved.is_core() {
                env.uses.missed_class(&resolved);
            }
            (resolved.is_core() || env.symbols.contains(&resolved)).then_some(resolved)
        }
    };
    let Some(class) = class else {
        env.diags.report(
            Diagnostic::error(
                code::E_DOC_SEE_UNRESOLVED,
                format!("`@see` names `{class_text}`, which resolves to no declaration"),
            )
            .with_primary(tag.argument, "no such class")
            .with_help("name a class this file can see, with its `use` line if it needs one"),
        );
        return;
    };
    let class_span = span_of(tag.argument, start, start + class_text.len());

    let Some(member_raw) = member else {
        env.targets.push(DocTarget {
            span: span_of(tag.argument, start, start + text.len()),
            class_span: Some(class_span),
            resolved: Resolved::Class(class),
        });
        return;
    };
    let member = member_raw
        .trim()
        .trim_end_matches("()")
        .trim_start_matches('$');
    if member.is_empty() {
        env.diags.report(
            Diagnostic::error(
                code::E_DOC_SEE_UNRESOLVED,
                format!("`@see` names no member of `{class}` after the `::`"),
            )
            .with_primary(tag.argument, "nothing to resolve")
            .with_help(format!("write the member's name after `{class_text}::`")),
        );
        return;
    }
    // The bare name's place in the file: after the `::`, any space, and a `$`.
    let member_offset = text.len() - member_raw.len() + member_raw.find(member).unwrap_or(0);
    let member_span = span_of(
        tag.argument,
        start + member_offset,
        start + member_offset + member.len(),
    );
    let kinds = [
        MemberKind::Method,
        MemberKind::Const,
        MemberKind::StaticProp,
        MemberKind::Prop,
    ];
    // A miss is recorded for each kind tried, so a property or a method of
    // this name added later reaches the file.
    let found = kinds
        .into_iter()
        .find_map(|kind| record_member_owner(&class, member, kind, env).map(|owner| (owner, kind)));
    if let Some((owner, kind)) = found {
        env.targets.push(DocTarget {
            span: span_of(tag.argument, start, start + text.len()),
            class_span: Some(class_span),
            resolved: Resolved::Member {
                class,
                owner,
                name: member.to_owned(),
                kind,
            },
        });
        return;
    }
    let mut candidates = Vec::new();
    for kind in kinds {
        collect_candidates(
            &class,
            kind,
            env.table,
            env.graph,
            &mut FxHashSet::default(),
            &mut candidates,
        );
    }
    let diag = Diagnostic::error(
        code::E_DOC_SEE_UNRESOLVED,
        format!("`{class}` has no member named `{member}` for `@see` to resolve"),
    )
    .with_primary(member_span, "no such member");
    let diag = match closest_name(member, candidates.iter().map(String::as_str)) {
        Some(near) => diag.with_fix(member_span, near, format!("change it to `{near}`")),
        None => diag.with_help(format!("name a member `{class}` declares or inherits")),
    };
    env.diags.report(diag);
}

/// One `@see` file target: `path`, and the `anchor` after its `#` if one was
/// written. `at` is the whole written target.
///
/// The path is relative to the folder of the file that wrote it, as an
/// `@example` and a `require` path are, and it must stay inside the project
/// ([`crate::lenders::project_of`]). The written text is judged before it is
/// joined: an absolute path, a drive or UNC prefix, and a `..` that climbs
/// above the project are refused on the text alone. After the join, the
/// canonical path must still start with the canonical project folder, so a
/// symbolic link cannot leave it either. The target must be a regular file.
///
/// Any file may be a target, and only a Markdown file takes an anchor. The
/// anchor must equal the slug of one of its headings
/// ([`nvs_syntax::doc_target::heading_slugs`]), and a miss gets the closest
/// slug as its fix.
///
/// Cost: one `stat` and one `canonicalize` per target, as `@example` costs.
/// An anchored target also reads the Markdown file once. Its text is dropped
/// when the tag is checked, and the read is recorded for test selection.
fn check_see_file(at: Span, path: &str, anchor: Option<&str>, src: &SourceFile, env: &mut Env<'_>) {
    let path_span = span_of(at, at.start as usize, at.start as usize + path.len());
    // A bare file name, such as `nvs check app.nvs`, has the empty path as its
    // folder, which `absolute` does not take: that folder is the current one.
    let folder = match src.path().and_then(Path::parent) {
        Some(folder) if !folder.as_os_str().is_empty() => {
            std::path::absolute(folder).unwrap_or_else(|_| folder.to_path_buf())
        }
        _ => std::env::current_dir().unwrap_or_default(),
    };
    let project = crate::lenders::project_of(&folder);
    let depth = folder
        .strip_prefix(&project)
        .map_or(0, |below| below.components().count());
    if path.is_empty() || leaves_project(path, depth) {
        env.diags.report(
            Diagnostic::error(
                code::E_DOC_SEE_UNRESOLVED,
                format!("`@see` names `{path}`, which is outside the project"),
            )
            .with_primary(path_span, "outside the project")
            .with_help(format!(
                "a `@see` path must name a file inside `{}`, the folder that holds the \
                 project's `nvs.toml` or `package.toml`",
                project.display()
            )),
        );
        return;
    }

    let joined = folder.join(path);
    nvs_footprint::exists(&joined);
    let canonical = joined.canonicalize().ok();
    let inside = canonical.as_ref().is_some_and(|canonical| {
        project
            .canonicalize()
            .is_ok_and(|project| canonical.starts_with(project))
    });
    if canonical.is_some() && !inside {
        env.diags.report(
            Diagnostic::error(
                code::E_DOC_SEE_UNRESOLVED,
                format!("`@see` names `{path}`, which leads outside the project"),
            )
            .with_primary(path_span, "a link that leaves the project")
            .with_help("a `@see` path must name a file inside the project, also through a link"),
        );
        return;
    }
    let Some(canonical) = canonical.filter(|canonical| canonical.is_file()) else {
        env.diags.report(
            Diagnostic::error(
                code::E_DOC_SEE_UNRESOLVED,
                format!("`@see` names `{path}`, which is not a file"),
            )
            .with_primary(path_span, "no such file")
            .with_help("the path is relative to the file that wrote it, as a `require` path is"),
        );
        return;
    };

    let Some(anchor) = anchor else {
        env.targets.push(DocTarget {
            span: at,
            class_span: None,
            resolved: Resolved::File {
                path: canonical,
                heading: None,
            },
        });
        return;
    };
    // `#` and the anchor after it, at the end of the target.
    let hash = span_of(at, at.end as usize - anchor.len() - 1, at.end as usize);
    let anchor_span = span_of(at, at.end as usize - anchor.len(), at.end as usize);
    if !nvs_syntax::doc_target::is_markdown(path) {
        env.diags.report(
            Diagnostic::error(
                code::E_DOC_SEE_UNRESOLVED,
                format!("`{path}` is not a Markdown file, so `#{anchor}` names nothing in it"),
            )
            .with_primary(hash, "an anchor on a file that is not Markdown")
            .with_fix(hash, "", "delete the anchor"),
        );
        return;
    }
    nvs_footprint::file(&canonical);
    let text = see_anchor_text(&canonical).unwrap_or_default();
    let slugs = nvs_syntax::doc_target::heading_slugs(&text);
    if let Some((_, line)) = slugs.iter().find(|(slug, _)| slug == anchor) {
        env.targets.push(DocTarget {
            span: at,
            class_span: None,
            resolved: Resolved::File {
                path: canonical,
                heading: Some(*line),
            },
        });
        return;
    }
    let diag = Diagnostic::error(
        code::E_DOC_SEE_UNRESOLVED,
        format!("`{path}` has no heading `#{anchor}`"),
    )
    .with_primary(anchor_span, "no such heading");
    let diag = match closest_name(anchor, slugs.iter().map(|(slug, _)| slug.as_str())) {
        Some(near) => diag.with_fix(anchor_span, near, format!("change it to `#{near}`")),
        None => diag.with_help(
            "an anchor is a heading's text in lowercase, with spaces as `-` and punctuation removed",
        ),
    };
    env.diags.report(diag);
}

/// The text of the Markdown file at `path` that a `@see` anchor names, or
/// `None` for a file that cannot be read.
///
/// This is the one read the checker matches an anchor against, through
/// [`nvs_syntax::doc_target::heading_slugs`]. The editor's completion after a
/// `#` reads the file here too, so it offers exactly the headings the checker
/// accepts.
#[must_use]
pub fn see_anchor_text(path: &Path) -> Option<String> {
    std::fs::read_to_string(path).ok()
}

/// Whether the written `@see` path leaves the project on its text alone:
/// it is absolute, starts with a drive or UNC prefix, or has a `..` that
/// climbs above the project. `depth` is how many folders the writing file's
/// folder sits below the project folder. Both `/` and `\` separate.
fn leaves_project(path: &str, depth: usize) -> bool {
    if path.starts_with(['/', '\\']) || path.as_bytes().get(1) == Some(&b':') {
        return true;
    }
    let mut at = depth;
    for segment in path.split(['/', '\\']) {
        match segment {
            "" | "." => {}
            ".." => match at.checked_sub(1) {
                Some(up) => at = up,
                None => return true,
            },
            _ => at += 1,
        }
    }
    false
}

/// The candidate closest to `written` by [`edit_distance`], when it is
/// within a third of the written length. Two at the same distance give the
/// one that sorts first, so the fix is the same on every run.
fn closest_name<'c>(
    written: &str,
    candidates: impl IntoIterator<Item = &'c str>,
) -> Option<&'c str> {
    let budget = written.chars().count() / 3;
    candidates
        .into_iter()
        .filter(|candidate| *candidate != written)
        .map(|candidate| (edit_distance(written, candidate), candidate))
        .filter(|(distance, _)| *distance <= budget)
        .min()
        .map(|(_, candidate)| candidate)
}

/// The directories a Novis project's test corpus walks, and so the only ones
/// an `@example` may name a file inside
/// (`rule:tooling/doc-comment-tags-are-closed-and-checked`): `examples/`, and
/// `tests/`, where a project's own cases live and which `nvs test` walks. The list is here rather than in configuration because the
/// guarantee the tag buys is that *something* compiles the file, and a
/// directory a project could name in a config file it also controls buys
/// nothing.
const WALKED_DIRECTORIES: [&str; 2] = ["examples", "tests"];

/// One `@example`'s target, which
/// `rule:tooling/doc-comment-tags-are-closed-and-checked` requires to exist and to
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
    if base.join(written).is_file() {
        if env.targets.is_on()
            && let Ok(path) = base.join(written).canonicalize()
        {
            env.targets.push(DocTarget {
                span: tag.argument,
                class_span: None,
                resolved: Resolved::File {
                    path,
                    heading: None,
                },
            });
        }
    } else {
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
        StmtKind::If { arms, else_ } => {
            for arm in arms {
                e!(&arm.cond);
                s!(&arm.then);
            }
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
/// `rule:testing/interaction-after-the-fact`'s method reference — `Mailer::send`,
/// which folds to the method's name rather than to a callable.
///
/// A roster of names rather than a question asked of the registry, because this
/// crate does not know one: `nvs_types::core_lib::method_ref_param` reads the
/// position back off the row that declared it — the parameter written
/// `nvs_stdlib::registry::CoreTy::MethodRef` — and is the home of *which*
/// positions there are. What this list buys is only that a missing method is
/// `nvs_types`' `E0829`, which names the test, rather than this pass's `E0309`.
/// A member missing from here is refused with the wrong code rather than
/// admitted somewhere it should not be.
const METHOD_REF_ARGS: &[(&str, &str, usize)] = &[
    (r"Core\Test", "assertCalled", 1),
    (r"Core\Test", "assertNeverCalled", 1),
];

/// [`walk_args`] for a static call, with the member half of the one argument
/// [`METHOD_REF_ARGS`] names left to `nvs_types`, which reports a missing
/// method there in the test's own words.
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
                Some(ExprKind::StaticCall {
                    args: CallArgs::MethodRef,
                    ..
                })
            )
        });
    let link = link_route_index(class, method, list, src, ctx);
    for (index, Arg { value, .. }) in list.iter().enumerate() {
        if let Some((at, member)) = link
            && at == index
            && links_root_target(value, member, src, ctx, env)
        {
            continue;
        }
        match (&value.kind, reference == Some(index)) {
            (ExprKind::StaticCall { class: named, .. }, true) => {
                walk_class_side(named, src, ctx, env);
            }
            _ => walk_expr(value, src, ctx, env),
        }
    }
}

/// `Core\Router`'s three link builders, whose route argument may name a class
/// under an `autoload links` root.
const LINK_BUILDERS: &[&str] = &["url", "urlAbsolute", "urlSigned"];

/// The written position of a `Core\Router` link's route argument, and which
/// link builder it is: the argument written `route:`, or else the first one.
fn link_route_index<'s>(
    class: &Expr,
    method: &MemberName,
    list: &[Arg],
    src: &'s SourceFile,
    ctx: &Ctx<'_>,
) -> Option<(usize, &'s str)> {
    let (ExprKind::ConstFetch(class_name), MemberName::Ident(name_span)) = (&class.kind, method)
    else {
        return None;
    };
    let member = src.span_text(*name_span)?;
    if !LINK_BUILDERS.contains(&member)
        || resolve_ref(name_text(src, class_name), ctx.namespace, ctx.imports).to_string()
            != r"Core\Router"
    {
        return None;
    }
    let named = list.iter().position(|arg| {
        arg.name
            .is_some_and(|name| src.span_text(name) == Some("route"))
    });
    let first = list
        .first()
        .filter(|arg| arg.name.is_none() && !arg.spread)
        .map(|_| 0);
    Some((named.or(first)?, member))
}

/// Checks a link's route argument when it names a class under an `autoload
/// links` root, and whether it did. Such a class is not declared, so the
/// ordinary walk would call it missing.
///
/// `url` and `urlAbsolute` may name it. The method must exist on the class,
/// and `nvs_types::links` checks its route. `urlSigned` may not, because a
/// signature is checked with the key of the program that made it
/// (`rule:core-classes/router-signed-url`).
fn links_root_target(
    value: &Expr,
    member: &str,
    src: &SourceFile,
    ctx: &Ctx<'_>,
    env: &mut Env<'_>,
) -> bool {
    let ExprKind::StaticCall {
        class,
        method: MemberName::Ident(method_span),
        args: CallArgs::MethodRef,
        ..
    } = &value.kind
    else {
        return false;
    };
    let ExprKind::ConstFetch(class_name) = &class.kind else {
        return false;
    };
    let qname = resolve_ref(name_text(src, class_name), ctx.namespace, ctx.imports);
    if env.symbols.contains(&qname) {
        return false;
    }
    let Some(target) = env.links.get(&qname) else {
        return false;
    };
    let name = src.span_text(*method_span).unwrap_or_default();
    if member == "urlSigned" {
        env.diags.report(
            Diagnostic::error(
                code::E_SIGNED_LINK_TO_ANOTHER_MODULE,
                format!(
                    "`Core\\Router::urlSigned` cannot link to `{qname}::{name}`, which is under \
                     an `autoload links` root"
                ),
            )
            .with_primary(value.span, "a route of another module")
            .with_help(
                "a signed link is checked by the module that created it. Call `urlSigned` in \
                 the module that declares this route, or link with `Core\\Router::url`",
            ),
        );
        return true;
    }
    if target.methods.iter().any(|method| method == name) {
        env.uses.member(&qname, name);
    } else {
        env.uses.missed_member(Some(&qname), name);
        env.diags.report(
            Diagnostic::error(
                code::E_UNDEFINED_MEMBER,
                format!("`{qname}` has no method named `{name}`"),
            )
            .with_primary(value.span, "referenced here"),
        );
    }
    true
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
        ExprKind::Interpolated(parts) | ExprKind::HtmlTemplate(parts) => {
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
        // `{ length: Length::Full }` at a call argument is the common place an
        // enum case is written, and a field value is checked as it is anywhere
        // else: a missed one reached `nvs-ir` with no case recorded.
        ExprKind::AnonObject(fields) => {
            for field in fields {
                e!(&field.value);
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
        | ExprKind::IsInit(expr)
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
                    // `rule:types/anonymous-function-self-name`: inside `fn fact(...) => … fact(…)`, the
                    // callee is this anonymous function and not a free function. Only in
                    // callee position — the name resolves the way `self::`
                    // does, so it is not a value and a bare `fact` below is
                    // still `E0319`.
                    if env.fn_self.as_deref() == Some(text) {
                        walk_args(args, src, ctx, env);
                        return;
                    }
                    if !env.refused_toplevel.contains(text) {
                        // Every name gets the same help, whose
                        // `Core\Str::length` is an example of the shape.
                        let help = "every function is a method of a class. The built-in \
                                    functions are in the `Core` namespace. Write \
                                    `Core\\Str::length($s)`, or write `use Core\\Str;` and then \
                                    `Str::length($s)`";
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
                // `rule:attributes/a-retrieval-matches-the-declaration-it-names`'s class target — `Foo::constructor(...)` — rests
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
                if !(name == "constructor" && matches!(args, CallArgs::MethodRef)) {
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
        ExprKind::New { target, args, .. } => {
            if env.in_block && matches!(target, NewTarget::StaticTy | NewTarget::ParentTy) {
                let written = if matches!(target, NewTarget::StaticTy) {
                    "static"
                } else {
                    "parent"
                };
                env.diags.report(
                    Diagnostic::error(
                        code::E_EXTEND_MEMBER_NOT_ADDABLE,
                        format!("an `extend` block cannot use `new {written}`"),
                    )
                    .with_primary(expr.span, "not allowed in an `extend` block")
                    .with_help(
                        "write `new self`. In an `extend` block, `self` is the extended type, and \
                         an added member is never inherited",
                    ),
                );
            }
            walk_args(args, src, ctx, env);
        }
        ExprKind::Fn(fn_expr) => {
            for param in &fn_expr.params {
                if let Some(default) = &param.default {
                    e!(default);
                }
            }
            // `rule:types/anonymous-function-self-name`'s self-name covers this body and no other — a
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
                // `html"…"` is an html template written with a string's
                // delimiter, one character from the form that compiles, and a
                // help about class constants reads as "there is no such
                // literal". The bare name followed by a quote is that guess
                // and nothing else, since no constant is ever followed by one.
                let quote_follows = src
                    .text()
                    .get(expr.span.end as usize..)
                    .is_some_and(|after| after.starts_with(['"', '\'']));
                let help = if text == "html" && quote_follows {
                    "an html template is written with backticks, `html`<p>{$name}</p>``: its \
                     text is a `Core\\Html\\Markup` and every `{$…}` hole in it is escaped"
                        .to_owned()
                } else if matches!(text, "__FILE__" | "__DIR__") {
                    "there are no magic constants. `Core\\Path::thisFile()` returns the path of \
                     this file, and `Core\\Path::thisDirectory()` returns its folder"
                        .to_owned()
                } else {
                    "every constant belongs to a class, so write `Class::NAME`. A built-in \
                     constant is on its `Core` class, such as `Core\\Math::PI`"
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
                    "write `{written}::` and then a method call, a constant or a static property. \
                     To get information about a class, use `Core\\Reflect`"
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
    // `Order::Status` on the left of `::Paid` names an owned enum, which
    // `check_member_ref` resolves (`rule:enums/class-scoped-enum`). A class
    // keyword in front of the enum is `nvs_types`' to refuse.
    if let ExprKind::ClassConstAccess { class: inner, .. } = &class.kind
        && matches!(
            inner.kind,
            ExprKind::SelfExpr
                | ExprKind::StaticExpr
                | ExprKind::ParentExpr
                | ExprKind::ConstFetch(_)
        )
    {
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
    if env.in_block && matches!(class_expr.kind, ExprKind::StaticExpr | ExprKind::ParentExpr) {
        let written = if matches!(class_expr.kind, ExprKind::StaticExpr) {
            "static"
        } else {
            "parent"
        };
        env.diags.report(
            Diagnostic::error(
                code::E_EXTEND_MEMBER_NOT_ADDABLE,
                format!("an `extend` block cannot use `{written}::`"),
            )
            .with_primary(class_expr.span, "not allowed in an `extend` block")
            .with_help(
                "write `self::`. In an `extend` block, `self` is the extended type, and an added \
                 member is never inherited",
            ),
        );
        return;
    }
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
        ExprKind::ConstFetch(class_name) => resolve_class_name(class_name, src, ctx, env),
        // `Order::Status::Paid`: the inner access names an enum its owner
        // declares (`rule:enums/class-scoped-enum`), and no other chain names a
        // class.
        ExprKind::ClassConstAccess {
            class: inner,
            name: member,
        } => {
            let ExprKind::ConstFetch(owner_name) = &inner.kind else {
                return;
            };
            let Some(owner) = resolve_class_name(owner_name, src, ctx, env) else {
                return;
            };
            let member = src.span_text(*member).unwrap_or_default();
            if owner.is_core() {
                return;
            }
            if let Some(key) =
                crate::resolve::owned_enum_using(&owner, member, env.symbols, ctx.imports)
            {
                env.uses.class(&key);
                Some(key)
            } else {
                env.uses.missed_member(Some(&owner), member);
                report_missing_owned_enum(&owner, member, class_expr.span, env);
                None
            }
        }
        _ => None,
    };

    let Some(qname) = qname else {
        return;
    };
    if record_member(&qname, name, kind, env) {
        return;
    }
    if let Some(block) = added_member(&qname, name, kind, ctx.imports, env) {
        if env.uses.is_on() {
            env.uses.class(&block);
        }
        return;
    }
    let nearest = nearest_member(&qname, name, kind, env.table, env.graph);
    let mut diagnostic = undefined_member(&qname, name, kind.describe(), class_expr.span, nearest);
    if let Some(block) = block_adding(&qname, name, kind, env) {
        diagnostic = diagnostic.with_help(format!(
            "the `extend` block `{block}` adds `{name}`. Write `use {block};` at the top of this \
             file to use it"
        ));
        // The help's `use` line is the fix an editor offers.
        if let Some(site) = crate::imports::import_site(env.stmts, src, class_expr.span.start) {
            diagnostic = diagnostic.with_fix(
                site.span(class_expr.span.file),
                site.use_line(&block),
                format!("import `{block}`"),
            );
        }
    }
    env.diags.report(diagnostic);
}

/// Whether `qname` is `target` or has it among its ancestors.
fn reaches(qname: &QName, target: &QName, graph: &ClassGraph) -> bool {
    qname == target
        || crate::find_in_ancestors(qname, graph, |class| (class == target).then_some(())).is_some()
}

/// Whether the block `block` declares the member `name` of `kind`.
fn block_declares(block: &QName, name: &str, kind: MemberKind, table: &MemberTable) -> bool {
    table.get(block).is_some_and(|members| match kind {
        MemberKind::Method => members.methods.contains(name),
        MemberKind::Const => members.consts.contains(name),
        MemberKind::StaticProp => members.static_props.contains(name),
        MemberKind::Prop => members.props.contains(name),
    })
}

/// The `extend` block named in `imports` that adds the member `name` to
/// `qname` or to one of its ancestors
/// (`rule:classes/an-extend-block-adds-members-to-a-type`). A file reaches an
/// added member only through a block its `use` list names, and a block's own
/// body sees its own members.
fn added_member(
    qname: &QName,
    name: &str,
    kind: MemberKind,
    imports: &FxHashMap<String, QName>,
    env: &Env<'_>,
) -> Option<QName> {
    imports
        .values()
        .find(|block| {
            env.graph
                .block_target(block)
                .is_some_and(|target| reaches(qname, target, env.graph))
                && block_declares(block, name, kind, env.table)
        })
        .cloned()
}

/// The first block of the program, by name, that adds `name` to `qname`: the
/// `use` line an unknown member's help offers.
fn block_adding(qname: &QName, name: &str, kind: MemberKind, env: &Env<'_>) -> Option<QName> {
    env.graph
        .blocks()
        .filter(|(block, target)| {
            reaches(qname, target, env.graph) && block_declares(block, name, kind, env.table)
        })
        .map(|(block, _)| block.to_string())
        .min()
        .map(|block| QName::parse(&block))
}

/// The class a bare or qualified name on the left of a `::` names: inside an
/// owner's body, the owner's own enum first (`rule:enums/class-scoped-enum`),
/// then what the namespace and imports resolve. A name nothing declares is
/// reported, and `None`.
fn resolve_class_name(
    class_name: &nvs_syntax::ast::Name,
    src: &SourceFile,
    ctx: &Ctx<'_>,
    env: &mut Env<'_>,
) -> Option<QName> {
    let text = name_text(src, class_name);
    if let Some(key) =
        crate::resolve::owned_enum_in_scope_using(text, ctx.current_class, env.symbols, ctx.imports)
    {
        env.uses.class(&key);
        return Some(key);
    }
    let resolved = resolve_ref(text, ctx.namespace, ctx.imports);
    if env.symbols.contains(&resolved) {
        env.uses.class(&resolved);
        return Some(resolved);
    }
    if resolved.is_core() {
        return Some(resolved);
    }
    env.uses.missed_class(&resolved);
    if let Some(diagnostic) = crate::hierarchy::undeclared_name(
        crate::hierarchy::Undeclared {
            qname: &resolved,
            text,
            span: class_name.span,
            namespace: ctx.namespace,
            imports: ctx.imports,
            stmts: env.stmts,
            src,
        },
        env.symbols,
        env.core,
    ) {
        env.diags.report(diagnostic);
    }
    None
}

/// `Owner::Name::Case` where `Owner` declares no enum `Name`. When an ancestor
/// does, the help names it: an enum is reached through the class that declares
/// it and is not inherited (`rule:enums/class-scoped-enum`).
fn report_missing_owned_enum(owner: &QName, name: &str, span: Span, env: &mut Env<'_>) {
    let mut diag = Diagnostic::error(
        code::E_UNDEFINED_MEMBER,
        format!("`{owner}` has no enum named `{name}`"),
    )
    .with_primary(span, "referenced here");
    let block = crate::resolve::owned_enum(owner, name, env.symbols)
        .and_then(|key| env.symbols.added_by(&key).cloned());
    if let Some(block) = block {
        diag = diag.with_help(format!(
            "The `extend` block `{block}` adds this enum. Write `use {block};` at the top of \
             this file"
        ));
    } else if let Some(declaring) = ancestor_owning_enum(owner, name, env) {
        diag = diag.with_help(format!(
            "`{declaring}::{name}` is where it is declared. An enum in a class body is used \
             through that class. A subclass does not inherit it"
        ));
    }
    env.diags.report(diag);
}

/// The nearest `extends`/`implements` ancestor of `owner` whose body declares
/// an enum called `name`, breadth-first.
fn ancestor_owning_enum(owner: &QName, name: &str, env: &Env<'_>) -> Option<QName> {
    let supertypes = |qname: &QName| {
        env.graph.get(qname).map_or_else(Vec::new, |links| {
            links
                .extends
                .iter()
                .chain(links.implements.iter())
                .cloned()
                .collect::<Vec<_>>()
        })
    };
    let mut seen = FxHashSet::default();
    let mut queue = supertypes(owner);
    let mut next = 0;
    while next < queue.len() {
        let candidate = queue[next].clone();
        next += 1;
        if !seen.insert(candidate.clone()) {
            continue;
        }
        if crate::resolve::owned_enum(&candidate, name, env.symbols).is_some() {
            return Some(candidate);
        }
        queue.extend(supertypes(&candidate));
    }
    None
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
    if record_member(current, name, MemberKind::Prop, env)
        || added_member(current, name, MemberKind::Prop, ctx.imports, env).is_some()
    {
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

/// Looks `name` up on `qname` with [`member_declared`], and records the
/// member it found, or the miss, into the file's uses (`crate::uses`). A
/// member of a `Core` class records nothing: no edit to the program changes it.
fn record_member(qname: &QName, name: &str, kind: MemberKind, env: &mut Env<'_>) -> bool {
    record_member_owner(qname, name, kind, env).is_some()
}

/// [`record_member`], returning the class that declares the member.
fn record_member_owner(
    qname: &QName,
    name: &str,
    kind: MemberKind,
    env: &mut Env<'_>,
) -> Option<QName> {
    let owner = member_declared(qname, name, kind, env.table, env.graph);
    if env.uses.is_on() {
        let written = match kind {
            MemberKind::StaticProp | MemberKind::Prop => format!("${name}"),
            MemberKind::Method | MemberKind::Const => name.to_owned(),
        };
        match owner {
            Some(owner) if owner.is_core() => {}
            Some(owner) => env.uses.member_through(qname, owner, &written),
            None => env.uses.missed_member(Some(qname), &written),
        }
    }
    owner.cloned()
}

/// The class that declares `qname`'s member `name`: `qname` itself or the
/// first ancestor that does, or `qname` when it is a `Core` class, whose
/// members are trusted. `None` when no class in the chain declares it.
pub(crate) fn member_declared<'a>(
    qname: &'a QName,
    name: &str,
    kind: MemberKind,
    table: &MemberTable,
    graph: &'a ClassGraph,
) -> Option<&'a QName> {
    let mut seen = FxHashSet::default();
    member_declared_rec(qname, name, kind, table, graph, &mut seen)
}

fn member_declared_rec<'a>(
    qname: &'a QName,
    name: &str,
    kind: MemberKind,
    table: &MemberTable,
    graph: &'a ClassGraph,
    seen: &mut FxHashSet<QName>,
) -> Option<&'a QName> {
    if qname.is_core() {
        return Some(qname);
    }
    if !seen.insert(qname.clone()) {
        return None;
    }
    if let Some(members) = table.get(qname) {
        let found = match kind {
            MemberKind::Method => members.methods.contains(name),
            MemberKind::Const => members.consts.contains(name),
            MemberKind::StaticProp => members.static_props.contains(name),
            MemberKind::Prop => members.props.contains(name),
        };
        if found {
            return Some(qname);
        }
    }
    let links = graph.get(qname)?;
    links
        .extends
        .iter()
        .chain(links.implements.iter())
        .find_map(|parent| member_declared_rec(parent, name, kind, table, graph, seen))
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
    nearest_name(name, candidates)
}

/// The candidate closest to `name` within [`suggestion_budget`], ties to the
/// alphabetically first, so the answer does not depend on the candidates'
/// order. Public because `nvs_types` reports an enum's unknown case from its
/// own enum table and must suggest exactly what this module suggests, or the
/// two reports of one mistake would differ and both be shown.
#[must_use]
pub fn nearest_name(name: &str, candidates: impl IntoIterator<Item = String>) -> Option<String> {
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

/// `E0309` for a member `qname` does not have, with the nearest name as its
/// help. One constructor, because `nvs_types` reports an enum's unknown case
/// too, and the diagnostic sink keeps one of two reports only when they are
/// identical.
#[must_use]
pub fn undefined_member(
    qname: &QName,
    name: &str,
    kind: &str,
    span: Span,
    nearest: Option<String>,
) -> Diagnostic {
    let diag = Diagnostic::error(
        code::E_UNDEFINED_MEMBER,
        format!("`{qname}` has no {kind} named `{name}`"),
    )
    .with_primary(span, "referenced here");
    match nearest {
        Some(nearest) => diag.with_help(format!("did you mean `{nearest}`?")),
        None => diag,
    }
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
///
/// Public because a name the language server's index does not hold is ranked
/// against the names it does by the same measure (`nvs_lsp::query::nearest`).
#[must_use]
pub fn edit_distance(a: &str, b: &str) -> usize {
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

    /// The help of every `E0320` in `src`.
    ///
    /// The check reports into a sink of its own: `resolve_file` runs a member
    /// check too, and its copy of each `E0320` is not the one under test.
    fn free_function_helps(src: &str) -> Vec<String> {
        let mut map = SourceMap::new();
        let file = map.add("t.nvs", src);
        let mut diags = Diagnostics::new();
        let stmts = parse_file(map.file(file), &mut diags);
        assert!(!diags.has_errors(), "fixture failed to parse: {diags:?}");
        let module = resolve_file(&stmts, map.file(file), &mut diags);
        let mut members = MemberResolver::new();
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

    /// A name PHP has and a name nobody has get the same help, and it names
    /// no PHP function (`rule:classes/no-free-functions-or-constants`).
    #[test]
    fn an_undefined_free_function_gets_the_general_help() {
        let general = "The built-in functions are in the `Core` namespace";
        for src in [
            "<?nvs\necho count([1, 2]);\n",
            "<?nvs\necho tally([1, 2]);\n",
        ] {
            let helps = free_function_helps(src);
            assert!(!helps.is_empty(), "the call is `E0320`");
            assert!(
                helps
                    .iter()
                    .all(|help| help.contains(general) && !help.contains("PHP")),
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

    /// A project in a scratch folder: an `nvs.toml`, `docs/guide.md` with
    /// three headings, `schema.sql`, and `src/a.nvs` holding `body`. The file
    /// is loaded from disk, so its `@see` paths resolve from `src/`. Gives
    /// what the check reported and what it recorded.
    fn check_see_in_project(
        name: &str,
        body: &str,
    ) -> (
        nvs_repo::Scratch,
        Diagnostics,
        crate::doc_targets::DocTargets,
    ) {
        let dir = nvs_repo::scratch(&format!("hir-see-{name}"));
        std::fs::create_dir_all(dir.join("docs")).expect("docs folder");
        std::fs::create_dir_all(dir.join("src")).expect("src folder");
        std::fs::write(dir.join("nvs.toml"), "").expect("nvs.toml");
        std::fs::write(
            dir.join("docs/guide.md"),
            "# Guide\n\n## Usage\n\nText.\n\n## Error codes\n",
        )
        .expect("guide.md");
        std::fs::write(dir.join("schema.sql"), "create table t (id int);\n").expect("schema");
        std::fs::write(dir.join("src/a.nvs"), body).expect("a.nvs");

        let mut map = SourceMap::new();
        let file = map.load(dir.join("src/a.nvs")).expect("load a.nvs");
        let mut diags = Diagnostics::new();
        let stmts = parse_file(map.file(file), &mut diags);
        assert!(!diags.has_errors(), "fixture failed to parse: {diags:?}");
        let module = resolve_file(&stmts, map.file(file), &mut Diagnostics::new());
        let mut members = MemberResolver::new();
        members.collect_members(&stmts, map.file(file));
        let mut targets = crate::doc_targets::DocTargets::recording();
        members.check_recording(
            &stmts,
            map.file(file),
            (&module.symbols, &module.graph),
            false,
            (&mut crate::uses::Uses::off(), &mut targets),
            &mut diags,
        );
        (dir, diags, targets)
    }

    /// The `E0323` messages a check reported.
    fn see_errors(diags: &Diagnostics) -> Vec<String> {
        diags
            .iter()
            .filter(|d| d.code == Some(code::E_DOC_SEE_UNRESOLVED))
            .map(|d| d.message.clone())
            .collect()
    }

    /// The one class every `@see` fixture documents, with `tags` as its doc
    /// comment's lines.
    fn see_fixture(tags: &[&str]) -> String {
        let mut src = String::from("<?nvs\n/// A price.\n///\n");
        for tag in tags {
            src.push_str("/// ");
            src.push_str(tag);
            src.push('\n');
        }
        src.push_str("class Price { function cents(): int { return 1; } }\n");
        src
    }

    #[test]
    fn a_see_file_target_that_exists_is_accepted() {
        let (_dir, diags, _) = check_see_in_project(
            "exists",
            &see_fixture(&[
                "@see ../docs/guide.md",
                "@see ../schema.sql",
                "@see ../docs/guide.md#usage",
                "@see ./../docs/guide.md#error-codes",
            ]),
        );
        assert!(!diags.has_errors(), "{diags:?}");
    }

    #[test]
    fn a_see_file_target_that_names_no_file_is_refused() {
        let (_dir, diags, _) =
            check_see_in_project("no-file", &see_fixture(&["@see ../docs/missing.md"]));
        assert_eq!(
            see_errors(&diags),
            ["`@see` names `../docs/missing.md`, which is not a file"]
        );
    }

    #[test]
    fn a_see_anchor_that_names_no_heading_is_refused() {
        let (_dir, diags, _) = check_see_in_project(
            "no-heading",
            &see_fixture(&["@see ../docs/guide.md#installation"]),
        );
        assert_eq!(
            see_errors(&diags),
            ["`../docs/guide.md` has no heading `#installation`"]
        );
    }

    #[test]
    fn a_see_anchor_on_a_file_that_is_not_markdown_is_refused() {
        let (_dir, diags, _) =
            check_see_in_project("not-markdown", &see_fixture(&["@see ../schema.sql#tables"]));
        assert_eq!(
            see_errors(&diags),
            ["`../schema.sql` is not a Markdown file, so `#tables` names nothing in it"]
        );
        let fix = diags
            .iter()
            .flat_map(|d| &d.suggestions)
            .next()
            .expect("a fix that deletes the anchor");
        assert!(fix.replacement.is_empty(), "{fix:?}");
    }

    #[test]
    fn a_see_path_that_leaves_the_project_is_refused() {
        let (_dir, diags, _) = check_see_in_project(
            "leaves",
            &see_fixture(&[
                "@see ../../outside.md",
                "@see /etc/hosts.md",
                "@see C:/Windows/win.ini",
                "@see \\\\server\\share\\notes.md",
            ]),
        );
        let errors = see_errors(&diags);
        assert_eq!(errors.len(), 4, "{errors:?}");
        assert!(
            errors
                .iter()
                .all(|e| e.ends_with("which is outside the project")),
            "{errors:?}"
        );
    }

    /// A link inside the project that points outside it is refused after the
    /// join, by the canonical path.
    #[cfg(unix)]
    #[test]
    fn a_see_symlink_that_leaves_the_project_is_refused() {
        let outside = nvs_repo::scratch("hir-see-outside");
        std::fs::write(outside.join("secret.md"), "# Secret\n").expect("secret.md");
        let dir = nvs_repo::scratch("hir-see-link");
        std::os::unix::fs::symlink(outside.join("secret.md"), dir.join("linked.md"))
            .expect("symlink");
        std::fs::write(dir.join("nvs.toml"), "").expect("nvs.toml");
        let body = see_fixture(&["@see linked.md"]);
        std::fs::write(dir.join("a.nvs"), &body).expect("a.nvs");
        let mut map = SourceMap::new();
        let file = map.load(dir.join("a.nvs")).expect("load a.nvs");
        let mut diags = Diagnostics::new();
        let stmts = parse_file(map.file(file), &mut diags);
        let module = resolve_file(&stmts, map.file(file), &mut Diagnostics::new());
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
        assert_eq!(
            see_errors(&diags),
            ["`@see` names `linked.md`, which leads outside the project"]
        );
    }

    #[test]
    fn a_see_miss_suggests_the_closest_name() {
        let src = see_fixture(&["@see Price::cennts", "@see ../docs/guide.md#usag"]);
        let (_dir, diags, _) = check_see_in_project("closest", &src);
        let mut fixes: Vec<(&str, &str)> = diags
            .iter()
            .filter(|d| d.code == Some(code::E_DOC_SEE_UNRESOLVED))
            .flat_map(|d| &d.suggestions)
            .map(|fix| {
                let written = &src[fix.span.start as usize..fix.span.end as usize];
                (written, fix.replacement.as_str())
            })
            .collect();
        fixes.sort_unstable();
        assert_eq!(fixes, [("cennts", "cents"), ("usag", "usage")]);
    }

    #[test]
    fn a_see_target_is_recorded_with_what_it_resolved_to() {
        let src = "<?nvs\n\
                   /// A price.\n\
                   ///\n\
                   /// @see Price::cents\n\
                   /// @see Price\n\
                   /// @see ../docs/guide.md#error-codes\n\
                   class Price {\n\
                   /// The amount.\n\
                   ///\n\
                   /// @throws Core\\Exception when it fails\n\
                   function cents(): int { return 1; }\n\
                   }\n";
        let (dir, diags, targets) = check_see_in_project("recorded", src);
        assert!(!diags.has_errors(), "{diags:?}");
        let recorded: Vec<(&str, &Resolved)> = targets
            .iter()
            .map(|target| {
                (
                    &src[target.span.start as usize..target.span.end as usize],
                    &target.resolved,
                )
            })
            .collect();
        let guide = dir
            .join("docs/guide.md")
            .canonicalize()
            .expect("guide.md exists");
        assert_eq!(
            recorded,
            [
                (
                    "Price::cents",
                    &Resolved::Member {
                        class: QName::parse("Price"),
                        owner: QName::parse("Price"),
                        name: "cents".to_owned(),
                        kind: MemberKind::Method,
                    }
                ),
                ("Price", &Resolved::Class(QName::parse("Price"))),
                (
                    "../docs/guide.md#error-codes",
                    &Resolved::File {
                        path: guide,
                        heading: Some(6),
                    }
                ),
                (
                    "Core\\Exception",
                    &Resolved::Class(QName::parse("Core\\Exception"))
                ),
            ]
        );
    }
}
