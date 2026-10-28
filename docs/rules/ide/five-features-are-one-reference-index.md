`nvs-lsp` builds one workspace symbol index, and five features are each one query against it — none
gets a walk of its own. `textDocument/references` is the index's read side, and it lists each `use` line
that imports the name among its uses; the CodeLens count leaves those lines out, because an import brings
a name into scope and uses nothing. `textDocument/documentHighlight`
is the same query narrowed to the open file, which is why it reads the index rather than the open
document alone: it needs resolution applied to every occurrence, not to one. CodeLens above a declaration shows
the reference count, a type's implementors, and the methods a method overrides and is overridden by; it is
gated on `nvs.codeLens.enable` (default `true`), because a lens is a request per visible declaration and a
large file is where it is least welcome. A click on a lens lists the places it counts. The lens carries
`nvs.showLocations` with the document, its position and those places, and only for a client that names
that id in `capabilities.experimental.commands`; every other client gets a label. The VS Code client
registers the id and opens the editor's reference view. The id is frozen like a contributed command, and
the places cost one file load per file that holds one, per lens request. While an open document does not
parse, its lens rows stay on the lines of the last text that did (`crates/nvs-lsp/src/lens_rows.rs`). A
client draws each line with a lens as a row of its own, so a half-typed declaration that drops or adds a
row would move every line below it on each keystroke. A row moves with the lines added or removed above
it, shows what the current text counts on its line, and shows `…` where it counts nothing. A lens on a
line that is not a row waits until the text parses. This costs one copy of the last text that parsed and
its lenses per open document, freed when the document closes. `textDocument/typeHierarchy` shows supertypes and subtypes,
including `rule:classes/interface-default-methods` and `rule:classes/delegation-by-property`, which is
where a reader most needs to see the shape rather than reconstruct it.

The fifth is **unused-member dimming**: a private member or constant with no reference anywhere in
the index is a diagnostic carrying LSP's `Unnecessary` tag, rendered as dimming rather than a squiggle.
An unused `use` line is not dimmed here: it is the `W1026` warning of
`rule:statements/an-unused-use-is-a-warning`, which carries the same tag.
It is only correct at workspace scope — a symbol unused in the open buffer is not unused — so it is
silent under the default of `rule:ide/check-scope-defaults-to-the-workspace` rather than wrong.

The index is also the **call graph**. Each occurrence records the method whose body it is written in,
so an occurrence that resolved to a method is an edge from that method to it. Beside the occurrences,
each file records two more lists. One holds each `Core\Test::request` whose literal method and path
the route table matched, as an edge to the route's handler. The other holds each call the graph does
not follow: a method taken as a callable value, and a call by name the checker did not resolve. A
method's declaration records the routes it handles and whether it is a `#[Test]` method.
`nvs_lsp::reach` walks these edges backwards for
`rule:ide/used-by-counts-the-routes-and-tests-that-reach-a-method`, and `textDocument/callHierarchy`
reads the same edges rather than a second index (`rule:ide/the-request-set-is-closed`). The graph
costs one method name per occurrence and two short lists per file, for as long as the file is indexed.

One more reader reads the same index and is not a reference feature: `nvs/fileKinds`
(`rule:ide/a-file-shows-what-it-declares`) lists the files that declare one type, from the kind the index
recorded as it read each file. It asks what a file declares rather than where a name is used, and it
builds no walk of its own either.

`workspace/symbol` reads it too: `SymbolIndex::matching` returns every declaration whose own name (a
type's last segment, or a member without a property's `$`) contains the query, upper and lower case
alike. It searches the declarations the index already holds, so it adds no store and no walk.

The same index answers a question asked by name. `nvs_lsp::query` holds the readers that take a
fully-qualified name instead of a cursor, and the editor feature that asks the same question calls the
same function: `textDocument/references` and `textDocument/implementation` read the cursor's name and
call `query`. A caller outside the server, such as `nvs code`, gets a `Workspace` from `server.rs`, which
builds the index for one question and drops it with the answer.

The structure is one constructor, `SymbolIndex::build` in `crates/nvs-lsp/src/index.rs`, and all seven
readers read what it built. The server keeps one warm index; a `.lspt` case and `nvs code` call the
same constructor for one question each. Five features that would each walk the workspace cost one walk between them, and the
warm index is what keeps references and highlight inside `rule:ide/a-full-reanalysis-stays-under-a-bound`.

- **Memory:** the index holds one entry per file the workspace's analyses reached, for the life of the
  server process. It is bounded by the tree `nvs.check.scope` selects and released by the invalidation a
  keystroke triggers. It is spent in the editor process and never on the request path.

**Declined:**
- **An index scoped to one `require` graph.** Four readers would work, but unused-member dimming is only
  correct at workspace scope. It would also make the scope a property of the index rather than of the
  query, so the fifth reader would need a second construction site.
