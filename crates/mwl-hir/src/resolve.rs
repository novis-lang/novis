//! Name resolution: namespaces, `use` imports, and a file's declared symbols.
//!
//! See the crate's module docs for what this covers and what M2 still needs.

use mwl_diagnostics::{Diagnostic, Diagnostics, SourceFile, Span, code};
use mwl_syntax::ast::{Name, NamespaceDecl, Stmt, StmtKind, TypeAliasDecl, TypeAtom, TypeKind};
use rustc_hash::FxHashMap;

use crate::qname::QName;
use crate::symbol::{Symbol, SymbolKind, SymbolTable};

/// One `use Path\To\Name;` import, checked against the declarations collected
/// from every file [`Resolver::collect_declarations`] has seen so far.
///
/// A target under `Core` ([`QName::is_core`]) is always trusted — `mwl-hir`
/// has no declarations for the built-in stdlib to check against yet.
#[derive(Clone, Debug)]
pub struct Import {
    /// The name this import binds — always the target's own last segment,
    /// since [ADR 0015](../../../docs/adr/0015-no-name-aliasing.md) § 2 makes
    /// `use … as …` a diagnostic rather than a second spelling.
    pub short_name: String,
    /// What it resolves to.
    pub target: QName,
    /// Where the imported path was written.
    pub span: Span,
    /// Whether `target` names a real declaration (or is trusted as a `Core`
    /// reference). `false` means `E_UNRESOLVED_IMPORT` was already reported
    /// for it.
    pub resolved: bool,
}

/// Everything `mwl-hir` resolves out of one or more parsed files, for this
/// slice of M2: every declared class/interface/trait/enum/type alias under
/// its fully-qualified name, every `use` import checked against that set, and
/// the class hierarchy (`extends`/`implements`/trait-use) resolved to real
/// symbols.
///
/// Not yet built — see the crate's module docs for the rest of M2's name
/// resolution this will grow into.
#[derive(Debug, Default)]
pub struct Module {
    /// Every class/interface/trait/enum/type-alias declaration collected.
    pub symbols: SymbolTable,
    /// Every `use` import seen, resolved or not.
    pub imports: Vec<Import>,
    /// Every class/interface/trait's resolved `extends`/`implements`/
    /// trait-use links.
    pub graph: crate::hierarchy::ClassGraph,
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
                }
                StmtKind::InterfaceDecl(decl) => {
                    self.declare(SymbolKind::Interface, &current_ns, &decl.name, src, diags);
                }
                StmtKind::TraitDecl(decl) => {
                    self.declare(SymbolKind::Trait, &current_ns, &decl.name, src, diags);
                }
                StmtKind::EnumDecl(decl) => {
                    self.declare(SymbolKind::Enum, &current_ns, &decl.name, src, diags);
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
    /// and is not trusted as a `Core` reference. Call once, after every file
    /// sharing this `Module` has run [`Self::collect_declarations`] — an
    /// import may legally name a declaration that appears later in its own
    /// file, or in a different file entirely.
    pub fn resolve_imports(&mut self, diags: &mut Diagnostics) {
        for import in &mut self.module.imports {
            import.resolved =
                import.target.is_core() || self.module.symbols.contains(&import.target);
            if !import.resolved {
                diags.report(
                    Diagnostic::error(
                        code::E_UNRESOLVED_IMPORT,
                        format!("`{}` does not resolve to any declaration", import.target),
                    )
                    .with_primary(
                        import.span,
                        "no matching class, interface, trait or enum declared",
                    ),
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

/// ADR 0015 § 6: a `type` alias may not be nothing but one bare class,
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
        TypeAtom::Name(_) | TypeAtom::SelfTy | TypeAtom::StaticTy | TypeAtom::Parent
    ) {
        diags.report(
            Diagnostic::error(
                code::E_TYPE_ALIAS_ALIASES_CLASS,
                "a `type` alias may not name a single class, interface or enum on its own",
            )
            .with_primary(decl.ty.span, "aliases exactly one class-shaped atom")
            .with_help(
                "give a shape a name instead — a union, an intersection, or an `array<...>` \
                 wrapper (ADR 0015 § 6); a class already has its own name",
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
#[must_use]
pub fn resolve_file(stmts: &[Stmt], src: &SourceFile, diags: &mut Diagnostics) -> Module {
    let mut resolver = Resolver::new();
    resolver.collect_declarations(stmts, src, diags);
    resolver.resolve_imports(diags);
    let mut hierarchy = crate::hierarchy::HierarchyResolver::new();
    hierarchy.collect_links(stmts, src);
    let graph = hierarchy.resolve(&resolver.module().symbols, diags);
    let mut module = resolver.into_module();
    module.graph = graph;
    module
}

#[cfg(test)]
mod tests {
    use mwl_diagnostics::SourceMap;
    use mwl_syntax::parse_file;

    use super::*;

    fn resolve(src: &str) -> (Module, Diagnostics) {
        let mut map = SourceMap::new();
        let file = map.add("t.mwl", src);
        let mut diags = Diagnostics::new();
        let stmts = parse_file(map.file(file), &mut diags);
        assert!(!diags.has_errors(), "fixture failed to parse: {diags:?}");
        let module = resolve_file(&stmts, map.file(file), &mut diags);
        (module, diags)
    }

    #[test]
    fn a_top_level_class_is_declared_in_the_global_namespace() {
        let (module, diags) = resolve("<?mwl\nclass Foo {}\n");
        assert!(!diags.has_errors());
        let sym = module.symbols.get(&QName::parse("Foo")).unwrap();
        assert_eq!(sym.kind, SymbolKind::Class);
    }

    #[test]
    fn statement_form_namespace_qualifies_the_rest_of_the_file() {
        let (module, diags) = resolve("<?mwl\nnamespace App\\Models;\nclass User {}\n");
        assert!(!diags.has_errors());
        assert!(module.symbols.contains(&QName::parse("App\\Models\\User")));
    }

    #[test]
    fn block_form_namespace_does_not_leak_into_what_follows() {
        let (module, diags) = resolve("<?mwl\nnamespace App { class A {} }\nclass B {}\n");
        assert!(!diags.has_errors());
        assert!(module.symbols.contains(&QName::parse("App\\A")));
        assert!(module.symbols.contains(&QName::parse("B")));
        assert!(!module.symbols.contains(&QName::parse("App\\B")));
    }

    #[test]
    fn duplicate_declarations_in_the_same_namespace_are_diagnosed() {
        let (module, diags) = resolve("<?mwl\nclass Foo {}\nclass Foo {}\n");
        assert!(
            diags
                .iter()
                .any(|d| d.code == Some(code::E_DUPLICATE_DECLARATION))
        );
        assert_eq!(module.symbols.len(), 1, "the first declaration wins");
    }

    #[test]
    fn the_same_short_name_in_different_namespaces_is_not_a_duplicate() {
        let (module, diags) =
            resolve("<?mwl\nnamespace A;\nclass Foo {}\nnamespace B;\nclass Foo {}\n");
        assert!(!diags.has_errors());
        assert_eq!(module.symbols.len(), 2);
    }

    #[test]
    fn a_use_import_resolves_against_a_declaration_from_earlier_in_the_file() {
        let (module, diags) =
            resolve("<?mwl\nnamespace App;\nclass User {}\nnamespace App\\Http;\nuse App\\User;\n");
        assert!(!diags.has_errors());
        let import = &module.imports[0];
        assert!(import.resolved);
        assert_eq!(import.short_name, "User");
    }

    #[test]
    fn a_use_import_of_an_undeclared_name_is_unresolved() {
        let (module, diags) = resolve("<?mwl\nuse Some\\Missing\\Thing;\n");
        assert!(!module.imports[0].resolved);
        assert!(
            diags
                .iter()
                .any(|d| d.code == Some(code::E_UNRESOLVED_IMPORT))
        );
    }

    #[test]
    fn a_use_import_under_core_is_trusted_without_a_declaration() {
        let (module, diags) = resolve("<?mwl\nuse Core\\Str;\n");
        assert!(!diags.has_errors());
        assert!(module.imports[0].resolved);
    }

    #[test]
    fn two_imports_of_the_same_short_name_in_one_scope_are_diagnosed() {
        let (_module, diags) = resolve("<?mwl\nuse Core\\Foo;\nuse Core\\Other\\Foo;\n");
        assert!(
            diags
                .iter()
                .any(|d| d.code == Some(code::E_DUPLICATE_DECLARATION))
        );
    }

    #[test]
    fn a_new_namespace_statement_resets_the_import_scope() {
        let (_module, diags) =
            resolve("<?mwl\nnamespace A;\nuse Core\\Foo;\nnamespace B;\nuse Core\\Foo;\n");
        assert!(
            !diags.has_errors(),
            "each namespace statement gets a fresh use scope: {diags:?}"
        );
    }

    #[test]
    fn aliasing_a_single_bare_class_is_rejected() {
        let (_module, diags) = resolve("<?mwl\nclass Foo {}\ntype Id = Foo;\n");
        assert!(
            diags
                .iter()
                .any(|d| d.code == Some(code::E_TYPE_ALIAS_ALIASES_CLASS))
        );
    }

    #[test]
    fn aliasing_a_shape_around_a_class_is_accepted() {
        let (_module, diags) =
            resolve("<?mwl\nclass Foo {}\ntype MaybeFoo = ?Foo;\ntype Ids = array<Foo>;\n");
        assert!(
            !diags
                .iter()
                .any(|d| d.code == Some(code::E_TYPE_ALIAS_ALIASES_CLASS))
        );
    }

    #[test]
    fn aliasing_a_scalar_is_accepted() {
        let (_module, diags) = resolve("<?mwl\ntype UserId = uint;\n");
        assert!(!diags.has_errors());
    }
}
