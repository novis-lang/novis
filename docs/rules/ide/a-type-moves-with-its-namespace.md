A type's rename to a qualified name moves it to that namespace: `Shop\Billing\Cart` or
`\Shop\Billing\Cart` as the new name of `Shop\Cart`. A bare new name keeps today's meaning and changes
the last segment only, and a qualified name in the type's own namespace is that same rename. The
editor's rename box takes the same text, and `prepareRename`'s range stays the last segment.
`nvs code rename` and the editor reach it through one function, `nvs_lsp::rename::rename`, and its
edit is checked before it is written (`rule:ide/a-refactoring-checks-its-edit-before-it-writes`).

- **Only a type that is its file's sole declaration moves.** An autoloaded file holds one declaration
  under its map path (`rule:programs/one-declaration-per-autoloaded-file`), so a type that shares its
  file is refused: move it to its own file first.
- **The file moves to the path the `autoload` map gives the new name,** the first root of its longest
  prefix (`nvs_hir::AutoloadMap::resolve`). Every program that reads the file is analysed once, and
  each whose map claims the file must give one path. Programs whose maps give different paths refuse
  the move and are named. A program whose map reaches no root for the new name refuses it, since that
  program would lose the type. A file only a `require` reaches keeps its path, and only its names
  change.
- **What is rewritten.** The `namespace` line of the moved file. Each `use` line of the type names
  the new name, and one in a file of the new namespace is deleted. A name with a `\` always starts at
  the root, so each qualified site, `@see` target and `@throws` name is the new full name. A file
  that wrote the bare name through the old namespace gets a `use` line, at `nvs_hir::import_site`.
  Inside the moved file, each type of the old namespace it wrote bare gets a `use` line, and each
  `use` line that names the new namespace is deleted.
- **Every `use` block the move touches is sorted with `nvs fmt`'s sort,** so a format after it moves
  nothing. A deleted line alone between two blank lines takes one of them with it, and lines added
  after a deleted line take its place.
- **A clash is refused, never aliased** (`rule:ide/no-refactoring-introduces-an-alias`): a full name
  that another type has, a built-in type as the short name, and a new short name a file with a site
  already writes. A place that writes the old short name where the index holds no site refuses it, as
  a type's rename does.
- **Directories.** `nvs code` creates the missing directories of the new path, each judged inside the
  project root before it is created. A directory the move empties stays. The editor's client creates
  them through the `WorkspaceEdit`'s rename. A namespace move from or to the global namespace is
  refused, since no `autoload` root reaches a type there.

**Memory:** nothing at runtime, since only source is edited. Per rename, one analysis of each program
that reads the moved file, to read its map, on top of the check's two, dropped with the answer.

**Declined:**
- **Keeping the file where it is.** Under an autoload root the path is the name, so the program would
  lose the type.
- **Writing every site as a full name instead of adding `use` lines.** A bare name stays bare, so the
  diff is the namespace, and the file reads the way it was written.
