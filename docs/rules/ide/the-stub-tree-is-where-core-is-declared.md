A `Core` class, enum, interface or compiler attribute is declared in a generated stub file, and a jump
to one opens that file on the line of the declaration. The server writes one `.nvs` file per name, at
the name's namespace path, under the directory `nvs.stubs.dir` names — or under this account's cache
directory when no client named one, so a bare `nvs lsp` answers too — and `nvs stubs --out <dir>`
writes the same tree from the same generator for any other reader. The tree is written once per
render and again when a file is missing; nothing repairs a file in place.

A stub is a declaration with its real signature and an empty body: a class with each method, constant
and constructor, an enum with its cases, an attribute as the shape-typed `type` alias a userland
attribute already is, each under the same reference card hover shows (`rule:core-api/reference-card`),
and a header saying the file is generated and edits are lost. Registry prose is rendered as it is
written. Every file is made read-only on disk, and the server publishes no diagnostics for a document
under the tree: the bodies are empty and do not type-check, the `namespace Core` line is one user
source may not write (`rule:core-api/reserved-namespace`), and nothing in a stub is the reader's to
fix. Hover, definition and semantic tokens inside a stub answer as in any document.

Where a name is in a stub comes from the render's own line table, `(qualified name, member) → line`,
never from parsing a stub. A definition falls back to it only for a name the graph holds no
declaration for, so a program's own class is never answered with a stub. In a `.lspt` case the tree is
written under the case's own directory and a location in it is spelled `stubs/Core/Str.nvs:L:C`, the
way a `--FILE lib/user.nvs--` section's is. Real files rather than virtual documents, because a `file:`
location works unchanged in every client and a cursor inside a stub keeps every answer
(`rule:ide/one-server-two-thin-clients`).
