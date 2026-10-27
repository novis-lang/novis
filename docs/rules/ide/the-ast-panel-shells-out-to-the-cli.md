The AST explorer panel renders `nvs ast --json` for the active file as a tree view — the resilient tree
by default, so the panel works on a file that does not compile. `nvs ast` ships the `--json` flag with a
frozen schema for that purpose; the `{stmts:#?}` debug print has no stability contract and is not what
the panel reads.

The panel is off by default. The view and the command's palette entry are shown under `nvs.ast.enable`
(default `false`), because few users read a syntax tree and a view nobody opens should not take space
in every Explorer. The panel runs `nvs ast` only when **Novis: Show AST** is run, and again when the
file it shows is saved. Turning the setting off drops the tree it holds.

It does not use `Core\Ast` (`rule:core-classes/ast-is-inert`). That is the language-level reflective
parse a running Novis program calls; the editor panel is simpler and shells out to the CLI, the same way
`nvs check` backs diagnostics. Both read the one tree there is (`rule:ide/one-grammar-one-tree`), so the
panel and the compiler cannot disagree about a file's shape.
