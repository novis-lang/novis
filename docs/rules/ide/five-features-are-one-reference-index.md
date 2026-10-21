`nvs-lsp` builds one workspace symbol index, and five features are each one query against it — none
gets a walk of its own. `textDocument/references` is the index's read side, and it lists each `use` line
that imports the name among its uses; the CodeLens count leaves those lines out, because an import brings
a name into scope and uses nothing. `textDocument/documentHighlight`
is the same query narrowed to the open file, which is why it reads the index rather than the open
document alone: it needs resolution applied to every occurrence, not to one. CodeLens above a declaration shows
the reference count, a type's implementors, and the methods a method overrides and is overridden by; it is
gated on `nvs.codeLens.enable` (default `true`), because a lens is a request per visible declaration and a
large file is where it is least welcome. `textDocument/typeHierarchy` shows supertypes and subtypes,
including `rule:classes/interface-default-methods` and `rule:classes/delegation-by-property`, which is
where a reader most needs to see the shape rather than reconstruct it.

The fifth is **unused-member dimming**: a private member, constant or `use` with no reference anywhere in
the index is a diagnostic carrying LSP's `Unnecessary` tag, rendered as dimming rather than a squiggle.
It is only correct at workspace scope — a symbol unused in the open buffer is not unused — so it is
silent under the default of `rule:ide/check-scope-defaults-to-the-workspace` rather than wrong.

Call hierarchy is not in this list yet. `textDocument/callHierarchy` reads call-site edges, and an
occurrence does not record the function it is written in, so the index has none.
`rule:ide/used-by-counts-the-routes-and-tests-that-reach-a-method` adds that record, and call hierarchy
then reads the same index rather than a second one (`rule:ide/the-request-set-is-closed`).

One more reader reads the same index and is not a reference feature: `nvs/fileKinds`
(`rule:ide/a-file-shows-what-it-declares`) lists the files that declare one type, from the kind the index
recorded as it read each file. It asks what a file declares rather than where a name is used, and it
builds no walk of its own either.

The same index answers a question asked by name. `nvs_lsp::query` holds the readers that take a
fully-qualified name instead of a cursor, and the editor feature that asks the same question calls the
same function: `textDocument/references` and `textDocument/implementation` read the cursor's name and
call `query`. A caller outside the server, such as `nvs code`, gets a `Workspace` from `server.rs`, which
builds the index for one question and drops it with the answer.

The structure is one constructor, `SymbolIndex::build` in `crates/nvs-lsp/src/index.rs`, and all six
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
