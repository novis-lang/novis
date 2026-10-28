//! Name resolution: namespaces, `use` imports, and a file's declared symbols.
//!
//! See the crate's module docs for where this sits beside the other resolvers.

use nvs_diagnostics::{Diagnostic, Diagnostics, SourceFile, Span, code};
use nvs_syntax::ast::{ClassMember, ClassMemberKind, Name, NamespaceDecl, Stmt, StmtKind};
use rustc_hash::FxHashMap;

use crate::qname::QName;
use crate::symbol::{ExtensionClass, Symbol, SymbolKind, SymbolTable};

/// One `use Path\To\Name;` import, checked against the declarations collected
/// from every file [`Resolver::collect_declarations`] has seen so far.
///
/// A target under `Core` ([`QName::is_core`]) is always trusted — `nvs-hir`
/// holds no declarations for the built-in stdlib to check it against.
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
    /// already reported for it, and [`SymbolTable::is_unresolved_import`]
    /// answers `true` for `target`, so no use of the short name reports it again
    /// (`rule:types/one-mistake-reports-once`).
    pub resolved: bool,
}

/// Everything `nvs-hir` resolves out of one or more parsed files: every
/// declared class/interface/enum/type alias under its fully-qualified name,
/// every `use` import checked against that set, the class hierarchy
/// (`extends`/`implements`) resolved to real symbols, each declaration's own
/// members and each alias's expansion.
#[derive(Debug, Default)]
pub struct Module {
    /// Every class/interface/enum/type-alias declaration collected.
    pub symbols: SymbolTable,
    /// Every `use` import seen, resolved or not.
    pub imports: Vec<Import>,
    /// Every class/interface's resolved `extends`/`implements` links.
    pub graph: crate::hierarchy::ClassGraph,
    /// Every class/interface/enum's own directly-declared members
    /// (see [`crate::members`]).
    pub members: crate::members::MemberTable,
    /// Every `type` alias's fully-substituted expansion (see
    /// [`crate::aliases`]).
    pub aliases: crate::aliases::AliasTable,
}

/// Resolves parsed files' top-level namespace/`use`/declaration structure
/// into a shared [`Module`].
///
/// Call [`resolve_file`] for the common case of one self-contained file.
/// Construct a `Resolver` directly, and call [`Self::collect_declarations`]
/// once per file before a single closing [`Self::resolve_imports`], when
/// several files (a statically-resolved `require` chain, which
/// [`crate::requires::resolve_program`] walks) need to share one symbol table,
/// since a `use` in one file may legally name a declaration from another.
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

    /// Declares the classes of the loaded extension set. Called before the
    /// first [`Self::collect_declarations`], so a source file declaring one
    /// of these names is the declaration refused.
    pub fn declare_extensions(&mut self, classes: &[ExtensionClass]) {
        for class in classes {
            self.module
                .symbols
                .declare_extension(&class.name, class.component.as_deref());
        }
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
                    self.declare_owned_enums(&current_ns, &decl.name, &decl.members, src, diags);
                }
                StmtKind::InterfaceDecl(decl) => {
                    self.declare(SymbolKind::Interface, &current_ns, &decl.name, src, diags);
                    self.declare_owned_enums(&current_ns, &decl.name, &decl.members, src, diags);
                }
                StmtKind::EnumDecl(decl) => {
                    self.declare(SymbolKind::Enum, &current_ns, &decl.name, src, diags);
                }
                StmtKind::TypeAliasDecl(decl) => {
                    self.declare(SymbolKind::TypeAlias, &current_ns, &decl.name, src, diags);
                }
                _ => {}
            }
        }
    }

    /// Every enum a class or interface body declares, under the key
    /// [`QName::member`] builds (`rule:enums/class-scoped-enum`). The key is no
    /// namespace path, so no written name resolves to it: the owner's name is
    /// the only way in, as `Owner::Name` or a bare `Name` inside the owner.
    fn declare_owned_enums(
        &mut self,
        namespace: &[String],
        owner: &Name,
        members: &[ClassMember],
        src: &SourceFile,
        diags: &mut Diagnostics,
    ) {
        let owner = QName::join(namespace, name_text(src, owner));
        for member in members {
            if let ClassMemberKind::Enum(decl) = &member.kind {
                let qname = QName::member(&owner, name_text(src, &decl.name));
                self.declare_qname(SymbolKind::Enum, qname, &decl.name, diags);
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
        self.declare_qname(kind, qname, name, diags);
    }

    fn declare_qname(
        &mut self,
        kind: SymbolKind,
        qname: QName,
        name: &Name,
        diags: &mut Diagnostics,
    ) {
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
        if self.module.symbols.is_extension(&qname) {
            diags.report(
                Diagnostic::error(
                    code::E_DUPLICATE_DECLARATION,
                    format!("`{qname}` is already declared by a loaded extension"),
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
    /// and a `Core` one against `core`'s importable names, reporting
    /// `E_UNRESOLVED_IMPORT` for any that names nothing. Call once, after
    /// every file sharing this `Module` has run [`Self::collect_declarations`]
    /// — an import may legally name a declaration that appears later in its
    /// own file, or in a different file entirely.
    pub fn resolve_imports(
        &mut self,
        core: crate::hierarchy::CoreRoster<'_>,
        diags: &mut Diagnostics,
    ) {
        for import in &mut self.module.imports {
            // The same set every other resolver in this crate trusts — see
            // `hierarchy::resolve_supertype`. Importing a reserved global is
            // not a corner case but the ordinary way a namespaced file reaches
            // the exception tree: `rule:statements/no-fallback-to-the-root-namespace`
            // gives a short name no fallback to the root, so `use Throwable;`
            // is how `catch (Throwable $e)` is written under a `namespace`.
            // A loaded extension's class is in the symbol table, under `Core`
            // or not.
            import.resolved = import.target.is_reserved_global_class()
                || import.target.is_reserved_global_interface()
                || self.module.symbols.contains(&import.target)
                || import.target.is_core() && core.imports(&import.target);
            if !import.resolved {
                diags.report(
                    Diagnostic::error(
                        code::E_UNRESOLVED_IMPORT,
                        format!("`{}`{DOES_NOT_RESOLVE}", import.target),
                    )
                    .with_primary(import.span, "no matching class, interface or enum declared"),
                );
                self.module.symbols.note_unresolved_import(&import.target);
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

/// The end of an `E_UNRESOLVED_IMPORT` headline, after the quoted target.
const DOES_NOT_RESOLVE: &str = " does not resolve to any declaration";

/// The target an `E_UNRESOLVED_IMPORT` diagnostic names, read back off its
/// headline, and `None` for any other diagnostic. The counterpart of
/// [`crate::hierarchy::undeclared_subject`], written beside the headline so the
/// two cannot drift apart.
#[must_use]
pub fn unresolved_import_subject(diagnostic: &Diagnostic) -> Option<QName> {
    if diagnostic.code != Some(code::E_UNRESOLVED_IMPORT) {
        return None;
    }
    let name = diagnostic
        .message
        .strip_prefix('`')?
        .strip_suffix(DOES_NOT_RESOLVE)?
        .strip_suffix('`')?;
    (!name.is_empty() && !name.contains('`')).then(|| QName::parse(name))
}

/// The key of the enum `owner`'s body declares under `name`, if it declares one
/// (`rule:enums/class-scoped-enum`). `Owner::Name` from anywhere reads this. A
/// loaded extension's manifest declares its owned enums under the same key, and
/// the only extension class a key holding `::` can name is one of those.
#[must_use]
pub fn owned_enum(owner: &QName, name: &str, symbols: &SymbolTable) -> Option<QName> {
    let key = QName::member(owner, name);
    let declared = matches!(symbols.get(&key), Some(symbol) if symbol.kind == SymbolKind::Enum);
    (declared || symbols.is_extension(&key)).then_some(key)
}

/// The enum a bare `text` names inside the body of `current_class`: the
/// owner's own enum comes before a name the namespace and imports resolve,
/// the order a bare alias name is read in (`rule:enums/class-scoped-enum`).
/// A qualified `text` is never an owned enum.
#[must_use]
pub fn owned_enum_in_scope(
    text: &str,
    current_class: Option<&QName>,
    symbols: &SymbolTable,
) -> Option<QName> {
    let owner = current_class?;
    if text.contains('\\') {
        return None;
    }
    owned_enum(owner, text, symbols)
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
    resolve_file_with_extensions(stmts, src, &[], diags)
}

/// [`resolve_file`] with the classes of a loaded extension set declared ahead
/// of the file, so a reference to one resolves like a reference to a class
/// the file declares.
#[must_use]
pub fn resolve_file_with_extensions(
    stmts: &[Stmt],
    src: &SourceFile,
    extensions: &[ExtensionClass],
    diags: &mut Diagnostics,
) -> Module {
    let mut resolver = Resolver::new();
    resolver.declare_extensions(extensions);
    resolver.collect_declarations(stmts, src, diags);
    resolver.resolve_imports(crate::hierarchy::CoreRoster::Trusted, diags);
    let mut hierarchy =
        crate::hierarchy::HierarchyResolver::new(crate::hierarchy::CoreRoster::Trusted);
    hierarchy.collect_links(stmts, src);
    let graph = hierarchy.resolve(&resolver.module().symbols, diags);
    let mut module = resolver.into_module();
    module.graph = graph;

    let mut members = crate::members::MemberResolver::new();
    for class in extensions {
        members.declare_extension(class);
    }
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

    /// The declarations and imports of several files, one namespace each.
    fn resolve_files(srcs: &[&str]) -> (Module, Diagnostics) {
        let mut map = SourceMap::new();
        let mut diags = Diagnostics::new();
        let mut resolver = Resolver::new();
        for (i, src) in srcs.iter().enumerate() {
            let file = map.add(format!("t{i}.nvs"), *src);
            let stmts = parse_file(map.file(file), &mut diags);
            assert!(!diags.has_errors(), "fixture failed to parse: {diags:?}");
            resolver.collect_declarations(&stmts, map.file(file), &mut diags);
        }
        resolver.resolve_imports(crate::hierarchy::CoreRoster::Trusted, &mut diags);
        (resolver.into_module(), diags)
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
        let (module, diags) = resolve_files(&[
            "<?nvs\nnamespace A;\nclass Foo {}\n",
            "<?nvs\nnamespace B;\nclass Foo {}\n",
        ]);
        assert!(!diags.has_errors());
        assert_eq!(module.symbols.len(), 2);
    }

    #[test]
    fn a_use_import_resolves_against_a_declaration_from_another_file() {
        let (module, diags) = resolve_files(&[
            "<?nvs\nnamespace App;\nclass User {}\n",
            "<?nvs\nnamespace App\\Http;\nuse App\\User;\n",
        ]);
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
    fn each_file_has_its_own_import_scope() {
        let (_module, diags) = resolve_files(&[
            "<?nvs\nnamespace A;\nuse Core\\Foo;\n",
            "<?nvs\nnamespace B;\nuse Core\\Foo;\n",
        ]);
        assert!(
            !diags.has_errors(),
            "each file gets a fresh use scope: {diags:?}"
        );
    }

    #[test]
    fn aliasing_a_scalar_is_accepted() {
        let (_module, diags) = resolve("<?nvs\ntype UserId = uint;\n");
        assert!(!diags.has_errors());
    }
}
