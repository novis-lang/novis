The AST explorer panel renders `nvs ast --json` for the active file as a tree view — the resilient tree
by default, so the panel works on a file that does not compile. `nvs ast` ships the `--json` flag with a
frozen schema for that purpose; the `{stmts:#?}` debug print has no stability contract and is not what
the panel reads.

It does not use `Core\Ast` (`rule:core-classes/ast-is-inert`). That is the language-level reflective
parse a running Novis program calls; the editor panel is simpler and shells out to the CLI, the same way
`nvs check` backs diagnostics. Both read the one tree there is (`rule:ide/one-grammar-one-tree`), so the
panel and the compiler cannot disagree about a file's shape.
