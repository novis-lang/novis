- **A `surfaces` suite assertion runs over the client's source as text, so a comment can fail it.**
  Those tests assert what a module does *not* do — `tasks.ts` spawns no `child_process`, the AST
  panel never passes `--strict` — by matching the file, and the module doc explaining why it does
  not do that names the very string the assertion refuses. Strip the comments before matching, the
  way `editors/vscode/test/surfaces/ast.test.ts` does, or keep the prose off that spelling.
  [until: gone editors/vscode/test/surfaces/tasks.test.ts:child_process]
