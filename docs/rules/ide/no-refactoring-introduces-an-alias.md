The refactorings are LSP requests — workspace-wide rename, extract to method or variable, and
organize-imports — and the deeper completion is signature help, cross-file symbol search, auto-import,
and inlay hints for `var`-inferred types (`rule:types/var-inference`) and call-site parameter names.

None of them writes an alias. Organize-imports is restricted to reordering and removing unused `use`
statements; auto-import inserts the correct fully-qualified name and nothing else. An editor that
resolved a clash with `use Foo as Bar` would be inventing a spelling the language does not have
(`rule:statements/nothing-gets-a-second-name`), and a second name introduced by tooling is exactly as
much a second name as one typed by hand.

**Built:** signature help, inlay hints, and the three places the server writes a `use` line: an
accepted completion, a pasted type (`rule:ide/a-pasted-type-carries-its-use-line`) and an undeclared
name's fix. Each writes the fully-qualified name at `nvs_hir::import_site` and never an alias
(`crates/nvs-lsp/src/imports.rs`). Rename of a member or a local, and the `require` paths a moved file
changes (`crates/nvs-lsp/src/rename.rs`): a method is renamed with every method that overrides it or
that it overrides, and a new name that some class of that family, above it or below it, already
declares is refused with a message. A local's new name may not be bound already in its body. Neither
is ever resolved with an alias. Rename of a class, an interface, an enum or a type alias declared
outside a class: the index records each type annotation and each class written in front of `::`, and
the rename changes the last segment at every site and in every `use` line. A new name that another
type in the workspace has, or that a file with a site already writes, is refused. A type declared
alone in a file named after it renames that file with it, so an `autoload` still finds it.
**Missing:** extract to method or variable; organize-imports; and cross-file symbol search.
