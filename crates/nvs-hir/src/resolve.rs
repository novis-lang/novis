//! Name resolution: namespaces, `use` imports, and a file's declared symbols.
//!
//! See the crate's module docs for what this covers and what M2 still needs.

use nvs_diagnostics::{Diagnostic, Diagnostics, SourceFile, Span, code};
use nvs_syntax::ast::{
    ClassMember, ClassMemberKind, Name, NamespaceDecl, Stmt, StmtKind, TypeAliasDecl, TypeAtom,
    TypeKind,
};
use rustc_hash::FxHashMap;

use crate::qname::QName;
use crate::symbol::{Symbol, SymbolKind, SymbolTable};

/// One `use Path\To\Name;` import, checked against the declarations collected
/// from every file [`Resolver::collect_declarations`] has seen so far.
///
/// A target under `Core` ([`QName::is_core`]) is always trusted — `nvs-hir`
/// has no declarations for the built-in stdlib to check against yet.
#[derive(Clone, Debug)]
pub struct Import {
    /// The name this import binds — always the target's own last segment,
    /// since `rule:statements/nothing-gets-a-second-name` makes
    /// `use … as …` a diagnostic rather than a second spelling.
    pub short_name: String,
    /// What it resolves to.
    pub target: QName,
    /// Where the imported path was written.
    pub span: Span,
    /// Whether `target` names a real declaration (or is trusted as a `Core`
    /// or reserved-global reference). `false` means `E_UNRESOLVED_IMPORT` was
    /// already reported
    /// for it.
    pub resolved: bool,
}

/// Everything `nvs-hir` resolves out of one or more parsed files, for this
/// slice of M2: every declared class/interface/enum/type alias under its
/// fully-qualified name, every `use` import checked against that set, and the
/// class hierarchy (`extends`/`implements`) resolved to real symbols.
///
/// Not yet built — see the crate's module docs for the rest of M2's name
/// resolution this will grow into.
#[derive(Debug, Default)]
pub struct Module {
    /// Every class/interface/enum/type-alias declaration collected.
    pub symbols: SymbolTable,
    /// Every `use` import seen, resolved or not.
    pub imports: Vec<Import>,
    /// Every class/interface's resolved `extends`/`implements` links.
    pub graph: crate::hierarchy::ClassGraph,
    /// Every class/interface/enum's own directly-declared members
    /// (M2 item 2 — see [`crate::members`]).
    pub members: crate::members::MemberTable,
    /// Every `type` alias's fully-substituted expansion (M2 item 3 — see
    /// [`crate::aliases`]).
    pub aliases: crate::aliases::AliasTable,
}

/// Resolves parsed files' top-level namespace/`use`/declaration structure
/// into a shared [`Module`].
///
/// Call [`resolve_file`] for the common case of one self-contained file.
/// Construct a `Resolver` directly, and call [`Self::collect_declarations`]
/// once per file before a single closing [`Self::resolve_imports`], when
/// several files (a future statically-resolved `require` chain — M2 item 4,
/// not wired up yet) need to share one symbol table, since a `use` in one
/// file may legally name a declaration from another.
#[derive(Debug, Default)]
pub struct Resolver {
    module: Module,
}

impl Resolver {
    /// A resolver with nothing collected yet.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Walks `stmts`' declarations into the symbol table and records its
    /// `use` imports (not yet checked — call [`Self::resolve_imports`] once
    /// every file has been collected). Recurses into `namespace { ... }`
    /// blocks; a `namespace Name;` statement changes the namespace, and
    /// resets the imported-short-name set, for the rest of this call's
    /// statement sequence, matching PHP's own per-namespace `use` scoping.
    pub fn collect_declarations(
        &mut self,
        stmts: &[Stmt],
        src: &SourceFile,
        diags: &mut Diagnostics,
    ) {
        self.collect_in(stmts, src, &[], diags);
    }

    fn collect_in(
        &mut self,
        stmts: &[Stmt],
        src: &SourceFile,
        namespace: &[String],
        diags: &mut Diagnostics,
    ) {
        let mut current_ns: Vec<String> = namespace.to_vec();
        let mut seen_imports: FxHashMap<String, Span> = FxHashMap::default();

        for stmt in stmts {
            match &stmt.kind {
                StmtKind::NamespaceDecl(NamespaceDecl { name, body, .. }) => {
                    let new_ns = name
                        .as_ref()
                        .map_or_else(Vec::new, |n| qname_segments(src, n));
                    match body {
                        Some(block) => self.collect_in(&block.stmts, src, &new_ns, diags),
                        None => {
                            current_ns = new_ns;
                            seen_imports.clear();
                        }
                    }
                }
                StmtKind::UseDecl(use_decl) => {
                    let target = QName::parse(name_text(src, &use_decl.path));
                    let short_name = target.short_name().to_owned();
                    if let Some(&prev_span) = seen_imports.get(&short_name) {
                        diags.report(
                            Diagnostic::error(
                                code::E_DUPLICATE_DECLARATION,
                                format!("`{short_name}` is already imported"),
                            )
                            .with_primary(use_decl.path.span, "duplicate import")
                            .with_secondary(prev_span, "previously imported here"),
                        );
                    } else {
                        seen_imports.insert(short_name.clone(), use_decl.path.span);
                    }
                    self.module.imports.push(Import {
                        short_name,
                        target,
                        span: use_decl.path.span,
                        resolved: false,
                    });
                }
                StmtKind::ClassDecl(decl) => {
                    self.declare(SymbolKind::Class, &current_ns, &decl.name, src, diags);
                    check_body_aliases(&decl.members, diags);
                }
                StmtKind::InterfaceDecl(decl) => {
                    self.declare(SymbolKind::Interface, &current_ns, &decl.name, src, diags);
                    check_body_aliases(&decl.members, diags);
                }
                StmtKind::EnumDecl(decl) => {
                    self.declare(SymbolKind::Enum, &current_ns, &decl.name, src, diags);
                    check_body_aliases(&decl.members, diags);
                }
                StmtKind::TypeAliasDecl(decl) => {
                    self.declare(SymbolKind::TypeAlias, &current_ns, &decl.name, src, diags);
                    check_alias_is_not_a_bare_class(decl, diags);
                }
                _ => {}
            }
        }
    }

    fn declare(
        &mut self,
        kind: SymbolKind,
        namespace: &[String],
        name: &Name,
        src: &SourceFile,
        diags: &mut Diagnostics,
    ) {
        let qname = QName::join(namespace, name_text(src, name));
        // The compiler declares two rosters in every program — the global
        // interfaces of [`crate::interfaces::RESERVED`] and the exception tree
        // of [`crate::errors::TREE`] — and every resolver in this crate
        // short-circuits on their names before it consults the symbol table
        // (`hierarchy::resolve_supertype`, `Self::resolve_imports`). A source
        // declaration of one is therefore not a shadow but dead text:
        // `implements Parses` binds to the compiler's interface whatever the
        // file in front of the reader says. That is the same collision
        // `rule:core-api/reserved-namespace` refuses under `Core`, so it is
        // refused here in the same words, and the compiler's declaration is
        // the one that stands.
        if qname.is_reserved_global_interface() || qname.is_reserved_global_class() {
            diags.report(
                Diagnostic::error(
                    code::E_DUPLICATE_DECLARATION,
                    format!("`{qname}` is already declared by the compiler"),
                )
                .with_primary(name.span, "duplicate declaration"),
            );
            return;
        }
        let symbol = Symbol {
            kind,
            qname: qname.clone(),
            decl_span: name.span,
        };
        if let Some(existing) = self.module.symbols.declare(symbol) {
            diags.report(
                Diagnostic::error(
                    code::E_DUPLICATE_DECLARATION,
                    format!(
                        "`{qname}` is already declared as {}",
                        existing.kind.describe()
                    ),
                )
                .with_primary(name.span, "duplicate declaration")
                .with_secondary(existing.decl_span, "previously declared here"),
            );
        }
    }

    /// Checks every `use` import collected so far against the symbol table,
    /// reporting `E_UNRESOLVED_IMPORT` for any that names nothing declared
    /// and is not trusted. Call once, after every file
    /// sharing this `Module` has run [`Self::collect_declarations`] — an
    /// import may legally name a declaration that appears later in its own
    /// file, or in a different file entirely.
    pub fn resolve_imports(&mut self, diags: &mut Diagnostics) {
        for import in &mut self.module.imports {
            // The same set every other resolver in this crate trusts — see
            // `hierarchy::resolve_supertype`. Importing a reserved global is
            // not a corner case but the ordinary way a namespaced file reaches
            // the exception tree: `rule:statements/no-fallback-to-the-root-namespace`
            // gives a short name no fallback to the root, so `use Throwable;`
            // is how `catch (Throwable $e)` is written under a `namespace`.
            import.resolved = import.target.is_core()
                || import.target.is_reserved_global_class()
                || import.target.is_reserved_global_interface()
                || self.module.symbols.contains(&import.target);
            if !import.resolved {
                diags.report(
                    Diagnostic::error(
                        code::E_UNRESOLVED_IMPORT,
                        format!("`{}` does not resolve to any declaration", import.target),
                    )
                    .with_primary(import.span, "no matching class, interface or enum declared"),
                );
            }
        }
    }

    /// Consumes the resolver, returning what it collected.
    #[must_use]
    pub fn into_module(self) -> Module {
        self.module
    }

    /// What has been collected so far.
    #[must_use]
    pub const fn module(&self) -> &Module {
        &self.module
    }
}

/// The aliases a body owns are not file-scope names, so nothing declares them
/// in the symbol table — [`crate::aliases`] keys them by their owner instead.
/// What is asked of them here is what is asked of the file-scope form: the
/// declaration itself gets one answer at both sites it is written.
fn check_body_aliases(members: &[ClassMember], diags: &mut Diagnostics) {
    for member in members {
        if let ClassMemberKind::TypeAlias(alias) = &member.kind {
            check_alias_is_not_a_bare_class(alias, diags);
        }
    }
}

/// `rule:types/alias-is-never-a-bare-class`: a `type` alias may not be nothing but one bare class,
/// interface or enum atom — that shape is `use … as …` wearing the type
/// grammar as a disguise. `?SomeClass`, a union, an intersection, and an
/// `array<...>` wrapper are all still fine; only the fully bare atom, with no
/// wrapping construct at all, is refused.
fn check_alias_is_not_a_bare_class(decl: &TypeAliasDecl, diags: &mut Diagnostics) {
    let TypeKind::Atom(atom) = &decl.ty.kind else {
        return;
    };
    if matches!(
        atom,
        TypeAtom::Name(..) | TypeAtom::SelfTy | TypeAtom::StaticTy | TypeAtom::Parent
    ) {
        diags.report(
            Diagnostic::error(
                code::E_TYPE_ALIAS_ALIASES_CLASS,
                "a `type` alias may not name a single class, interface or enum on its own",
            )
            .with_primary(decl.ty.span, "aliases exactly one class-shaped atom")
            .with_help(
                "give a shape a name instead — a union, an intersection, or an `array<...>` \
                 wrapper (`rule:types/alias-is-never-a-bare-class`); a class already has its own name",
            ),
        );
    }
}

pub(crate) fn name_text<'a>(src: &'a SourceFile, name: &Name) -> &'a str {
    src.span_text(name.span).unwrap_or_default()
}

pub(crate) fn qname_segments(src: &SourceFile, name: &Name) -> Vec<String> {
    QName::parse(name_text(src, name)).segments().to_vec()
}

/// Resolves one parsed file's namespace/`use`/declaration structure in one
/// call. See [`Resolver`] for building a [`Module`] out of more than one
/// file.
///
/// **Every name under `Core` is trusted here.** One file resolved on its own
/// has no program around it and no caller holding the stdlib's roster, so this
/// door passes [`crate::hierarchy::CoreRoster::Trusted`];
/// [`crate::requires::resolve_program`] is the door that carries a roster, and
/// so the one that refuses a `Core` name nothing declares.
#[must_use]
pub fn resolve_file(stmts: &[Stmt], src: &SourceFile, diags: &mut Diagnostics) -> Module {
    let mut resolver = Resolver::new();
    resolver.collect_declarations(stmts, src, diags);
    resolver.resolve_imports(diags);
    let mut hierarchy =
        crate::hierarchy::HierarchyResolver::new(crate::hierarchy::CoreRoster::Trusted);
    hierarchy.collect_links(stmts, src);
    let graph = hierarchy.resolve(&resolver.module().symbols, diags);
    let mut module = resolver.into_module();
    module.graph = graph;

    let mut members = crate::members::MemberResolver::new();
    members.collect_members(stmts, src);
    members.check(stmts, src, &module.symbols, &module.graph, false, diags);
    module.members = members.into_table();

    let mut aliases = crate::aliases::AliasResolver::new();
    aliases.collect_aliases(stmts, src);
    module.aliases = aliases.resolve(diags);

    module
}

#[cfg(test)]
mod tests {
    use nvs_diagnostics::SourceMap;
    use nvs_syntax::parse_file;

    use super::*;

    fn resolve(src: &str) -> (Module, Diagnostics) {
        let mut map = SourceMap::new();
        let file = map.add("t.nvs", src);
        let mut diags = Diagnostics::new();
        let stmts = parse_file(map.file(file), &mut diags);
        assert!(!diags.has_errors(), "fixture failed to parse: {diags:?}");
        let module = resolve_file(&stmts, map.file(file), &mut diags);
        (module, diags)
    }

    #[test]
    fn a_top_level_class_is_declared_in_the_global_namespace() {
        let (module, diags) = resolve("<?nvs\nclass Foo {}\n");
        assert!(!diags.has_errors());
        let sym = module.symbols.get(&QName::parse("Foo")).unwrap();
        assert_eq!(sym.kind, SymbolKind::Class);
    }

    #[test]
    fn statement_form_namespace_qualifies_the_rest_of_the_file() {
        let (module, diags) = resolve("<?nvs\nnamespace App\\Models;\nclass User {}\n");
        assert!(!diags.has_errors());
        assert!(module.symbols.contains(&QName::parse("App\\Models\\User")));
    }

    /// The braced form is refused by the parser (`E0243`) and still built, so
    /// this pass gives the node it is handed the reading the block form
    /// implies: the block scopes its own declarations and nothing after it.
    /// The fixture therefore carries that parser diagnostic, which is why it
    /// does not go through [`resolve`].
    #[test]
    fn block_form_namespace_does_not_leak_into_what_follows() {
        let mut map = SourceMap::new();
        let file = map.add("t.nvs", "<?nvs\nnamespace App { class A {} }\nclass B {}\n");
        let mut diags = Diagnostics::new();
        let stmts = parse_file(map.file(file), &mut diags);
        assert_eq!(
            diags.iter().filter_map(|d| d.code).collect::<Vec<_>>(),
            vec![nvs_diagnostics::code::E_BRACED_NAMESPACE_UNSUPPORTED],
            "{diags:?}"
        );
        let module = resolve_file(&stmts, map.file(file), &mut diags);
        assert!(module.symbols.contains(&QName::parse("App\\A")));
        assert!(module.symbols.contains(&QName::parse("B")));
        assert!(!module.symbols.contains(&QName::parse("App\\B")));
    }

    #[test]
    fn duplicate_declarations_in_the_same_namespace_are_diagnosed() {
        let (module, diags) = resolve("<?nvs\nclass Foo {}\nclass Foo {}\n");
        assert!(
            diags
                .iter()
                .any(|d| d.code == Some(code::E_DUPLICATE_DECLARATION))
        );
        assert_eq!(module.symbols.len(), 1, "the first declaration wins");
    }

    /// A program may not declare either compiler-owned roster's names, and
    /// `Parses` is on one of them: the interface a binding site asks a class
    /// for exists in every program, so a file declaring its own would be
    /// writing text nothing could ever reach.
    #[test]
    fn a_program_declaring_its_own_interface_named_parses_is_refused() {
        let (module, diags) = resolve("<?nvs\ninterface Parses {}\n");
        assert!(
            diags
                .iter()
                .any(|d| d.code == Some(code::E_DUPLICATE_DECLARATION)),
            "{diags:?}"
        );
        assert!(!module.symbols.contains(&QName::parse("Parses")));
    }

    /// The other roster, on the same terms — `nvs_hir::errors::TREE`'s classes
    /// are reached by `extends` and `new` without a declaration anywhere.
    #[test]
    fn a_program_declaring_its_own_copy_of_an_exception_class_is_refused() {
        let (_module, diags) = resolve("<?nvs\nclass Throwable {}\n");
        assert!(
            diags
                .iter()
                .any(|d| d.code == Some(code::E_DUPLICATE_DECLARATION)),
            "{diags:?}"
        );
    }

    /// The refusal is on the bare global name and nothing wider: a namespace
    /// of one's own is where a `Parses` of one's own is legal.
    #[test]
    fn a_namespaced_name_matching_a_reserved_one_is_an_ordinary_declaration() {
        let (module, diags) = resolve("<?nvs\nnamespace App;\ninterface Parses {}\n");
        assert!(!diags.has_errors(), "{diags:?}");
        assert!(module.symbols.contains(&QName::parse("App\\Parses")));
    }

    #[test]
    fn the_same_short_name_in_different_namespaces_is_not_a_duplicate() {
        let (module, diags) =
            resolve("<?nvs\nnamespace A;\nclass Foo {}\nnamespace B;\nclass Foo {}\n");
        assert!(!diags.has_errors());
        assert_eq!(module.symbols.len(), 2);
    }

    #[test]
    fn a_use_import_resolves_against_a_declaration_from_earlier_in_the_file() {
        let (module, diags) =
            resolve("<?nvs\nnamespace App;\nclass User {}\nnamespace App\\Http;\nuse App\\User;\n");
        assert!(!diags.has_errors());
        let import = &module.imports[0];
        assert!(import.resolved);
        assert_eq!(import.short_name, "User");
    }

    #[test]
    fn a_use_import_of_an_undeclared_name_is_unresolved() {
        let (module, diags) = resolve("<?nvs\nuse Some\\Missing\\Thing;\n");
        assert!(!module.imports[0].resolved);
        assert!(
            diags
                .iter()
                .any(|d| d.code == Some(code::E_UNRESOLVED_IMPORT))
        );
    }

    #[test]
    fn a_use_import_under_core_is_trusted_without_a_declaration() {
        let (module, diags) = resolve("<?nvs\nuse Core\\Str;\n");
        assert!(!diags.has_errors());
        assert!(module.imports[0].resolved);
    }

    #[test]
    fn two_imports_of_the_same_short_name_in_one_scope_are_diagnosed() {
        let (_module, diags) = resolve("<?nvs\nuse Core\\Foo;\nuse Core\\Other\\Foo;\n");
        assert!(
            diags
                .iter()
                .any(|d| d.code == Some(code::E_DUPLICATE_DECLARATION))
        );
    }

    #[test]
    fn a_new_namespace_statement_resets_the_import_scope() {
        let (_module, diags) =
            resolve("<?nvs\nnamespace A;\nuse Core\\Foo;\nnamespace B;\nuse Core\\Foo;\n");
        assert!(
            !diags.has_errors(),
            "each namespace statement gets a fresh use scope: {diags:?}"
        );
    }

    #[test]
    fn aliasing_a_single_bare_class_is_rejected() {
        let (_module, diags) = resolve("<?nvs\nclass Foo {}\ntype Id = Foo;\n");
        assert!(
            diags
                .iter()
                .any(|d| d.code == Some(code::E_TYPE_ALIAS_ALIASES_CLASS))
        );
    }

    /// The refusal is the declaration's, not the site's, so a member is held to
    /// it exactly as a file-scope alias is — `rule:types/alias-is-never-a-bare-class`
    /// has one form and a body does not get a second.
    #[test]
    fn a_class_scoped_alias_of_a_bare_class_is_refused() {
        let (_module, diags) =
            resolve("<?nvs\nclass Foo {}\nclass Order { type Id = Foo; type Ids = array<Foo>; }\n");
        assert!(
            diags
                .iter()
                .filter(|d| d.code == Some(code::E_TYPE_ALIAS_ALIASES_CLASS))
                .count()
                == 1,
            "the bare member is refused and the wrapped one is not: {diags:?}"
        );
    }

    #[test]
    fn aliasing_a_shape_around_a_class_is_accepted() {
        let (_module, diags) =
            resolve("<?nvs\nclass Foo {}\ntype MaybeFoo = ?Foo;\ntype Ids = array<Foo>;\n");
        assert!(
            !diags
                .iter()
                .any(|d| d.code == Some(code::E_TYPE_ALIAS_ALIASES_CLASS))
        );
    }

    #[test]
    fn aliasing_a_scalar_is_accepted() {
        let (_module, diags) = resolve("<?nvs\ntype UserId = uint;\n");
        assert!(!diags.has_errors());
    }
}
