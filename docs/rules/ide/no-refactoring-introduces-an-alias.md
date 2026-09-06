The refactorings are LSP requests — workspace-wide rename, extract to method or variable, and
organize-imports — and the deeper completion is signature help, cross-file symbol search, auto-import,
and inlay hints for `var`-inferred types (`rule:types/var-inference`) and call-site parameter names.

None of them writes an alias. Organize-imports is restricted to reordering and removing unused `use`
statements; auto-import inserts the correct fully-qualified name and nothing else. An editor that
resolved a clash with `use Foo as Bar` would be inventing a spelling the language does not have
(`rule:statements/nothing-gets-a-second-name`), and a second name introduced by tooling is exactly as
much a second name as one typed by hand.
