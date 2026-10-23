A `Core` class, enum, interface or compiler attribute is declared in a generated stub file, and a jump
to one opens that file on the line of the declaration. The server writes one `.nvs` file per name, at
the name's namespace path, and writes an enum its class owns inside that class's file
(`rule:enums/class-scoped-enum`), under the directory `nvs.stubs.dir` names — or under `lsp/<version>/` in
the data folder when no client named one, so a bare `nvs lsp` answers too — and `nvs stubs --out <dir>`
writes the same tree from the same generator for any other reader. The tree is written once per
render and again when a file is missing; nothing repairs a file in place.

A stub is a declaration with its real signature and an empty body: a class with each method, constant,
constructor and owned enum, an enum with its cases, an attribute as the shape-typed `type` alias a userland
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

A hover on a `Core` name shows the card the stub carries, through the one renderer a completion row
uses too: a class, an enum, a case, a constant and a compiler attribute each show their own. A cursor
on the class half of a member access shows the class, not the member. A cursor on a namespace segment
in front of the type, `Core\Http` in `Core\Http\Method::Get`, shows what that namespace contains: the
classes, enums, interfaces and attributes directly under it, and the namespaces below it. A namespace
has no file to open, so a jump from a segment opens the type at the end of the path. Inside an
attribute's payload a name resolves as it does in a body, so a case written there hovers and jumps the
same way.

Nothing here is on a request path. A hover is a registry lookup. The first jump of a process renders
the whole tree and writes it at most once per version; every later jump is a map lookup and one
stamp read. A stub is rendered from the registry's own prose, so a card that reads badly reads badly
in its stub too, and is fixed in the card. Read-only holds on disk only: an editor may still let the
reader type into a stub, a save then fails with the editor's own message, and the file is written over
at the next stamp change either way.

**Memory:** the rendered tree and its line table, a few hundred kilobytes of text, held for the
server process's life after its first `Core` jump, and one copy on disk per server version.

**Declined:**
- **Virtual documents under a URI scheme of the server's.** Every client would have to learn the
  scheme and register a content provider for it, and every request would need extra code for a cursor
  inside such a document. Real files cost one directory.
- **Parsing a stub to find a member's line.** It is a second reading of text the generator wrote a
  moment before, and one parse per jump. The generator knows every line as it writes it.
- **A `.lspt` stub location written as an absolute path.** An absolute path in a frozen expectation
  is a path no other checkout has. Under the case's own directory it is written like every other
  location in another file.
